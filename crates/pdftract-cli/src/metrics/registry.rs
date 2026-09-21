//! The pdftract metrics registry: the 13 metrics of the plan's
//! "Monitoring and Alerting" surface, plus their OpenMetrics rendering.
//!
//! A [`Registry`] is a cheap cloneable handle (`Arc` + interior
//! synchronisation); every clone observes the same counters and gauges,
//! so `serve` and `mcp` handlers can increment concurrently without a
//! shared mutable borrow. There is deliberately no global registry —
//! the future listener child wires one instance through its handlers.
//!
//! Metric names, types, labels, and HELP text follow
//! `docs/plan/plan.md` ("Monitoring and Alerting") exactly. Counters are
//! declared here without their OpenMetrics `_total` sample suffix; the
//! formatter adds the suffix where the format requires it.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::openmetrics::{format_float, MetricFamily, MetricKind, Sample, SampleValue};

/// An atomically shared counter cell.
pub type CounterHandle = Arc<AtomicU64>;

/// Ordered label set in declared label order; the sort key for children.
type Labels = Vec<(&'static str, String)>;

/// Upper bucket bounds (seconds) for `pdftract_extraction_duration_seconds`.
pub const EXTRACTION_DURATION_BUCKETS: &[f64] =
    &[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0];

/// Lock a mutex even if a previous holder panicked; metrics must not
/// poison the serving path.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A counter family keyed by exact label sets.
///
/// Children are stored in a `BTreeMap` so rendering order is
/// deterministic; each child is an `Arc<AtomicU64>` that can be
/// incremented without holding the map lock again.
struct CounterVec {
    /// Family name without the `_total` sample suffix.
    family: &'static str,
    help: &'static str,
    label_names: &'static [&'static str],
    children: Mutex<BTreeMap<Labels, CounterHandle>>,
}

impl CounterVec {
    fn new(family: &'static str, help: &'static str, label_names: &'static [&'static str]) -> Self {
        let mut children = BTreeMap::new();
        if label_names.is_empty() {
            // Unlabeled families expose their single zero-valued child
            // from process start, so the time series exists before the
            // first increment (the shipped alert rules divide
            // pdftract_cache_hits_total by hits + misses).
            children.insert(Vec::new(), Arc::new(AtomicU64::new(0)));
        }
        Self {
            family,
            help,
            label_names,
            children: Mutex::new(children),
        }
    }

    /// The counter cell for an exact label set, creating it on first use.
    ///
    /// `values` must match [`Self::label_names`] in order and length.
    fn handle(&self, values: &[&str]) -> CounterHandle {
        debug_assert_eq!(
            values.len(),
            self.label_names.len(),
            "label arity mismatch for {}",
            self.family
        );
        let labels: Labels = self
            .label_names
            .iter()
            .copied()
            .zip(values.iter().copied().map(str::to_string))
            .collect();
        lock(&self.children)
            .entry(labels)
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone()
    }

    fn increment(&self, values: &[&str]) {
        self.handle(values).fetch_add(1, Ordering::Relaxed);
    }

    fn add(&self, values: &[&str], amount: u64) {
        self.handle(values).fetch_add(amount, Ordering::Relaxed);
    }

    fn snapshot(&self) -> MetricFamily {
        let children = lock(&self.children);
        let mut family = MetricFamily::new(self.family, self.help, MetricKind::Counter);
        family.samples.reserve(children.len());
        for (labels, counter) in children.iter() {
            family.samples.push(Sample::labeled(
                format!("{}_total", self.family),
                labels.clone(),
                SampleValue::Int(counter.load(Ordering::Relaxed)),
            ));
        }
        family
    }
}

/// `f64` stored in an `AtomicU64` via bit patterns (no MSRV dependency
/// on a native atomic float).
struct AtomicF64(AtomicU64);

impl AtomicF64 {
    fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }

    fn load(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::Relaxed))
    }

    fn store(&self, value: f64) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }

    fn fetch_add(&self, delta: f64) {
        let mut prev = self.0.load(Ordering::Relaxed);
        loop {
            let next = (f64::from_bits(prev) + delta).to_bits();
            match self
                .0
                .compare_exchange_weak(prev, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return,
                Err(seen) => prev = seen,
            }
        }
    }
}

/// Cumulative-bucket histogram (`pdftract_extraction_duration_seconds`).
struct Histogram {
    family: &'static str,
    help: &'static str,
    upper_bounds: &'static [f64],
    /// Per-bucket observation counts; cumulated at snapshot time.
    bucket_counts: Vec<AtomicU64>,
    count: AtomicU64,
    sum: AtomicF64,
}

impl Histogram {
    fn new(family: &'static str, help: &'static str, upper_bounds: &'static [f64]) -> Self {
        Self {
            family,
            help,
            upper_bounds,
            bucket_counts: upper_bounds.iter().map(|_| AtomicU64::new(0)).collect(),
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        }
    }

    /// Record one observation. Non-finite values are ignored; they have
    /// no meaningful bucket or contribution to the sum.
    fn observe(&self, value: f64) {
        if !value.is_finite() {
            return;
        }
        let index = self
            .upper_bounds
            .iter()
            .position(|&bound| value <= bound)
            .unwrap_or(self.upper_bounds.len());
        self.bucket_counts[index].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value);
    }

    fn snapshot(&self) -> MetricFamily {
        let total = self.count.load(Ordering::Relaxed);
        let mut family = MetricFamily::new(self.family, self.help, MetricKind::Histogram);
        family.samples.reserve(self.upper_bounds.len() + 3);
        let mut cumulative = 0u64;
        for (index, &bound) in self.upper_bounds.iter().enumerate() {
            cumulative += self.bucket_counts[index].load(Ordering::Relaxed);
            family.samples.push(Sample::labeled(
                format!("{}_bucket", self.family),
                vec![("le", format_float(bound))],
                SampleValue::Int(cumulative),
            ));
        }
        // The +Inf bucket is mandatory and equals the observation count.
        family.samples.push(Sample::labeled(
            format!("{}_bucket", self.family),
            vec![("le", "+Inf".to_string())],
            SampleValue::Int(total),
        ));
        family.samples.push(Sample::plain(
            format!("{}_sum", self.family),
            SampleValue::Float(self.sum.load()),
        ));
        family.samples.push(Sample::plain(
            format!("{}_count", self.family),
            SampleValue::Int(total),
        ));
        family
    }
}

/// A gauge family with a single constant-1 labeled sample
/// (`pdftract_build_info`).
struct BuildInfo {
    labels: Mutex<Labels>,
}

impl BuildInfo {
    fn new(version: &str, git_sha: &str, features: &str) -> Self {
        Self {
            labels: Mutex::new(vec![
                ("version", version.to_string()),
                ("git_sha", git_sha.to_string()),
                ("features", features.to_string()),
            ]),
        }
    }

    fn snapshot(&self) -> MetricFamily {
        let labels = lock(&self.labels).clone();
        let mut family = MetricFamily::new(
            "pdftract_build_info",
            "Build identification for the info join",
            MetricKind::Gauge,
        );
        family.samples.push(Sample::labeled(
            "pdftract_build_info",
            labels,
            SampleValue::Int(1),
        ));
        family
    }
}

/// Cloneable, thread-safe handle over pdftract's 13 Prometheus metrics.
///
/// Clones share one underlying state (`Arc`), so a `serve` handler and
/// an `mcp` handler each holding a clone observe the same counters.
///
/// # Examples
///
/// ```
/// use pdftract_cli::metrics::Registry;
///
/// let registry = Registry::new();
/// registry.inc_extraction("success", false);
/// registry.inc_cache_hit();
/// let text = registry.render();
/// assert!(text.contains("pdftract_extractions_total{result=\"success\",ocr=\"false\"} 1\n"));
/// assert!(text.ends_with("# EOF\n"));
/// ```
#[derive(Clone)]
pub struct Registry {
    extractions: Arc<CounterVec>,
    extraction_duration: Arc<Histogram>,
    pages_extracted: Arc<CounterVec>,
    cache_hits: Arc<CounterVec>,
    cache_misses: Arc<CounterVec>,
    cache_size_bytes: Arc<AtomicU64>,
    mcp_requests: Arc<CounterVec>,
    http_requests: Arc<CounterVec>,
    remote_bytes_downloaded: Arc<CounterVec>,
    diagnostics: Arc<CounterVec>,
    inflight_extractions: Arc<AtomicU64>,
    rayon_pool_utilization: Arc<AtomicF64>,
    build_info: Arc<BuildInfo>,
}

impl Registry {
    /// Create a registry with all 13 metric families at their zero values.
    ///
    /// `pdftract_build_info` is populated from the build script's
    /// `GIT_SHA` and `COMPILED_FEATURES` environment plus the crate
    /// version.
    pub fn new() -> Self {
        Self::with_build_info(
            env!("CARGO_PKG_VERSION"),
            env!("GIT_SHA"),
            env!("COMPILED_FEATURES"),
        )
    }

    /// Create a registry with explicit `pdftract_build_info` label values.
    pub fn with_build_info(version: &str, git_sha: &str, features: &str) -> Self {
        Self {
            extractions: Arc::new(CounterVec::new(
                "pdftract_extractions",
                "Extractions started, partitioned by outcome and OCR path",
                &["result", "ocr"],
            )),
            extraction_duration: Arc::new(Histogram::new(
                "pdftract_extraction_duration_seconds",
                "Wall-clock extraction time per request; buckets: [0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60]",
                EXTRACTION_DURATION_BUCKETS,
            )),
            pages_extracted: Arc::new(CounterVec::new(
                "pdftract_pages_extracted",
                "Pages emitted (sum across requests)",
                &[],
            )),
            cache_hits: Arc::new(CounterVec::new(
                "pdftract_cache_hits",
                "Cache hits (Phase 6.9)",
                &[],
            )),
            cache_misses: Arc::new(CounterVec::new(
                "pdftract_cache_misses",
                "Cache misses",
                &[],
            )),
            cache_size_bytes: Arc::new(AtomicU64::new(0)),
            mcp_requests: Arc::new(CounterVec::new(
                "pdftract_mcp_requests",
                "MCP tool invocations",
                &["tool"],
            )),
            http_requests: Arc::new(CounterVec::new(
                "pdftract_http_requests",
                "HTTP requests by endpoint and status code",
                &["endpoint", "status"],
            )),
            remote_bytes_downloaded: Arc::new(CounterVec::new(
                "pdftract_remote_bytes_downloaded",
                "HTTP range-read traffic from remote adapter (Phase 1.8)",
                &[],
            )),
            diagnostics: Arc::new(CounterVec::new(
                "pdftract_diagnostic_emitted",
                "Diagnostics emitted, partitioned by code",
                &["code", "severity"],
            )),
            inflight_extractions: Arc::new(AtomicU64::new(0)),
            rayon_pool_utilization: Arc::new(AtomicF64::new(0.0)),
            build_info: Arc::new(BuildInfo::new(version, git_sha, features)),
        }
    }

    /// Increment `pdftract_extractions_total` for one extraction start.
    ///
    /// `ocr` renders as the `ocr="true"|"false"` label.
    pub fn inc_extraction(&self, result: &str, ocr: bool) {
        let ocr = if ocr { "true" } else { "false" };
        self.extractions.increment(&[result, ocr]);
    }

    /// Observe one extraction's wall-clock duration (seconds).
    ///
    /// Non-finite observations are ignored.
    pub fn observe_extraction_duration(&self, seconds: f64) {
        self.extraction_duration.observe(seconds);
    }

    /// Add emitted pages to `pdftract_pages_extracted_total`.
    pub fn add_pages_extracted(&self, pages: u64) {
        self.pages_extracted.add(&[], pages);
    }

    /// Increment `pdftract_cache_hits_total`.
    pub fn inc_cache_hit(&self) {
        self.cache_hits.increment(&[]);
    }

    /// Increment `pdftract_cache_misses_total`.
    pub fn inc_cache_miss(&self) {
        self.cache_misses.increment(&[]);
    }

    /// Set `pdftract_cache_size_bytes` to the current on-disk cache size.
    pub fn set_cache_size_bytes(&self, bytes: u64) {
        self.cache_size_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Increment `pdftract_mcp_requests_total` for one tool invocation.
    pub fn inc_mcp_request(&self, tool: &str) {
        self.mcp_requests.increment(&[tool]);
    }

    /// Increment `pdftract_http_requests_total`.
    ///
    /// `endpoint` must be a registered route template, never a raw
    /// request path (cardinality policy of the plan).
    pub fn inc_http_request(&self, endpoint: &str, status: u16) {
        self.http_requests
            .increment(&[endpoint, &status.to_string()]);
    }

    /// Add downloaded bytes to `pdftract_remote_bytes_downloaded_total`.
    pub fn add_remote_bytes_downloaded(&self, bytes: u64) {
        self.remote_bytes_downloaded.add(&[], bytes);
    }

    /// Increment `pdftract_diagnostic_emitted_total`.
    pub fn inc_diagnostic(&self, code: &str, severity: &str) {
        self.diagnostics.increment(&[code, severity]);
    }

    /// Increment `pdftract_inflight_extractions`.
    pub fn inc_inflight_extractions(&self) {
        self.inflight_extractions.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement `pdftract_inflight_extractions`.
    ///
    /// Saturates at zero so a stray decrement cannot render a negative
    /// gauge.
    pub fn dec_inflight_extractions(&self) {
        let gauge = &*self.inflight_extractions;
        let mut current = gauge.load(Ordering::Relaxed);
        while current > 0 {
            match gauge.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(seen) => current = seen,
            }
        }
    }

    /// Set `pdftract_rayon_pool_utilization`.
    ///
    /// The value is clamped to the plan's 0..1 range; NaN becomes 0.0.
    pub fn set_rayon_pool_utilization(&self, fraction: f64) {
        let clamped = if fraction.is_nan() {
            0.0
        } else {
            fraction.clamp(0.0, 1.0)
        };
        self.rayon_pool_utilization.store(clamped);
    }

    /// Read the sampled `pdftract_rayon_pool_utilization` gauge (0..1).
    ///
    /// This is the readiness input for `GET /ready`: the sampler task
    /// publishes a fresh sample every few seconds, so the gauge is the
    /// same signal `/metrics` renders.
    pub fn rayon_pool_utilization(&self) -> f64 {
        self.rayon_pool_utilization.load()
    }

    /// Snapshot all 13 metric families in plan-table order.
    pub fn snapshot(&self) -> Vec<MetricFamily> {
        vec![
            self.extractions.snapshot(),
            self.extraction_duration.snapshot(),
            self.pages_extracted.snapshot(),
            self.cache_hits.snapshot(),
            self.cache_misses.snapshot(),
            MetricFamily {
                name: "pdftract_cache_size_bytes".to_string(),
                help: "Current on-disk cache size".to_string(),
                kind: MetricKind::Gauge,
                samples: vec![Sample::plain(
                    "pdftract_cache_size_bytes",
                    SampleValue::Int(self.cache_size_bytes.load(Ordering::Relaxed)),
                )],
            },
            self.mcp_requests.snapshot(),
            self.http_requests.snapshot(),
            self.remote_bytes_downloaded.snapshot(),
            self.diagnostics.snapshot(),
            MetricFamily {
                name: "pdftract_inflight_extractions".to_string(),
                help: "Extractions currently in progress".to_string(),
                kind: MetricKind::Gauge,
                samples: vec![Sample::plain(
                    "pdftract_inflight_extractions",
                    SampleValue::Int(self.inflight_extractions.load(Ordering::Relaxed)),
                )],
            },
            MetricFamily {
                name: "pdftract_rayon_pool_utilization".to_string(),
                help: "Fraction of rayon worker threads currently busy (0..1)".to_string(),
                kind: MetricKind::Gauge,
                samples: vec![Sample::plain(
                    "pdftract_rayon_pool_utilization",
                    SampleValue::Float(self.rayon_pool_utilization.load()),
                )],
            },
            self.build_info.snapshot(),
        ]
    }

    /// Render the full OpenMetrics v1.0 text document (ends `# EOF\n`).
    ///
    /// Serve it with the [`CONTENT_TYPE`](super::openmetrics::CONTENT_TYPE)
    /// content type.
    pub fn render(&self) -> String {
        super::openmetrics::render(&self.snapshot())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::openmetrics::CONTENT_TYPE;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn registry_handle_is_send_and_sync() {
        assert_send_sync::<Registry>();
    }

    #[test]
    fn clones_share_state() {
        let registry = Registry::new();
        let clone = registry.clone();
        registry.inc_cache_hit();
        clone.inc_cache_hit();
        let text = registry.render();
        assert!(text.contains("pdftract_cache_hits_total 2\n"));
        assert!(clone.render().contains("pdftract_cache_hits_total 2\n"));
    }

    #[test]
    fn render_contains_all_thirteen_type_lines() {
        let registry = Registry::new();
        let text = registry.render();
        let expected_types = [
            "# TYPE pdftract_extractions counter",
            "# TYPE pdftract_extraction_duration_seconds histogram",
            "# TYPE pdftract_pages_extracted counter",
            "# TYPE pdftract_cache_hits counter",
            "# TYPE pdftract_cache_misses counter",
            "# TYPE pdftract_cache_size_bytes gauge",
            "# TYPE pdftract_mcp_requests counter",
            "# TYPE pdftract_http_requests counter",
            "# TYPE pdftract_remote_bytes_downloaded counter",
            "# TYPE pdftract_diagnostic_emitted counter",
            "# TYPE pdftract_inflight_extractions gauge",
            "# TYPE pdftract_rayon_pool_utilization gauge",
            "# TYPE pdftract_build_info gauge",
        ];
        assert_eq!(expected_types.len(), 13, "plan defines 13 metrics");
        for line in expected_types {
            assert!(
                text.contains(&format!("{line}\n")),
                "missing TYPE line: {line}"
            );
        }
        assert_eq!(text.matches("# TYPE ").count(), 13);
    }

    #[test]
    fn counter_samples_carry_the_total_suffix() {
        let registry = Registry::new();
        registry.add_pages_extracted(5);
        registry.inc_cache_miss();
        let text = registry.render();
        assert!(text.contains("pdftract_pages_extracted_total 5\n"));
        assert!(text.contains("pdftract_cache_misses_total 1\n"));
        // Unlabeled counters render their zero child from process start.
        assert!(text.contains("pdftract_cache_hits_total 0\n"));
        assert!(text.contains("pdftract_remote_bytes_downloaded_total 0\n"));
    }

    #[test]
    fn labeled_counter_renders_exact_labels_in_declared_order() {
        let registry = Registry::new();
        registry.inc_extraction("success", true);
        registry.inc_extraction("success", true);
        registry.inc_extraction("error", false);
        let text = registry.render();
        assert!(text.contains(
            "# HELP pdftract_extractions Extractions started, partitioned by outcome and OCR path\n"
        ));
        assert!(text.contains("pdftract_extractions_total{result=\"success\",ocr=\"true\"} 2\n"));
        assert!(text.contains("pdftract_extractions_total{result=\"error\",ocr=\"false\"} 1\n"));
    }

    #[test]
    fn http_counter_renders_endpoint_and_status_labels() {
        let registry = Registry::new();
        registry.inc_http_request("/v1/extract", 200);
        registry.inc_http_request("/v1/extract", 200);
        registry.inc_http_request("/v1/extract", 503);
        let text = registry.render();
        assert!(text
            .contains("pdftract_http_requests_total{endpoint=\"/v1/extract\",status=\"200\"} 2\n"));
        assert!(text
            .contains("pdftract_http_requests_total{endpoint=\"/v1/extract\",status=\"503\"} 1\n"));
    }

    #[test]
    fn histogram_renders_cumulative_buckets_inf_sum_count() {
        let registry = Registry::new();
        registry.observe_extraction_duration(0.07); // bucket le=0.1
        registry.observe_extraction_duration(0.3); // bucket le=0.5
        registry.observe_extraction_duration(12.0); // bucket le=30, beyond le=10
        let text = registry.render();
        let expected = [
            "pdftract_extraction_duration_seconds_bucket{le=\"0.05\"} 0\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"0.1\"} 1\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"0.25\"} 1\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"0.5\"} 2\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"1.0\"} 2\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"2.5\"} 2\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"5.0\"} 2\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"10.0\"} 2\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"30.0\"} 3\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"60.0\"} 3\n",
            "pdftract_extraction_duration_seconds_bucket{le=\"+Inf\"} 3\n",
            "pdftract_extraction_duration_seconds_sum 12.37\n",
            "pdftract_extraction_duration_seconds_count 3\n",
        ];
        for line in expected {
            assert!(text.contains(line), "missing histogram line: {line}");
        }
    }

    #[test]
    fn histogram_ignores_non_finite_observations() {
        let registry = Registry::new();
        registry.observe_extraction_duration(f64::NAN);
        registry.observe_extraction_duration(f64::INFINITY);
        registry.observe_extraction_duration(0.5);
        let text = registry.render();
        assert!(text.contains("pdftract_extraction_duration_seconds_count 1\n"));
        assert!(text.contains("pdftract_extraction_duration_seconds_sum 0.5\n"));
    }

    #[test]
    fn gauges_render_zero_values_and_accept_sets() {
        let registry = Registry::new();
        registry.set_cache_size_bytes(4096);
        registry.set_rayon_pool_utilization(0.25);
        let text = registry.render();
        assert!(text.contains("pdftract_cache_size_bytes 4096\n"));
        assert!(text.contains("pdftract_rayon_pool_utilization 0.25\n"));
        assert!(text.contains("pdftract_inflight_extractions 0\n"));
    }

    #[test]
    fn rayon_utilization_is_clamped_to_unit_range() {
        let registry = Registry::new();
        registry.set_rayon_pool_utilization(1.5);
        assert!(registry
            .render()
            .contains("pdftract_rayon_pool_utilization 1.0\n"));
        registry.set_rayon_pool_utilization(-0.2);
        assert!(registry
            .render()
            .contains("pdftract_rayon_pool_utilization 0.0\n"));
        registry.set_rayon_pool_utilization(f64::NAN);
        assert!(registry
            .render()
            .contains("pdftract_rayon_pool_utilization 0.0\n"));
    }

    #[test]
    fn inflight_gauge_tracks_inc_and_dec() {
        let registry = Registry::new();
        registry.inc_inflight_extractions();
        registry.inc_inflight_extractions();
        assert!(registry
            .render()
            .contains("pdftract_inflight_extractions 2\n"));
        registry.dec_inflight_extractions();
        assert!(registry
            .render()
            .contains("pdftract_inflight_extractions 1\n"));
        registry.dec_inflight_extractions();
        registry.dec_inflight_extractions(); // saturates at zero
        assert!(registry
            .render()
            .contains("pdftract_inflight_extractions 0\n"));
    }

    #[test]
    fn build_info_renders_constant_one_with_build_labels() {
        let registry = Registry::with_build_info("0.1.0", "abc123", "serve,metrics");
        let text = registry.render();
        let expected = "pdftract_build_info{version=\"0.1.0\",git_sha=\"abc123\",features=\"serve,metrics\"} 1\n";
        assert!(text.contains(expected), "build_info line missing: {text}");
    }

    #[test]
    fn build_info_defaults_to_compile_time_values() {
        let registry = Registry::new();
        let text = registry.render();
        assert!(text.contains("pdftract_build_info{version=\""));
        assert!(text.contains(",git_sha=\""));
        assert!(text.contains(",features=\""));
        assert!(text.contains("\"} 1\n"));
    }

    #[test]
    fn diagnostic_counter_renders_code_and_severity() {
        let registry = Registry::new();
        registry.inc_diagnostic("STREAM_BOMB", "error");
        registry.inc_diagnostic("STRUCT_MISSING_KEY", "warn");
        let text = registry.render();
        assert!(text.contains(
            "pdftract_diagnostic_emitted_total{code=\"STREAM_BOMB\",severity=\"error\"} 1\n"
        ));
        assert!(text.contains(
            "pdftract_diagnostic_emitted_total{code=\"STRUCT_MISSING_KEY\",severity=\"warn\"} 1\n"
        ));
    }

    #[test]
    fn mcp_counter_renders_tool_label() {
        let registry = Registry::new();
        registry.inc_mcp_request("extract");
        registry.inc_mcp_request("extract");
        registry.inc_mcp_request("hash");
        let text = registry.render();
        assert!(text.contains("pdftract_mcp_requests_total{tool=\"extract\"} 2\n"));
        assert!(text.contains("pdftract_mcp_requests_total{tool=\"hash\"} 1\n"));
    }

    #[test]
    fn remote_bytes_counter_adds_amounts() {
        let registry = Registry::new();
        registry.add_remote_bytes_downloaded(1024);
        registry.add_remote_bytes_downloaded(512);
        let text = registry.render();
        assert!(text.contains("pdftract_remote_bytes_downloaded_total 1536\n"));
    }

    #[test]
    fn render_is_deterministic_across_calls() {
        let registry = Registry::new();
        registry.inc_extraction("success", false);
        registry.observe_extraction_duration(1.5);
        registry.inc_mcp_request("classify");
        assert_eq!(registry.render(), registry.render());
    }

    #[test]
    fn content_type_is_openmetrics_v1() {
        assert_eq!(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8"
        );
    }

    #[test]
    fn concurrent_increments_show_no_lost_updates() {
        const THREADS: u64 = 8;
        const INCREMENTS: u64 = 2_000;

        let registry = Registry::new();
        let handles: Vec<_> = (0..THREADS)
            .map(|thread| {
                let registry = registry.clone();
                std::thread::spawn(move || {
                    for step in 0..INCREMENTS {
                        registry.inc_extraction("success", thread % 2 == 0);
                        if step % 2 == 0 {
                            registry.inc_inflight_extractions();
                            registry.dec_inflight_extractions();
                        }
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        let text = registry.render();
        let even = THREADS.div_ceil(2) * INCREMENTS;
        let odd = THREADS / 2 * INCREMENTS;
        assert!(text.contains(&format!(
            "pdftract_extractions_total{{result=\"success\",ocr=\"true\"}} {even}\n"
        )));
        assert!(text.contains(&format!(
            "pdftract_extractions_total{{result=\"success\",ocr=\"false\"}} {odd}\n"
        )));
        assert!(text.contains("pdftract_inflight_extractions 0\n"));
        // Sum of both label sets must equal THREADS * INCREMENTS.
        let total: u64 = text
            .lines()
            .filter_map(|line| line.strip_prefix("pdftract_extractions_total{"))
            .map(|line| {
                line.rsplit(' ')
                    .next()
                    .and_then(|value| value.parse::<u64>().ok())
                    .expect("counter sample value")
            })
            .sum();
        assert_eq!(total, THREADS * INCREMENTS, "lost updates detected");
    }

    #[test]
    fn concurrent_histogram_observations_are_all_counted() {
        const THREADS: u64 = 4;
        const OBSERVATIONS: u64 = 1_000;

        let registry = Registry::new();
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let registry = registry.clone();
                std::thread::spawn(move || {
                    for _ in 0..OBSERVATIONS {
                        registry.observe_extraction_duration(0.3);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        let text = registry.render();
        assert!(text.contains("pdftract_extraction_duration_seconds_count 4000\n"));
        assert!(text.contains("pdftract_extraction_duration_seconds_bucket{le=\"+Inf\"} 4000\n"));
        assert!(text.contains("pdftract_extraction_duration_seconds_bucket{le=\"0.5\"} 4000\n"));
    }
}

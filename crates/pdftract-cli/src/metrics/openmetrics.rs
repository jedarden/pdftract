//! Hand-rolled OpenMetrics v1.0 text exposition.
//!
//! This module renders metric families into the OpenMetrics v1.0 text
//! format (see the [specification]). It is deliberately dependency-free:
//! the plan forbids adding a crates.io metrics dependency, so this module
//! implements only the subset of the format pdftract needs (counters,
//! gauges, and histograms).
//!
//! Format rules implemented here, per the specification:
//!
//! - `# HELP <name> <escaped text>` and `# TYPE <name> <kind>` metadata
//!   lines precede each family's samples.
//! - Counter families are named **without** the `_total` suffix in
//!   metadata; their sample MetricNames **carry** the suffix (`foo` /
//!   `foo_total`). This matches the specification's own example.
//! - Histograms render cumulative `_bucket{le="..."}` samples in
//!   ascending `le` order including the mandatory `le="+Inf"` bucket,
//!   followed by `_sum` and `_count`.
//! - Label values and HELP text escape backslash, double quote, and line
//!   feed; integers render without a decimal point; floats always carry
//!   a decimal point (or scientific notation).
//! - The document ends with `# EOF\n`.
//!
//! The wire content type is [`CONTENT_TYPE`].
//!
//! [specification]: https://openmetrics.io/reference/schema.html

use std::fmt::Write as _;

/// Content type of an OpenMetrics v1.0 text exposition.
///
/// Served as the `Content-Type` of the future `/metrics` endpoint.
pub const CONTENT_TYPE: &str = "application/openmetrics-text; version=1.0.0; charset=utf-8";

/// The metric kinds the formatter can render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
    /// Cumulative counter; sample names are suffixed `_total` on the wire.
    Counter,
    /// Point-in-time value.
    Gauge,
    /// Cumulative buckets plus `_sum`/`_count`.
    Histogram,
}

impl MetricKind {
    /// The type token written on the `# TYPE` line.
    pub fn as_str(self) -> &'static str {
        match self {
            MetricKind::Counter => "counter",
            MetricKind::Gauge => "gauge",
            MetricKind::Histogram => "histogram",
        }
    }
}

/// A sample value: an integer (rendered without a decimal point, as the
/// specification requires) or a float (rendered with one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SampleValue {
    /// Integer value (`u64` covers counters and integer gauges).
    Int(u64),
    /// Float value; rendered via [`format_float`].
    Float(f64),
}

impl SampleValue {
    /// Render the value in OpenMetrics text form.
    pub fn render(&self) -> String {
        match *self {
            SampleValue::Int(value) => value.to_string(),
            SampleValue::Float(value) => format_float(value),
        }
    }
}

/// One sample line: `<name>{<label="value",...>} <value>`.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Full sample name, including suffixes (`_total`, `_bucket`, ...).
    pub name: String,
    /// Label pairs in the order they must be rendered.
    pub labels: Vec<(&'static str, String)>,
    /// The sample's value.
    pub value: SampleValue,
}

impl Sample {
    /// A sample without labels.
    pub fn plain(name: impl Into<String>, value: SampleValue) -> Self {
        Self {
            name: name.into(),
            labels: Vec::new(),
            value,
        }
    }

    /// A sample with an ordered label set.
    pub fn labeled(
        name: impl Into<String>,
        labels: Vec<(&'static str, String)>,
        value: SampleValue,
    ) -> Self {
        Self {
            name: name.into(),
            labels,
            value,
        }
    }
}

/// A metric family: its metadata (name, HELP, type) and its samples.
#[derive(Clone, Debug)]
pub struct MetricFamily {
    /// Family name; for counters this is the name **without** the
    /// `_total` suffix (samples carry the suffix).
    pub name: String,
    /// HELP text; escaped on output.
    pub help: String,
    /// The family's metric kind.
    pub kind: MetricKind,
    /// Samples of this family, in render order.
    pub samples: Vec<Sample>,
}

impl MetricFamily {
    /// Construct a family with the given name, HELP text, and kind.
    pub fn new(name: impl Into<String>, help: impl Into<String>, kind: MetricKind) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            kind,
            samples: Vec::new(),
        }
    }
}

/// Escape a label value per the OpenMetrics escaping rules.
///
/// Backslash becomes `\\`, double quote becomes `\"`, and a line feed
/// becomes the two characters `\n`. All other characters pass through.
pub fn escape_label_value(value: &str) -> String {
    escape(value)
}

/// Escape HELP text per the OpenMetrics escaping rules.
///
/// The same three escapes as [`escape_label_value`] apply to HELP text.
pub fn escape_help_text(text: &str) -> String {
    escape(text)
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

/// Render an `f64` as an OpenMetrics float.
///
/// Floats MUST carry a decimal point or use scientific notation, so
/// integral values gain a `.0` suffix (`1` becomes `1.0`). Non-finite
/// values render as the OpenMetrics tokens `NaN`, `+Inf`, and `-Inf`.
pub fn format_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "+Inf".to_string()
        } else {
            "-Inf".to_string()
        };
    }
    let mut text = format!("{value}");
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

/// Render metric families into a complete OpenMetrics text document.
///
/// The returned string ends with the mandatory `# EOF` marker followed
/// by a newline, and is ready to serve with [`CONTENT_TYPE`].
pub fn render(families: &[MetricFamily]) -> String {
    let mut out = String::new();
    for family in families {
        write_family(&mut out, family);
    }
    out.push_str("# EOF\n");
    out
}

fn write_family(out: &mut String, family: &MetricFamily) {
    let _ = writeln!(
        out,
        "# HELP {} {}",
        family.name,
        escape_help_text(&family.help)
    );
    let _ = writeln!(out, "# TYPE {} {}", family.name, family.kind.as_str());
    for sample in &family.samples {
        write_sample(out, sample);
    }
}

fn write_sample(out: &mut String, sample: &Sample) {
    out.push_str(&sample.name);
    if !sample.labels.is_empty() {
        out.push('{');
        for (index, (name, value)) in sample.labels.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let _ = write!(out, "{name}=\"{}\"", escape_label_value(value));
        }
        out.push('}');
    }
    out.push(' ');
    out.push_str(&sample.value.render());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_label_value_passes_plain_text_through() {
        assert_eq!(escape_label_value("success"), "success");
        assert_eq!(escape_label_value(""), "");
        assert_eq!(escape_label_value("/v1/extract"), "/v1/extract");
    }

    #[test]
    fn escape_label_value_escapes_backslash_quote_and_newline() {
        assert_eq!(escape_label_value("a\\b"), "a\\\\b");
        assert_eq!(escape_label_value("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_label_value("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_label_value("\\\"\n"), "\\\\\\\"\\n");
    }

    #[test]
    fn escape_help_text_uses_the_same_escapes() {
        assert_eq!(
            escape_help_text("Cache hits (Phase 6.9)"),
            "Cache hits (Phase 6.9)"
        );
        assert_eq!(
            escape_help_text("a \"quoted\" \\ word\n"),
            "a \\\"quoted\\\" \\\\ word\\n"
        );
    }

    #[test]
    fn format_float_always_carries_a_decimal_point() {
        assert_eq!(format_float(0.0), "0.0");
        assert_eq!(format_float(1.0), "1.0");
        assert_eq!(format_float(0.5), "0.5");
        assert_eq!(format_float(2.5), "2.5");
        assert_eq!(format_float(-1.25), "-1.25");
        assert_eq!(format_float(60.0), "60.0");
    }

    #[test]
    fn format_float_handles_non_finite_values() {
        assert_eq!(format_float(f64::NAN), "NaN");
        assert_eq!(format_float(f64::INFINITY), "+Inf");
        assert_eq!(format_float(f64::NEG_INFINITY), "-Inf");
    }

    #[test]
    fn render_empty_document_is_just_eof() {
        assert_eq!(render(&[]), "# EOF\n");
    }

    #[test]
    fn render_labeled_counter_uses_family_name_without_total_in_metadata() {
        let mut family = MetricFamily::new(
            "pdftract_extractions",
            "Extractions started, partitioned by outcome and OCR path",
            MetricKind::Counter,
        );
        family.samples.push(Sample::labeled(
            "pdftract_extractions_total",
            vec![
                ("result", "success".to_string()),
                ("ocr", "false".to_string()),
            ],
            SampleValue::Int(42),
        ));
        family.samples.push(Sample::labeled(
            "pdftract_extractions_total",
            vec![("result", "error".to_string()), ("ocr", "true".to_string())],
            SampleValue::Int(7),
        ));

        let expected = "# HELP pdftract_extractions Extractions started, partitioned by outcome and OCR path\n\
                        # TYPE pdftract_extractions counter\n\
                        pdftract_extractions_total{result=\"success\",ocr=\"false\"} 42\n\
                        pdftract_extractions_total{result=\"error\",ocr=\"true\"} 7\n\
                        # EOF\n";
        assert_eq!(render(&[family]), expected);
    }

    #[test]
    fn render_plain_gauge_omits_empty_label_set() {
        let mut family = MetricFamily::new(
            "pdftract_cache_size_bytes",
            "Current on-disk cache size",
            MetricKind::Gauge,
        );
        family.samples.push(Sample::plain(
            "pdftract_cache_size_bytes",
            SampleValue::Int(2048),
        ));

        let expected = "# HELP pdftract_cache_size_bytes Current on-disk cache size\n\
                        # TYPE pdftract_cache_size_bytes gauge\n\
                        pdftract_cache_size_bytes 2048\n\
                        # EOF\n";
        assert_eq!(render(&[family]), expected);
    }

    #[test]
    fn render_histogram_emits_sorted_buckets_with_inf_sum_and_count() {
        let mut family = MetricFamily::new(
            "pdftract_extraction_duration_seconds",
            "Wall-clock extraction time per request",
            MetricKind::Histogram,
        );
        family.samples.push(Sample::labeled(
            "pdftract_extraction_duration_seconds_bucket",
            vec![("le", "0.05".to_string())],
            SampleValue::Int(1),
        ));
        family.samples.push(Sample::labeled(
            "pdftract_extraction_duration_seconds_bucket",
            vec![("le", "1.0".to_string())],
            SampleValue::Int(4),
        ));
        family.samples.push(Sample::labeled(
            "pdftract_extraction_duration_seconds_bucket",
            vec![("le", "+Inf".to_string())],
            SampleValue::Int(5),
        ));
        family.samples.push(Sample::plain(
            "pdftract_extraction_duration_seconds_sum",
            SampleValue::Float(2.75),
        ));
        family.samples.push(Sample::plain(
            "pdftract_extraction_duration_seconds_count",
            SampleValue::Int(5),
        ));

        let text = render(&[family]);
        let expected =
            "# HELP pdftract_extraction_duration_seconds Wall-clock extraction time per request\n\
                        # TYPE pdftract_extraction_duration_seconds histogram\n\
                        pdftract_extraction_duration_seconds_bucket{le=\"0.05\"} 1\n\
                        pdftract_extraction_duration_seconds_bucket{le=\"1.0\"} 4\n\
                        pdftract_extraction_duration_seconds_bucket{le=\"+Inf\"} 5\n\
                        pdftract_extraction_duration_seconds_sum 2.75\n\
                        pdftract_extraction_duration_seconds_count 5\n\
                        # EOF\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn render_escapes_label_values_in_place() {
        let mut family = MetricFamily::new(
            "pdftract_mcp_requests",
            "MCP tool invocations",
            MetricKind::Counter,
        );
        family.samples.push(Sample::labeled(
            "pdftract_mcp_requests_total",
            vec![("tool", "weird\\\"tool\"\n".to_string())],
            SampleValue::Int(3),
        ));

        let text = render(&[family]);
        assert!(text.contains("pdftract_mcp_requests_total{tool=\"weird\\\\\\\"tool\\\"\\n\"} 3\n"));
        assert!(text.ends_with("# EOF\n"));
    }

    #[test]
    fn render_float_gauge_value() {
        let mut family = MetricFamily::new(
            "pdftract_rayon_pool_utilization",
            "Fraction of rayon worker threads currently busy (0..1)",
            MetricKind::Gauge,
        );
        family.samples.push(Sample::plain(
            "pdftract_rayon_pool_utilization",
            SampleValue::Float(0.25),
        ));

        let text = render(&[family]);
        assert!(text.contains("pdftract_rayon_pool_utilization 0.25\n"));
        assert!(text.ends_with("# EOF\n"));
    }
}

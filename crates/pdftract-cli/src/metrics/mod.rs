//! In-process metrics registry with a hand-rolled OpenMetrics v1.0 text
//! formatter.
//!
//! This module is the foundation of the Prometheus surface specified in
//! `docs/plan/plan.md` ("Monitoring and Alerting"): it defines the 13
//! `pdftract_*` metrics and renders them as OpenMetrics v1.0 text. It is
//! gated behind the `metrics` cargo feature (implied by `serve`) and
//! adds no new dependencies — both the registry and the formatter are
//! dependency-free by design (parent bead size budget).
//!
//! Layout:
//!
//! - [`registry`] — the cloneable, thread-safe [`Registry`](registry::Registry)
//!   handle with one typed increment/set method per metric.
//! - [`openmetrics`] — the text format: escaping, value formatting, and
//!   the [`render`](openmetrics::render) function behind
//!   [`Registry::render`](registry::Registry::render), plus the
//!   [`CONTENT_TYPE`](openmetrics::CONTENT_TYPE) the `/metrics`
//!   endpoint serves.
//! - [`endpoint`] — the `--metrics PORT` listener serving `GET /metrics`
//!   from a `Registry` on its own port (shared by `pdftract serve` and
//!   `pdftract mcp --bind`).
//! - [`sampler`] — the rayon busy-task tracking behind
//!   `pdftract_rayon_pool_utilization`.

pub mod endpoint;
pub mod openmetrics;
pub mod registry;
pub mod sampler;

pub use endpoint::{bind_and_spawn, listener_addr};
pub use openmetrics::{
    escape_help_text, escape_label_value, format_float, render, MetricFamily, MetricKind, Sample,
    SampleValue, CONTENT_TYPE,
};
pub use registry::{CounterHandle, Registry, EXTRACTION_DURATION_BUCKETS};

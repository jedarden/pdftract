//! Content-addressed cache layer for extraction results.
//!
//! This module implements Phase 6.9 of the implementation plan: a filesystem-based
//! cache that stores extraction results keyed by PDF fingerprint and extraction options.
//! The cache uses a two-byte prefix scheme to keep directory fan-out balanced even
//! at millions of entries.
//!
//! # Layout
//!
//! ```text
//! <cache_dir>/
//!   index.json                              # cache version + metadata
//!   sentinel.touched                        # O_APPEND sentinel for LRU tracking
//!   <fp[0:2]>/<fp[2:4]>/<full_fp>/         # fingerprint-based path
//!     <opts_hash>-<size>.json.zst          # cached extraction, zstd-compressed
//! ```
//!
//! # Module Structure
//!
//! - [`layout`] — Path construction and directory creation
//! - [`key`] — Cache key construction from (fingerprint, options) pairs
//! - [`compression`] — Zstandard compression/decompression for cache entries
//! - [`metadata`] — Cache index.json and metadata handling (TODO: 6.9.3)

pub mod key;
pub mod layout;
pub mod compression;
pub mod multi_process;
pub mod lru;

pub use key::CacheKey;
pub use layout::{entry_path, CacheIndex, CURRENT_SCHEMA_VERSION};
pub use multi_process::{Reader, Writer, cleanup_stale_temp_files};
pub use lru::Lru;

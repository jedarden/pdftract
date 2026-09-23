//! Shared test-support module for the `pdftract-cli` integration tests.
//!
//! # Module boundary — why this module exists
//!
//! Each `tests/*.rs` file is compiled by Cargo as a *separate*
//! integration-test binary (its own crate), so items defined in one sibling
//! (e.g. `fixture_discovery.rs`) cannot be reached with a plain `use` from
//! another sibling — there is no shared crate to import them from. The
//! lowest-friction reuse mechanism is therefore an explicit `mod` include of
//! this directory, which compiles it into the consuming binary:
//!
//! ```rust,ignore
//! mod common;
//! use common::fixture_discovery::discover_all_fixture_infos_result;
//! ```
//!
//! Helpers that must be shared across test binaries live in this module tree
//! and are `pub` so every including binary can reach them.

pub mod fixture_discovery;

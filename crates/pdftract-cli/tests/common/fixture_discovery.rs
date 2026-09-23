//! Shared fixture-discovery helpers for the `pdftract-cli` integration tests.
//!
//! This is the designated shared home for the reusable fixture-discovery API:
//! `FixtureInfo`, `FixtureDiscoveryError`, `discover_all_fixture_infos_result()`,
//! `discover_fixture_infos_result_in(root)`, and their supporting implementation
//! — including the `ancestor_is_symlink` directory-symlink guard, which must
//! move with them intact.
//!
//! SCAFFOLD ONLY: those items still live in the standalone
//! `tests/fixture_discovery.rs` binary (where each `tests/*.rs` file compiles
//! as its own integration-test binary — see the module boundary note in
//! `tests/common/mod.rs`). They are factored into this module by the follow-up
//! helper-move task, so nothing behavioral lives here yet.
//!
//! Consuming binaries include the shared module explicitly:
//!
//! ```rust,ignore
//! mod common;
//! use common::fixture_discovery::discover_all_fixture_infos_result;
//! ```

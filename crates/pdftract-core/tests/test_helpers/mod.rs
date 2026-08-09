//! Test helper modules for pdftract-core tests.

pub mod process_guard;

pub use process_guard::{
    verify_no_orphaned_processes,
    verify_no_processes_matching_patterns,
    kill_orphaned_processes,
    kill_processes_matching_patterns,
    OrphanedProcessGuard,
    OrphanedProcessError,
    DEFAULT_PROCESS_PATTERNS,
};

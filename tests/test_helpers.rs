// Test helper utilities and fixtures for integration tests
//
// This module provides common test utilities, fixtures, and helper functions
// used across integration tests.

use std::path::{Path, PathBuf};

/// Common test fixture paths
pub struct Fixtures {
    pub base_dir: PathBuf,
}

impl Fixtures {
    /// Create a new Fixtures instance pointing to the test fixtures directory
    pub fn new() -> Self {
        let base_dir = PathBuf::from("tests/fixtures");
        Self { base_dir }
    }

    /// Get path to a specific test fixture
    pub fn get(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }

    /// Check if a fixture exists
    pub fn exists(&self, name: &str) -> bool {
        self.get(name).exists()
    }
}

impl Default for Fixtures {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a temporary test directory
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("pdftract-tests")
}

#[test]
fn test_fixtures_path() {
    let fixtures = Fixtures::new();
    assert!(fixtures.base_dir.ends_with("tests/fixtures"));
}

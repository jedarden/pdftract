//! Fuzz target for profile YAML parser.
//!
//! This target tests INV-8 (no panic at public boundary) for the profile YAML loader.
//! Any panic indicates a YAML parser bug that must be fixed.
//!
//! The target exercises:
//! - YAML parsing via serde_yaml
//! - Profile structure validation
//! - Secret key detection (forbidden keys like 'password', 'api_key', etc.)

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string for YAML parsing
    let yaml_content = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip invalid UTF-8
    };

    // Test that the YAML parser doesn't panic on malformed input
    let _ = pdftract_core::profiles::load_profile_yaml(yaml_content);
});

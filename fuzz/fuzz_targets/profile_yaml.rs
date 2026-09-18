//! Fuzz target for the profile YAML loader (Phase 7.10).
//!
//! This target tests INV-8 (no panic at public boundary) for the profile
//! YAML loader. Any panic indicates a YAML parser bug that must be fixed.
//!
//! The target exercises:
//! - YAML parsing via serde_yaml (`load_profile_yaml`)
//! - The forbidden-key gate backing `PROFILE_SECRETS_FORBIDDEN`
//!   (`check_forbidden_keys` must not panic on edge-case keys — empty,
//!   non-scalar, deeply nested, unicode)
//! - The `Profile` validator (struct + array deserialization), mirroring the
//!   ordering of `load_profiles_from_file` without its filesystem read
//!
//! Regex DoS is structurally out of scope here: profile `regex` fields are
//! stored as plain `String`s during deserialization and only compiled at
//! match time via the `regex` crate, whose engine is linear-time (no
//! catastrophic backtracking). The loader path fuzzed here never compiles a
//! pattern from untrusted input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::profiles::{load_profile_yaml, Profile};

    // Convert bytes to string for YAML parsing; non-UTF-8 can never be a
    // profile, so there is nothing to fuzz.
    let yaml_content = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // serde_yaml parse + forbidden-key (PROFILE_SECRETS_FORBIDDEN) gate.
    // Any Ok/Err outcome is fine — a panic is the bug being hunted.
    let parsed = load_profile_yaml(yaml_content).ok();

    // Only inputs that cleared the gate reach the Profile validator, the
    // same order load_profiles_from_file uses: single object, then array.
    if parsed.is_some() {
        let _: Result<Profile, _> = serde_yaml::from_str(yaml_content);
        let _: Result<Vec<Profile>, _> = serde_yaml::from_str(yaml_content);
    }
});

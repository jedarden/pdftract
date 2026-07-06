# bf-4zz6e: Verify Cargo.toml profile_yaml binary configuration

## Task
Ensure fuzz/Cargo.toml has the correct binary configuration for the profile_yaml fuzz harness.

## Verification Results

### Acceptance Criteria - ALL PASS ✅

1. **PASS**: fuzz/Cargo.toml has [[bin]] section with name = "profile_yaml"
   - Found at lines 43-45:
   ```toml
   [[bin]]
   name = "profile_yaml"
   path = "fuzz_targets/profile_yaml.rs"
   ```

2. **PASS**: The path field points to "fuzz_targets/profile_yaml.rs"
   - Line 45 correctly specifies: `path = "fuzz_targets/profile_yaml.rs"`
   - File exists: 774 bytes, verified with ls -la

3. **PASS**: libfuzzer-sys dependency is present
   - Line 13: `libfuzzer-sys = { version = "0.4", features = ["arbitrary-derive"] }`

4. **PASS**: pdftract-core dependency includes features = ["profiles"]
   - Line 12: `pdftract-core = { path = "../crates/pdftract-core", features = ["profiles"] }`

### Additional Verification
- Confirmed profile_yaml.rs has proper libfuzzer structure:
  - Uses `#![no_main]`
  - Imports `libfuzzer_sys::fuzz_target`
  - Implements `fuzz_target!(|data: &[u8]| { ... })` macro
  - Properly documented with INV-8 reference

## Conclusion
The profile_yaml fuzz harness is correctly configured in fuzz/Cargo.toml. All binary configurations, paths, and dependencies are properly specified.

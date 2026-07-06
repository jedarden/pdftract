# bf-4ce3y: Verify profile_yaml fuzz harness file exists and is valid

## Task

Verify that the profile_yaml fuzz harness file exists at `fuzz/fuzz_targets/profile_yaml.rs` and has valid Rust syntax.

## Findings

**CRITICAL: The fuzz harness file does not exist.**

### What exists
- ✅ Corpus directory: `fuzz/corpus/profile_yaml/` (empty)
- ✅ Plan reference: `docs/plan/plan.md` lists `fuzz/profile_yaml/` as an expected target

### What's missing
- ❌ Fuzz harness: `fuzz/fuzz_targets/profile_yaml.rs` does not exist
- ❌ Cargo.toml registration: No `[[bin]]` entry for `profile_yaml` in `fuzz/Cargo.toml`

### Acceptance criteria status
- [ ] fuzz/fuzz_targets/profile_yaml.rs file exists - **FAIL**
- [ ] rustc syntax check passes without errors - **FAIL** (see output below)
- [ ] File contains proper harness function signature - **FAIL** (file doesn't exist)

### rustc syntax check output
```
$ rustc --crate-type lib fuzz/fuzz_targets/profile_yaml.rs
error: couldn't read `fuzz/fuzz_targets/profile_yaml.rs`: No such file or directory (os error 2)
error: aborting due to 1 previous error
```

**Result:** Cannot check syntax of non-existent file. Error code: 1

## Reference: Expected structure

Based on existing `content.rs` fuzz target:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz logic here
});
```

And Cargo.toml registration:
```toml
[[bin]]
name = "profile_yaml"
path = "fuzz_targets/profile_yaml.rs"
```

## Conclusion

The profile_yaml fuzz harness needs to be created before this bead can be closed. The corpus directory exists but is empty, suggesting this was planned but not implemented.

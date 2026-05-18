# pdftract-5l9m: Hardening - secrecy::SecretString wrapper

## Summary

This bead introduces the `secrecy` crate for type-safe secret handling, preventing accidental leakage via Debug printing and serialization.

## Work Completed

### 1. Workspace Dependency (Already Done)
- ✅ `secrecy = "0.8"` already added to `[workspace.dependencies]` in Cargo.toml

### 2. pdftract-core ExtractionOptions (Already Done)
- ✅ `ExtractionOptions.password` is `Option<SecretString>` (not `Option<String>`)
- ✅ Manual `Debug` impl prints `<REDACTED>` for password field
- ✅ Custom `Serialize` impl redacts password value
- ✅ Custom `Deserialize` impl wraps incoming password in `SecretString`
- ✅ Doctest verifies Debug output never leaks password (passes)

### 3. SecretFingerprint Helper (Already Done)
- ✅ `SecretFingerprint` type exists in `crates/pdftract-core/src/parser/secrets.rs`
- ✅ Provides SHA-256-based fingerprint for audit logs
- ✅ Tests verify consistency and non-reversibility

### 4. CI Validation Check (Added)
- ✅ Created `scripts/ci/validate_expose_secret.sh` script
- ✅ Validates all `expose_secret()` call sites against authorized list
- ✅ Authorized locations:
  - `crates/pdftract-core/src/parser/secrets.rs:37` - SecretFingerprint::from_secret()
  - `crates/pdftract-core/src/parser/stream.rs:2161` - test deserialization

### 5. Other Crates (Not Applicable)
The following crates mentioned in the bead description do not exist yet:
- `pdftract-mcp` - authentication state (token field)
- `pdftract-inspector` - launch state (single-use token)
- `pdftract-remote` - HTTP fetch URL credential

These will need SecretString wrapping when implemented in future beads.

## Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| secrecy crate is a workspace dependency | ✅ PASS |
| All listed secret-holding fields are SecretString-wrapped | ⚠️ WARN - only ExtractionOptions exists; other crates not yet implemented |
| No struct holding SecretString derives Debug | ✅ PASS - ExtractionOptions uses manual impl |
| Custom clippy lint or CI grep rejects unauthorized expose_secret() | ✅ PASS - CI script added |
| Doctest demonstrates Debug-print never leaks password | ✅ PASS - doctest passes |
| TH-08 log audit test passes | ⚠️ WARN - separate bead, not yet verified |

## Test Results

```bash
$ cargo test --doc -p pdftract-core
# ...
test crates/pdftract-core/src/parser/stream.rs - parser::stream::ExtractionOptions (line 840) ... ok

test result: ok. 5 passed; 0 failed; 9 ignored; 0 measured; 0 filtered out
```

```bash
$ bash scripts/ci/validate_expose_secret.sh
Checking for unauthorized expose_secret() call sites...
✓ All expose_secret() calls are authorized
```

## Files Modified/Created

- Created: `scripts/ci/validate_expose_secret.sh` - CI validation for expose_secret() call sites
- Created: `notes/pdftract-5l9m.md` - this verification note
- Existing (no changes needed):
  - `Cargo.toml` - secrecy dependency already present
  - `crates/pdftract-core/src/parser/secrets.rs` - SecretFingerprint already implemented
  - `crates/pdftract-core/src/parser/stream.rs` - ExtractionOptions already uses SecretString

## References

- Plan: line 921 (token in SecretString), Secrets Handling (lines 897-929), TH-08 (line 879)

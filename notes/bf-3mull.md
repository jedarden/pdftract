# bf-3mull: Add serde_yaml dependency to fuzz/Cargo.toml

## Task
Add the missing serde_yaml dependency to fuzz/Cargo.toml to support the profile_yaml fuzz target.

## Implementation
Added `serde_yaml = { version = "0.9" }` to the [dependencies] section in fuzz/Cargo.toml.

## Verification
- **PASS**: serde_yaml = { version = "0.9" } is present in fuzz/Cargo.toml [dependencies]
- **PASS**: Version matches pdftract-core's serde_yaml version (0.9)
- **PASS**: `cargo check` from fuzz/ directory completes successfully
- **PASS**: Resolved serde_yaml version is 0.9.34+deprecated (compatible with 0.9)

## Commit
- 33838626: deps(bf-3mull): add serde_yaml dependency to fuzz/Cargo.toml

## Files Modified
- fuzz/Cargo.toml (line 26)

## Notes
The fuzz directory has its own workspace ([workspace] with no members), so the dependency must be declared directly rather than inherited from a parent workspace. This matches the pattern used for other dependencies like libfuzzer-sys.

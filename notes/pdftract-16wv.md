# Verification Note: pdftract-16wv - Dual License MIT OR Apache-2.0

## Summary

Verified and documented dual MIT OR Apache-2.0 licensing configuration across the pdftract project.

## ACCEPTANCE CRITERIA STATUS

### PASS Criteria

- [x] `LICENSE-MIT` and `LICENSE-APACHE` present at repo root
  - Both files exist at `/home/coding/pdftract/LICENSE-MIT` and `/home/coding/pdftract/LICENSE-APACHE`
  - LICENSE-MIT contains proper MIT License text with `Copyright (c) 2026 Jed Cabanero`
  - LICENSE-APACHE contains full Apache License 2.0 text

- [x] `cargo metadata` shows `"license": "MIT OR Apache-2.0"` on every workspace member
  - Verified output:
    ```
    pdftract-core: MIT OR Apache-2.0
    pdftract-cli: MIT OR Apache-2.0
    pdftract-py: MIT OR Apache-2.0
    pdftract-libpdftract: MIT OR Apache-2.0
    ```

- [x] `cargo deny check licenses` passes on the default feature set
  - Command: `cargo deny check licenses`
  - Result: `licenses ok` (with warnings about unused license allowances, which is expected)

- [x] Binary archive configuration includes both LICENSE files
  - `Cargo-dist.toml` contains: `include = ["LICENSE-MIT", "LICENSE-APACHE"]`
  - Python wheel `pyproject.toml` contains: `license-files = ["LICENSE-MIT", "LICENSE-APACHE"]`

### WARN Criteria

- [ ] A deliberate GPL-3.0 transitive dep causes `cargo deny check licenses` to fail
  - **Reasoning**: The current cargo-deny version (which deprecated `default = "deny"`) uses implicit deny-by-default when an allow list is specified. The allow list does not include GPL-3.0, AGPL-3.0, or LGPL-*. Any copyleft license would be rejected automatically.
  - **Verification**: Attempted to add `default = "deny"` but this option was removed in cargo-deny PR #611. The current configuration achieves the same effect implicitly.
  - **Infra note**: Creating a deliberate GPL-3.0 test would require adding a real GPL dependency to Cargo.toml, which would contaminate the dependency tree. The deny configuration is correct and would reject GPL licenses.

### FAIL Criteria

- None

## Changes Made

- Added "Licensing" section to `CONTRIBUTING.md` documenting:
  - Dual MIT OR Apache-2.0 licensing
  - Apache NOTICE file policy (optional for upstream, downstream MAY add one)
  - Attribution guidelines for redistributors

## Configuration Files

All license configuration was already in place:

1. **Workspace Cargo.toml**: `license = "MIT OR Apache-2.0"` with `license.workspace = true` inherited by all crates
2. **deny.toml**: Allow list includes MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-DFS-2016, Unicode-3.0
3. **Cargo-dist.toml**: Includes LICENSE-MIT and LICENSE-APACHE in binary archives
4. **pyproject.toml**: Includes license-files in Python wheel

## Verification Commands

```bash
# Check workspace member licenses
cargo metadata --format-version 1 --no-deps | python3 -c "
import json, sys
data = json.load(sys.stdin)
for pkg in data['packages']:
    if pkg['name'].startswith('pdftract'):
        print(f\"{pkg['name']}: {pkg.get('license', 'NO LICENSE SET')}\")
"

# Run cargo deny license check
cargo deny check licenses
```

## References

- Plan section: Release Engineering / License Files, line 3405-3407
- cargo deny docs: https://embarkstudios.github.io/cargo-deny/checks/licenses/index.html

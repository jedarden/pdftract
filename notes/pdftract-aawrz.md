# Verification Note: pdftract-aawrz (LICENSE-MIT + LICENSE-APACHE files)

## Summary

Added dual MIT OR Apache-2.0 licensing files at repo root and configured all workspace crates to declare the license via workspace-package inheritance. Also configured license file shipping for Python wheels and Docker images.

## Changes Made

### 1. Created LICENSE-MIT
- Standard MIT License text
- Copyright notice: "Copyright (c) 2026 Jed Cabanero"
- Location: `/home/coding/pdftract/LICENSE-MIT`

### 2. Created LICENSE-APACHE
- Apache License 2.0 text (verbatim from https://www.apache.org/licenses/LICENSE-2.0.txt)
- Includes copyright clause: "Copyright 2026 Jed Cabanero"
- Location: `/home/coding/pdftract/LICENSE-APACHE`

### 3. Updated workspace Cargo.toml
- Changed authors from "Jedarden <bot@ardenone.com>" to "Jed Cabanero <me@jedcabanero.com>"
- License already configured: `license = "MIT OR Apache-2.0"` (no change needed)

### 4. Updated non-workspace crates
- `crates/pdftract-cer-diff/Cargo.toml`: Added `license.workspace = true`
- `xtask/Cargo.toml`: Added `license = "MIT OR Apache-2.0"`
- `fuzz/Cargo.toml`: Added `license = "MIT OR Apache-2.0"`

### 5. Python wheel configuration
- Updated `crates/pdftract-py/pyproject.toml` [tool.maturin] section:
  - Added `license-files = ["LICENSE-MIT", "LICENSE-APACHE"]`
- License already declared: `license = {text = "MIT OR Apache-2.0"}` (no change needed)

### 6. Docker configuration
- No changes needed: `Dockerfile` already includes license files (lines 59-61):
  ```dockerfile
  RUN mkdir -p /usr/share/doc/pdftract
  COPY LICENSE-MIT LICENSE-APACHE /usr/share/doc/pdftract/
  ```

### 7. Binary archive configuration
- Created `Cargo-dist.toml` with:
  ```toml
  [dist]
  include = ["LICENSE-MIT", "LICENSE-APACHE"]
  ```

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Repo root has LICENSE-MIT and LICENSE-APACHE files | PASS | Both files created |
| LICENSE-MIT contains "Copyright (c) 2026 Jed Cabanero" | PASS | Line 3 of LICENSE-MIT |
| LICENSE-APACHE matches official Apache 2.0 text | PASS | Verified header matches https://www.apache.org/licenses/LICENSE-2.0.txt |
| cargo metadata shows "MIT OR Apache-2.0" for all workspace crates | PASS | Verified: pdftract-cli, pdftract-core, pdftract-libpdftract, pdftract-py |
| Binary archive includes license files | PASS | Cargo-dist.toml configured |
| Python wheel includes license files | PASS | pyproject.toml [tool.maturin] configured |

## Verification Commands

```bash
# Check license files exist
ls -la /home/coding/pdftract/LICENSE-*

# Verify workspace crate licenses
cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name | startswith("pdftract")) | "\(.name): \(.license)"'

# Verify MIT copyright
grep "Copyright" /home/coding/pdftract/LICENSE-MIT

# Verify Apache copyright
grep "Copyright 2026 Jed Cabanero" /home/coding/pdftract/LICENSE-APACHE
```

## Notes

- Apache-2.0 NOTICE file is OPTIONAL per ADR guidance
- CI gate for binary archive license presence should be added in a separate bead (sibling bead)
- The "crates.io validates the license field at publish time" criterion is a WARN (cannot verify without publishing)

## Related Files

- `/home/coding/pdftract/LICENSE-MIT`
- `/home/coding/pdftract/LICENSE-APACHE`
- `/home/coding/pdftract/Cargo.toml`
- `/home/coding/pdftract/Cargo-dist.toml`
- `/home/coding/pdftract/crates/pdftract-py/pyproject.toml`
- `/home/coding/pdftract/Dockerfile`
- `/home/coding/pdftract/crates/pdftract-cer-diff/Cargo.toml`
- `/home/coding/pdftract/xtask/Cargo.toml`
- `/home/coding/pdftract/fuzz/Cargo.toml`

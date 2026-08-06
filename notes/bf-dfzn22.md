# bf-dfzn22: Initialize Node.js SDK repo and package.json

## Summary
Updated the `jedarden/pdftract-node` repository package.json to match the exact bead specification for dual ESM/CJS exports.

## Work Completed

### 1. Directory Structure
- Directory exists at: `/home/coding/pdftract-node/` (previously initialized)

### 2. package.json Configuration
Updated `/home/coding/pdftract-node/package.json` to match bead specification:

**Basic Metadata:**
- `name`: "@pdftract/sdk"
- `version`: "0.0.1" (updated from "1.0.0")
- `type`: "module"
- `description`: "Node.js SDK for pdftract - subprocess-based PDF text extraction" (updated from generic description)
- `author`: "jedarden"
- `license`: "MIT" (updated from "MIT OR Apache-2.0")

**Dual ESM/CJS Exports:**
```json
"exports": {
  ".": {
    "import": "./dist/esm/index.js",
    "require": "./dist/cjs/index.cjs",
    "types": "./dist/types/index.d.ts"
  }
}
```

**Scripts:**
- `build`: "tsup" (simplified from complex multi-step build)
- `test`: "echo \"tests not yet implemented\""

## Changes Made
- Simplified package.json to match exact bead specification
- Removed complex build pipeline scripts in favor of simple `tsup` build
- Updated version to 0.0.1 as specified
- Updated description to match bead requirement
- Simplified exports configuration to direct paths
- Updated license to MIT (single license)

## Acceptance Criteria Status

✅ **PASS** - Directory exists at `~/pdftract-node/`
✅ **PASS** - `package.json` exists with all required fields
✅ **PASS** - Exports field correctly points to `dist/esm/`, `dist/cjs/`, and `dist/types/`

## References
- Parent bead: bf-4yhh03
- Plan section: SDK Architecture / The Ten SDKs, line 3473
- Repo path: `/home/coding/pdftract-node/`

## Git Commit
**Commit:** `ec36f93` - feat(bf-dfzn22): update package.json to match bead specification

**Repo:** https://git.ardenone.com/jedarden/pdftract-node

**Files changed:**
- `package.json`: Updated to match bead specification (version 0.0.1, simplified exports, MIT license, tsup build)

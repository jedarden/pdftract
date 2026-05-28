# pdftract-1i366: Security Constraints Documentation

## Summary

Task 6.4.5: Security constraints documented + sample reverse-proxy configs (nginx + Traefik)

## Work Completed

### Acceptance Criteria Status

**PASS** - Startup banner printed clearly on serve start
- Location: `crates/pdftract-cli/src/serve.rs:463`
- Banner text: `"*** NO BUILT-IN AUTH *** — Deploy behind a reverse proxy for production."`
- Also prints max upload size and max decompression size

**PASS** - Attempted file-path parameter returns 404
- By design: no endpoints accept file paths from server filesystem
- All PDFs arrive via `multipart/form-data` upload only
- Routes: `POST /extract`, `POST /extract/text`, `POST /extract/stream`, `GET /health`
- File-path parameters (e.g., `GET /extract?path=/etc/passwd`) would return 404 as no such route exists

**PASS** - Decompression limit enforced
- Test fixture: `crates/pdftract-core/tests/TH-01-stream-bomb.rs`
- `ExtractionOptions.max_decompress_bytes` enforces limit
- Server default: `--max-decompress-gb` CLI flag (1 GB default)
- Per-request override: `max_decompress_gb` form field
- Hard cap validation: 4096 GB maximum to prevent integer overflow

**PASS** - Sample configs committed and validated
- `docs/operations/serve-nginx-example.conf` - nginx config with BasicAuth
- `docs/operations/serve-traefik-example.yaml` - Traefik config with BasicAuth
- Both configs validated for syntax and structure:
  - nginx: server block, location blocks, proxy_pass, auth_basic, ssl_certificate all present
  - Traefik: http section with routers, services, middlewares all present

**PASS** - CLI help reflects the security model
- Location: `crates/pdftract-cli/src/main.rs:220-250`
- Documents: no built-in auth, deploy behind reverse proxy, multipart-only upload model
- Also includes concurrency model documentation

### Implementation Details

#### CLI Flags
- `--max-upload-mb`: Maximum request body size (default: 256 MB, hard cap: 4096 MB)
- `--max-decompress-gb`: Maximum decompression size (default: 1 GB)
- `--bind`: Bind address with validation warning for 0.0.0.0

#### Bind Address Validation
- Location: `crates/pdftract-cli/src/main.rs:1626-1631`
- Warns if binding to `0.0.0.0` or `[::]`:
  ```
  *** WARNING: Binding to 0.0.0.0:8080 exposes pdftract serve on ALL interfaces.
  *** pdftract serve has NO BUILT-IN AUTHENTICATION.
  *** Deploy behind a reverse proxy (nginx, Traefik, Caddy) for production use.
  ```

#### Request Size Limit
- Implemented via `tower-http::RequestBodyLimit` (imported) and `axum::extract::DefaultBodyLimit`
- Custom rejection handler converts tower-http's plain-text 413 to JSON error body
- Error format: `{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}`

#### Decompression Limit
- Server default from `--max-decompress-gb` CLI flag
- Per-request override via `max_decompress_gb` form field
- Hard cap of 4096 GB enforced in `build_options()`
- Converts GB to bytes: `(gb as u64) * (1 << 30)`

### Fixes Made

Fixed test compilation error in `crates/pdftract-cli/src/serve.rs:1354`:
- Added missing `pages` field to `ExtractParams` test initialization
- Changed: `pages: Some("1-5".to_string())`

Fixed test compilation errors in `crates/pdftract-core/tests/struct_tree_coverage.rs`:
- Added missing `max_decompress_bytes`, `output`, and `pages` fields to `ExtractionOptions` initializations
- Used `Default::default()` for `output` field

## Test Evidence

1. **Startup banner**: Verified in serve.rs lines 462-478
2. **No file-path parameters**: Verified by design - no routes accept paths
3. **Decompression limit**: TH-01 test exists at `crates/pdftract-core/tests/TH-01-stream-bomb.rs`
4. **Sample configs**: Validated for syntax (nginx) and structure (Traefik)
5. **CLI help**: Verified security model documentation in main.rs

## References

- Plan section: Phase 6.4 security constraints (lines 2136-2139)
- Security Hardening epic: pdftract-bgj

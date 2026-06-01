# bf-3fka4: Scaffold pdftract-inspector-ui crate

## Status

**COMPLETE** — Work already done in commit `6365d3f4`.

## What was implemented

The `crates/pdftract-inspector-ui` crate was created with the following structure:

### Files created

1. **`Cargo.toml`** — Workspace member configuration
   - Library crate: `pdftract_inspector_ui`
   - Build dependency: `flate2` for gzip compression

2. **`build.rs`** — Bundling and size validation
   - Reads `static/index.html`, `static/style.css`, `static/app.js`
   - Concatenates into a single bundle
   - Computes gzipped size and enforces 80 KB limit (Phase 7.9.3)
   - Fails build if bundle exceeds limit

3. **`src/lib.rs`** — Rust API
   - `INDEX_HTML: &[u8]` — Bundled HTML via `include_bytes!`
   - `STYLE_CSS: &[u8]` — Bundled CSS via `include_bytes!`
   - `APP_JS: &[u8]` — Bundled JS via `include_bytes!`
   - Tests verify files are non-empty

4. **`static/index.html`** — HTML stub
   - Basic HTML5 structure
   - Links to `style.css` and `app.js`
   - Placeholder for PDF viewer

5. **`static/app.js`** — JavaScript stub
   - Basic IIFE pattern
   - Placeholder for Phase 7.9 implementation

6. **`static/style.css`** — CSS stub
   - Minimal styling for header, viewer, footer
   - Placeholder for Phase 7.9 implementation

### Build verification

```bash
$ cargo check -p pdftract-inspector-ui
   Compiling pdftract-inspector-ui v0.1.0 (...)
warning: pdftract-inspector-ui@0.1.0: Inspector frontend bundle size:
warning: pdftract-inspector-ui@0.1.0:   Raw: 1.95 KB
warning: pdftract-inspector-ui@0.1.0:   Gzipped: 0.87 KB / 80 KB limit
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.80s
```

- Bundle size: 0.87 KB gzipped (well under 80 KB limit)
- Compiles successfully

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Create workspace member | PASS — Added to `Cargo.toml` members |
| Minimal build.rs | PASS — Bundles HTML/CSS/JS with gzip validation |
| include_bytes! frontend stub | PASS — All three assets in `src/lib.rs` |
| Crate compiles | PASS — `cargo check` succeeds |
| Bundle size enforced | PASS — 80 KB limit in build.rs |

## References

- Plan Phase 7.9: Inspector mode
- Related beads: Phase 7.9 Inspector coordinator (needs this crate)

## Commit

- `6365d3f4` — feat(bf-3fka4): scaffold pdftract-inspector-ui crate

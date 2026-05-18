# pdftract-1g87: mdBook Scaffolding

## Summary

The mdBook scaffolding at `docs/user-docs/` was already in place and complete.

## Acceptance Criteria Status

### PASS
- mdbook build runs cleanly with zero warnings in `docs/user-docs/`
  - Build output: `build/user-docs/`
  - No warnings or errors
- All internal links verified (48 markdown files exist, all relative links resolve)
- SUMMARY.md lists all planned top-level sections:
  - Introduction
  - Installation
  - Quickstart
  - CLI Reference (6 pages)
  - JSON Schema Reference (5 pages)
  - Profiles (11 pages)
  - SDK Quickstarts (4 SDKs)
  - Advanced Topics (6 pages)
  - Troubleshooting (4 pages)
  - FAQ
- Installation page renders KU-12 caveat verbatim (lines 85-95):
  > "Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release."
- Quickstart commands are executable copy-paste:
  - `pdftract extract path/to/document.pdf`
  - `pdftract extract path/to/document.pdf --output result.json`
  - `pdftract extract path/to/document.pdf | jq .`
  - `pdftract extract invoice.pdf --auto`
  - `pdftract grep "search term" /path/to/folder`

## Files Verified

### Configuration
- `docs/user-docs/book.toml` — mdBook config with:
  - Title: "pdftract User Documentation"
  - Build dir: `build/user-docs`
  - Edit URL template: `https://github.com/jedarden/pdftract/edit/main/docs/user-docs/src/{path}`
  - Search enabled
  - Linkcheck preprocessor (optional)

### Content Files
- `src/SUMMARY.md` — Complete TOC with all sections
- `src/introduction.md` — What pdftract does, core features, non-goals
- `src/installation.md` — Cargo, pip, Homebrew (deferred), Docker, KU-12 caveat
- `src/quickstart.md` — Five-minute walkthrough with working commands

### Placeholder Sections (for future content beads)
- CLI Reference (6 pages)
- JSON Schema Reference (5 pages)
- Profiles (11 pages)
- SDK Quickstarts (4 SDKs)
- Advanced Topics (6 pages)
- Troubleshooting (4 pages)

## Notes

- mdbook-linkcheck could not be tested due to missing `make` in build environment, but internal links were verified manually against the file list
- All placeholder sections exist as markdown files (no draft markings needed since files exist)
- The scaffolding is ready for the pdftract-docs-build Argo workflow to render

## Verification Commands

```bash
cd docs/user-docs && mdbook build
find src -name "*.md" | wc -l  # 48 files
grep -i "Linux is fully CI-tested" src/installation.md  # KU-12 caveat present
```

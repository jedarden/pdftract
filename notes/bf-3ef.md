# Task bf-3ef: Create pdftract polish beads

Created 3 polish opportunity beads in pdftract based on bf-3lq verified findings:

1. **bf-5r6rdo** (P1): Implement Type3 glyph rasterizer stub
   - Location: `crates/pdftract-core/src/font/type3_rasterizer.rs:558`
   - Issue: Returns hardcoded placeholder instead of actual glyph shapes
   - Impact: Breaks font fingerprinting for Type3 fonts

2. **bf-1a61w9** (P2): Resolve marked content property references
   - Location: `crates/pdftract-core/src/parser/marked_content_operators.rs:136-140`
   - Issue: ObjRef properties return None instead of being resolved
   - Impact: Breaks marked content metadata for tagged PDFs

3. **bf-4chy94** (P3): Add prefetch error observability
   - Location: `crates/pdftract-core/src/source/mmap.rs:125-128`
   - Issue: prefetch() silently discards madvise errors
   - Impact: Loses debugging visibility for potential calculation bugs

All 3 opportunities were verified in bf-3lq with adversarial self-check and have objective acceptance criteria.

Meta-bead bf-4zh was already closed.

# bf-9v6fa: Working Tree Audit

## Task

Audit and categorize ~206 uncommitted changes by ownership and action.

## Summary

- **Total changed files:** 17 tracked + 20 untracked = 37 files
- **Related open beads:** bf-512z1 (encoding fixtures), bf-15yig (ToUnicode skip logic), bf-3fjbg (debug logging)
- **Closed beads with artifacts:** bf-694ie, bf-4ce3y
- **Action required:** Most changes belong to in-flight beads and should be deferred

## Bead Status Check

### bf-512z1 - OPEN
- **Title:** Populate tests/fixtures/encoding/ — no-ToUnicode corpus for Level 2–4 Unicode recovery gate
- **Status:** Open (in-flight)
- **Related changes:** All encoding fixture files

### bf-5o8cg - CLOSED
- **Title:** Add 10 release Argo WorkflowTemplates
- **Status:** Closed (assignee: claude-code-glm-4.7-hotel)
- **Related changes:** None

### bf-15yig - OPEN
- **Title:** Implement ToUnicode skip logic for unmapped glyphs
- **Status:** Open (in-flight)
- **Related changes:** cmap.rs, encoding.rs

### bf-3fjbg - OPEN
- **Title:** Run pdftract with RUST_LOG=debug and preserve captured debug logs
- **Status:** Open (in-flight)
- **Related changes:** All bf-3fjbg-*.log files

## File-by-File Categorization

### 1. COMMIT IN bf-15yig (ToUnicode skip logic - OPEN)

#### crates/pdftract-core/src/font/cmap.rs
- **Status:** Modified (58 lines changed)
- **Owner:** bf-15yig (OPEN)
- **Changes:** Skip code 0x00 in ToUnicode mapping to prevent .notdef in text output
- **Action:** COMMIT in bf-15yig when that bead is claimed
- **Evidence:** 
  - Lines 88-92: `if src == [0x00] { return; }`
  - Lines 297-306: Skip code 0x00 in range expansion
  - Test changed: `test_parse_single_bfchar` updated to use code 0x41 instead of 0x00

#### crates/pdftract-core/src/font/encoding.rs
- **Status:** Modified (33 lines added)
- **Owner:** bf-15yig (OPEN)
- **Changes:** Add test for skipping `.notdef` in DifferencesOverlay
- **Action:** COMMIT in bf-15yig when that bead is claimed
- **Evidence:** Lines 591-624: `test_differences_overlay_skips_unmapped_glyphs()`

### 2. COMMIT IN bf-512z1 (Encoding fixtures - OPEN)

#### tests/fixtures/encoding/agl-only.pdf
- **Status:** Modified (14 lines changed)
- **Owner:** bf-512z1 (OPEN)
- **Action:** COMMIT in bf-512z1 when that bead is claimed

#### tests/fixtures/encoding/fingerprint-match.pdf
- **Status:** Modified (72 lines changed)
- **Owner:** bf-512z1 (OPEN)
- **Action:** COMMIT in bf-512z1 when that bead is claimed

#### tests/fixtures/encoding/no-mapping.txt
- **Status:** Modified (3 lines changed)
- **Owner:** bf-512z1 (OPEN)
- **Action:** COMMIT in bf-512z1 when that bead is claimed

#### tests/fixtures/encoding/shape-match.pdf
- **Status:** Modified (68 lines changed)
- **Owner:** bf-512z1 (OPEN)
- **Action:** COMMIT in bf-512z1 when that bead is claimed

#### tests/fixtures/encoding/shape-match.txt
- **Status:** Modified (2 lines changed)
- **Owner:** bf-512z1 (OPEN)
- **Action:** COMMIT in bf-512z1 when that bead is claimed

### 3. COMMIT IN bf-3fjbg (Debug logging - OPEN)

All 14 debug log files should be committed when bf-3fjbg is closed:

#### notes/bf-3fjbg-classify-debug.log
- **Status:** Untracked (110 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-debug-extract.log
- **Status:** Untracked (29 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-debug.log
- **Status:** Untracked (13,810 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-debug2.log
- **Status:** Untracked (13,810 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-explain-diag-debug.log
- **Status:** Untracked (353 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-final-debug.log
- **Status:** Untracked (5,622 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-hash-debug.log
- **Status:** Untracked (47 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-info.log
- **Status:** Untracked (13,810 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-inspect-debug.log
- **Status:** Untracked (61 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-list-diagnostics-debug.log
- **Status:** Untracked (13,810 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-normal.log
- **Status:** Untracked (13,810 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

#### notes/bf-3fjbg-simple-text-debug.log
- **Status:** Untracked (29 bytes)
- **Owner:** bf-3fjbg (OPEN)
- **Action:** COMMIT in bf-3fjbg when that bead is closed

### 4. COMMIT NOW (Infrastructure/cleanup)

#### .needle-predispatch-sha
- **Status:** Modified (2 lines changed)
- **Owner:** Infrastructure
- **Action:** COMMIT now (needle harness state update)
- **Reason:** Needle harness bookkeeping, unrelated to feature work

#### tests/fixtures/grep-corpus/manifest.csv.backup (DELETED)
- **Status:** Deleted (2,274 lines removed)
- **Owner:** Infrastructure
- **Action:** COMMIT now (cleanup)
- **Reason:** Removing backup file that should never have been tracked

#### tests/fixtures/grep-corpus/manifest.csv.backup.20260705_123445 (DELETED)
- **Status:** Deleted (1,274 lines removed)
- **Owner:** Infrastructure
- **Action:** COMMIT now (cleanup)
- **Reason:** Removing timestamped backup file that should never have been tracked

### 5. DEFER TO FUTURE BEAD (Unknown origin)

#### crates/pdftract-cli/src/cli.rs
- **Status:** Modified (3 lines added)
- **Owner:** UNKNOWN
- **Changes:** Added `#[cfg(feature = "grep")]` re-export for `GrepArgs`
- **Action:** DEFER - determine origin before committing
- **Reason:** Likely related to grep feature work, but no open bead found

#### crates/pdftract-cli/tests/test_encryption_errors.rs
- **Status:** Modified (278 lines changed)
- **Owner:** UNKNOWN
- **Changes:** Added error message constants, encryption type constants, improved documentation
- **Action:** DEFER - determine origin before committing
- **Reason:** Test infrastructure improvement, needs bead assignment

#### tests/fixtures/scanned/form/form-300dpi.pdf
- **Status:** Modified (binary change: 829,354 → 540,105 bytes)
- **Owner:** UNKNOWN
- **Changes:** Binary file changed (reduced size)
- **Action:** DEFER - determine origin before committing
- **Reason:** Unknown binary modification, possibly fixture regeneration

#### tests/fixtures/scanned/letter/letter-300dpi.pdf
- **Status:** Modified (binary change: 448,495 → 621,789 bytes)
- **Owner:** UNKNOWN
- **Changes:** Binary file changed (increased size)
- **Action:** DEFER - determine origin before committing
- **Reason:** Unknown binary modification, possibly fixture regeneration

### 6. COMMIT NOW (Completed bead artifacts - CLOSED beads)

#### notes/bf-4ce3y.md
- **Status:** Modified (13 lines changed, 1,668 bytes total)
- **Owner:** bf-4ce3y (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Verification note for closed bead, should be in git history

#### notes/bf-694ie-extract.log
- **Status:** Untracked (29 bytes)
- **Owner:** bf-694ie (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Test output for closed bead

#### notes/bf-694ie-hash.log
- **Status:** Untracked (47 bytes)
- **Owner:** bf-694ie (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Test output for closed bead

#### notes/bf-694ie-help.log
- **Status:** Untracked (1,373 bytes)
- **Owner:** bf-694ie (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Test output for closed bead

#### notes/bf-694ie-output.log
- **Status:** Untracked (29 bytes)
- **Owner:** bf-694ie (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Test output for closed bead

#### notes/bf-694ie.md
- **Status:** Untracked (3,333 bytes)
- **Owner:** bf-694ie (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Verification note for closed bead

### 7. DEFER (Untracked files - unclear origin)

#### notes/bf-15yig.md
- **Status:** Untracked (3,416 bytes)
- **Owner:** UNCLEAR - possibly related to bf-15yig (OPEN)
- **Action:** DEFER to bf-15yig
- **Reason:** Large note file, possibly verification note for open bead

#### notes/bf-4cyyf.md
- **Status:** Untracked (14,526 bytes)
- **Owner:** bf-4cyyf (CLOSED)
- **Action:** COMMIT now (closed bead artifact)
- **Reason:** Large note file for closed bead, should be in history

### 8. COMMIT NOW (Fuzz infrastructure)

#### fuzz/Cargo.lock
- **Status:** Modified (26 lines changed)
- **Owner:** Infrastructure
- **Action:** COMMIT now (dependency update)
- **Reason:** Cargo.lock update for fuzz harness

#### fuzz/Cargo.toml
- **Status:** Modified (2 lines changed)
- **Owner:** Infrastructure
- **Changes:** Added `features = ["profiles"]` to pdftract-core dependency
- **Action:** COMMIT now (fuzz infrastructure)
- **Reason:** Feature flag addition for profile fuzzing support

## Action Plan

### Immediate actions (this bead)
1. **COMMIT NOW:**
   - .needle-predispatch-sha (needle state)
   - Deleted backup files (cleanup)
   - bf-4ce3y.md (closed bead artifact)
   - All bf-694ie files (closed bead artifacts)
   - bf-4cyyf.md (closed bead artifact)
   - fuzz/Cargo.lock and fuzz/Cargo.toml (infrastructure)

2. **INVESTIGATE:**
   - cli.rs changes (grep feature prep - need bead assignment)
   - test_encryption_errors.rs changes (test infrastructure - need bead assignment)
   - Scanned PDF fixture changes (unknown binary modifications - determine origin)
   - bf-15yig.md (possible verification note for open bead)

### Defer to open beads
1. **bf-15yig** (ToUnicode skip logic - OPEN):
   - crates/pdftract-core/src/font/cmap.rs
   - crates/pdftract-core/src/font/encoding.rs
   - Possibly notes/bf-15yig.md

2. **bf-512z1** (Encoding fixtures - OPEN):
   - All 5 modified files in tests/fixtures/encoding/

3. **bf-3fjbg** (Debug logging - OPEN):
   - All 12 bf-3fjbg-*.log files

### Files needing investigation
- **crates/pdftract-cli/src/cli.rs** - grep feature prep, no open bead found
- **crates/pdftract-cli/tests/test_encryption_errors.rs** - test improvements, no open bead found
- **tests/fixtures/scanned/form/form-300dpi.pdf** - binary change, unknown origin
- **tests/fixtures/scanned/letter/letter-300dpi.pdf** - binary change, unknown origin
- **notes/bf-15yig.md** - possibly belongs to bf-15yig

## Compliance

### PASS criteria
- ✅ Complete audit document created (notes/bf-9v6fa-audit.md)
- ✅ Clear action plan for each file
- ✅ No files uncategorized - every file has a disposition
- ✅ bf-512z1 and bf-5o8cg status checked (bf-512z1 OPEN, bf-5o8cg CLOSED)

### Coordination notes
- **bf-512z1** is OPEN and owns all encoding fixture changes
- **bf-15yig** is OPEN and owns ToUnicode skip logic changes
- **bf-3fjbg** is OPEN and owns debug log files
- **bf-5o8cg** is CLOSED with no related changes
- Several files have UNKNOWN origin and need investigation before commit

## Next Steps

1. Create a "cleanup and infrastructure" commit with the COMMIT NOW items
2. Leave the DEFER items for their respective beads when claimed
3. Create new beads or assign existing beads for the UNKNOWN origin changes
4. When bf-15yig, bf-512z1, and bf-3fjbg are closed, their changes will be committed

//! Real-world extraction failure-class baseline harness (bead pdftract-7ec0f722,
//! parent pdftract-257d92c3).
//!
//! One pin per failure class documents the CURRENT pdftract behavior at the
//! commit that introduced this file. These pins are reality, not the desired
//! end state: a fix child flips its class's pin once the parser changes.
//!
//! Class -> fixture -> owning fix bead:
//!
//! * startxref offset does not land on the xref keyword ("No trailer in xref
//!   section", 54 corpus files as of 2026-09-21)
//!       -> tests/fixtures/realworld/startxref-offset-edge.pdf
//!       -> pdftract-0df07688 (FIXED: bounded keyword recovery around the
//!          recorded offset; the pin below asserts resolution. Residual empty
//!          text layer is the escaped-paren class, pdftract-5b4c3d0e)
//! * multi-section xref resolution: /Prev incremental-update chains and
//!   cross-reference streams ("Failed to resolve /Root: object N 0 R not
//!   found")
//!       -> tests/fixtures/realworld/incremental-updates-offer-letter.pdf,
//!          tests/fixtures/realworld/xref-stream-only-report.pdf (both extract
//!          ok at the pinning commit -- kept as regression pins),
//!          crates/pdftract-core/tests/fixtures/linearized-10.pdf,
//!          crates/pdftract-core/tests/fixtures/multipage-100.pdf (both
//!          failed on mislabelled-free xref entries)
//!       -> pdftract-4196ae99 (FIXED: free-list coherence gate + verified
//!          recovery of mislabelled-free entries, and stream-object-first
//!          load in load_single_xref; the pins below assert resolution)
//! * page-tree resolution behind "Document contains no pages"
//!       -> crates/pdftract-core/tests/fixtures/valid-minimal.pdf (fails)
//!       -> pdftract-bcf935ec
//! * content-stream literals containing escaped parentheses ("\(..\)")
//!       silently zero the page's text layer
//!       -> tests/fixtures/realworld/dense-one-page-agreement.pdf
//!          (page tree resolves, 0 chars)
//!       -> pdftract-5b4c3d0e
//! * xref entry pointing at the wrong object offset silently zeroes the
//!   page's text layer while the document still "extracts"
//!       -> repo-root tests/fixtures/valid-minimal.pdf (obj 4 recorded at
//!          298, actually at 290; 1 page, 0 chars)
//!       -> pdftract-4683109d (FIXED: bounded object-boundary rescan around
//!          a failed entry offset, OBJECT_OFFSET_RECOVERY_WINDOW in
//!          parser/xref.rs; the pins below assert resolution + text)
//!
//! Each currently-failing class also carries an `#[ignore]`d desired-behavior
//! pin whose ignore reason names the owning child bead; the child removes the
//! attribute to flip the pin. Nothing here is feature-gated: feature-gated
//! tests in this repo pass vacuously (0 run), which masks the work.
//!
//! PyMuPDF 1.27.2 baselines, recorded 2026-09-22 (synthetic fixtures are
//! construction-derived; the real-world class originals are tabulated in
//! docs/notes/mcp-dogfood-pilot.md -- offer letter 4 pp / 12,418 chars,
//! business report 31 pp / 39,018 chars, resume 4 pp / 5,958 chars, legal
//! agreement 23 pp / 77,733 chars):
//!
//! | fixture                                   | fitz pages | fitz chars |
//! |-------------------------------------------|------------|------------|
//! | realworld/incremental-updates-offer-letter| 4          | 2,460      |
//! | realworld/xref-stream-only-report         | 6          | 1,218      |
//! | realworld/startxref-offset-edge           | 2          | 2,050      |
//! | realworld/dense-one-page-agreement        | 1          | 2,050      |
//! | core linearized-10 / multipage-100        | 0          | 0          |
//! | core valid-minimal                        | 1          | 12         |
//! | repo-root valid-minimal                   | 1          | 5          |
//!
//! fitz opens ALL of the synthetic fixtures -- including the deliberately
//! mis-offset one -- which is exactly the real-world shape: the independent
//! baseline extracts text where pdftract yields errors or silently empty
//! output (re-validated 2026-09-22 with PyMuPDF 1.27.2.2; the numbers in the
//! table above reproduce byte-for-byte).

use std::path::{Path, PathBuf};

use pdftract_core::extract::extract_pdf;
use pdftract_core::sdk;
use pdftract_core::ExtractionOptions;

/// Repo-root fixtures (this crate lives at <repo>/crates/pdftract-core).
/// Never CWD-relative: CWD-relative fixture paths are a known failure mode
/// (tests/document_model.rs fails 15/15 at pristine HEAD for exactly this).
fn repo_fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/fixtures/{rel}"))
}

/// Fixtures owned by this crate's test suite.
fn core_fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{rel}"))
}

fn opts() -> ExtractionOptions {
    ExtractionOptions::default()
}

/// Extract, asserting success, and return the page count.
fn extract_pages(path: &Path) -> usize {
    match extract_pdf(path, &opts()) {
        Ok(result) => result.pages.len(),
        Err(e) => panic!(
            "extraction of {} failed: {e:#}\n(this pin expected extracts-ok; \
             if a fix landed and behavior changed, flip the pin)",
            path.display()
        ),
    }
}

/// Extract, asserting failure, and return the rendered error chain.
fn extract_failure(path: &Path) -> String {
    match extract_pdf(path, &opts()) {
        Ok(result) => panic!(
            "extraction of {} unexpectedly SUCCEEDED with {} pages\n\
             (this pin documents a current failure; a fix must have landed -- \
             flip the pin to the new behavior)",
            path.display(),
            result.pages.len()
        ),
        Err(e) => format!("{e:#}"),
    }
}

/// Full text via the SDK entry point, asserting success.
fn extract_text(path: &Path) -> String {
    sdk::extract_text(path, &opts())
        .unwrap_or_else(|e| panic!("text extraction of {} failed: {e:#}", path.display()))
}

// ---------------------------------------------------------------------------
// Class: /Prev incremental-update chain (Docusign offer letter class)
// ---------------------------------------------------------------------------

/// Base revision + 2 updates chained by /Prev, 3 %%EOF markers, /Root kept on
/// the base revision's catalog in the LAST trailer. PyMuPDF: 4 pp / 2,460
/// chars. The real 3-update Docusign document failed in the dogfood pilot;
/// this 2-update synthetic shape resolves at the pinning commit, so the pin
/// guards the /Prev walk against regression rather than documenting a failure.
#[test]
fn pin_incremental_updates_offer_letter_current_behavior() {
    let path = repo_fixture("realworld/incremental-updates-offer-letter.pdf");
    assert_eq!(extract_pages(&path), 4);
    let text = extract_text(&path);
    assert!(
        text.contains("OFFER OF EMPLOYMENT"),
        "expected offer-letter text, got {} chars",
        text.len()
    );
}

// ---------------------------------------------------------------------------
// Class: xref-stream-only (business report class)
// ---------------------------------------------------------------------------

/// No classic xref table anywhere; a single PDF 1.5 /Type /XRef stream.
/// PyMuPDF: 6 pp / 1,218 chars. Extracts at the pinning commit; regression pin.
#[test]
fn pin_xref_stream_only_report_current_behavior() {
    let path = repo_fixture("realworld/xref-stream-only-report.pdf");
    assert_eq!(extract_pages(&path), 6);
    let text = extract_text(&path);
    assert!(
        text.contains("QUARTERLY BUSINESS REPORT"),
        "expected report text, got {} chars",
        text.len()
    );
}

// ---------------------------------------------------------------------------
// Class: startxref offset does not land on the keyword (trailer loss)
// ---------------------------------------------------------------------------

/// Valid classic-xref doc whose startxref VALUE is 6 bytes short, landing
/// inside the last object's "endobj". Mirrors the 54-file corpus class
/// "No trailer in xref section" (2026-09-21 re-check, unowned before
/// pdftract-0df07688). FIXED by pdftract-0df07688: the bounded keyword
/// recovery around the recorded offset (XREF_KEYWORD_RECOVERY_WINDOW in
/// parser/xref.rs) finds the keyword 6 bytes forward, so the document now
/// resolves like PyMuPDF (2 pp). The text layer is still empty here --
/// the fixture's literals carry escaped parentheses, which is the separate
/// zeroed-text-layer class owned by pdftract-5b4c3d0e.
#[test]
fn pin_startxref_offset_edge_current_behavior() {
    let path = repo_fixture("realworld/startxref-offset-edge.pdf");
    assert_eq!(extract_pages(&path), 2);
}

#[test]
#[ignore = "full PyMuPDF parity (2,050 chars) is blocked by pdftract-5b4c3d0e \
           (escaped parentheses in literals zero the text layer); the \
           trailer-loss half was fixed and flipped by pdftract-0df07688"]
fn desired_startxref_offset_edge_extracts_like_pymupdf() {
    let path = repo_fixture("realworld/startxref-offset-edge.pdf");
    assert_eq!(extract_pages(&path), 2);
    assert!(extract_text(&path).contains("MUTUAL NON-DISCLOSURE"));
}

// ---------------------------------------------------------------------------
// Class: escaped parentheses in content-stream literals zero the text layer
// (dense single page with a valid page tree; resume / legal agreement class)
// ---------------------------------------------------------------------------

/// Plain valid single-page document whose literals contain escaped
/// parentheses ("(MUTUAL NON-DISCLOSURE AGREEMENT \(startxref offset edge
/// fixture\))"). PyMuPDF: 1 pp / 2,050 chars. CURRENT behavior at the
/// pinning commit: the page tree resolves and extraction returns Ok, but the
/// text layer is EMPTY -- one escaped paren zeroes every span on the page.
/// Isolation: the byte-identical document with the parens replaced by a dash
/// extracts 2,014 chars (probes in notes/pdftract-7ec0f722.md).
#[test]
fn pin_dense_one_page_agreement_current_behavior() {
    let path = repo_fixture("realworld/dense-one-page-agreement.pdf");
    assert_eq!(extract_pages(&path), 1);
    let text = extract_text(&path);
    assert!(
        text.is_empty(),
        "text layer no longer empty ({} chars) -- the escaped-paren fix \
         landed; flip this pin to assert the text",
        text.len()
    );
}

#[test]
#[ignore = "desired behavior -- flip when pdftract-5b4c3d0e (text layer \
           survives escaped parentheses in literals) lands"]
fn desired_dense_one_page_agreement_extracts_text() {
    let path = repo_fixture("realworld/dense-one-page-agreement.pdf");
    assert_eq!(extract_pages(&path), 1);
    assert!(extract_text(&path).contains("MUTUAL NON-DISCLOSURE"));
}

// ---------------------------------------------------------------------------
// Existing in-repo reproductions (committed fixtures probed 2026-09-21/22)
// ---------------------------------------------------------------------------

/// Linearized-format stub whose single classic xref section types EVERY
/// in-use entry `f` (mislabelled free) and mis-points several offsets
/// (obj 3's entry says 390, which holds `5 0 obj`; the catalog actually sits
/// at 222). At the pinning commit /Root was unresolved ("object 3 0 R not
/// found"): the resolver treated a Free entry as dead without ever
/// consulting its first field, so a document whose free-type fields are
/// producer garbage could not resolve anything. FLIPPED by pdftract-4196ae99:
/// when a section's free entries do not form the coherent free list the
/// spec requires, a Free entry with a nonzero first field is recovered
/// through the same verified parse as an in-use entry. PyMuPDF opens the
/// file but reports 0 pages -- the fixture is a stub; the pin asserts
/// resolution, not a page count.
#[test]
fn core_linearized_10_resolves_root() {
    let path = core_fixture("linearized-10.pdf");
    let pages = extract_pages(&path);
    assert_eq!(pages, 10, "10 page objects in the /Pages kids array");
}

#[test]
fn desired_core_linearized_10_resolves_root() {
    let path = core_fixture("linearized-10.pdf");
    assert_eq!(extract_pages(&path), 10);
}

/// 100-page doc with the same all-entries-mislabeled-free malformation:
/// entries 1, 2 and 3 all record 887 (which holds `3 0 obj`); the catalog
/// (obj 1) actually sits at 36. At the pinning commit /Root was unresolved
/// ("object 1 0 R not found"). FLIPPED by pdftract-4196ae99 (see the
/// linearized-10 note): the mislabelled-free recovery resolves the catalog
/// and the bounded offset rescan (pdftract-4683109d) lands each mis-pointed
/// entry on its real header.
#[test]
fn core_multipage_100_resolves_root() {
    let path = core_fixture("multipage-100.pdf");
    assert_eq!(extract_pages(&path), 100);
}

#[test]
fn desired_core_multipage_100_resolves_root() {
    let path = core_fixture("multipage-100.pdf");
    assert_eq!(extract_pages(&path), 100);
}

/// Minimal PDF whose page tree failed to flatten at the pinning commit
/// ("Document contains no pages") while PyMuPDF sees 1 pp / 12 chars.
/// Real-world class: the dogfood pilot's legal agreement and resume.
/// FLIPPED by pdftract-4683109d, not pdftract-bcf935ec: this fixture carries
/// the same drifted-xref-entry class as the repo-root W3C dummy (obj 3
/// recorded at 115, actually at 117 -- the page-tree walk killer behind the
/// old "Document contains no pages"; obj 4 at 268 vs 243; obj 5 at 345 vs
/// 313; startxref 439 vs 406), so the bounded object-boundary rescan
/// recovers the page tree and the content stream too. PyMuPDF parity note:
/// fitz sees 12 chars ("Hello World"); the pin asserts the word itself.
#[test]
fn pin_core_valid_minimal_current_behavior() {
    let path = core_fixture("valid-minimal.pdf");
    assert_eq!(extract_pages(&path), 1);
    let text = extract_text(&path);
    assert!(
        text.contains("Hello World"),
        "expected the core-suite text layer, got {} chars",
        text.len()
    );
}

#[test]
fn desired_core_valid_minimal_page_tree_resolves() {
    let path = core_fixture("valid-minimal.pdf");
    assert_eq!(extract_pages(&path), 1); // PyMuPDF: 1 pp
}

// ---------------------------------------------------------------------------
// Controls (the harness detects success where the failure map says PASS)
// ---------------------------------------------------------------------------

/// Repo-root W3C dummy PDF. Distinct file from the core-suite valid-minimal
/// above (PyMuPDF: 1 pp / 5 chars). The file's xref entry for the
/// content-stream object records offset 298 while the object actually sits
/// at 290; at the original pinning commit the shifted strict read failed the
/// object resolve and the page's text layer came out EMPTY while extraction
/// still returned Ok. FIXED by pdftract-4683109d: the bounded object-boundary
/// rescan around a failed entry offset (OBJECT_OFFSET_RECOVERY_WINDOW in
/// parser/xref.rs) finds the real "4 0 obj" header and the text extracts.
/// Isolation: fixing only that entry (298 -> 290, everything else still
/// nonconforming) made "Test" extract; fixing only the mis-pointed startxref
/// did not (probes in notes/pdftract-7ec0f722.md).
#[test]
fn pin_repo_root_valid_minimal_control_current_behavior() {
    let path = repo_fixture("valid-minimal.pdf");
    assert_eq!(extract_pages(&path), 1);
    let text = extract_text(&path);
    assert!(
        text.contains("Test"),
        "expected the W3C-dummy text layer, got {} chars",
        text.len()
    );
}

#[test]
fn desired_repo_root_valid_minimal_extracts_text() {
    let path = repo_fixture("valid-minimal.pdf");
    assert_eq!(extract_pages(&path), 1);
    assert!(extract_text(&path).contains("Test"));
}

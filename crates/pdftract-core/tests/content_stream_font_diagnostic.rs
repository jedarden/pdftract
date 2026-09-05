//! Regression tests for the `Tf` font-resolution diagnostics in content stream
//! processing (bead bf-3gm5ay).
//!
//! `Tf` binds a font *name*; resolving that name to an actual font is deferred
//! (Phase 3.2). The two outcomes must be reported distinctly:
//!
//! - name absent from the resource dictionary → `FONT_RESOURCE_NOT_FOUND`
//! - name present (resolution deferred) → **no** diagnostic
//!
//! The second case previously emitted `FONT_RESOURCE_NOT_FOUND` with a message
//! admitting the font *had* been found — a code/message mismatch that made the
//! diagnostic stream unusable as a signal for genuinely missing resources.

use pdftract_core::content_stream::{execute_with_do, ProcessingMode};
use pdftract_core::diagnostics::DiagCode;
use pdftract_core::parser::object::{intern, ObjRef};
use pdftract_core::parser::resources::ResourceDict;

fn run(content: &[u8], resources: &ResourceDict) -> Vec<DiagCode> {
    execute_with_do(
        content,
        resources,
        ProcessingMode::PositionHint,
        None,
        None,
        &[],
        None,
    )
    .diagnostics
    .into_iter()
    .map(|d| d.code)
    .collect()
}

#[test]
fn tf_with_font_absent_from_resources_reports_not_found() {
    let resources = ResourceDict::new();
    let codes = run(b"BT /UnknownFont 12 Tf ET", &resources);

    assert!(
        codes.contains(&DiagCode::FontResourceNotFound),
        "expected FONT_RESOURCE_NOT_FOUND when the font name is absent from resources, got {codes:?}"
    );
}

#[test]
fn tf_with_font_present_in_resources_emits_no_diagnostic() {
    let mut resources = ResourceDict::new();
    resources
        .fonts
        .insert(intern("F1"), ObjRef::new(1, 0));
    let codes = run(b"BT /F1 12 Tf ET", &resources);

    assert!(
        !codes.contains(&DiagCode::FontResourceNotFound),
        "font found in resources must not emit FONT_RESOURCE_NOT_FOUND, got {codes:?}"
    );
}

#[test]
fn tf_with_font_present_in_ancestor_scope_emits_no_diagnostic() {
    // A form XObject with no /Resources inherits the page scope. The name is
    // still resolvable, so the lookup must succeed and stay quiet.
    let mut resources = ResourceDict::new();
    resources
        .fonts
        .insert(intern("F1"), ObjRef::new(1, 0));
    let codes = run(b"q BT /F1 12 Tf ET Q", &resources);

    assert!(
        !codes.contains(&DiagCode::FontResourceNotFound),
        "font inherited from the page scope must not emit FONT_RESOURCE_NOT_FOUND, got {codes:?}"
    );
}

#[test]
fn tf_with_zero_size_still_reports_not_found_for_missing_font() {
    // The size clamp (FontSizeZeroOrNegative) is independent of font lookup:
    // a missing font must still be reported even when the size was clamped.
    let resources = ResourceDict::new();
    let codes = run(b"BT /UnknownFont 0 Tf ET", &resources);

    assert!(
        codes.contains(&DiagCode::FontResourceNotFound),
        "expected FONT_RESOURCE_NOT_FOUND alongside the size clamp, got {codes:?}"
    );
}

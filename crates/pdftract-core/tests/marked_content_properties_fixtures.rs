//! Integration tests for marked-content `/Properties` references (bead
//! pdftract-966fddc5, split child 1/4 of bf-1a61w9).
//!
//! The fixtures live in the workspace-root `tests/fixtures/tagged/` directory
//! and are documented there (`README.md` plus an entry in
//! `tests/fixtures/PROVENANCE.md`). Each fixture is a minimal tagged PDF whose
//! single page marks text with `/P /MC0 BDC ... EMC`, where `/MC0` is resolved
//! through the page `/Resources /Properties` namespace:
//!
//! - `mc_properties_indirect.pdf` — `/Properties << /MC0 4 0 R >>` where
//!   object 4 is `<< /MCID 0 /ActualText (Tagged content) >>`.
//! - `mc_properties_direct.pdf` — `/Properties << /MC0 << /MCID 0
//!   /ActualText (Tagged content) >> >>` (direct inline dict, no object 4).
//!
//! Unlike the unit tests in `marked_content_operators.rs` (which prime the
//! resolver with hand-built dicts via `cache_object`), these tests parse a
//! real file so the production chain is exercised: xref load ->
//! `resolve_with_source` -> `flatten_page_tree` (whose `merge_resources`
//! builds the page `ResourceDict`) -> `parse_bdc` ->
//! `ResourceDict::lookup_properties` -> resolver -> `/MCID` extraction.

use pdftract_core::parser::marked_content_operators::parse_bdc;
use pdftract_core::parser::marked_content_stack::MarkedContentStack;
use pdftract_core::parser::object::{intern, ObjRef, PdfObject};
use pdftract_core::parser::pages::{flatten_page_tree, PageDict};
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use std::path::PathBuf;

/// Locate a fixture under the workspace-root `tests/fixtures/tagged/`.
///
/// `CARGO_MANIFEST_DIR` is the compile-time absolute path of
/// `crates/pdftract-core`, so the first candidate works regardless of the
/// process CWD; the second covers a workspace-root CWD.
fn fixture_path(name: &str) -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tagged")
            .join(name),
        PathBuf::from("tests/fixtures/tagged").join(name),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    let tried: Vec<String> = candidates
        .iter()
        .map(|p| format!("  {}", p.display()))
        .collect();
    panic!("Fixture {} not found. Tried:\n{}", name, tried.join("\n"));
}

/// Parse a fixture PDF and return its single page plus the resolver.
///
/// `XrefResolver::resolve` is cache-only (unbacked lookups return Null), so
/// the page-tree nodes are warmed through `resolve_with_source` first — the
/// same way source-backed production consumers populate the cache — and the
/// page tree is then flattened through the regular production code, which
/// builds the page's merged `ResourceDict` via `merge_resources`.
fn load_fixture(name: &str) -> (PageDict, XrefResolver) {
    let path = fixture_path(name);
    let source = FileSource::open(&path)
        .unwrap_or_else(|e| panic!("Failed to open fixture {}: {}", path.display(), e));

    // Locate the startxref offset in the file tail.
    let size = source.len().expect("Failed to get fixture size");
    let read_size = 1024.min(size as usize);
    let tail_offset = size - read_size as u64;
    let tail = source
        .read_at(tail_offset, read_size)
        .expect("Failed to read fixture tail");
    // The binary comment marker near the head can leak into a 1KB tail of a
    // tiny fixture, so scan lossily; the startxref digits themselves are ASCII.
    let tail_str = String::from_utf8_lossy(&tail);
    let pos = tail_str
        .find("startxref")
        .unwrap_or_else(|| panic!("Fixture {} has no startxref", name));
    let startxref: u64 = tail_str[pos + "startxref".len()..]
        .split_whitespace()
        .next()
        .expect("startxref has no offset")
        .parse()
        .expect("startxref offset is not a number");

    let xref = load_xref_with_prev_chain(&source, startxref);
    let resolver = XrefResolver::from_section(xref.clone());

    // /Root -> catalog dict -> /Pages ref
    let root_ref = xref
        .trailer
        .as_ref()
        .and_then(|t| t.get("Root"))
        .and_then(|o| o.as_ref())
        .unwrap_or_else(|| panic!("Fixture {} trailer has no /Root", name));
    let catalog = resolver
        .resolve_with_source(root_ref, &source)
        .unwrap_or_else(|e| panic!("Failed to resolve catalog of {}: {}", name, e));
    let pages_ref = catalog
        .as_dict()
        .and_then(|d| d.get("Pages"))
        .and_then(|o| o.as_ref())
        .unwrap_or_else(|| panic!("Fixture {} catalog has no /Pages", name));

    // Warm the /Pages node and every /Kid so flatten_page_tree (which uses
    // the cache-only resolve) can walk the tree.
    resolver
        .resolve_with_source(pages_ref, &source)
        .unwrap_or_else(|e| panic!("Failed to resolve /Pages of {}: {}", name, e));
    let pages_node = resolver
        .resolve(pages_ref)
        .expect("Pages node was just cached");
    if let Some(kids) = pages_node
        .as_dict()
        .and_then(|d| d.get("Kids"))
        .and_then(|o| o.as_array())
    {
        for kid in kids.iter() {
            if let PdfObject::Ref(kid_ref) = kid {
                resolver
                    .resolve_with_source(*kid_ref, &source)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to resolve /Kids entry {} of {}: {}",
                            kid_ref, name, e
                        )
                    });
            }
        }
    }

    let pages = flatten_page_tree(&resolver, pages_ref).unwrap_or_else(|diags| {
        let msg = diags
            .first()
            .map(|d| d.message.to_string())
            .unwrap_or_default();
        panic!("Failed to flatten page tree of {}: {}", name, msg)
    });
    assert_eq!(
        pages.len(),
        1,
        "fixture {} must have exactly one page",
        name
    );
    let page = pages.into_iter().next().unwrap();

    // Warm any indirect /Properties targets: parse_bdc resolves property
    // references through the cache-only `resolve`, the same way this harness
    // warms the page-tree nodes above. (The direct-dict fixture has no
    // indirect targets, so this loop is a no-op for it.)
    for prop_obj in page.resources.properties.values() {
        if let PdfObject::Ref(prop_ref) = prop_obj {
            resolver
                .resolve_with_source(*prop_ref, &source)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to resolve /Properties target {} of {}: {}",
                        prop_ref, name, e
                    )
                });
        }
    }

    (page, resolver)
}

/// Run `/P /MC0 BDC` against a page's merged resources and return
/// (accepted, recovered MCID, diagnostics).
fn run_bdc_mc0(page: &PageDict, resolver: &XrefResolver) -> (bool, Option<u32>, Vec<String>) {
    // Content-stream name operands parse WITHOUT the leading slash
    // (Lexer::lex_name consumes it), matching how the object parser stores
    // dictionary keys.
    let props = PdfObject::Name(intern("MC0"));
    let mut stack = MarkedContentStack::new();
    let mut diagnostics = Vec::new();
    let accepted = parse_bdc(
        &mut stack,
        intern("P"),
        &props,
        &page.resources,
        None,
        Some(&mut diagnostics),
        Some(resolver),
    );
    let messages = diagnostics
        .iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>();
    (accepted, stack.innermost_mcid(), messages)
}

/// Fixture integrity (indirect): object 4 parses to the expected property
/// dict. Parsed dictionary keys carry NO leading slash (the lexer consumes
/// it), so `/MCID` is stored under the key `MCID` — this is the convention
/// `extract_mcid_from_dict` must match when it reads the resolved dict.
#[test]
fn fixture_indirect_property_object_parses_to_mcid_dict() {
    let name = "mc_properties_indirect.pdf";
    let path = fixture_path(name);
    let source = FileSource::open(&path).expect("Failed to open fixture");
    let size = source.len().expect("Failed to get fixture size");
    let tail = source
        .read_at(size - 256, 256)
        .expect("Failed to read fixture tail");
    let tail_str = std::str::from_utf8(&tail).expect("Fixture tail is not UTF-8");
    let pos = tail_str.find("startxref").expect("no startxref");
    let startxref: u64 = tail_str[pos + 9..]
        .split_whitespace()
        .next()
        .expect("startxref has no offset")
        .parse()
        .expect("startxref offset is not a number");
    let xref = load_xref_with_prev_chain(&source, startxref);
    let resolver = XrefResolver::from_section(xref);

    let obj = resolver
        .resolve_with_source(ObjRef::new(4, 0), &source)
        .expect("object 4 must resolve through the xref");
    let dict = obj
        .as_dict()
        .unwrap_or_else(|| panic!("object 4 must be a dictionary, got {:?}", obj));

    assert!(
        dict.contains_key("MCID"),
        "property dict keys are stored without the leading slash; got {:?}",
        dict.keys().collect::<Vec<_>>()
    );
    assert!(dict.contains_key("ActualText"));
    assert_eq!(dict.get("MCID"), Some(&PdfObject::Integer(0)));
}

/// Fixture 1: `/Properties << /MC0 4 0 R >>` — the property list is an
/// INDIRECT reference whose target dict carries `/MCID 0` and `/ActualText`.
///
/// The merge step must keep the reference (it stores `PdfObject::Ref`
/// entries, which is correct here), and `parse_bdc` must resolve it to
/// recover MCID 0.
#[test]
fn bdc_name_props_indirect_reference_recovers_mcid() {
    let (page, resolver) = load_fixture("mc_properties_indirect.pdf");

    // The indirect reference survived merge_resources into /Properties.
    assert_eq!(
        page.resources.lookup_properties("MC0"),
        Some(&PdfObject::Ref(ObjRef::new(4, 0))),
        "/Properties must map MC0 to indirect object 4"
    );

    let (accepted, mcid, diagnostics) = run_bdc_mc0(&page, &resolver);
    assert!(
        accepted,
        "parse_bdc should accept /P /MC0; diagnostics: {:?}",
        diagnostics
    );
    assert_eq!(
        mcid,
        Some(0),
        "MCID must be recovered from the /MC0 target dict \
         (4 0 R -> << /MCID 0 /ActualText ... >>); diagnostics: {:?}",
        diagnostics
    );
}

/// Fixture 2: `/Properties << /MC0 << /MCID 0 ... >> >>` — the property list
/// is a DIRECT inline dict in the page /Resources.
///
/// `merge_resources()` stores /Properties values verbatim (bead
/// pdftract-ff642b7b), so the direct dict survives the merge as
/// `PdfObject::Dict`, and `parse_bdc` reads /MCID straight out of it without
/// consulting the resolver.
#[test]
fn bdc_name_props_direct_inline_dict_recovers_mcid() {
    let (page, resolver) = load_fixture("mc_properties_direct.pdf");

    // The direct inline dict must survive merge_resources for this to pass.
    assert!(
        page.resources.lookup_properties("MC0").is_some(),
        "the direct inline /MC0 dict did not survive merge_resources"
    );

    let (accepted, mcid, diagnostics) = run_bdc_mc0(&page, &resolver);
    assert!(
        accepted,
        "parse_bdc should accept /P /MC0; diagnostics: {:?}",
        diagnostics
    );
    assert_eq!(
        mcid,
        Some(0),
        "MCID must be recovered from the direct inline /MC0 dict; diagnostics: {:?}",
        diagnostics
    );
}

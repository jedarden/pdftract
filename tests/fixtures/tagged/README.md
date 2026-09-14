# tests/fixtures/tagged — marked-content `/Properties` fixtures

Minimal hand-authored tagged PDFs (PDF 1.7, one page each) exercising the
`BDC /P /MC0 ... EMC` marked-content chain where the `/MC0` property name is
resolved through the page `/Resources /Properties` namespace.

Bead: **pdftract-966fddc5** (split child 1/4 of bf-1a61w9, "Resolve marked
content property references"). Test material only — no parser changes.

Consumer: `crates/pdftract-core/tests/marked_content_properties_fixtures.rs`.

## mc_properties_indirect.pdf

`/Properties << /MC0 4 0 R >>` — the property list is an **indirect
reference** whose target dict carries `/MCID 0` and `/ActualText`.

| Object | Contents |
|---|---|
| 1 | `/Type /Catalog /Pages 2 0 R` |
| 2 | `/Type /Pages /Kids [3 0 R] /Count 1` |
| 3 | `/Type /Page` — `/MediaBox [0 0 612 792]`, `/Resources <</Properties <</MC0 4 0 R>> /Font <</F1 5 0 R>>>>`, `/Contents 6 0 R` |
| 4 | **The property list:** `<< /MCID 0 /ActualText (Tagged content) >>` |
| 5 | `/Type /Font /Subtype /Type1 /BaseFont /Helvetica` |
| 6 | Content stream: unmarked line, then `/P /MC0 BDC BT /F1 12 Tf 72 700 Td (Tagged content) Tj ET EMC` |

Trailer: `/Size 7 /Root 1 0 R`; classic xref table; `startxref` at 576.

Expected chain: `parse_bdc` sees the name operand `MC0` (content-stream name
operands parse *without* the leading slash, as do dictionary keys) →
`ResourceDict::lookup_properties("MC0")` returns `ObjRef(4,0)` (this works —
`merge_resources` stores `PdfObject::Ref` entries) → the resolver resolves
object 4 → `/MCID` read → MCID `Some(0)`.

**Actual result at bead time (FAIL):** the lookup hits, but
`XrefResolver::resolve` is cache-only — an uncached object returns
`PdfObject::Null`, so `extract_mcid_from_props` emits
`BDC property 'MC0' resolved to non-dict object` and recovers no MCID.
Full resolution exists only as `resolve_with_source`, which `parse_bdc` does
not receive (and `process_with_mode` passes `None` outright). Additionally,
once a resolved dict *is* obtained from a real file, its keys are stored
**without** the leading slash (`MCID`), while `extract_mcid_from_dict` reads
`dict.get("/MCID")` — a second mismatch the fix must cover. The passing
integrity test `fixture_indirect_property_object_parses_to_mcid_dict` pins the
file itself as well-formed, so the defect is squarely in the resolution path.

## mc_properties_direct.pdf

`/Properties << /MC0 << /MCID 0 /ActualText (Tagged content) >> >>` — the
property list is a **direct inline dict**; there is no property object.

| Object | Contents |
|---|---|
| 1 | `/Type /Catalog /Pages 2 0 R` |
| 2 | `/Type /Pages /Kids [3 0 R] /Count 1` |
| 3 | `/Type /Page` — `/MediaBox [0 0 612 792]`, `/Resources <</Properties <</MC0 <</MCID 0 /ActualText (Tagged content)>>>> /Font <</F1 4 0 R>>>>`, `/Contents 5 0 R` |
| 4 | `/Type /Font /Subtype /Type1 /BaseFont /Helvetica` |
| 5 | Content stream: same shape as the indirect fixture |

Trailer: `/Size 6 /Root 1 0 R`; classic xref table.

**Confirmed gap:** `merge_resources()`
(`crates/pdftract-core/src/parser/resources.rs`, the `/Properties` merge —
`if let Some(ref_) = obj.as_ref() { merged.properties.insert(...) }`) **drops
the entry** because it only stores values where `obj.as_ref()` is `Some`; a
direct inline dict yields `None`. `/MC0` therefore never reaches
`ResourceDict.properties`, `lookup_properties("MC0")` misses, and
`parse_bdc` emits `UnknownMarkedContentProps`. Note the structural shape of
the fix: `ResourceDict.properties` is `IndexMap<Arc<str>, ObjRef>`, so keeping
the dict requires widening that type (the `/ColorSpace` namespace at
`resources.rs` already stores raw `PdfObject`s unconditionally — in-tree
precedent). Until child-2 lands, the desired-behavior test
`bdc_name_props_direct_inline_dict_recovers_mcid` is `#[ignore]`d with this
rationale in its message.

## Provenance

- **Generator:** hand-authored via a throwaway Python script (byte-exact xref
  offsets computed at write time); the script was not kept — the PDFs are
  779 / 736 bytes and fully described by the object tables above.
- **Generated:** 2026-09-14 (bead pdftract-966fddc5).
- Registered in `tests/fixtures/PROVENANCE.md`.

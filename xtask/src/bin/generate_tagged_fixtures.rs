//! Generate tagged PDF test fixtures for Phase 7.1 StructTree integration tests.
//!
//! This program creates tagged PDF test files:
//! - `tagged-ua-simple.pdf` — minimal PDF/UA-1 with heading + paragraph elements
//! - `tagged-ua-table.pdf` — PDF/UA with a tagged table (TR/TD structure)
//! - `tagged-a-2a.pdf` — PDF/A-2a document with StructTree
//! - `tagged-mcid-ordering.pdf` — document testing MCID-to-structure-element mapping
//!
//! All PDFs are written to tests/fixtures/tagged/.

use lopdf::dictionary;
use lopdf::object::{Dictionary, Object};
use lopdf::{Document, ObjectId};
use std::fs::File;
use std::io::Write;

fn main() {
    println!("Generating tagged PDF test fixtures...");

    create_tagged_ua_simple();
    create_tagged_ua_table();
    create_tagged_a_2a();
    create_tagged_mcid_ordering();

    println!("\nAll tagged fixtures generated successfully!");
}

/// Create a minimal PDF/UA-1 document with heading + paragraph elements.
///
/// Structure:
/// - Document root
///   - H1 (heading) with MCID 0, text "Chapter 1"
///   - P (paragraph) with MCID 1, text "This is a paragraph."
fn create_tagged_ua_simple() {
    let mut doc = Document::with_version("1.7");

    // Create font resource
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    // Page 1 resources
    let resources = dictionary! {
        "Font" => dictionary! {
            "F1" => font_dict
        }
    };

    // Content stream with marked content for H1 and P
    // BDC /P <</MCID 0>> starts marked content with MCID 0
    // EMC ends marked content
    let content = b"
BT
/F1 12 Tf
/DOI true
/Tag <</MCID 0>> BDC
100 700 Td
(Chapter 1) Tj
EMC
/Tag <</MCID 1>> BDC
100 680 Td
(This is a paragraph.) Tj
EMC
ET
";

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));

    // Page dictionary with /StructParents = 0
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", Object::Dictionary(resources));
    page_dict.set("Contents", Object::Reference(content_stream_id));
    page_dict.set("StructParents", Object::Integer(0)); // Key for ParentTree

    let page_id = doc.add_object(page_dict);

    // Pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![
        Object::Reference(page_id),
    ]));

    let pages_id = doc.add_object(pages_dict);

    // Update page parent reference
    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create structure elements

    // H1 element (MCID 0)
    let mut h1_dict = Dictionary::new();
    h1_dict.set("Type", "StructElem");
    h1_dict.set("S", "H1");
    h1_dict.set("Pg", Object::Reference(page_id));
    h1_dict.set("K", Object::Array(vec![Object::Integer(0)])); // MCID 0
    let h1_id = doc.add_object(h1_dict);

    // P element (MCID 1)
    let mut p_dict = Dictionary::new();
    p_dict.set("Type", "StructElem");
    p_dict.set("S", "P");
    p_dict.set("Pg", Object::Reference(page_id));
    p_dict.set("K", Object::Array(vec![Object::Integer(1)])); // MCID 1
    let p_id = doc.add_object(p_dict);

    // Document element
    let mut doc_elem_dict = Dictionary::new();
    doc_elem_dict.set("Type", "StructElem");
    doc_elem_dict.set("S", "Document");
    doc_elem_dict.set("K", Object::Array(vec![
        Object::Reference(h1_id),
        Object::Reference(p_id),
    ]));
    let doc_elem_id = doc.add_object(doc_elem_dict);

    // ParentTree: maps /StructParents value to array of StructElem refs (indexed by MCID)
    // /Nums array: [key0, value0, key1, value1, ...]
    let parent_tree_nums = Object::Array(vec![
        Object::Integer(0), // Key = /StructParents value
        Object::Array(vec![
            Object::Reference(h1_id), // MCID 0 -> H1 element
            Object::Reference(p_id),  // MCID 1 -> P element
        ].into()),
    ].into());

    let mut parent_tree_dict = Dictionary::new();
    parent_tree_dict.set("Nums", parent_tree_nums);
    let parent_tree_id = doc.add_object(parent_tree_dict);

    // StructTreeRoot
    let mut struct_tree_dict = Dictionary::new();
    struct_tree_dict.set("Type", "StructTreeRoot");
    struct_tree_dict.set("K", Object::Array(vec![
        Object::Reference(doc_elem_id),
    ]));
    struct_tree_dict.set("ParentTree", Object::Reference(parent_tree_id));
    let struct_tree_id = doc.add_object(struct_tree_dict);

    // MarkInfo (PDF/UA requires /Marked)
    let mut mark_info_dict = Dictionary::new();
    mark_info_dict.set("Marked", "true");
    let mark_info_id = doc.add_object(mark_info_dict);

    // Catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("StructTreeRoot", Object::Reference(struct_tree_id));
    catalog_dict.set("MarkInfo", Object::Reference(mark_info_id));
    // PDF/UA-1 requires /Lang
    catalog_dict.set("Lang", "en-US");

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Write the PDF
    let mut file = File::create("tests/fixtures/tagged/tagged-ua-simple.pdf").unwrap();
    file.write_all(doc.to_vec().as_slice()).unwrap();
    println!("Created tagged/tagged-ua-simple.pdf (PDF/UA-1 with H1 + P)");
}

/// Create a PDF/UA document with a tagged table.
///
/// Structure:
/// - Document root
///   - H1 heading
///   - Table element
///     - THead
///       - TR (row 1)
///         - TH (header cell 1) with MCID 2
///         - TH (header cell 2) with MCID 3
///     - TBody
///       - TR (row 2)
///         - TD (data cell 1) with MCID 4
///         - TD (data cell 2) with MCID 5
fn create_tagged_ua_table() {
    let mut doc = Document::with_version("1.7");

    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    let resources = dictionary! {
        "Font" => dictionary! {
            "F1" => font_dict
        }
    };

    // Content stream with marked content for table
    let content = b"
BT
/F1 12 Tf
/Tag <</MCID 0>> BDC
100 700 Td
(Table Example) Tj
EMC
/Tag <</MCID 2>> BDC
100 680 Td
(Header 1) Tj
EMC
/Tag <</MCID 3>> BDC
200 680 Td
(Header 2) Tj
EMC
/Tag <</MCID 4>> BDC
100 660 Td
(Data 1) Tj
EMC
/Tag <</MCID 5>> BDC
200 660 Td
(Data 2) Tj
EMC
ET
";

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", Object::Dictionary(resources));
    page_dict.set("Contents", Object::Reference(content_stream_id));
    page_dict.set("StructParents", Object::Integer(0));

    let page_id = doc.add_object(page_dict);

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));

    let pages_id = doc.add_object(pages_dict);

    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create structure elements for table

    // TH cell 1 (MCID 2)
    let mut th1_dict = Dictionary::new();
    th1_dict.set("Type", "StructElem");
    th1_dict.set("S", "TH");
    th1_dict.set("Pg", Object::Reference(page_id));
    th1_dict.set("K", Object::Array(vec![Object::Integer(2)]));
    let th1_id = doc.add_object(th1_dict);

    // TH cell 2 (MCID 3)
    let mut th2_dict = Dictionary::new();
    th2_dict.set("Type", "StructElem");
    th2_dict.set("S", "TH");
    th2_dict.set("Pg", Object::Reference(page_id));
    th2_dict.set("K", Object::Array(vec![Object::Integer(3)]));
    let th2_id = doc.add_object(th2_dict);

    // Row 1 (header row)
    let mut tr1_dict = Dictionary::new();
    tr1_dict.set("Type", "StructElem");
    tr1_dict.set("S", "TR");
    tr1_dict.set("K", Object::Array(vec![
        Object::Reference(th1_id),
        Object::Reference(th2_id),
    ]));
    let tr1_id = doc.add_object(tr1_dict);

    // TD cell 1 (MCID 4)
    let mut td1_dict = Dictionary::new();
    td1_dict.set("Type", "StructElem");
    td1_dict.set("S", "TD");
    td1_dict.set("Pg", Object::Reference(page_id));
    td1_dict.set("K", Object::Array(vec![Object::Integer(4)]));
    let td1_id = doc.add_object(td1_dict);

    // TD cell 2 (MCID 5)
    let mut td2_dict = Dictionary::new();
    td2_dict.set("Type", "StructElem");
    td2_dict.set("S", "TD");
    td2_dict.set("Pg", Object::Reference(page_id));
    td2_dict.set("K", Object::Array(vec![Object::Integer(5)]));
    let td2_id = doc.add_object(td2_dict);

    // Row 2 (data row)
    let mut tr2_dict = Dictionary::new();
    tr2_dict.set("Type", "StructElem");
    tr2_dict.set("S", "TR");
    tr2_dict.set("K", Object::Array(vec![
        Object::Reference(td1_id),
        Object::Reference(td2_id),
    ]));
    let tr2_id = doc.add_object(tr2_dict);

    // THead group
    let mut thead_dict = Dictionary::new();
    thead_dict.set("Type", "StructElem");
    thead_dict.set("S", "THead");
    thead_dict.set("K", Object::Array(vec![Object::Reference(tr1_id)]));
    let thead_id = doc.add_object(thead_dict);

    // TBody group
    let mut tbody_dict = Dictionary::new();
    tbody_dict.set("Type", "StructElem");
    tbody_dict.set("S", "TBody");
    tbody_dict.set("K", Object::Array(vec![Object::Reference(tr2_id)]));
    let tbody_id = doc.add_object(tbody_dict);

    // Table element
    let mut table_dict = Dictionary::new();
    table_dict.set("Type", "StructElem");
    table_dict.set("S", "Table");
    table_dict.set("K", Object::Array(vec![
        Object::Reference(thead_id),
        Object::Reference(tbody_id),
    ]));
    let table_id = doc.add_object(table_dict);

    // H1 heading (MCID 0)
    let mut h1_dict = Dictionary::new();
    h1_dict.set("Type", "StructElem");
    h1_dict.set("S", "H1");
    h1_dict.set("Pg", Object::Reference(page_id));
    h1_dict.set("K", Object::Array(vec![Object::Integer(0)]));
    let h1_id = doc.add_object(h1_dict);

    // Document element
    let mut doc_elem_dict = Dictionary::new();
    doc_elem_dict.set("Type", "StructElem");
    doc_elem_dict.set("S", "Document");
    doc_elem_dict.set("K", Object::Array(vec![
        Object::Reference(h1_id),
        Object::Reference(table_id),
    ]));
    let doc_elem_id = doc.add_object(doc_elem_dict);

    // ParentTree - note we use null (0 0 R) for MCID 1 which doesn't exist
    let parent_tree_nums = Object::Array(vec![
        Object::Integer(0),
        Object::Array(vec![
            Object::Reference(h1_id),
            Object::Reference(0.into()), // MCID 1 is null/unused
            Object::Reference(th1_id),
            Object::Reference(th2_id),
            Object::Reference(td1_id),
            Object::Reference(td2_id),
        ].into()),
    ].into());

    let mut parent_tree_dict = Dictionary::new();
    parent_tree_dict.set("Nums", parent_tree_nums);
    let parent_tree_id = doc.add_object(parent_tree_dict);

    let mut struct_tree_dict = Dictionary::new();
    struct_tree_dict.set("Type", "StructTreeRoot");
    struct_tree_dict.set("K", Object::Array(vec![Object::Reference(doc_elem_id)]));
    struct_tree_dict.set("ParentTree", Object::Reference(parent_tree_id));
    let struct_tree_id = doc.add_object(struct_tree_dict);

    let mut mark_info_dict = Dictionary::new();
    mark_info_dict.set("Marked", "true");
    let mark_info_id = doc.add_object(mark_info_dict);

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("StructTreeRoot", Object::Reference(struct_tree_id));
    catalog_dict.set("MarkInfo", Object::Reference(mark_info_id));
    catalog_dict.set("Lang", "en-US");

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut file = File::create("tests/fixtures/tagged/tagged-ua-table.pdf").unwrap();
    file.write_all(doc.to_vec().as_slice()).unwrap();
    println!("Created tagged/tagged-ua-table.pdf (PDF/UA with Table + TR/TD)");
}

/// Create a PDF/A-2a document with StructTree.
///
/// PDF/A-2a requires accessibility features (tagged PDF).
/// This fixture includes XMP metadata and OutputIntents for PDF/A compliance.
fn create_tagged_a_2a() {
    let mut doc = Document::with_version("2.0"); // PDF 2.0 for PDF/A-2

    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    let resources = dictionary! {
        "Font" => dictionary! {
            "F1" => font_dict
        }
    };

    let content = b"
BT
/F1 12 Tf
/Tag <</MCID 0>> BDC
100 700 Td
(PDF/A-2a Document) Tj
EMC
/Tag <</MCID 1>> BDC
100 680 Td
(This is a PDF/A-2a compliant tagged document.) Tj
EMC
ET
";

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", Object::Dictionary(resources));
    page_dict.set("Contents", Object::Reference(content_stream_id));
    page_dict.set("StructParents", Object::Integer(0));

    let page_id = doc.add_object(page_dict);

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));

    let pages_id = doc.add_object(pages_dict);

    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Structure elements
    let mut h1_dict = Dictionary::new();
    h1_dict.set("Type", "StructElem");
    h1_dict.set("S", "H1");
    h1_dict.set("Pg", Object::Reference(page_id));
    h1_dict.set("K", Object::Array(vec![Object::Integer(0)]));
    let h1_id = doc.add_object(h1_dict);

    let mut p_dict = Dictionary::new();
    p_dict.set("Type", "StructElem");
    p_dict.set("S", "P");
    p_dict.set("Pg", Object::Reference(page_id));
    p_dict.set("K", Object::Array(vec![Object::Integer(1)]));
    let p_id = doc.add_object(p_dict);

    let mut doc_elem_dict = Dictionary::new();
    doc_elem_dict.set("Type", "StructElem");
    doc_elem_dict.set("S", "Document");
    doc_elem_dict.set("K", Object::Array(vec![
        Object::Reference(h1_id),
        Object::Reference(p_id),
    ]));
    let doc_elem_id = doc.add_object(doc_elem_dict);

    let parent_tree_nums = Object::Array(vec![
        Object::Integer(0),
        Object::Array(vec![
            Object::Reference(h1_id),
            Object::Reference(p_id),
        ].into()),
    ].into());

    let mut parent_tree_dict = Dictionary::new();
    parent_tree_dict.set("Nums", parent_tree_nums);
    let parent_tree_id = doc.add_object(parent_tree_dict);

    let mut struct_tree_dict = Dictionary::new();
    struct_tree_dict.set("Type", "StructTreeRoot");
    struct_tree_dict.set("K", Object::Array(vec![Object::Reference(doc_elem_id)]));
    struct_tree_dict.set("ParentTree", Object::Reference(parent_tree_id));
    let struct_tree_id = doc.add_object(struct_tree_dict);

    // XMP Metadata for PDF/A-2a identification
    // Using \xEF\xBB\xBF for UTF-8 BOM instead of the Unicode character
    let xmp_uuid = b"<?xpacket begin=\"\xEF\xBB\xBF\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">
  <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">
    <rdf:Description rdf:about=\"\"
        xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">
      <pdfaid:part>2</pdfaid:part>
      <pdfaid:conformance>A</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end=\"w\"?>";

    let metadata_id = doc.new_object_id();
    doc.objects.insert(metadata_id, Object::Stream(lopdf::Stream::new(
        dictionary! {
            "Type" => "Metadata",
            "Subtype" => "XML"
        },
        xmp_uuid.to_vec()
    )));

    // OutputIntent (required for PDF/A)
    let mut dest_output_profile_dict = Dictionary::new();
    dest_output_profile_dict.set("Type", "OutputConditionIdentifier");
    dest_output_profile_dict.set("OutputConditionIdentifier", "sRGB IEC61966-2.1");
    let dest_profile_id = doc.add_object(dest_output_profile_dict);

    let mut output_intent_dict = Dictionary::new();
    output_intent_dict.set("Type", "OutputIntent");
    output_intent_dict.set("S", "GTS_PDFA1");
    output_intent_dict.set("OutputConditionIdentifier", "sRGB IEC61966-2.1");
    output_intent_dict.set("DestOutputProfile", Object::Reference(dest_profile_id));
    let output_intent_id = doc.add_object(output_intent_dict);

    let mut mark_info_dict = Dictionary::new();
    mark_info_dict.set("Marked", "true");
    let mark_info_id = doc.add_object(mark_info_dict);

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("StructTreeRoot", Object::Reference(struct_tree_id));
    catalog_dict.set("MarkInfo", Object::Reference(mark_info_id));
    catalog_dict.set("Metadata", Object::Reference(metadata_id));
    catalog_dict.set("OutputIntents", Object::Array(vec![Object::Reference(output_intent_id)]));
    catalog_dict.set("Lang", "en-US");

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut file = File::create("tests/fixtures/tagged/tagged-a-2a.pdf").unwrap();
    file.write_all(doc.to_vec().as_slice()).unwrap();
    println!("Created tagged/tagged-a-2a.pdf (PDF/A-2a with StructTree)");
}

/// Create a document testing MCID-to-structure-element mapping.
///
/// This fixture tests:
/// - Multiple MCIDs in sequence
/// - Non-sequential MCIDs
/// - Orphan MCIDs (MCIDs with no StructElem mapping)
/// - Nested structure elements with different MCIDs
fn create_tagged_mcid_ordering() {
    let mut doc = Document::with_version("1.7");

    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    let resources = dictionary! {
        "Font" => dictionary! {
            "F1" => font_dict
        }
    };

    // Content with specific MCID ordering to test mapping
    // MCIDs: 0, 1, 2, 3, 5, 6 (note: MCID 4 is intentionally skipped)
    let content = b"
BT
/F1 12 Tf
/Tag <</MCID 0>> BDC
100 700 Td
(MCID 0) Tj
EMC
/Tag <</MCID 1>> BDC
100 680 Td
(MCID 1) Tj
EMC
/Tag <</MCID 2>> BDC
100 660 Td
(MCID 2) Tj
EMC
/Tag <</MCID 3>> BDC
100 640 Td
(MCID 3) Tj
EMC
/Tag <</MCID 5>> BDC
100 620 Td
(MCID 5 - skipping 4) Tj
EMC
/Tag <</MCID 6>> BDC
100 600 Td
(MCID 6) Tj
ET
";

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", Object::Dictionary(resources));
    page_dict.set("Contents", Object::Reference(content_stream_id));
    page_dict.set("StructParents", Object::Integer(0));

    let page_id = doc.add_object(page_dict);

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));

    let pages_id = doc.add_object(pages_dict);

    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create structure elements for specific MCIDs

    // MCID 0
    let mut p0_dict = Dictionary::new();
    p0_dict.set("Type", "StructElem");
    p0_dict.set("S", "P");
    p0_dict.set("Pg", Object::Reference(page_id));
    p0_dict.set("K", Object::Array(vec![Object::Integer(0)]));
    let p0_id = doc.add_object(p0_dict);

    // MCID 1
    let mut p1_dict = Dictionary::new();
    p1_dict.set("Type", "StructElem");
    p1_dict.set("S", "P");
    p1_dict.set("Pg", Object::Reference(page_id));
    p1_dict.set("K", Object::Array(vec![Object::Integer(1)]));
    let p1_id = doc.add_object(p1_dict);

    // MCID 2 and 3 in a nested container (Div)
    let mut p2_dict = Dictionary::new();
    p2_dict.set("Type", "StructElem");
    p2_dict.set("S", "P");
    p2_dict.set("Pg", Object::Reference(page_id));
    p2_dict.set("K", Object::Array(vec![Object::Integer(2)]));
    let p2_id = doc.add_object(p2_dict);

    let mut p3_dict = Dictionary::new();
    p3_dict.set("Type", "StructElem");
    p3_dict.set("S", "P");
    p3_dict.set("Pg", Object::Reference(page_id));
    p3_dict.set("K", Object::Array(vec![Object::Integer(3)]));
    let p3_id = doc.add_object(p3_dict);

    let mut div_dict = Dictionary::new();
    div_dict.set("Type", "StructElem");
    div_dict.set("S", "Div");
    div_dict.set("K", Object::Array(vec![
        Object::Reference(p2_id),
        Object::Reference(p3_id),
    ]));
    let div_id = doc.add_object(div_dict);

    // MCID 5 and 6 (MCID 4 is intentionally an orphan - no StructElem mapping)
    let mut p5_dict = Dictionary::new();
    p5_dict.set("Type", "StructElem");
    p5_dict.set("S", "P");
    p5_dict.set("Pg", Object::Reference(page_id));
    p5_dict.set("K", Object::Array(vec![Object::Integer(5)]));
    let p5_id = doc.add_object(p5_dict);

    let mut p6_dict = Dictionary::new();
    p6_dict.set("Type", "StructElem");
    p6_dict.set("S", "P");
    p6_dict.set("Pg", Object::Reference(page_id));
    p6_dict.set("K", Object::Array(vec![Object::Integer(6)]));
    let p6_id = doc.add_object(p6_dict);

    // Document element
    let mut doc_elem_dict = Dictionary::new();
    doc_elem_dict.set("Type", "StructElem");
    doc_elem_dict.set("S", "Document");
    doc_elem_dict.set("K", Object::Array(vec![
        Object::Reference(p0_id),
        Object::Reference(p1_id),
        Object::Reference(div_id),
        Object::Reference(p5_id),
        Object::Reference(p6_id),
    ]));
    let doc_elem_id = doc.add_object(doc_elem_dict);

    // ParentTree with MCID 4 as null (orphan)
    // Array index maps to MCID value
    // Index 0 -> MCID 0 -> p0_id
    // Index 1 -> MCID 1 -> p1_id
    // Index 2 -> MCID 2 -> p2_id
    // Index 3 -> MCID 3 -> p3_id
    // Index 4 -> MCID 4 -> null (orphan)
    // Index 5 -> MCID 5 -> p5_id
    // Index 6 -> MCID 6 -> p6_id
    let parent_tree_nums = Object::Array(vec![
        Object::Integer(0),
        Object::Array(vec![
            Object::Reference(p0_id),
            Object::Reference(p1_id),
            Object::Reference(p2_id),
            Object::Reference(p3_id),
            Object::Reference(0.into()), // MCID 4 is orphan (null ref)
            Object::Reference(p5_id),
            Object::Reference(p6_id),
        ].into()),
    ].into());

    let mut parent_tree_dict = Dictionary::new();
    parent_tree_dict.set("Nums", parent_tree_nums);
    let parent_tree_id = doc.add_object(parent_tree_dict);

    let mut struct_tree_dict = Dictionary::new();
    struct_tree_dict.set("Type", "StructTreeRoot");
    struct_tree_dict.set("K", Object::Array(vec![Object::Reference(doc_elem_id)]));
    struct_tree_dict.set("ParentTree", Object::Reference(parent_tree_id));
    let struct_tree_id = doc.add_object(struct_tree_dict);

    let mut mark_info_dict = Dictionary::new();
    mark_info_dict.set("Marked", "true");
    let mark_info_id = doc.add_object(mark_info_dict);

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("StructTreeRoot", Object::Reference(struct_tree_id));
    catalog_dict.set("MarkInfo", Object::Reference(mark_info_id));
    catalog_dict.set("Lang", "en-US");

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut file = File::create("tests/fixtures/tagged/tagged-mcid-ordering.pdf").unwrap();
    file.write_all(doc.to_vec().as_slice()).unwrap();
    println!("Created tagged/tagged-mcid-ordering.pdf (MCID-to-StructElem mapping test)");
}

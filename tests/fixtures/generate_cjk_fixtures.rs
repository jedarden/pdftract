/// Generate CJK encoding test fixtures for Phase 2.3.
///
/// This creates minimal PDF fixtures with Type 0 composite fonts using CJK CMaps:
/// 1. cjk-chinese-gb18030.pdf - Simplified Chinese text in GB18030 encoding
/// 2. cjk-japanese-shiftjis.pdf - Japanese text using Shift-JIS CMaps
/// 3. cjk-korean-euckr.pdf - Korean text using EUC-KR CMaps
/// 4. cjk-tc-big5.pdf - Traditional Chinese with Big5 encoding
///
/// Each fixture includes:
/// - A Type 0 composite font with appropriate CMap
/// - ToUnicode CMap for proper decoding
/// - Sample text in the target encoding
/// - Corresponding .txt ground truth file
///
/// Run with: cargo run --bin generate_cjk_fixtures
///
/// These fixtures test the Phase 2.3 CJK encoding pipeline, specifically:
/// - Multi-byte code parsing via codespace ranges
/// - Encoding decoding via encoding_rs
/// - ToUnicode CMap resolution
///
/// Reference: Plan section 2.3 CJK Encoding (line 1389-1415)

use lopdf::lopdf::{self, dictionary, Document, Object, Stream, StringFormat};

/// Helper to create a minimal PDF with CJK text
struct CjkPdfBuilder {
    objects: Vec<Object>,
    offsets: Vec<usize>,
}

impl CjkPdfBuilder {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            offsets: Vec::new(),
        }
    }

    fn add_object(&mut self, obj: Object) -> usize {
        self.objects.push(obj);
        self.objects.len()
    }

    fn build(self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use lopdf::{Document, ObjectId};
        use std::io::Cursor;

        let mut doc = Document::with_version("1.4");

        // Build document from objects
        for (i, obj) in self.objects.into_iter().enumerate() {
            doc.objects.insert(ObjectId::new(i as u16, 0), obj);
        }

        // Set trailer
        let mut catalog_id = ObjectId::new(1, 0);
        doc.trailer.set("Root", catalog_id);

        doc.save(output_path)?;
        Ok(())
    }
}

/// Create GB18030 (Simplified Chinese) fixture
fn create_gb18030_fixture() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, StringFormat};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::with_version("1.4");

    // Add pages
    let pages_id = doc.new_object_id();
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(doc.get_object_id(&3)?), Object::Reference(doc.get_object_id(&7)?)],
        "Count" => 2,
    }));

    // Page 1: GB18030 encoded text
    let (page1_id, content1_id) = doc.add_page([
        (Box::new("MediaBox").into(), Box::new(Object::Rectangle(vec![0, 0, 612, 792])).into()),
        (Box::new("Parent").into(), Box::new(Object::Reference(pages_id)).into()),
        (Box::new("Resources").into(), Box::new(Object::Dictionary(dictionary! {
            "Font" => Object::Dictionary(dictionary! {
                "F1" => Object::Reference(doc.get_object_id(&4)?),
            }),
        })).into()),
    ], None);

    // Content stream for page 1 - minimal Chinese text
    let mut content1 = Content::new();
    content1.operations.push(Operation::new("BT", vec![]));
    content1.operations.push(Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(12)]));
    content1.operations.push(Operation::new("Td", vec![Object::Integer(50), Object::Integer(700)]));

    // "Hello World" in Simplified Chinese encoded as GB18030 byte sequence
    let chinese_text = vec![0xC4, 0xE3, 0xBA, 0xC3, 0xCA, 0xC0, 0xBD, 0xE7];
    content1.operations.push(Operation::new("Tj", vec![Object::String(chinese_text, StringFormat::Literal)]));
    content1.operations.push(Operation::new("ET", vec![]));

    let content_stream_id = doc.add_object(Stream::new(dictionary! {}, content1.operations.to_bytes()?));

    // Update page content reference
    if let Ok(Object::Dictionary(ref mut page)) = doc.get_object_mut(page1_id) {
        page.set("Contents", Object::Reference(content_stream_id));
    }

    // Font dictionary - Type 0 with GB18030 CMap
    let font_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "AdobeSongStd-Light",
        "Encoding" => "GBpc-EUC-H",  // GB18030 encoding
        "DescendantFonts" => vec![Object::Reference(doc.get_object_id(&5)?)],
    }));

    // CIDFont descendant
    let cidfont_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType0",
        "BaseFont" => "AdobeSongStd-Light",
        "CIDSystemInfo" => Object::Dictionary(dictionary! {
            "Registry" => Object::String(b"Adobe".to_vec(), StringFormat::Literal),
            "Ordering" => Object::String(b"GB1".to_vec(), StringFormat::Literal),
            "Supplement" => 0,
        }),
    }));

    // ToUnicode CMap for GB18030
    let cmap_stream = Stream::new(dictionary! {}, b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
4 beginbfchar
<0xD4D3> <4F60>
<0xBAC3> <597D>
<0xCAC0> <4E16>
<0xBDE7> <754C>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
".to_vec());
    let cmap_id = doc.add_object(Object::Stream(cmap_stream));

    // Update font with ToUnicode
    if let Ok(Object::Dictionary(ref mut font)) = doc.get_object_mut(font_id) {
        font.set("ToUnicode", Object::Reference(cmap_id));
    }

    // Page 2: More GB18030 text
    let (page2_id, _) = doc.add_page([
        (Box::new("MediaBox").into(), Box::new(Object::Rectangle(vec![0, 0, 612, 792])).into()),
        (Box::new("Parent").into(), Box::new(Object::Reference(pages_id)).into()),
        (Box::new("Resources").into(), Box::new(Object::Dictionary(dictionary! {
            "Font" => Object::Dictionary(dictionary! {
                "F1" => Object::Reference(font_id),
            }),
        })).into()),
    ], None);

    let mut content2 = Content::new();
    content2.operations.push(Operation::new("BT", vec![]));
    content2.operations.push(Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(12)]));
    content2.operations.push(Operation::new("Td", vec![Object::Integer(50), Object::Integer(700)]));

    // "Test Chinese" text
    let test_text = vec![0xB2, 0xE2, 0xCA, 0xD4, 0xD6, 0xD0, 0xCE, 0xC4];
    content2.operations.push(Operation::new("Tj", vec![Object::String(test_text, StringFormat::Literal)]));
    content2.operations.push(Operation::new("ET", vec![]));

    let content2_id = doc.add_object(Stream::new(dictionary! {}, content2.operations.to_bytes()?));
    if let Ok(Object::Dictionary(ref mut page)) = doc.get_object_mut(page2_id) {
        page.set("Contents", Object::Reference(content2_id));
    }

    // Set catalog
    let catalog_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    }));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save
    let pdf_path = "tests/fixtures/cjk/cjk-chinese-gb18030.pdf";
    doc.save(pdf_path)?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt
    let truth_path = "tests/fixtures/cjk/cjk-chinese-gb18030.txt";
    // "Hello World\nTest Chinese" in Simplified Chinese
    let truth_content = "\xE4\xBD\xA0\xE5\xA5\xBD\xE4\xB8\x96\xE7\x95\x8C\n\xE6\xB5\x8B\xE8\xAF\x95\xE4\xB8\xAD\xE6\x96\x87";
    std::fs::write(truth_path, truth_content)?;
    println!("Created: {}", truth_path);

    Ok(())
}

/// Create Shift-JIS (Japanese) fixture
fn create_shiftjis_fixture() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, StringFormat};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::with_version("1.4");

    // Pages
    let pages_id = doc.new_object_id();
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(doc.get_object_id(&3)?)],
        "Count" => 1,
    }));

    // Page
    let (page_id, _) = doc.add_page([
        (Box::new("MediaBox").into(), Box::new(Object::Rectangle(vec![0, 0, 612, 792])).into()),
        (Box::new("Parent").into(), Box::new(Object::Reference(pages_id)).into()),
        (Box::new("Resources").into(), Box::new(Object::Dictionary(dictionary! {
            "Font" => Object::Dictionary(dictionary! {
                "F1" => Object::Reference(doc.get_object_id(&4)?),
            }),
        })).into()),
    ], None);

    // Font - Type 0 with Shift-JIS encoding
    let font_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "HeiseiMin-W3",
        "Encoding" => "90ms-RKSJ-H",  // Shift-JIS encoding
        "DescendantFonts" => vec![Object::Reference(doc.get_object_id(&5)?)],
    }));

    // CIDFont
    let cidfont_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType0",
        "BaseFont" => "HeiseiMin-W3",
        "CIDSystemInfo" => Object::Dictionary(dictionary! {
            "Registry" => Object::String(b"Adobe".to_vec(), StringFormat::Literal),
            "Ordering" => Object::String(b"Japan1".to_vec(), StringFormat::Literal),
            "Supplement" => 4,
        }),
    }));

    // ToUnicode CMap for Shift-JIS
    let cmap_stream = Stream::new(dictionary! {}, b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
6 beginbfchar
<838B> <3053>
<818F> <308C>
<82C5> <304B>
<82A0> <3042>
<838E> <3057>
<82CD> <3044>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
".to_vec());
    let cmap_id = doc.add_object(Object::Stream(cmap_stream));

    // Update font with ToUnicode
    if let Ok(Object::Dictionary(ref mut font)) = doc.get_object_mut(font_id) {
        font.set("ToUnicode", Object::Reference(cmap_id));
    }

    // Content stream - "こんにちは" (Konnichiwa - Hello)
    let mut content = Content::new();
    content.operations.push(Operation::new("BT", vec![]));
    content.operations.push(Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(12)]));
    content.operations.push(Operation::new("Td", vec![Object::Integer(50), Object::Integer(700)]));

    // Shift-JIS byte sequence for "Hello" in Japanese
    let japanese_text = vec![0x83, 0x8B, 0x82, 0x8F, 0x82, 0xC5, 0x82, 0xA0, 0x82, 0xB5, 0x82, 0xCD];
    content.operations.push(Operation::new("Tj", vec![Object::String(japanese_text, StringFormat::Literal)]));
    content.operations.push(Operation::new("ET", vec![]));

    let content_id = doc.add_object(Stream::new(dictionary! {}, content.operations.to_bytes()?));
    if let Ok(Object::Dictionary(ref mut page)) = doc.get_object_mut(page_id) {
        page.set("Contents", Object::Reference(content_id));
    }

    // Catalog
    let catalog_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    }));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save
    let pdf_path = "tests/fixtures/cjk/cjk-japanese-shiftjis.pdf";
    doc.save(pdf_path)?;
    println!("Created: {}", pdf_path);

    // Ground truth
    let truth_path = "tests/fixtures/cjk/cjk-japanese-shiftjis.txt";
    // "Hello" in Japanese (Konnichiwa)
    let truth = "\xE3\x81\x93\xE3\x82\x93\xE3\x81\xAB\xE3\x81\xA1\xE3\x81\xAF";
    std::fs::write(truth_path, truth)?;
    println!("Created: {}", truth_path);

    Ok(())
}

/// Create EUC-KR (Korean) fixture
fn create_euckr_fixture() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, StringFormat};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::with_version("1.4");

    let pages_id = doc.new_object_id();
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(doc.get_object_id(&3)?)],
        "Count" => 1,
    }));

    let (page_id, _) = doc.add_page([
        (Box::new("MediaBox").into(), Box::new(Object::Rectangle(vec![0, 0, 612, 792])).into()),
        (Box::new("Parent").into(), Box::new(Object::Reference(pages_id)).into()),
        (Box::new("Resources").into(), Box::new(Object::Dictionary(dictionary! {
            "Font" => Object::Dictionary(dictionary! {
                "F1" => Object::Reference(doc.get_object_id(&4)?),
            }),
        })).into()),
    ], None);

    // Font with EUC-KR encoding
    let font_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "HYSMyeongJo-Medium",
        "Encoding" => "KSCms-UHC-H",  // EUC-KR encoding
        "DescendantFonts" => vec![Object::Reference(doc.get_object_id(&5)?)],
    }));

    let cidfont_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType0",
        "BaseFont" => "HYSMyeongJo-Medium",
        "CIDSystemInfo" => Object::Dictionary(dictionary! {
            "Registry" => Object::String(b"Adobe".to_vec(), StringFormat::Literal),
            "Ordering" => Object::String(b"Korea1".to_vec(), StringFormat::Literal),
            "Supplement" => 1,
        }),
    }));

    // ToUnicode CMap for EUC-KR
    let cmap_stream = Stream::new(dictionary! {}, b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
4 beginbfchar
<C5E5> <C548>
<B3D7> <B155>
<C7CF> <D55C>
<B1E8> <AD6D>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
".to_vec());
    let cmap_id = doc.add_object(Object::Stream(cmap_stream));

    if let Ok(Object::Dictionary(ref mut font)) = doc.get_object_mut(font_id) {
        font.set("ToUnicode", Object::Reference(cmap_id));
    }

    // Content - "Hello" in Korean (Annyeonghaseyo)
    let korean_text = vec![0xC5, 0xE5, 0xB3, 0xD7, 0xC7, 0xCF, 0xBC, 0xE9, 0xBF, 0xF4];

    let mut content = Content::new();
    content.operations.push(Operation::new("BT", vec![]));
    content.operations.push(Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(12)]));
    content.operations.push(Operation::new("Td", vec![Object::Integer(50), Object::Integer(700)]));
    content.operations.push(Operation::new("Tj", vec![Object::String(korean_text, StringFormat::Literal)]));
    content.operations.push(Operation::new("ET", vec![]));

    let content_id = doc.add_object(Stream::new(dictionary! {}, content.operations.to_bytes()?));
    if let Ok(Object::Dictionary(ref mut page)) = doc.get_object_mut(page_id) {
        page.set("Contents", Object::Reference(content_id));
    }

    let catalog_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    }));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save
    let pdf_path = "tests/fixtures/cjk/cjk-korean-euckr.pdf";
    doc.save(pdf_path)?;
    println!("Created: {}", pdf_path);

    // Ground truth
    let truth_path = "tests/fixtures/cjk/cjk-korean-euckr.txt";
    // "Hello" in Korean (Annyeonghaseyo)
    let truth = "\xEC\x95\x88\xEB\x85\x95\xED\x95\x98\xEC\x84\xB8\xEC\x9A\x94";
    std::fs::write(truth_path, truth)?;
    println!("Created: {}", truth_path);

    Ok(())
}

/// Create Big5 (Traditional Chinese) fixture
fn create_big5_fixture() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, StringFormat};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::with_version("1.4");

    let pages_id = doc.new_object_id();
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(doc.get_object_id(&3)?)],
        "Count" => 1,
    }));

    let (page_id, _) = doc.add_page([
        (Box::new("MediaBox").into(), Box::new(Object::Rectangle(vec![0, 0, 612, 792])).into()),
        (Box::new("Parent").into(), Box::new(Object::Reference(pages_id)).into()),
        (Box::new("Resources").into(), Box::new(Object::Dictionary(dictionary! {
            "Font" => Object::Dictionary(dictionary! {
                "F1" => Object::Reference(doc.get_object_id(&4)?),
            }),
        })).into()),
    ], None);

    // Font with Big5 encoding
    let font_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "PMingLiU-Light",
        "Encoding" => "ETen-B5-H",  // Big5 encoding
        "DescendantFonts" => vec![Object::Reference(doc.get_object_id(&5)?)],
    }));

    let cidfont_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType0",
        "BaseFont" => "PMingLiU-Light",
        "CIDSystemInfo" => Object::Dictionary(dictionary! {
            "Registry" => Object::String(b"Adobe".to_vec(), StringFormat::Literal),
            "Ordering" => Object::String(b"CNS1".to_vec(), StringFormat::Literal),
            "Supplement" => 0,
        }),
    }));

    // ToUnicode CMap for Big5
    let cmap_stream = Stream::new(dictionary! {}, b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
4 beginbfchar
<A741> <4F60>  // 你
<A753> <597D>  // 好
<A4E1> <4E16>  // 世
        <A4C1> <754C>  // 界
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
".to_vec());
    let cmap_id = doc.add_object(Object::Stream(cmap_stream));

    if let Ok(Object::Dictionary(ref mut font)) = doc.get_object_mut(font_id) {
        font.set("ToUnicode", Object::Reference(cmap_id));
    }

    // Content - "你好世界" (Hello World in Traditional Chinese)
    // 你: 0xA7 0x41, 好: 0xA7 0x53, 世: 0xA4 0xE1, 界: 0xA4 0xC1
    let tc_text = vec![0xA7, 0x41, 0xA7, 0x53, 0xA4, 0xE1, 0xA4, 0xC1];

    let mut content = Content::new();
    content.operations.push(Operation::new("BT", vec![]));
    content.operations.push(Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(12)]));
    content.operations.push(Operation::new("Td", vec![Object::Integer(50), Object::Integer(700)]));
    content.operations.push(Operation::new("Tj", vec![Object::String(tc_text, StringFormat::Literal)]));
    content.operations.push(Operation::new("ET", vec![]));

    let content_id = doc.add_object(Stream::new(dictionary! {}, content.operations.to_bytes()?));
    if let Ok(Object::Dictionary(ref mut page)) = doc.get_object_mut(page_id) {
        page.set("Contents", Object::Reference(content_id));
    }

    let catalog_id = doc.add_object(Object::Dictionary(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    }));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save
    let pdf_path = "tests/fixtures/cjk/cjk-tc-big5.pdf";
    doc.save(pdf_path)?;
    println!("Created: {}", pdf_path);

    // Ground truth
    let truth_path = "tests/fixtures/cjk/cjk-tc-big5.txt";
    std::fs::write(truth_path, "你好世界")?;
    println!("Created: {}", truth_path);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating CJK encoding fixtures for Phase 2.3...\n");

    println!("Creating GB18030 (Simplified Chinese) fixture...");
    create_gb18030_fixture()?;
    println!();

    println!("Creating Shift-JIS (Japanese) fixture...");
    create_shiftjis_fixture()?;
    println!();

    println!("Creating EUC-KR (Korean) fixture...");
    create_euckr_fixture()?;
    println!();

    println!("Creating Big5 (Traditional Chinese) fixture...");
    create_big5_fixture()?;
    println!();

    println!("All CJK fixtures generated successfully!");
    println!("\nFixtures created:");
    println!("  tests/fixtures/cjk/cjk-chinese-gb18030.pdf (+ .txt)");
    println!("  tests/fixtures/cjk/cjk-japanese-shiftjis.pdf (+ .txt)");
    println!("  tests/fixtures/cjk/cjk-korean-euckr.pdf (+ .txt)");
    println!("  tests/fixtures/cjk/cjk-tc-big5.pdf (+ .txt)");

    Ok(())
}

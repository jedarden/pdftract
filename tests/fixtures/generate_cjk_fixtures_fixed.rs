/// Generate CJK encoding test fixtures for Phase 2.3.
///
/// This creates minimal PDF fixtures with CJK text.
/// Run with: rustc --edition 2021 generate_cjk_fixtures_fixed.rs --extern lopdf=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="lopdf") | .targets[0].src_path' | xargs dirname)/target/release/deps/liblopdf*.rlib -L $(cargo metadata --format-version 1 | jq -r '.target_directory')/release/deps

use std::fs;
use std::path::Path;

// Since lopdf isn't available as a simple binary dependency, we'll create the fixtures
// using a simpler approach - writing PDF structure directly with proper CJK encoding

fn create_gb18030_fixture() -> Result<(), Box<dyn std::error::Error>> {
    // For GB18030, we create a minimal PDF with proper structure
    // The ground truth is UTF-8 encoded Chinese text: "你好世界\n测试中文"
    let truth_content = "你好世界\n测试中文";
    let truth_path = "tests/fixtures/cjk/cjk-chinese-gb18030.txt";

    // Ensure directory exists
    fs::create_dir_all("tests/fixtures/cjk")?;
    fs::write(truth_path, truth_content)?;
    println!("Created: {}", truth_path);

    // Create minimal PDF with GB18030 encoded content
    // For simplicity, we'll create a basic PDF structure
    let pdf_content = format!("%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type0
/BaseFont /AdobeSongStd-Light
/Encoding /GBpc-EUC-H
/DescendantFonts [6 0 R]
/ToUnicode 7 0 R
>>
endobj
5 0 obj
<<
/Length 0
>>
stream
BT
/F1 12 Tf
50 700 Td
<C4E3BAC3CAC0BDE7> Tj
ET
endstream
endobj
6 0 obj
<<
/Type /Font
/Subtype /CIDFontType0
/BaseFont /AdobeSongStd-Light
/CIDSystemInfo <<
/Registry (Adobe)
/Ordering (GB1)
/Supplement 0
>>
>>
endobj
7 0 obj
<<
/Length 132
>>
stream
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
4 beginbfchar
<D4D3> <4F60>
<BAC3> <597D>
<CAC0> <4E16>
<BDE7> <754C>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000367 00000 n
0000000424 00000 n
0000000513 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
641
%%EOF");

    let pdf_path = "tests/fixtures/cjk/cjk-chinese-gb18030.pdf";
    fs::write(pdf_path, pdf_content)?;
    println!("Created: {}", pdf_path);

    Ok(())
}

fn create_shiftjis_fixture() -> Result<(), Box<dyn std::error::Error>> {
    // Ground truth: "こんにちは" (Hello in Japanese)
    let truth = "こんにちは";
    let truth_path = "tests/fixtures/cjk/cjk-japanese-shiftjis.txt";
    fs::write(truth_path, truth)?;
    println!("Created: {}", truth_path);

    // Create minimal PDF with Shift-JIS encoded content
    let pdf_content = format!("%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type0
/BaseFont /HeiseiMin-W3
/Encoding /90ms-RKSJ-H
/DescendantFonts [6 0 R]
/ToUnicode 7 0 R
>>
endobj
5 0 obj
<<
/Length 0
>>
stream
BT
/F1 12 Tf
50 700 Td
<838B818F82C582A0838E82CD> Tj
ET
endstream
endobj
6 0 obj
<<
/Type /Font
/Subtype /CIDFontType0
/BaseFont /HeiseiMin-W3
/CIDSystemInfo <<
/Registry (Adobe)
/Ordering (Japan1)
/Supplement 4
>>
>>
endobj
7 0 obj
<<
/Length 132
>>
stream
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
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000367 00000 n
0000000424 00000 n
0000000513 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
641
%%EOF");

    let pdf_path = "tests/fixtures/cjk/cjk-japanese-shiftjis.pdf";
    fs::write(pdf_path, pdf_content)?;
    println!("Created: {}", pdf_path);

    Ok(())
}

fn create_euckr_fixture() -> Result<(), Box<dyn std::error::Error>> {
    // Ground truth: "안녕하세요" (Hello in Korean)
    let truth = "안녕하세요";
    let truth_path = "tests/fixtures/cjk/cjk-korean-euckr.txt";
    fs::write(truth_path, truth)?;
    println!("Created: {}", truth_path);

    // Create minimal PDF with EUC-KR encoded content
    let pdf_content = format!("%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type0
/BaseFont /HYSMyeongJo-Medium
/Encoding /KSCms-UHC-H
/DescendantFonts [6 0 R]
/ToUnicode 7 0 R
>>
endobj
5 0 obj
<<
/Length 0
>>
stream
BT
/F1 12 Tf
50 700 Td
<C5E5B3D7C7CFB1E8> Tj
ET
endstream
endobj
6 0 obj
<<
/Type /Font
/Subtype /CIDFontType0
/BaseFont /HYSMyeongJo-Medium
/CIDSystemInfo <<
/Registry (Adobe)
/Ordering (Korea1)
/Supplement 1
>>
>>
endobj
7 0 obj
<<
/Length 132
>>
stream
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
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000367 00000 n
0000000424 00000 n
0000000513 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
641
%%EOF");

    let pdf_path = "tests/fixtures/cjk/cjk-korean-euckr.pdf";
    fs::write(pdf_path, pdf_content)?;
    println!("Created: {}", pdf_path);

    Ok(())
}

fn create_big5_fixture() -> Result<(), Box<dyn std::error::Error>> {
    // Ground truth: "你好世界" (Hello World in Traditional Chinese)
    let truth = "你好世界";
    let truth_path = "tests/fixtures/cjk/cjk-tc-big5.txt";
    fs::write(truth_path, truth)?;
    println!("Created: {}", truth_path);

    // Create minimal PDF with Big5 encoded content
    let pdf_content = format!("%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 4 0 R
>>
>>
/Contents 5 0 R
>>
endobj
4 0 obj
<<
/Type /Font
/Subtype /Type0
/BaseFont /PMingLiU-Light
/Encoding /ETen-B5-H
/DescendantFonts [6 0 R]
/ToUnicode 7 0 R
>>
endobj
5 0 obj
<<
/Length 0
>>
stream
BT
/F1 12 Tf
50 700 Td
<A741A753A4E1A4C1> Tj
ET
endstream
endobj
6 0 obj
<<
/Type /Font
/Subtype /CIDFontType0
/BaseFont /PMingLiU-Light
/CIDSystemInfo <<
/Registry (Adobe)
/Ordering (CNS1)
/Supplement 0
>>
>>
endobj
7 0 obj
<<
/Length 132
>>
stream
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
4 beginbfchar
<A741> <4F60>
<A753> <597D>
<A4E1> <4E16>
<A4C1> <754C>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000367 00000 n
0000000424 00000 n
0000000513 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
641
%%EOF");

    let pdf_path = "tests/fixtures/cjk/cjk-tc-big5.pdf";
    fs::write(pdf_path, pdf_content)?;
    println!("Created: {}", pdf_path);

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

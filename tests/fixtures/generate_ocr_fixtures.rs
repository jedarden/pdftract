//! Generate OCR test fixtures.
//!
//! This script creates three types of OCR fixtures:
//! 1. Clean Lorem Ipsum at 300 DPI (WER < 2% target)
//! 2. Multi-language English+French (WER < 3% target)
//! 3. 10-page performance fixture
//!
//! Usage: cargo run --bin generate_ocr_fixtures

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating OCR test fixtures...");

    // Generate clean Lorem Ipsum fixture
    generate_clean_lorem_ipsum()?;

    // Generate multi-language fixture
    generate_multi_language()?;

    // Generate 10-page performance fixture
    generate_performance_fixture()?;

    println!("All OCR fixtures generated successfully!");
    Ok(())
}

fn generate_clean_lorem_ipsum() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating clean_lorem_ipsum fixture...");

    let output_dir = Path::new("tests/fixtures/ocr/clean_lorem_ipsum");
    fs::create_dir_all(output_dir)?;

    // Ground truth text (Lorem Ipsum)
    let ground_truth = r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.

Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur.

Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur, vel illum qui dolorem eum fugiat quo voluptas nulla pariatur? At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident.

Similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus."#;

    // Write ground truth
    let gt_path = output_dir.join("ground_truth.txt");
    let mut gt_file = File::create(&gt_path)?;
    gt_file.write_all(ground_truth.as_bytes())?;

    // Create a simple text file that can be converted to PDF
    // For a real implementation, we'd use a PDF library like printpdf or lopdf
    // For now, we'll create a README explaining how to generate the PDF
    let readme = r#"# Clean Lorem Ipsum Fixture

This fixture is designed for testing OCR WER (Word Error Rate) with a target of < 2%.

## Ground Truth

The ground_truth.txt file contains the exact text that should be extracted.

## Generating source.pdf

To generate the source.pdf at 300 DPI with a Tesseract-friendly font:

1. Using LibreOffice:
   ```bash
   libreoffice --headless --convert-to pdf --outdir . source.odt
   ```
   Where source.odt contains the ground_truth.txt with:
   - Font: Arial or Helvetica (Tesseract-friendly)
   - Font size: 12pt
   - Page size: Letter (8.5" x 11")
   - DPI: 300

2. Using Python with reportlab:
   ```python
   from reportlab.pdfgen import canvas
   from reportlab.lib.pagesizes import letter
   from reportlab.pdfbase import pdfmetrics
   from reportlab.pdfbase.ttfonts import TTFont

   c = canvas.Canvas("source.pdf", pagesize=letter)

   # Register Arial font
   # pdfmetrics.registerFont(TTFont('Arial', 'Arial.ttf'))

   c.setFont("Helvetica", 12)
   text = open("ground_truth.txt").read()

   # Draw text with appropriate margins and line spacing
   y_position = 750
   for line in text.split('\n'):
       if y_position < 50:
           c.showPage()
           y_position = 750
       c.drawString(50, y_position, line)
       y_position -= 18

   c.save()
   ```

## Expected WER

On a clean 300 DPI scan with Arial/Helvetica font, Tesseract should achieve WER < 2%.
"#;

    let readme_path = output_dir.join("README.md");
    let mut readme_file = File::create(&readme_path)?;
    readme_file.write_all(readme.as_bytes())?;

    // Create a placeholder source.txt for manual PDF generation
    let source_path = output_dir.join("source.txt");
    let mut source_file = File::create(&source_path)?;
    source_file.write_all(ground_truth.as_bytes())?;

    println!("  Created: {}", gt_path.display());
    println!("  Created: {}", readme_path.display());
    println!("  Created: {}", source_path.display());
    println!("  NOTE: source.pdf needs to be generated manually (see README.md)");

    Ok(())
}

fn generate_multi_language() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating eng_fra_mixed fixture...");

    let output_dir = Path::new("tests/fixtures/ocr/eng_fra_mixed");
    fs::create_dir_all(output_dir)?;

    // Ground truth with English and French paragraphs
    let ground_truth = r#"The quick brown fox jumps over the lazy dog. This is a standard English sentence that contains common words and demonstrates basic OCR capabilities for the English language.

Le renard brun rapide saute par-dessus le chien paresseux. C'est une phrase française standard qui contient des mots communs et démontre les capacités OCR de base pour la langue française.

The weather today is quite beautiful with clear blue skies and pleasant temperatures perfect for outdoor activities.

La météo d'aujourd'hui est assez belle avec un ciel bleu clair et des températures agréables parfaites pour les activités de plein air.

English text contains words like "computer", "keyboard", "mouse", and "monitor" which are common in technical documentation.

Le texte français contient des mots comme "ordinateur", "clavier", "souris" et "moniteur" qui sont courants dans la documentation technique."#;

    // Write ground truth
    let gt_path = output_dir.join("ground_truth.txt");
    let mut gt_file = File::create(&gt_path)?;
    gt_file.write_all(ground_truth.as_bytes())?;

    let readme = r#"# Multi-Language English+French Fixture

This fixture tests OCR with multiple language packs (eng+fra) with a target WER < 3%.

## Ground Truth

The ground_truth.txt file contains alternating English and French paragraphs.

## Generating source.pdf

To generate the source.pdf at 300 DPI:

1. Ensure both English (eng) and French (fra) language packs are installed:
   ```bash
   apt-get install tesseract-ocr-eng tesseract-ocr-fra
   ```

2. Using Python with reportlab:
   ```python
   from reportlab.pdfgen import canvas
   from reportlab.lib.pagesizes import letter

   c = canvas.Canvas("source.pdf", pagesize=letter)
   c.setFont("Helvetica", 12)

   text = open("ground_truth.txt").read()
   y_position = 750

   for line in text.split('\n'):
       if y_position < 50:
           c.showPage()
           y_position = 750
       c.drawString(50, y_position, line)
       y_position -= 18

   c.save()
   ```

## Expected WER

With both eng+fra language packs loaded, Tesseract should achieve WER < 3%.
Missing language packs will result in significantly higher WER.
"#;

    let readme_path = output_dir.join("README.md");
    let mut readme_file = File::create(&readme_path)?;
    readme_file.write_all(readme.as_bytes())?;

    let source_path = output_dir.join("source.txt");
    let mut source_file = File::create(&source_path)?;
    source_file.write_all(ground_truth.as_bytes())?;

    println!("  Created: {}", gt_path.display());
    println!("  Created: {}", readme_path.display());
    println!("  Created: {}", source_path.display());
    println!("  NOTE: source.pdf needs to be generated manually (see README.md)");

    Ok(())
}

fn generate_performance_fixture() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating perf_10_page fixture...");

    let output_dir = Path::new("tests/fixtures/ocr/perf_10_page");
    fs::create_dir_all(output_dir)?;

    // Generate 10 pages of diverse content
    let pages = vec![
        // Page 1: Text-heavy content
        r#"Chapter 1: Introduction

This document serves as a performance test fixture for OCR processing. It contains ten pages with diverse content types including text-heavy sections, forms, tables, and mixed layouts.

The primary objective is to measure OCR processing time on a multi-page document. The target is to complete OCR on all ten pages in less than thirty seconds on a standard four-core CI runner.

Performance optimization is critical for production OCR systems. The implementation uses thread-local Tesseract instances to minimize initialization overhead across pages processed in parallel."#,

        // Page 2: Form-like content
        r#"APPLICATION FORM

First Name: _________________________ Last Name: _______________________

Address: _____________________________________________________________
          City: ______________________ State: ____ ZIP: ______________

Email: ______________________________________________________________
Phone: (___) ___-_____

Please check all that apply:
[ ] Full-time employee    [ ] Part-time employee
[ ] Independent contractor  [ ] Student

Signature: _____________________________ Date: _________________"#,

        // Page 3: Table content
        r#"SALES REPORT - Q1 2024

+------------+--------+--------+-------+--------+
| Region     | Jan    | Feb    | Mar   | Total  |
+------------+--------+--------+-------+--------+
| North      | 12,500 | 13,200 | 14,100|  39,800|
| South      |  8,300 |  9,100 |  9,800|  27,200|
| East       | 15,200 | 14,800 | 16,200|  46,200|
| West       | 10,100 | 11,300 | 11,900|  33,300|
+------------+--------+--------+-------+--------+
| TOTAL      | 46,100 | 48,400 | 52,000| 146,500|
+------------+--------+--------+-------+--------+

Growth rate: 12.8% quarter over quarter."#,

        // Page 4: Technical documentation
        r#"API Reference: extract_pdf()

Parameters:
- path: &str - Path to the PDF file
- options: ExtractionOptions - Configuration options

Returns: Result<ExtractionResult, Error>

The extract_pdf function processes PDF documents and returns structured text extraction results. It supports various extraction modes including full text, layout-aware extraction, and OCR for scanned content.

Options:
- ocr_enabled: bool - Enable OCR for scanned pages (default: true)
- ocr_language: Vec<String> - Language codes for OCR (default: ["eng"])
- dpi: u32 - Rendering DPI for OCR (default: 300)

Example:
    let result = extract_pdf("document.pdf", ExtractionOptions::default())?;"#,

        // Page 5: Legal text
        r#"TERMS AND CONDITIONS

1. ACCEPTANCE OF TERMS
By accessing and using this service, you acknowledge that you have read, understood, and agree to be bound by these Terms and Conditions.

2. LICENSE GRANT
Subject to the terms of this agreement, we grant you a limited, non-exclusive, non-transferable license to use the service for internal business purposes.

3. LIMITATION OF LIABILITY
In no event shall we be liable for any indirect, incidental, special, consequential, or punitive damages, including without limitation, loss of profits, data, use, goodwill, or other intangible losses.

4. INDEMNIFICATION
You agree to indemnify and hold harmless the company from any claims resulting from your use of the service."#,

        // Page 6: Financial data
        r#"BALANCE SHEET - December 31, 2024

ASSETS
Current Assets:
  Cash and Equivalents        $125,000
  Accounts Receivable          $89,500
  Inventory                    $67,200
  Prepaid Expenses             $12,800
  Total Current Assets        $294,500

Non-Current Assets:
  Property, Plant & Equipment $450,000
  Less: Accumulated Depreciation ($125,000)
  Net PPE                     $325,000
  Intangible Assets            $50,000
  Total Non-Current Assets   $375,000

TOTAL ASSETS                 $669,500

LIABILITIES AND EQUITY
  Current Liabilities         $125,000
  Long-term Debt              $200,000
  Total Liabilities           $325,000
  Shareholders' Equity        $344,500

TOTAL L&E                    $669,500"#,

        // Page 7: Scientific content
        r#"Abstract: A Study on Optical Character Recognition Accuracy

This research examines the factors affecting Word Error Rate (WER) in commercial OCR systems. We conducted experiments across various document types, fonts, and scanning resolutions.

Methodology:
- 500 test documents spanning 5 categories
- Resolution range: 200-400 DPI
- Fonts: Arial, Times New Roman, Helvetica, Courier
- Languages: English, French, German, Spanish

Results:
Average WER by DPI:
- 200 DPI: 4.2%
- 300 DPI: 1.8%
- 400 DPI: 1.5%

Conclusion: 300 DPI provides the optimal balance between accuracy and processing time for most document types."#,

        // Page 8: Mixed content list
        r#"PROJECT TASK LIST

Week 1: Planning
- [x] Define project scope
- [x] Identify stakeholders
- [ ] Create timeline
- [ ] Allocate resources

Week 2: Development
- [ ] Set up development environment
- [ ] Implement core features
- [ ] Write unit tests
- [ ] Code review

Week 3: Testing
- [ ] Integration testing
- [ ] Performance testing
- [ ] Security audit
- [ ] User acceptance testing

Week 4: Deployment
- [ ] Production deployment
- [ ] Monitor performance
- [ ] Address issues
- [ ] Document lessons learned

Priority Key:
High: [!]
Medium: [*]
Low: [ ]"#,

        // Page 9: Correspondence
        r#"Dear Customer,

Thank you for your recent purchase. We are committed to providing you with the best possible service and support.

Order Details:
Order Number: ORD-2024-78542
Date: May 15, 2024
Items: 3
Total: $247.50

Your order has been processed and will be shipped within 2-3 business days. You will receive a shipping confirmation email with tracking information once your package has been dispatched.

If you have any questions or concerns, please do not hesitate to contact our customer service team at:

Email: support@example.com
Phone: 1-800-555-0123
Hours: Monday-Friday, 8AM-6PM EST

Thank you for choosing our company. We value your business and look forward to serving you again in the future.

Sincerely,
Customer Service Team"#,

        // Page 10: Summary page
        r#"EXECUTIVE SUMMARY

This ten-page document demonstrates OCR performance across diverse content types:

Content Distribution:
- Text-heavy pages: 5 (50%)
- Forms: 1 (10%)
- Tables: 2 (20%)
- Technical documentation: 1 (10%)
- Correspondence: 1 (10%)

Performance Metrics Target:
- Processing time: < 30 seconds (10 pages @ 3 sec/page)
- Throughput: > 20 pages/minute on 4-core CI runner
- Memory usage: < 500MB per worker thread

Quality Metrics:
- Clean text WER: < 2%
- Multi-language WER: < 3%
- Table cell accuracy: > 95%

The fixture is designed to stress-test the OCR pipeline while providing reproducible benchmarks for performance regression testing.

End of Document"#,
    ];

    // Combine all pages into ground truth
    let all_text = pages.join("\n\n");

    // Write ground truth
    let gt_path = output_dir.join("ground_truth.txt");
    let mut gt_file = File::create(&gt_path)?;
    gt_file.write_all(all_text.as_bytes())?;

    // Write individual page files for reference
    for (i, page) in pages.iter().enumerate() {
        let page_path = output_dir.join(format!("page_{}.txt", i + 1));
        let mut page_file = File::create(&page_path)?;
        page_file.write_all(page.as_bytes())?;
    }

    let readme = r#"# 10-Page Performance Fixture

This fixture tests OCR performance on a multi-page document with a target processing time of < 30 seconds on a 4-core CI runner.

## Structure

- ground_truth.txt: Complete text from all 10 pages
- page_*.txt: Individual page text for reference

## Content Types

1. Text-heavy documentation
2. Forms with fields
3. Tabular data
4. Technical documentation
5. Legal text
6. Financial statements
7. Scientific content
8. Task lists
9. Correspondence
10. Summary

## Generating source.pdf

To generate the 10-page source.pdf at 300 DPI:

Using Python with reportlab:
```python
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter

c = canvas.Canvas("source.pdf", pagesize=letter)
c.setFont("Helvetica", 12)

for i in range(1, 11):
    with open(f"page_{i}.txt") as f:
        text = f.read()

    y_position = 750
    for line in text.split('\n'):
        if y_position < 50:
            c.showPage()
            y_position = 750
        c.drawString(50, y_position, line)
        y_position -= 16

    c.showPage()

c.save()
```

## Expected Performance

Target: < 30 seconds for full document OCR on 4-core CI runner.

This allows approximately 3 seconds per page, accounting for:
- Tesseract initialization (first page per thread)
- Image preprocessing
- OCR processing
- HOCR parsing
- Coordinate conversion"#;

    let readme_path = output_dir.join("README.md");
    let mut readme_file = File::create(&readme_path)?;
    readme_file.write_all(readme.as_bytes())?;

    println!("  Created: {}", gt_path.display());
    println!("  Created: {}", readme_path.display());
    for i in 1..=10 {
        println!("  Created: {}/page_{}.txt", output_dir.display(), i);
    }
    println!("  NOTE: source.pdf needs to be generated manually (see README.md)");

    Ok(())
}

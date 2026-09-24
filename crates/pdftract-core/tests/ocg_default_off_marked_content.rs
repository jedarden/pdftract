//! Regression coverage for the catalog-to-marked-content OCG visibility path.
//!
//! The fixture is assembled here instead of checked in as opaque binary data so
//! the `/OCProperties /D /OFF` and `/OC` BDC relationship stays visible in the
//! test review. The generated PDF has a real xref table and a Helvetica font,
//! so this exercises the public extraction API rather than only the operator
//! parser.

use std::io::Write;

use pdftract_core::extract::{extract_pdf, extract_text};
use pdftract_core::options::ExtractionOptions;
use tempfile::NamedTempFile;

fn pdf_fixture(with_default_off_ocg: bool) -> NamedTempFile {
    let content = if with_default_off_ocg {
        b"BT\n/F1 12 Tf\n72 700 Td\n/OC /Hidden BDC\n(HIDDEN) Tj\nEMC\n(VISIBLE) Tj\nET\n".to_vec()
    } else {
        b"BT\n/F1 12 Tf\n72 700 Td\n(CONTROL) Tj\nET\n".to_vec()
    };

    let mut objects = vec![
        if with_default_off_ocg {
            b"<< /Type /Catalog /Pages 2 0 R /OCProperties 7 0 R >>".to_vec()
        } else {
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()
        },
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        if with_default_off_ocg {
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /Properties << /Hidden 8 0 R >> >> /Contents 5 0 R >>".to_vec()
        } else {
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec()
        },
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];

    let mut content_object = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_object.extend_from_slice(&content);
    content_object.extend_from_slice(b"endstream");
    objects.push(content_object);

    if with_default_off_ocg {
        objects.push(b"<< /Type /OCG /Name (Hidden layer) >>".to_vec());
        objects.push(b"<< /OCGs [6 0 R] /D << /BaseState /ON /OFF [6 0 R] >> >>".to_vec());
        objects.push(b"<< /OCG 6 0 R >>".to_vec());
    }

    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );

    let mut file = NamedTempFile::new().expect("create OCG regression fixture");
    file.write_all(&pdf).expect("write OCG regression fixture");
    file.flush().expect("flush OCG regression fixture");
    file
}

fn joined_span_text(path: &std::path::Path, options: &ExtractionOptions) -> String {
    let result = extract_pdf(path, options).expect("extract OCG regression fixture");
    assert_eq!(result.pages.len(), 1, "fixture should contain one page");
    assert!(result.pages[0].error.is_none(), "page extraction failed");
    result.pages[0]
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

#[test]
fn default_off_ocg_is_hidden_while_visible_and_no_ocg_text_survive() {
    let hidden_fixture = pdf_fixture(true);
    let default_options = ExtractionOptions::default();

    // The public default extraction contract omits text from a default-OFF OCG.
    let default_text = extract_text(hidden_fixture.path(), &default_options)
        .expect("default extraction should succeed");
    assert!(!default_text.contains("HIDDEN"));
    assert!(default_text.contains("VISIBLE"));
    assert_eq!(
        joined_span_text(hidden_fixture.path(), &default_options),
        "VISIBLE"
    );

    // The same marked content remains available when callers opt in to hidden layers.
    let mut include_hidden = default_options.clone();
    include_hidden.output.include_hidden_layers = true;
    let opted_in_text = extract_text(hidden_fixture.path(), &include_hidden)
        .expect("opt-in extraction should succeed");
    assert!(opted_in_text.contains("HIDDEN"));
    assert!(opted_in_text.contains("VISIBLE"));

    // A document without OCProperties is unchanged by the OCG visibility path.
    let control_fixture = pdf_fixture(false);
    let control_text = extract_text(control_fixture.path(), &default_options)
        .expect("no-OCG extraction should succeed");
    assert_eq!(control_text, "CONTROL\n");
    assert_eq!(
        joined_span_text(control_fixture.path(), &default_options),
        "CONTROL"
    );
}

//! Generate .expected.json files for document model test fixtures.
//!
//! Run with: cargo run --bin generate_expected_json

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("Generating .expected.json files for document model fixtures...");

    let fixtures_dir = PathBuf::from("tests/document_model/fixtures");

    let fixtures = [
        ("encrypted_rc4_test", Some("test")),
        ("encrypted_aes128_test", Some("test")),
        ("encrypted_aes256_test", Some("test")),
        ("encrypted_empty_password", Some("")),
        ("encrypted_unknown_handler", None),
        ("tagged_3_level_outline", None),
        ("ocg_default_off", None),
        ("multi_revision_3", None),
        ("inheritance_grandparent_mediabox", None),
        ("missing_mediabox", None),
        ("partial_resource_override", None),
        ("js_in_openaction", None),
        ("xfa_form", None),
        ("pdfa_1b_conformance", None),
        ("page_labels_roman_arabic", None),
    ];

    for (name, _password) in fixtures.iter() {
        let pdf_path = fixtures_dir.join(format!("{}.pdf", name));
        let expected_path = fixtures_dir.join(format!("{}.expected.json", name));

        if !pdf_path.exists() {
            eprintln!("Warning: PDF fixture not found: {}", pdf_path.display());
            continue;
        }

        // For now, parse the PDF and build a minimal expected.json
        // This is a placeholder - the actual implementation would use
        // pdftract_core to parse the PDF and build the JSON
        match generate_expected_json(&pdf_path, name) {
            Ok(json) => {
                fs::write(&expected_path, &json)
                    .expect(&format!("Failed to write {}", expected_path.display()));
                println!("Created {}", expected_path.display());
            }
            Err(e) => {
                eprintln!("Error generating JSON for {}: {}", name, e);
            }
        }
    }

    println!("\nAll .expected.json files generated!");
}

fn generate_expected_json(pdf_path: &Path, name: &str) -> Result<String, String> {
    // Placeholder implementation
    // This should be replaced with actual PDF parsing using pdftract_core
    let placeholder = match name {
        "encrypted_rc4_test" => r#"{
  "page_count": 1,
  "is_encrypted": true,
  "encryption_algorithm": "RC4-40",
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "encrypted_aes128_test" => r#"{
  "page_count": 1,
  "is_encrypted": true,
  "encryption_algorithm": "AES-128",
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "encrypted_aes256_test" => r#"{
  "page_count": 1,
  "is_encrypted": true,
  "encryption_algorithm": "AES-256",
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "encrypted_empty_password" => r#"{
  "page_count": 1,
  "is_encrypted": true,
  "encryption_algorithm": "RC4-40",
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "encrypted_unknown_handler" => r#"{
  "page_count": 1,
  "is_encrypted": true,
  "encryption_status": "unsupported handler /Adobe.PubSec",
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "tagged_3_level_outline" => r#"{
  "page_count": 2,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "outline": {
    "count": 2,
    "items": [
      {
        "title": "Chapter 1",
        "dest_page": 0,
        "children": [
          {
            "title": "Section 1.1",
            "dest_page": 0
          }
        ]
      },
      {
        "title": "Chapter 2",
        "dest_page": 1
      }
    ]
  },
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 1,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "ocg_default_off" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": true,
  "ocg_default_state": "OFF",
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "multi_revision_3" => r#"{
  "page_count": 3,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 1,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 2,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "inheritance_grandparent_mediabox" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0,
      "inherits_mediabox": true
    }
  ]
}"#,
        "missing_mediabox" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0,
      "default_mediabox": true
    }
  ]
}"#,
        "partial_resource_override" => r#"{
  "page_count": 2,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0,
      "resources": {
        "Font": {
          "F3": "Courier"
        }
      },
      "inherited_resources": {
        "XObject": {
          "Im1": "inherited"
        }
      }
    },
    {
      "page_index": 1,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "js_in_openaction" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": true,
  "contains_xfa": false,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "xfa_form" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": true,
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "pdfa_1b_conformance" => r#"{
  "page_count": 1,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "conformance": "PDF/A-1B",
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        "page_labels_roman_arabic" => r#"{
  "page_count": 6,
  "is_encrypted": false,
  "is_tagged": false,
  "ocg_present": false,
  "contains_javascript": false,
  "contains_xfa": false,
  "page_labels": [
    {"index": 0, "style": "roman", "value": "i"},
    {"index": 1, "style": "roman", "value": "ii"},
    {"index": 2, "style": "roman", "value": "iii"},
    {"index": 3, "style": "roman", "value": "iv"},
    {"index": 4, "style": "arabic", "value": "1"},
    {"index": 5, "style": "arabic", "value": "2"}
  ],
  "pages": [
    {
      "page_index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 1,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 2,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 3,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 4,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    },
    {
      "page_index": 5,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": [0.0, 0.0, 612.0, 792.0],
      "rotate": 0
    }
  ]
}"#,
        _ => return Err(format!("Unknown fixture: {}", name)),
    };

    Ok(placeholder.to_string())
}

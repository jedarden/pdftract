//! Golden output tests for the object parser.
//!
//! Each fixture in tests/object_parser/fixtures/ has a corresponding .expected.json
//! file. This test verifies that parsing each fixture produces the expected output.
//!
//! Run with BLESS=1 to update the .expected.json files:
//!   BLESS=1 cargo test --test object_parser

use pdftract_core::parser::object::{ObjectParser, PdfObject};
use std::fs;
use std::path::{Path, PathBuf};

/// Fixture name and its file
struct Fixture {
    name: &'static str,
    pdf_in_path: PathBuf,
    expected_path: PathBuf,
}

impl Fixture {
    fn new(name: &'static str) -> Self {
        let fixtures_dir = PathBuf::from("tests/object_parser/fixtures");
        Fixture {
            name,
            pdf_in_path: fixtures_dir.join(format!("{}.pdf.in", name)),
            expected_path: fixtures_dir.join(format!("{}.expected.json", name)),
        }
    }
}

fn all_fixtures() -> Vec<Fixture> {
    vec![
        Fixture::new("nested_dict"),
        Fixture::new("mixed_array"),
        Fixture::new("indirect_simple"),
        Fixture::new("indirect_stream"),
        Fixture::new("objstm_basic"),
        Fixture::new("objstm_extends"),
        Fixture::new("circular_self"),
        Fixture::new("circular_three"),
        Fixture::new("truncated_dict"),
        Fixture::new("deep_nesting"),
    ]
}

fn serialize_object_to_json(obj: &PdfObject) -> serde_json::Value {
    match obj {
        PdfObject::Null => serde_json::json!({"type": "null"}),
        PdfObject::Bool(b) => serde_json::json!({"type": "boolean", "value": b}),
        PdfObject::Integer(i) => serde_json::json!({"type": "integer", "value": i}),
        PdfObject::Real(r) => serde_json::json!({"type": "real", "value": r}),
        PdfObject::String(s) => serde_json::json!({
            "type": "string",
            "value": String::from_utf8_lossy(s)
        }),
        PdfObject::Name(n) => serde_json::json!({"type": "name", "value": n.as_ref()}),
        PdfObject::Array(arr) => {
            let elements: Vec<serde_json::Value> = arr.iter().map(serialize_object_to_json).collect();
            serde_json::json!({"type": "array", "value": elements})
        }
        PdfObject::Dict(dict) => {
            let mut map = serde_json::Map::new();
            for (key, value) in dict.iter() {
                map.insert(key.as_ref().to_string(), serialize_object_to_json(value));
            }
            serde_json::json!({"type": "dictionary", "value": map})
        }
        PdfObject::Ref(r) => serde_json::json!({"type": "reference", "value": format!("{} {} R", r.object, r.generation)}),
        PdfObject::Stream(s) => {
            let mut dict_map = serde_json::Map::new();
            for (key, value) in s.dict.iter() {
                dict_map.insert(key.as_ref().to_string(), serialize_object_to_json(value));
            }
            serde_json::json!({
                "type": "stream",
                "offset": s.offset,
                "len_hint": s.len_hint,
                "dict": dict_map
            })
        }
        PdfObject::Indirect(ind) => {
            serde_json::json!({
                "type": "indirect",
                "id": format!("{} {} R", ind.id.object, ind.id.generation),
                "object": serialize_object_to_json(&ind.obj)
            })
        }
    }
}

#[test]
fn test_object_parser_fixtures() {
    let bless = std::env::var("BLESS").is_ok();

    for fixture in all_fixtures() {
        // Read the fixture
        let input = fs::read_to_string(&fixture.pdf_in_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture.name, e));

        // Parse it
        let mut parser = ObjectParser::new(input.as_bytes());
        let result = parser.parse_indirect_object();

        if bless {
            // Write the expected output
            let json_value = match result {
                Some(indirect) => serialize_object_to_json(&PdfObject::Indirect(Box::new(indirect))),
                None => serde_json::json!({"type": "eof", "value": null}),
            };
            let json_str = serde_json::to_string_pretty(&json_value).unwrap();
            fs::write(&fixture.expected_path, json_str)
                .unwrap_or_else(|e| panic!("Failed to write expected file for {}: {}", fixture.name, e));
            println!("Blessed {}", fixture.name);
        } else {
            // Read the expected output
            if !fixture.expected_path.exists() {
                panic!("Expected file missing for {}: run with BLESS=1 to generate", fixture.name);
            }
            let expected_json = fs::read_to_string(&fixture.expected_path)
                .unwrap_or_else(|e| panic!("Failed to read expected file for {}: {}", fixture.name, e));
            let expected: serde_json::Value = serde_json::from_str(&expected_json)
                .unwrap_or_else(|e| panic!("Failed to parse expected JSON for {}: {}", fixture.name, e));

            // Compare
            let actual_json = match result {
                Some(indirect) => serialize_object_to_json(&PdfObject::Indirect(Box::new(indirect))),
                None => serde_json::json!({"type": "eof", "value": null}),
            };

            if actual_json != expected {
                panic!(
                    "Fixture {} mismatch:\nExpected:\n{}\nActual:\n{}",
                    fixture.name,
                    serde_json::to_string_pretty(&expected).unwrap(),
                    serde_json::to_string_pretty(&actual_json).unwrap()
                );
            }
        }
    }
}

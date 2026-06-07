//! Debug test to identify which fixture is causing hangs.

use pdftract_core::parser::object::ObjectParser;
use std::fs;

#[test]
fn debug_each_fixture() {
    let fixtures = ["nested_dict", "mixed_array", "indirect_simple", "indirect_stream", "objstm_basic", "objstm_extends", "circular_self", "circular_three", "truncated_dict", "deep_nesting"];

    for fixture in fixtures {
        println!("Testing {}...", fixture);
        let input = fs::read_to_string(format!("tests/object_parser/fixtures/{}.pdf.in", fixture))
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", fixture, e));

        let wrapped = if input.trim().contains(" obj ") {
            input.clone()
        } else {
            format!("1 0 obj {}\nendobj", input.trim())
        };

        let mut parser = ObjectParser::new(wrapped.as_bytes());
        let result = parser.parse_indirect_object();
        println!("  {}: {:?}", fixture, result.is_some());
    }
}

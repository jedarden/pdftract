#!/bin/bash

CRATE_ROOT="crates/pdftract-core/src"
OUTPUT_FILE="target/doc_coverage_report.txt"

{
    echo "Calculating rustdoc coverage for pdftract-core..."
    echo "Generated: $(date)"
    echo ""
    echo "=== Public Item Counts ==="

    pub_fn_count=$(rg "^pub fn " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_struct_count=$(rg "^pub struct " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_enum_count=$(rg "^pub enum " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_trait_count=$(rg "^pub trait " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_type_count=$(rg "^pub type " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_const_count=$(rg "^pub const " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')
    pub_static_count=$(rg "^pub static " "$CRATE_ROOT" --no-heading | wc -l | tr -d ' ')

    total_items=$((pub_fn_count + pub_struct_count + pub_enum_count + pub_trait_count + pub_type_count + pub_const_count + pub_static_count))

    echo "Functions: $pub_fn_count"
    echo "Structs: $pub_struct_count"
    echo "Enums: $pub_enum_count"
    echo "Traits: $pub_trait_count"
    echo "Types: $pub_type_count"
    echo "Constants: $pub_const_count"
    echo "Statics: $pub_static_count"
    echo "Total: $total_items"
    echo ""

    echo "=== Key Public API Files (doc comment count) ==="

    for entry in "lib.rs:lib.rs" "extract.rs:extract.rs" "document.rs:document.rs" "options.rs:options.rs" "schema/mod.rs:schema/mod.rs" "source/mod.rs:source/mod.rs" "font/mod.rs:font/mod.rs" "table/mod.rs:table/mod.rs" "layout/mod.rs:layout/mod.rs" "forms/mod.rs:forms/mod.rs"; do
        file="${CRATE_ROOT}/${entry%:*}"
        name="${entry#*:}"
        
        if [ -f "$file" ]; then
            pub_items=$(rg "^pub (fn|struct|enum|trait|type)" "$file" --no-heading | wc -l | tr -d ' ')
            doc_lines=$(rg "^///" "$file" --count-matches | tr -d ' ' || echo 0)
            echo "  $name: $doc_lines doc comments, $pub_items public items"
        fi
    done

    echo ""
    echo "=== Coverage Note ==="
    echo "This is a rough estimate. The 80% target requires worked examples, not just doc comments."

} > "$OUTPUT_FILE"

cat "$OUTPUT_FILE"
echo ""
echo "Coverage report written to $OUTPUT_FILE"

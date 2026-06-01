#!/bin/bash
# Analyze rustdoc coverage by module

cd "$(dirname "$0")/.."

echo "=== MODULE-LEVEL DOC COVERAGE ANALYSIS ==="
echo ""

# For each module, count public items and examples
for mod_file in crates/pdftract-core/src/*.rs crates/pdftract-core/src/*/mod.rs; do
    if [ -f "$mod_file" ]; then
        rel_path="${mod_file#crates/pdftract-core/src/}"
        mod_name="${rel_path%/mod.rs}"
        mod_name="${mod_name%.rs}"

        # Count public items in this file
        pub_count=$(rg '^pub (fn|struct|enum|trait|type|mod) ' "$mod_file" --type rust -c 2>/dev/null || echo 0)
        # Count example blocks
        ex_count=$(rg '```rust' "$mod_file" --type rust -c 2>/dev/null || echo 0)
        # Check for module-level doc
        has_mod_doc=$(head -30 "$mod_file" | grep -c "^//!" || echo 0)

        if [ "$pub_count" -gt 0 ] || [ "$mod_name" = "lib" ]; then
            printf "%-30s pub:%3d ex:%2d mod_doc:%d\n" "$mod_name" "$pub_count" "$ex_count" "$has_mod_doc"
        fi
    fi
done | sort -t: -k2 -rn

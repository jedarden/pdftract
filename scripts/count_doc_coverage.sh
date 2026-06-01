#!/bin/bash
# Count public API items and their documentation coverage in pdftract-core

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CORE_SRC="crates/pdftract-core/src"

echo "=== pdftract-core Documentation Coverage ==="
echo

# Count public API items by type
echo "Public API item counts:"
grep -rh "^pub " "$CORE_SRC" --include="*.rs" 2>/dev/null | {
    total=0
    types=0 funcs=0 enums=0 structs=0 traits=0 consts=0 type_aliases=0 modules=0

    while read -r line; do
        ((total++))
        case "$line" in
            "pub struct"*) ((structs++)) ;;
            "pub enum"*) ((enums++)) ;;
            "pub fn"*) ((funcs++)) ;;
            "pub trait"*) ((traits++)) ;;
            "pub const"*) ((consts++)) ;;
            "pub type"*) ((type_aliases++)) ;;
            "pub mod"*) ((modules++)) ;;
        esac
    done

    echo "  Total public items: $total"
    echo "    - Functions: $funcs"
    echo "    - Structs: $structs"
    echo "    - Enums: $enums"
    echo "    - Traits: $traits"
    echo "    - Type aliases: $type_aliases"
    echo "    - Constants: $consts"
    echo "    - Modules: $modules"
}

echo
echo "=== Detailed coverage by module ==="

for module in $(find "$CORE_SRC" -name "*.rs" -exec grep -l "^pub " {} \; 2>/dev/null | sort); do
    module_name="${module#$CORE_SRC/}"
    module_name="${module_name%.rs}"
    module_name="${module_name//\//::}"

    pub_items=$(grep "^pub " "$module" 2>/dev/null | wc -l)
    if [ "$pub_items" -gt 0 ]; then
        echo "$module_name: $pub_items public items"
    fi
done | head -20

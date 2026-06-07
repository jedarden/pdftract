#!/bin/bash
# Find public items in pdftract-core that lack examples

cd crates/pdftract-core/src

for file in $(find . -name "*.rs" | sort); do
    echo "=== $file ==="
    
    # Find pub items and check for preceding examples
    awk '
    BEGIN { in_doc = 0; has_example = 0; item_line = 0; item_name = "" }
    
    # Track doc blocks
    /^\/\/\// || /^\/\/!/ { 
        in_doc = 1
        if ($0 ~ /```rust/ || $0 ~ /```no_run/ || $0 ~ /```ignore/) {
            has_example = 1
        }
        next
    }
    
    # Reset doc block state on empty lines or non-doc comments
    /^[^\/]/ && !/^pub/ {
        if (in_doc && item_line > 0) {
            if (!has_example) {
                print "NO_EXAMPLE: " item_name " (line " item_line ")"
            }
            in_doc = 0
            has_example = 0
            item_line = 0
            item_name = ""
        }
        next
    }
    
    # Track public items
    /^pub (fn|struct|enum|trait|type|const|mod) / {
        if (in_doc && !has_example && item_line > 0) {
            print "NO_EXAMPLE: " item_name " (line " item_line ")"
        }
        
        item_line = NR
        in_doc = 0
        has_example = 0
        
        # Extract item name
        if ($2 ~ /fn|struct|enum|trait|type|const/) {
            item_name = $3
            # Remove trailing punctuation
            gsub(/[,(].*/, "", item_name)
        } else if ($2 == "mod") {
            item_name = $3
            gsub(/;.*/, "", item_name)
        }
    }
    ' "$file"
done

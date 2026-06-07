#!/usr/bin/env bash
# Count rustdoc coverage for pdftract-core

CORE_DIR="crates/pdftract-core/src"

# Count public items (pub fn, pub struct, pub enum, pub type, pub mod, pub trait)
# And count how many have examples (```rust code blocks)

pub_items=0
items_with_examples=0

# For each Rust file
find "$CORE_DIR" -name "*.rs" -type f | while read -r file; do
    # Skip if private module (no pub mod at top level in lib.rs)
    # We'll parse each file and count pub items with examples

    # Use awk to find public items and check for examples
    awk '
    BEGIN { in_pub=0; has_example=0; item_type=""; brace_count=0 }

    # Match public item declarations
    /^pub (fn|struct|enum|trait|type|mod|const|static) / {
        in_pub=1
        item_type=$2
        has_example=0
        pub_items++
        next
    }

    # Within a public item, look for code examples
    in_pub && /```rust/ {
        has_example=1
    }

    # End of public item (simplified - at next top-level declaration or empty line)
    in_pub && /^$/ && !/^pub / {
        if (has_example) items_with_examples++
        in_pub=0
    }

    END {
        print pub_items, items_with_examples
    }
    ' "$file"
done

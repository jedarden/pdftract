#!/usr/bin/env python3
"""Count rustdoc coverage for pdftract-core."""

import os
import re
from pathlib import Path
from collections import defaultdict

CORE_DIR = Path("crates/pdftract-core/src")

# Patterns for public items
PUB_PATTERNS = {
    "fn": re.compile(r'^pub (?:async\s+)?fn\s+(\w+)'),
    "struct": re.compile(r'^pub struct\s+(\w+)'),
    "enum": re.compile(r'^pub enum\s+(\w+)'),
    "trait": re.compile(r'^pub trait\s+(\w+)'),
    "type": re.compile(r'^pub type\s+(\w+)'),
    "mod": re.compile(r'^pub mod\s+(\w+)'),
    "const": re.compile(r'^pub const\s+(\w+)'),
    "static": re.compile(r'^pub static\s+(\w+)'),
}

# Pattern for code blocks in doc comments
EXAMPLE_PATTERN = re.compile(r'```rust[^`]*```')
DOC_COMMENT_PATTERN = re.compile(r'///.*|//!.*')

def count_public_items_and_examples(file_path: Path):
    """Count public items and examples in a single file."""
    with open(file_path) as f:
        lines = f.readlines()

    pub_items = []
    i = 0
    while i < len(lines):
        line = lines[i]

        # Look for public items
        for item_type, pattern in PUB_PATTERNS.items():
            match = pattern.match(line.strip())
            if match:
                item_name = match.group(1)
                pub_items.append({
                    "type": item_type,
                    "name": item_name,
                    "line": i + 1,
                    "has_example": False
                })
                break
        i += 1

    # Now check each pub item for examples
    # This is simplified - we need to scan doc comments before each item
    for item in pub_items:
        line_idx = item["line"] - 1
        # Scan backwards for doc comments
        doc_lines = []
        j = line_idx - 1
        while j >= 0 and (lines[j].strip().startswith("///") or lines[j].strip().startswith("//!")):
            doc_lines.insert(0, lines[j])
            j -= 1

        # Check if any doc comment contains a code block
        doc_text = "".join(doc_lines)
        if EXAMPLE_PATTERN.search(doc_text):
            item["has_example"] = True

    return pub_items


def main():
    all_items = []
    for rs_file in CORE_DIR.rglob("*.rs"):
        # Skip lib.rs top-level module exports
        if rs_file.name == "lib.rs":
            continue

        items = count_public_items_and_examples(rs_file)
        all_items.extend(items)

    total = len(all_items)
    with_examples = sum(1 for item in all_items if item["has_example"])
    coverage = (with_examples / total * 100) if total > 0 else 0

    print(f"Total public items: {total}")
    print(f"With worked examples: {with_examples}")
    print(f"Coverage: {coverage:.1f}%")

    # Breakdown by type
    by_type = defaultdict(list)
    for item in all_items:
        by_type[item["type"]].append(item)

    print("\nBy type:")
    for item_type, items in sorted(by_type.items()):
        with_ex = sum(1 for i in items if i["has_example"])
        print(f"  {item_type}: {with_ex}/{len(items)} ({with_ex/len(items)*100:.1f}%)")


if __name__ == "__main__":
    main()

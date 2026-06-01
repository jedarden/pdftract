#!/usr/bin/env python3
"""Measure rustdoc coverage for pdftract-core."""

import os
import re
from pathlib import Path
from collections import defaultdict

def count_items_in_file(file_path):
    """Count public items, doc items, and example items in a single file."""
    with open(file_path, 'r') as f:
        content = f.read()

    # Count public items
    pub_pattern = r'^pub\s+(fn|struct|enum|trait|type|const|static|mod|use)\s+'
    public_items = len(re.findall(pub_pattern, content, re.MULTILINE))

    # Count doc comments (/// or //! at line start)
    doc_pattern = r'^///|//!'
    doc_items = len(re.findall(doc_pattern, content, re.MULTILINE))

    # Count examples (```rust blocks)
    example_pattern = r'```rust'
    example_items = len(re.findall(example_pattern, content))

    return public_items, doc_items, example_items

def main():
    src_dir = Path('crates/pdftract-core/src')

    if not src_dir.exists():
        print(f"Error: {src_dir} does not exist")
        return

    total_public = 0
    total_doc = 0
    total_examples = 0

    file_gaps = []

    for rs_file in src_dir.rglob('*.rs'):
        pub, doc, ex = count_items_in_file(rs_file)
        total_public += pub
        total_doc += doc
        total_examples += ex

        if pub > 0:
            gap = pub - doc
            if gap > 0:
                file_gaps.append((str(rs_file.relative_to(src_dir.parent)), gap))

    print("Measuring rustdoc coverage for pdftract-core...")
    print()
    print(f"Public items found: {total_public}")
    print(f"Items with docs: {total_doc}")
    print(f"Items with examples: {total_examples}")
    print()

    if total_public > 0:
        doc_coverage = (total_doc * 100) // total_public
        example_coverage = (total_examples * 100) // total_public
        print(f"Documentation coverage: {doc_coverage}%")
        print(f"Example coverage: {example_coverage}%")
        print()
        print(f"Target: 80% example coverage")
        print()

    print("Files with most undocumented public items:")
    print()
    file_gaps.sort(key=lambda x: x[1], reverse=True)
    for file_path, gap in file_gaps[:20]:
        print(f"  {file_path}: {gap} undocumented items")

if __name__ == '__main__':
    main()

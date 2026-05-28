#!/usr/bin/env python3
"""
Measure rustdoc coverage for pdftract-core.

This script counts:
- Total public items (pub fn/struct/enum/trait/type/const)
- Items with /// doc comments (excluding module-level //!)
- Items with worked examples (```rust blocks)

Usage:
    python3 scripts/doc_coverage.py
"""
import re
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple

PUBLIC_ITEM_RE = re.compile(r'^pub (fn|struct|enum|trait|type|const|mod)\s+(\w+)')
DOC_COMMENT_RE = re.compile(r'^///')
EXAMPLE_RE = re.compile(r'```rust[^`]*```', re.MULTILINE)

def count_public_items(filepath: Path) -> Tuple[int, int, int]:
    """Count public items, doc comments, and examples in a file."""
    content = filepath.read_text()
    lines = content.split('\n')

    total_items = 0
    with_doc = 0
    with_example = 0

    i = 0
    while i < len(lines):
        line = lines[i]

        # Check for public items
        match = PUBLIC_ITEM_RE.match(line)
        if match:
            total_items += 1
            item_type, name = match.groups()

            # Look back for doc comments (///, not //!)
            has_doc = False
            has_example = False
            j = i - 1
            doc_lines = []
            while j >= 0 and (lines[j].startswith('///') or lines[j].strip() == '' or lines[j].startswith('//!')):
                if lines[j].startswith('///'):
                    has_doc = True
                    doc_lines.append(lines[j])
                j -= 1

            # Look ahead for doc comments (/// style after attrs)
            if not has_doc:
                j = i + 1
                while j < len(lines) and (lines[j].startswith('///') or lines[j].strip() == ''):
                    if lines[j].startswith('///'):
                        has_doc = True
                        doc_lines.append(lines[j])
                    j += 1

            if has_doc:
                with_doc += 1
                # Check for examples in the accumulated doc lines
                doc_text = '\n'.join(doc_lines)
                if EXAMPLE_RE.search(doc_text):
                    with_example += 1

        i += 1

    return total_items, with_doc, with_example


def main():
    core_src = Path('/home/coding/pdftract/crates/pdftract-core/src')

    total_items = 0
    total_with_doc = 0
    total_with_example = 0

    file_counts: Dict[str, Tuple[int, int, int]] = {}

    for rs_file in core_src.rglob('*.rs'):
        if 'parser/primitives' in str(rs_file):
            continue  # Skip generated files

        items, docs, examples = count_public_items(rs_file)
        if items > 0:
            file_counts[str(rs_file.relative_to(core_src))] = (items, docs, examples)
            total_items += items
            total_with_doc += docs
            total_with_example += examples

    print(f"pdftract-core Documentation Coverage")
    print(f"=" * 60)
    print(f"Total public items: {total_items}")
    print(f"Items with doc comments: {total_with_doc} ({100 * total_with_doc / total_items:.1f}%)")
    print(f"Items with worked examples: {total_with_example} ({100 * total_with_example / total_items:.1f}%)")
    print()

    # Top 20 files by public item count
    print("Top 20 files needing documentation:")
    sorted_files = sorted(
        file_counts.items(),
        key=lambda x: (x[1][0] - x[1][1], x[1][0]),  # Sort by undocumented count, then total
        reverse=True
    )
    for rel_path, (items, docs, examples) in sorted_files[:20]:
        coverage = 100 * docs / items if items > 0 else 0
        print(f"  {coverage:5.1f}% ({items:3d} items, {docs:3d} docs, {examples:3d} examples) {rel_path}")


if __name__ == '__main__':
    main()

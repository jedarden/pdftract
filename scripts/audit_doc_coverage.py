#!/usr/bin/env python3
"""
Audit documentation coverage for pdftract-core public API.
Counts public items and checks for rustdoc examples.
"""
import ast
import os
import re
import subprocess
from pathlib import Path
from collections import defaultdict

# Patterns for doc comments containing examples
EXAMPLE_PATTERNS = [
    r'```rust',
    r'```ignore',
    r'```no_run',
]

def extract_rust_items(file_path: Path):
    """Extract public items from a Rust file."""
    try:
        content = file_path.read_text()
    except:
        return []

    items = []
    lines = content.split('\n')

    # Simple regex-based extraction for public items
    for i, line in enumerate(lines):
        # Look for public fn, struct, enum, trait, type, const, mod
        for pattern in [
            r'pub\s+(?:unsafe\s+)?(?:async\s+)?fn\s+(\w+)',
            r'pub\s+struct\s+(\w+)',
            r'pub\s+enum\s+(\w+)',
            r'pub\s+trait\s+(\w+)',
            r'pub\s+type\s+(\w+)',
            r'pub\s+const\s+(\w+)',
            r'pub\s+mod\s+(\w+)',
        ]:
            match = re.search(pattern, line)
            if match and not line.strip().startswith('//'):
                item_name = match.group(1)
                # Look backward for doc comments
                has_doc = False
                has_example = False
                j = i - 1
                while j >= 0:
                    prev_line = lines[j].strip()
                    if prev_line.startswith('///') or prev_line.startswith('//!'):
                        has_doc = True
                        # Check for example patterns
                        for ex_pat in EXAMPLE_PATTERNS:
                            if re.search(ex_pat, lines[j]):
                                has_example = True
                        j -= 1
                    elif prev_line and not prev_line.startswith('//') and not prev_line.startswith('#'):
                        break
                    else:
                        j -= 1

                items.append({
                    'name': item_name,
                    'line': i + 1,
                    'has_doc': has_doc,
                    'has_example': has_example,
                    'file': file_path,
                })

    return items


def scan_directory(crate_src: Path):
    """Scan all Rust files in the crate source directory."""
    all_items = []
    for rs_file in crate_src.rglob('*.rs'):
        if 'target' in str(rs_file):
            continue
        items = extract_rust_items(rs_file)
        all_items.extend(items)
    return all_items


def main():
    pdftract_root = Path('/home/coding/pdftract')
    core_src = pdftract_root / 'crates' / 'pdftract-core' / 'src'

    if not core_src.exists():
        print(f"Source directory not found: {core_src}")
        return 1

    items = scan_directory(core_src)

    # Count coverage
    total = len(items)
    with_doc = sum(1 for i in items if i['has_doc'])
    with_example = sum(1 for i in items if i['has_example'])
    without_doc = total - with_doc

    print(f"Documentation Coverage for pdftract-core")
    print(f"=" * 50)
    print(f"Total public items: {total}")
    print(f"With documentation: {with_doc} ({100*with_doc/total:.1f}%)")
    print(f"With examples: {with_example} ({100*with_example/total:.1f}%)")
    print(f"Without documentation: {without_doc}")
    print()

    # Show items without documentation
    if without_doc > 0:
        print("Items missing documentation:")
        for item in items:
            if not item['has_doc']:
                rel_path = item['file'].relative_to(pdftract_root)
                print(f"  - {item['name']} ({rel_path}:{item['line']})")
        print()

    # Show items without examples (but have docs)
    no_example_items = [i for i in items if i['has_doc'] and not i['has_example']]
    if no_example_items:
        print(f"Items with docs but no examples ({len(no_example_items)}):")
        for item in no_example_items[:20]:  # Show first 20
            rel_path = item['file'].relative_to(pdftract_root)
            print(f"  - {item['name']} ({rel_path}:{item['line']})")
        if len(no_example_items) > 20:
            print(f"  ... and {len(no_example_items) - 20} more")

    return 0


if __name__ == '__main__':
    exit(main())

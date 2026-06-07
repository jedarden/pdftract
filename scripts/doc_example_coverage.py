#!/usr/bin/env python3
"""
Measure rustdoc example coverage for pdftract-core.

Counts public items and determines how many have at least one worked example.
"""

import os
import re
import subprocess
import json
from pathlib import Path
from collections import defaultdict

# Patterns to detect public items
PUBLIC_PATTERNS = {
    'fn': re.compile(r'pub\s+(?:async\s+)?fn\s+(\w+)'),
    'struct': re.compile(r'pub\s+struct\s+(\w+)'),
    'enum': re.compile(r'pub\s+enum\s+(\w+)'),
    'trait': re.compile(r'pub\s+trait\s+(\w+)'),
    'mod': re.compile(r'pub\s+mod\s+(\w+)'),
    'type': re.compile(r'pub\s+type\s+(\w+)'),
    'const': re.compile(r'pub\s+(?:const|static)\s+(\w+)'),
    'impl': re.compile(r'impl\s+(?:<[^>]*>)?\s*(\w+)\s*(?:<[^>]*>)?\s*\{'),  # For trait impls that add methods
}

# Pattern to detect doc code blocks
EXAMPLE_PATTERN = re.compile(r'```rust[^`]*```', re.MULTILINE)
DOC_COMMENT_PATTERN = re.compile(r'///[^\n]*|//![^\n]*')

def has_item_with_examples(content, item_name, item_type):
    """Check if a public item has at least one worked example."""
    # Look for the item and its associated doc comments
    # This is a simplified check - we look for doc comments with code blocks
    # near the item declaration

    # Split by item and look for doc comments immediately before
    lines = content.split('\n')
    item_line = None
    for i, line in enumerate(lines):
        if item_name in line and any(f'pub {t}' in line for t in ['fn', 'struct', 'enum', 'trait', 'mod', 'type', 'const', 'static']):
            item_line = i
            break

    if item_line is None:
        return False

    # Look backwards for doc comments
    doc_lines = []
    for i in range(item_line - 1, max(0, item_line - 50), -1):
        line = lines[i].strip()
        if line.startswith('///') or line.startswith('//!'):
            doc_lines.insert(0, line)
        elif line and not line.startswith('//') and not line.startswith('#['):
            # Stop at non-comment, non-attribute line
            break

    doc_content = '\n'.join(doc_lines)

    # Check for code blocks
    return bool(EXAMPLE_PATTERN.search(doc_content))

def find_public_items_in_file(filepath):
    """Find all public items in a Rust source file."""
    content = filepath.read_text()

    items = []
    for item_type, pattern in PUBLIC_PATTERNS.items():
        for match in pattern.finditer(content):
            item_name = match.group(1)
            # Skip common non-public items
            if item_name.startswith('_'):
                continue
            items.append((item_type, item_name, match.start()))

    return items, content

def scan_crate(src_path):
    """Scan the crate for public items and example coverage."""
    src_path = Path(src_path)
    results = {
        'total_items': 0,
        'items_with_examples': 0,
        'by_type': defaultdict(lambda: {'total': 0, 'with_examples': 0}),
        'files': {}
    }

    # Get all .rs files
    rs_files = list(src_path.rglob('*.rs'))

    for rs_file in rs_files:
        # Skip build.rs and tests
        if 'build.rs' in str(rs_file) or 'tests/' in str(rs_file):
            continue

        try:
            items, content = find_public_items_in_file(rs_file)

            if items:
                file_results = {
                    'total': len(items),
                    'with_examples': 0,
                    'items': []
                }

                for item_type, item_name, _ in items:
                    results['total_items'] += 1
                    results['by_type'][item_type]['total'] += 1
                    file_results['total'] += 1

                    has_examples = has_item_with_examples(content, item_name, item_type)

                    file_results['items'].append({
                        'name': item_name,
                        'type': item_type,
                        'has_examples': has_examples
                    })

                    if has_examples:
                        results['items_with_examples'] += 1
                        results['by_type'][item_type]['with_examples'] += 1
                        file_results['with_examples'] += 1

                results['files'][str(rs_file.relative_to(src_path.parent.parent))] = file_results
        except Exception as e:
            print(f"Error processing {rs_file}: {e}", flush=True)

    return results

def main():
    pdftract_core = Path('/home/coding/pdftract/crates/pdftract-core/src')
    results = scan_crate(pdftract_core)

    coverage = (results['items_with_examples'] / results['total_items'] * 100) if results['total_items'] > 0 else 0

    print("=" * 60)
    print(f"Rustdoc Example Coverage Report for pdftract-core")
    print("=" * 60)
    print(f"\nTotal public items: {results['total_items']}")
    print(f"Items with examples: {results['items_with_examples']}")
    print(f"Coverage: {coverage:.1f}%")
    print(f"\nTarget: 80%")
    print(f"Status: {'✓ PASS' if coverage >= 80 else '✗ FAIL'}")

    print("\n" + "=" * 60)
    print("Coverage by Type")
    print("=" * 60)
    for item_type, counts in sorted(results['by_type'].items()):
        total = counts['total']
        with_ex = counts['with_examples']
        cov = (with_ex / total * 100) if total > 0 else 0
        print(f"{item_type:12} {with_ex:4}/{total:4} ({cov:5.1f}%) {'✓' if cov >= 80 else '✗'}")

    # Show files that need work
    print("\n" + "=" * 60)
    print("Files Needing Examples (showing items without examples)")
    print("=" * 60)

    for file_path, file_results in sorted(results['files'].items()):
        file_cov = (file_results['with_examples'] / file_results['total'] * 100) if file_results['total'] > 0 else 0
        missing = [item for item in file_results['items'] if not item['has_examples']]
        if missing and file_cov < 80:
            print(f"\n{file_path} ({file_cov:.0f}% coverage)")
            for item in sorted(missing, key=lambda x: (x['type'], x['name'])):
                print(f"  - {item['type']:8} {item['name']}")

    print("\n" + "=" * 60)

    # Output JSON for scripts
    output_json = {
        'coverage': coverage,
        'total_items': results['total_items'],
        'items_with_examples': results['items_with_examples'],
        'pass': coverage >= 80
    }

    json_path = Path('/tmp/doc_example_coverage.json')
    json_path.write_text(json.dumps(output_json, indent=2))

    return 0 if coverage >= 80 else 1

if __name__ == '__main__':
    exit(main())

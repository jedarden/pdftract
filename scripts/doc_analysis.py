#!/usr/bin/env python3
"""Analyze rustdoc coverage for pdftract-core public API."""

import os
import re
from pathlib import Path
from collections import defaultdict

def extract_items_with_docs(file_path):
    """Extract public items and their documentation status from a Rust file."""
    content = file_path.read_text()
    lines = content.split('\n')

    items = []
    i = 0
    while i < len(lines):
        line = lines[i]

        # Skip comments and empty lines to find next item
        if line.strip().startswith('//') or not line.strip():
            i += 1
            continue

        # Look for public items
        pub_match = re.match(r'^\s*pub\s+(fn|struct|enum|trait|type|const|static|mod)\s+(\w+)', line)
        if pub_match:
            item_kind = pub_match.group(1)
            item_name = pub_match.group(2)

            # Look backwards for doc comments
            has_doc = False
            has_example = False
            j = i - 1
            doc_lines = []

            while j >= 0:
                prev_line = lines[j].strip()
                if prev_line.startswith('///') or prev_line.startswith('//!'):
                    has_doc = True
                    doc_lines.insert(0, prev_line)
                    j -= 1
                elif prev_line.startswith('//') or not prev_line:
                    j -= 1
                else:
                    break

            # Check for examples in doc
            for doc_line in doc_lines:
                if '```rust' in doc_line or '```no_run' in doc_line or '```ignore' in doc_line:
                    has_example = True
                    break

            items.append({
                'kind': item_kind,
                'name': item_name,
                'has_doc': has_doc,
                'has_example': has_example,
                'line': i + 1
            })

        i += 1

    return items


def analyze_directory(src_dir):
    """Analyze all Rust files in a directory."""
    results = {
        'total_items': 0,
        'with_docs': 0,
        'with_examples': 0,
        'by_kind': defaultdict(lambda: {'total': 0, 'docs': 0, 'examples': 0}),
        'by_file': {},
    }

    for rs_file in Path(src_dir).rglob('*.rs'):
        # Skip test files and modules.rs that just re-export
        if 'test' in rs_file.name or rs_file.name == 'tests.rs':
            continue

        try:
            items = extract_items_with_docs(rs_file)
            if items:
                file_results = {
                    'total': len(items),
                    'docs': 0,
                    'examples': 0,
                    'items': items
                }

                for item in items:
                    results['total_items'] += 1
                    results['by_kind'][item['kind']]['total'] += 1

                    if item['has_doc']:
                        results['with_docs'] += 1
                        file_results['docs'] += 1
                        results['by_kind'][item['kind']]['docs'] += 1

                    if item['has_example']:
                        results['with_examples'] += 1
                        file_results['examples'] += 1
                        results['by_kind'][item['kind']]['examples'] += 1

                results['by_file'][str(rs_file)] = file_results
        except Exception as e:
            print(f"Error processing {rs_file}: {e}")

    return results


def print_results(results):
    """Print analysis results."""
    print("=" * 70)
    print("PDFTRACT-CORE DOCUMENTATION COVERAGE ANALYSIS")
    print("=" * 70)
    print()

    total = results['total_items']
    with_docs = results['with_docs']
    with_examples = results['with_examples']

    doc_coverage = (with_docs / total * 100) if total > 0 else 0
    example_coverage = (with_examples / total * 100) if total > 0 else 0

    print(f"Total public items: {total}")
    print(f"With documentation: {with_docs} ({doc_coverage:.1f}%)")
    print(f"With examples: {with_examples} ({example_coverage:.1f}%)")
    print()

    print("By item type:")
    print("-" * 70)
    for kind in sorted(results['by_kind'].keys()):
        data = results['by_kind'][kind]
        cov = (data['docs'] / data['total'] * 100) if data['total'] > 0 else 0
        ex_cov = (data['examples'] / data['total'] * 100) if data['total'] > 0 else 0
        print(f"  {kind:12} {data['total']:4} total | {data['docs']:4} docs ({cov:5.1f}%) | {data['examples']:4} examples ({ex_cov:5.1f}%)")

    print()
    print("Files with most undocumented items (need priority attention):")
    print("-" * 70)

    undocumented_files = []
    for file_path, file_data in results['by_file'].items():
        undocumented = file_data['total'] - file_data['docs']
        if undocumented > 0:
            # Get relative path from src dir
            rel_path = file_path.replace('/home/coding/pdftract/crates/pdftract-core/src/', '')
            undocumented_files.append((rel_path, undocumented, file_data))

    undocumented_files.sort(key=lambda x: x[1], reverse=True)

    for rel_path, undocumented, file_data in undocumented_files[:15]:
        print(f"  {rel_path:50} {undocumented:3} missing docs ({file_data['total']} total)")

    print()
    print("Files with most items missing examples:")
    print("-" * 70)

    missing_examples = []
    for file_path, file_data in results['by_file'].items():
        missing = file_data['total'] - file_data['examples']
        if missing > 0:
            rel_path = file_path.replace('/home/coding/pdftract/crates/pdftract-core/src/', '')
            missing_examples.append((rel_path, missing, file_data))

    missing_examples.sort(key=lambda x: x[1], reverse=True)

    for rel_path, missing, file_data in missing_examples[:15]:
        print(f"  {rel_path:50} {missing:3} missing examples ({file_data['total']} total)")


if __name__ == '__main__':
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')
    results = analyze_directory(src_dir)
    print_results(results)

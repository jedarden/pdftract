#!/usr/bin/env python3
"""
Audit script to find public items in pdftract-core that are missing documentation.
"""
import re
import subprocess
from pathlib import Path
from collections import defaultdict

PUBLIC_PATTERNS = [
    (r'pub fn (\w+)', 'function'),
    (r'pub struct (\w+)', 'struct'),
    (r'pub enum (\w+)', 'enum'),
    (r'pub trait (\w+)', 'trait'),
    (r'pub type (\w+)', 'type'),
    (r'pub const (\w+)', 'const'),
    (r'pub mod (\w+)', 'module'),
    (r'pub (?:static|async) (\w+)', 'other'),
]

def has_doc_comment(lines, line_idx):
    """Check if there's a doc comment before the given line."""
    for i in range(line_idx - 1, -1, -1):
        line = lines[i].strip()
        if line.startswith('///') or line.startswith('//!'):
            return True
        if line and not line.startswith('//') and not line.startswith('#'):
            break
    return False

def audit_file(filepath):
    """Audit a single Rust file for missing documentation."""
    items = []
    lines = filepath.read_text(encoding='utf-8').split('\n')

    for line_idx, line in enumerate(lines):
        for pattern, item_type in PUBLIC_PATTERNS:
            match = re.search(pattern, line)
            if match:
                item_name = match.group(1)
                has_docs = has_doc_comment(lines, line_idx)
                items.append({
                    'name': item_name,
                    'type': item_type,
                    'has_docs': has_docs,
                    'line': line_idx + 1,
                    'file': str(filepath.relative_to('/home/coding/pdftract/crates/pdftract-core/src'))
                })
    return items

def main():
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')

    all_items = []
    for rs_file in sorted(src_dir.rglob('*.rs')):
        all_items.extend(audit_file(rs_file))

    # Group by type and coverage
    by_type = defaultdict(lambda: {'total': 0, 'with_docs': 0, 'missing': []})
    for item in all_items:
        by_type[item['type']]['total'] += 1
        if item['has_docs']:
            by_type[item['type']]['with_docs'] += 1
        else:
            by_type[item['type']]['missing'].append(item)

    # Print summary
    print("=" * 60)
    print("PDFTRACT-CORE DOCUMENTATION AUDIT")
    print("=" * 60)
    print()

    total_items = len(all_items)
    total_with_docs = sum(1 for i in all_items if i['has_docs'])

    print(f"TOTAL PUBLIC ITEMS: {total_items}")
    print(f"WITH DOCUMENTATION: {total_with_docs} ({100 * total_with_docs / total_items:.1f}%)")
    print(f"MISSING DOCUMENTATION: {total_items - total_with_docs} ({100 * (total_items - total_with_docs) / total_items:.1f}%)")
    print()

    print("BY TYPE:")
    print("-" * 40)
    for item_type, data in sorted(by_type.items()):
        coverage = 100 * data['with_docs'] / data['total'] if data['total'] > 0 else 0
        print(f"{item_type:12}: {data['with_docs']:4}/{data['total']:<4} ({coverage:5.1f}%)")
    print()

    # Print top missing items
    if any(by_type[t]['missing'] for t in by_type):
        print("TOP ITEMS MISSING DOCS (first 20 by type):")
        print("-" * 40)
        for item_type in sorted(by_type.keys()):
            missing = by_type[item_type]['missing'][:10]
            for item in missing:
                print(f"  [{item_type}] {item['name']} at {item['file']}:{item['line']}")

    print()
    print("=" * 60)

    # Return exit code based on 80% threshold
    coverage = 100 * total_with_docs / total_items if total_items > 0 else 0
    if coverage >= 80:
        print(f"✓ PASS: {coverage:.1f}% coverage meets 80% threshold")
        return 0
    else:
        print(f"✗ FAIL: {coverage:.1f}% coverage below 80% threshold")
        return 1

if __name__ == '__main__':
    exit(main())

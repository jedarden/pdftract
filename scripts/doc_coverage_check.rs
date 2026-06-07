#!/usr/bin/env python3
"""
Measure rustdoc coverage for pdftract-core.

Counts public items and determines how many have worked examples.
Goal: 80%+ of public items should have at least one worked example.
"""

import os
import re
import subprocess
from pathlib import Path
from collections import defaultdict

# Patterns for public items
PUB_PATTERNS = {
    'pub fn': re.compile(r'pub\s+fn\s+(\w+)\s*\('),
    'pub async fn': re.compile(r'pub\s+async\s+fn\s+(\w+)\s*\('),
    'pub struct': re.compile(r'pub\s+struct\s+(\w+)'),
    'pub enum': re.compile(r'pub\s+enum\s+(\w+)'),
    'pub trait': re.compile(r'pub\s+trait\s+(\w+)'),
    'pub type': re.compile(r'pub\s+type\s+(\w+)\s*='),
    'pub const': re.compile(r'pub\s+const\s+(\w+)\s*:'),
    'pub mod': re.compile(r'pub\s+mod\s+(\w+)'),
    'pub use': re.compile(r'pub\s+use\s+([^;]+)'),
}

# Patterns for examples in doc comments
EXAMPLE_PATTERNS = [
    re.compile(r'```rust[^-]'),  # ```rust (not ```rust,no_run)
    re.compile(r'```rust,no_run'),
    re.compile(r'```rust,ignore'),
]


def has_example(doc_comment: str) -> bool:
    """Check if a doc comment contains at least one code example."""
    if not doc_comment:
        return False
    for pattern in EXAMPLE_PATTERNS:
        if pattern.search(doc_comment):
            return True
    return False


def extract_doc_comment(lines: List[str], start_idx: int) -> str:
    """Extract doc comment lines before an item definition."""
    doc_lines = []
    i = start_idx - 1
    while i >= 0:
        line = lines[i].strip()
        if line.startswith('///') or line.startswith('//!'):
            doc_lines.insert(0, line)
            i -= 1
        elif line.startswith('//') and not line.startswith('///'):
            # Regular comment, not doc comment
            i -= 1
        else:
            break
    return '\n'.join(doc_lines)


def analyze_file(file_path: Path) -> dict:
    """Analyze a single Rust source file for public items and documentation."""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    content = ''.join(lines)
    items = []

    for i, line in enumerate(lines):
        line_stripped = line.strip()

        # Skip lines that are inside a comment or string
        if line_stripped.startswith('//') or line_stripped.startswith('/*'):
            continue

        # Check each pub pattern
        for item_type, pattern in PUB_PATTERNS.items():
            match = pattern.search(line)
            if match:
                item_name = match.group(1).split('(')[0].strip()  # Handle complex use statements
                doc_comment = extract_doc_comment(lines, i)
                has_ex = has_example(doc_comment)

                items.append({
                    'type': item_type,
                    'name': item_name,
                    'line': i + 1,
                    'has_example': has_ex,
                    'doc_length': len(doc_comment),
                })

    return items


def main():
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')

    all_items = []

    # Find all Rust files
    for rs_file in src_dir.rglob('*.rs'):
        # Skip test fixtures and tests directory
        if 'test' in str(rs_file) or 'fixture' in str(rs_file):
            continue

        items = analyze_file(rs_file)
        if items:
            all_items.extend(items)

    # Calculate coverage
    total = len(all_items)
    with_examples = sum(1 for item in all_items if item['has_example'])
    coverage = (with_examples / total * 100) if total > 0 else 0

    # Group by type
    by_type = defaultdict(lambda: {'total': 0, 'with_examples': 0})
    for item in all_items:
        by_type[item['type']]['total'] += 1
        if item['has_example']:
            by_type[item['type']]['with_examples'] += 1

    # Print report
    print("=" * 70)
    print("Rustdoc Coverage Report for pdftract-core")
    print("=" * 70)
    print(f"\nTotal public items: {total}")
    print(f"Items with examples: {with_examples} ({coverage:.1f}%)")
    print(f"\nGoal: 80%+ coverage")
    print(f"Status: {'✓ PASS' if coverage >= 80 else '✗ FAIL'}")

    print("\n" + "-" * 70)
    print("Breakdown by item type:")
    print("-" * 70)

    for item_type, counts in sorted(by_type.items()):
        type_coverage = (counts['with_examples'] / counts['total'] * 100) if counts['total'] > 0 else 0
        print(f"{item_type:20s}: {counts['with_examples']:4d}/{counts['total']:4d} ({type_coverage:5.1f}%)")

    # Items without examples (top 20)
    without_examples = [item for item in all_items if not item['has_example']]
    if without_examples:
        print("\n" + "-" * 70)
        print("Sample of items lacking examples (first 20):")
        print("-" * 70)
        for item in without_examples[:20]:
            print(f"  [{item['type']:12s}] {item['name']}")

        if len(without_examples) > 20:
            print(f"  ... and {len(without_examples) - 20} more")

    print("\n" + "=" * 70)
    return 0 if coverage >= 80 else 1


if __name__ == '__main__':
    import sys
    sys.exit(main())

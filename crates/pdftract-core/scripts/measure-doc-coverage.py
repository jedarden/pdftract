#!/usr/bin/env python3
"""
Measure rustdoc coverage for pdftract-core.

Counts:
- Total public items (pub fn/struct/enum/trait/type/const/mod)
- Items with doc comments (/// or //!)
- Items with worked examples (```rust code blocks)

Usage: python3 scripts/measure-doc-coverage.py
"""

import os
import re
from pathlib import Path
from typing import Dict, List, Tuple

# Simple Rust parser for extracting public items
def extract_public_items(file_path: Path) -> List[Tuple[str, str, str, List[str]]]:
    """
    Extract public items from a Rust source file.

    Returns: List of (item_type, name, doc_comment, location)
    """
    items = []
    content = file_path.read_text()
    lines = content.split('\n')

    # Track preceding doc comments
    doc_comment = []

    for i, line in enumerate(lines, 1):
        stripped = line.strip()

        # Collect doc comments
        if stripped.startswith('///') or stripped.startswith('//!'):
            doc_comment.append(stripped)
            continue
        elif doc_comment and (stripped.startswith('//') or stripped == ''):
            # Allow blank lines and regular comments within doc blocks
            continue
        elif not stripped or stripped.startswith('//') or stripped.startswith('#'):
            # Reset if we hit a blank line without a pub item
            if not stripped.startswith('#'):
                doc_comment = []
            continue

        # Check for public items
        if stripped.startswith('pub '):
            # Parse the item
            item_type = None
            name = None

            if 'pub fn ' in stripped:
                item_type = 'fn'
                match = re.search(r'pub\s+fn\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub struct ' in stripped:
                item_type = 'struct'
                match = re.search(r'pub\s+struct\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub enum ' in stripped:
                item_type = 'enum'
                match = re.search(r'pub\s+enum\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub trait ' in stripped:
                item_type = 'trait'
                match = re.search(r'pub\s+trait\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub type ' in stripped:
                item_type = 'type'
                match = re.search(r'pub\s+type\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub const ' in stripped:
                item_type = 'const'
                match = re.search(r'pub\s+const\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub mod ' in stripped:
                item_type = 'mod'
                match = re.search(r'pub\s+mod\s+(\w+)', stripped)
                if match:
                    name = match.group(1)
            elif 'pub use ' in stripped:
                # Skip re-exports for now (they inherit docs from the original)
                doc_comment = []
                continue

            if name:
                items.append((
                    item_type,
                    name,
                    '\n'.join(doc_comment),
                    f"{file_path.relative_to('/home/coding/pdftract/crates/pdftract-core/src')}:{i}"
                ))

        doc_comment = []

    return items


def has_worked_example(doc: str) -> bool:
    """Check if doc comment contains a worked example (```rust block)."""
    if not doc:
        return False
    return '```rust' in doc or '```rust,no_run' in doc or '```rust,ignore' in doc


def measure_coverage(src_dir: Path) -> Dict:
    """Measure documentation coverage across all source files."""
    results = {
        'total_items': 0,
        'with_docs': 0,
        'with_examples': 0,
        'by_type': {},
        'items_missing_examples': [],
    }

    for rs_file in src_dir.rglob('*.rs'):
        # Skip tests directory
        if 'tests' in str(rs_file):
            continue

        items = extract_public_items(rs_file)

        for item_type, name, doc, location in items:
            results['total_items'] += 1

            if item_type not in results['by_type']:
                results['by_type'][item_type] = {
                    'total': 0,
                    'with_docs': 0,
                    'with_examples': 0,
                }

            results['by_type'][item_type]['total'] += 1

            if doc:
                results['with_docs'] += 1
                results['by_type'][item_type]['with_docs'] += 1

            if has_worked_example(doc):
                results['with_examples'] += 1
                results['by_type'][item_type]['with_examples'] += 1
            else:
                results['items_missing_examples'].append((item_type, name, location))

    return results


def main():
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')
    results = measure_coverage(src_dir)

    total = results['total_items']
    with_docs = results['with_docs']
    with_examples = results['with_examples']

    doc_coverage = (with_docs / total * 100) if total > 0 else 0
    example_coverage = (with_examples / total * 100) if total > 0 else 0

    print(f"=== Rustdoc Coverage Report for pdftract-core ===\n")
    print(f"Total public items: {total}")
    print(f"With documentation: {with_docs} ({doc_coverage:.1f}%)")
    print(f"With worked examples: {with_examples} ({example_coverage:.1f}%)")
    print()

    print("By item type:")
    for item_type, stats in sorted(results['by_type'].items()):
        t_total = stats['total']
        t_docs = stats['with_docs']
        t_examples = stats['with_examples']
        t_doc_cov = (t_docs / t_total * 100) if t_total > 0 else 0
        t_ex_cov = (t_examples / t_total * 100) if t_total > 0 else 0
        print(f"  {item_type:8s}: {t_examples:3d}/{t_total:3d} with examples ({t_ex_cov:.0f}%)")

    print()

    if example_coverage < 80.0:
        print(f"⚠️  Target: 80% coverage. Current: {example_coverage:.1f}%")
        print(f"    Need {int(total * 0.8 - with_examples)} more examples.\n")

        # Show first 20 items missing examples
        missing = results['items_missing_examples'][:20]
        print(f"First 20 items missing examples (showing {len(missing)} of {len(results['items_missing_examples'])}):")
        for item_type, name, location in missing:
            print(f"  - {item_type:8s} {name:30s} ({location})")

        if len(results['items_missing_examples']) > 20:
            print(f"  ... and {len(results['items_missing_examples']) - 20} more")
    else:
        print(f"✅ Target met: {example_coverage:.1f}% >= 80%")


if __name__ == '__main__':
    main()

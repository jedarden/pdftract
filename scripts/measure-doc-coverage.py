#!/usr/bin/env python3
"""
Measure rustdoc worked-example coverage for pdftract-core public API.

This script scans source files and counts:
1. Total public items (pub fn, pub struct, pub enum, pub trait, pub type, pub const, pub mod)
2. Public items with at least one ```rust example block in their doc comment

The coverage percentage is (items_with_examples / total_public_items) * 100.
Target: 80%+ coverage.
"""

import os
import re
from pathlib import Path
from dataclasses import dataclass
from typing import List, Set, Tuple

@dataclass
class DocCoverage:
    """Documentation coverage metrics."""
    total_items: int = 0
    items_with_docs: int = 0
    items_with_examples: int = 0
    items_by_type: dict = None

    def __post_init__(self):
        if self.items_by_type is None:
            self.items_by_type = {}

    def coverage_pct(self) -> float:
        """Return percentage of items with examples."""
        if self.total_items == 0:
            return 0.0
        return (self.items_with_examples / self.total_items) * 100


def extract_public_items(content: str) -> List[Tuple[str, str, int]]:
    """
    Extract public items from Rust source content.

    Returns list of (item_type, name, line_number).
    """
    items = []
    lines = content.split('\n')
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        # Skip comments and empty lines
        if stripped.startswith('//') or stripped.startswith('/*') or not stripped:
            i += 1
            continue

        # Check for public items
        if 'pub ' in stripped or stripped.startswith('pub('):
            # Extract item type and name
            if 'pub fn ' in stripped:
                match = re.search(r'pub\s+(?:unsafe\s+)?(?:async\s+)?fn\s+(\w+)', stripped)
                if match:
                    items.append(('fn', match.group(1), i + 1))
            elif 'pub struct ' in stripped:
                match = re.search(r'pub\s+struct\s+(\w+)', stripped)
                if match:
                    items.append(('struct', match.group(1), i + 1))
            elif 'pub enum ' in stripped:
                match = re.search(r'pub\s+enum\s+(\w+)', stripped)
                if match:
                    items.append(('enum', match.group(1), i + 1))
            elif 'pub trait ' in stripped:
                match = re.search(r'pub\s+trait\s+(\w+)', stripped)
                if match:
                    items.append(('trait', match.group(1), i + 1))
            elif 'pub type ' in stripped:
                match = re.search(r'pub\s+type\s+(\w+)', stripped)
                if match:
                    items.append(('type', match.group(1), i + 1))
            elif 'pub const ' in stripped:
                match = re.search(r'pub\s+const\s+(\w+)', stripped)
                if match:
                    items.append(('const', match.group(1), i + 1))
            elif 'pub mod ' in stripped:
                match = re.search(r'pub\s+mod\s+(\w+)', stripped)
                if match:
                    items.append(('mod', match.group(1), i + 1))
            elif re.search(r'pub\s+use\s+.*;', stripped):
                # Skip pub use statements (re-exports)
                pass

        i += 1

    return items


def find_doc_comment_for_item(lines: List[str], item_line: int) -> str:
    """
    Find the doc comment for an item at the given line.

    Returns the full doc comment text (multiple lines).
    """
    # Look backwards from the item line for doc comments
    doc_lines = []
    i = item_line - 2  # Convert to 0-index and start before the item

    while i >= 0:
        line = lines[i].rstrip()
        if line.startswith('///'):
            doc_lines.insert(0, line[3:])  # Remove '///'
        elif line.startswith('//!'):
            doc_lines.insert(0, line[3:])  # Remove '//!'
        elif line.strip() and not (line.startswith('//') or line.strip() == '*'):
            # End of doc comment block
            break
        i -= 1

    return '\n'.join(doc_lines)


def has_rust_example(doc_comment: str) -> bool:
    """Check if a doc comment contains a ```rust example block."""
    return '```rust' in doc_comment


def measure_file_coverage(filepath: Path) -> DocCoverage:
    """Measure documentation coverage for a single Rust source file."""
    content = filepath.read_text()
    lines = content.split('\n')
    items = extract_public_items(content)

    coverage = DocCoverage()
    coverage.total_items = len(items)

    for item_type, item_name, item_line in items:
        doc_comment = find_doc_comment_for_item(lines, item_line)

        # Track items by type
        if item_type not in coverage.items_by_type:
            coverage.items_by_type[item_type] = {'total': 0, 'with_examples': 0}
        coverage.items_by_type[item_type]['total'] += 1

        if doc_comment:
            coverage.items_with_docs += 1
            if has_rust_example(doc_comment):
                coverage.items_with_examples += 1
                coverage.items_by_type[item_type]['with_examples'] += 1

    return coverage


def main():
    """Main entry point."""
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')

    if not src_dir.exists():
        print(f"Error: Source directory not found: {src_dir}")
        return 1

    # Find all .rs files
    rs_files = list(src_dir.rglob('*.rs'))

    total_coverage = DocCoverage()

    print(f"Scanning {len(rs_files)} Rust source files in {src_dir}...")
    print()

    for filepath in sorted(rs_files):
        relative_path = filepath.relative_to(src_dir)
        coverage = measure_file_coverage(filepath)

        if coverage.total_items > 0:
            print(f"{relative_path}: {coverage.items_with_examples}/{coverage.total_items} items with examples ({coverage.coverage_pct():.1f}%)")
            total_coverage.total_items += coverage.total_items
            total_coverage.items_with_docs += coverage.items_with_docs
            total_coverage.items_with_examples += coverage.items_with_examples

            # Merge type counts
            for item_type, counts in coverage.items_by_type.items():
                if item_type not in total_coverage.items_by_type:
                    total_coverage.items_by_type[item_type] = {'total': 0, 'with_examples': 0}
                total_coverage.items_by_type[item_type]['total'] += counts['total']
                total_coverage.items_by_type[item_type]['with_examples'] += counts['with_examples']

    print()
    print("=" * 60)
    print("TOTAL COVERAGE")
    print("=" * 60)
    print(f"Public items with doc comments: {total_coverage.items_with_docs}/{total_coverage.total_items} ({(total_coverage.items_with_docs/total_coverage.total_items*100):.1f}%)")
    print(f"Public items with examples: {total_coverage.items_with_examples}/{total_coverage.total_items} ({total_coverage.coverage_pct():.1f}%)")
    print()

    print("Breakdown by item type:")
    for item_type in sorted(total_coverage.items_by_type.keys()):
        counts = total_coverage.items_by_type[item_type]
        pct = (counts['with_examples'] / counts['total'] * 100) if counts['total'] > 0 else 0
        print(f"  {item_type:8s}: {counts['with_examples']:4d}/{counts['total']:4d} ({pct:5.1f}%)")

    print()
    target_pct = 80.0
    if total_coverage.coverage_pct() >= target_pct:
        print(f"✓ PASS: {total_coverage.coverage_pct():.1f}% >= {target_pct}% target")
        return 0
    else:
        print(f"✗ FAIL: {total_coverage.coverage_pct():.1f}% < {target_pct}% target (need {target_pct - total_coverage.coverage_pct():.1f}% more)")
        return 1


if __name__ == '__main__':
    exit(main())

#!/usr/bin/env python3
"""Analyze rustdoc coverage for pdftract-core.

This script counts:
- Total public items (fn, struct, enum, trait, type, const, mod)
- Items with rustdoc examples (```rust blocks)
- Coverage percentage
"""

import re
import subprocess
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass

@dataclass
class DocStats:
    """Statistics for documentation coverage."""
    total_items: int = 0
    items_with_docs: int = 0
    items_with_examples: int = 0
    items_by_type: dict = None

    def __post_init__(self):
        if self.items_by_type is None:
            self.items_by_type = defaultdict(lambda: dict(total=0, with_docs=0, with_examples=0))

    def coverage_pct(self):
        """Return percentage of items with documentation."""
        if self.total_items == 0:
            return 0.0
        return (self.items_with_docs / self.total_items) * 100

    def example_pct(self):
        """Return percentage of items with examples."""
        if self.total_items == 0:
            return 0.0
        return (self.items_with_examples / self.total_items) * 100


def extract_rustdoc_items(content: str, file_path: str) -> list:
    """Extract public items and their associated documentation from Rust source.

    Returns list of (item_type, name, has_doc, has_example, doc_content) tuples.
    """
    items = []
    lines = content.split('\n')
    i = 0

    # Patterns for public items
    patterns = {
        'fn': re.compile(r'pub\s+(?:async\s+)?fn\s+(\w+)'),
        'struct': re.compile(r'pub\s+struct\s+(\w+)'),
        'enum': re.compile(r'pub\s+enum\s+(\w+)'),
        'trait': re.compile(r'pub\s+trait\s+(\w+)'),
        'type': re.compile(r'pub\s+type\s+(\w+)'),
        'const': re.compile(r'pub\s+(?:const\s+|async\s+)?(\w+)\s*:'),
        'mod': re.compile(r'pub\s+mod\s+(\w+)'),
        'impl': re.compile(r'pub\s+impl'),  # impl blocks (trait impls)
    }

    # Track pending documentation
    pending_doc = []
    in_doc = False

    while i < len(lines):
        line = lines[i]

        # Check for doc comments
        if line.strip().startswith('///') or line.strip().startswith('//!'):
            pending_doc.append(line)
            in_doc = True
        elif in_doc and line.strip() and not line.strip().startswith('//'):
            # End of doc block, check for public item
            in_doc = False
            doc_content = '\n'.join(pending_doc)
            pending_doc = []

            # Check each pattern
            found_item = False
            for item_type, pattern in patterns.items():
                match = pattern.search(line)
                if match:
                    name = match.group(1) if item_type != 'impl' else f'<anonymous_{i}>'
                    has_example = '```rust' in doc_content
                    has_doc = len(doc_content) > 0

                    # Skip trait impls - they inherit doc from trait
                    if item_type != 'impl':
                        items.append((item_type, name, has_doc, has_example, doc_content))
                    found_item = True
                    break

            if not found_item and line.strip():
                # Check next few lines for the actual item
                for j in range(i+1, min(i+5, len(lines))):
                    for item_type, pattern in patterns.items():
                        match = pattern.search(lines[j])
                        if match:
                            name = match.group(1) if item_type != 'impl' else f'<anonymous_{j}>'
                            has_example = '```rust' in doc_content
                            has_doc = len(doc_content) > 0
                            if item_type != 'impl':
                                items.append((item_type, name, has_doc, has_example, doc_content))
                            break
        elif not in_doc and not line.strip().startswith('//'):
            # Check for public item without preceding doc
            for item_type, pattern in patterns.items():
                match = pattern.search(line)
                if match:
                    name = match.group(1) if item_type != 'impl' else f'<anonymous_{i}>'
                    if item_type != 'impl':
                        items.append((item_type, name, False, False, ''))
                    break

        i += 1

    return items


def analyze_source_file(file_path: Path) -> tuple:
    """Analyze a single Rust source file for documentation coverage.

    Returns (file_path, items_list)
    """
    try:
        content = file_path.read_text()
        items = extract_rustdoc_items(content, str(file_path))
        return (file_path, items)
    except Exception as e:
        print(f"Error reading {file_path}: {e}")
        return (file_path, [])


def main():
    """Main entry point."""
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')

    if not src_dir.exists():
        print(f"Source directory not found: {src_dir}")
        return

    # Find all Rust files
    rust_files = list(src_dir.rglob('*.rs'))
    print(f"Found {len(rust_files)} Rust files")

    # Analyze each file
    all_items = []
    for file_path in rust_files:
        _, items = analyze_source_file(file_path)
        all_items.extend([(file_path, *item) for item in items])

    # Calculate statistics
    stats = DocStats()
    for file_path, item_type, name, has_doc, has_example, _ in all_items:
        stats.total_items += 1
        if has_doc:
            stats.items_with_docs += 1
        if has_example:
            stats.items_with_examples += 1

        stats.items_by_type[item_type]['total'] += 1
        if has_doc:
            stats.items_by_type[item_type]['with_docs'] += 1
        if has_example:
            stats.items_by_type[item_type]['with_examples'] += 1

    # Print report
    print("\n" + "="*70)
    print("PDFTRACT-CORE RUSTDOC COVERAGE REPORT")
    print("="*70)
    print(f"\nTotal public items: {stats.total_items}")
    print(f"Items with documentation: {stats.items_with_docs} ({stats.coverage_pct():.1f}%)")
    print(f"Items with examples: {stats.items_with_examples} ({stats.example_pct():.1f}%)")
    print(f"\nTarget: 80%+ example coverage")
    print(f"Status: {'✓ PASS' if stats.example_pct() >= 80 else '✗ FAIL'}")

    print("\n" + "-"*70)
    print("BY TYPE")
    print("-"*70)
    print(f"{'Type':<12} {'Total':>8} {'With Doc':>10} {'With Ex':>10} {'Ex %':>8}")
    print("-"*70)

    for item_type in ['fn', 'struct', 'enum', 'trait', 'type', 'const', 'mod']:
        if item_type in stats.items_by_type:
            data = stats.items_by_type[item_type]
            total = data['total']
            with_docs = data['with_docs']
            with_ex = data['with_examples']
            ex_pct = (with_ex / total * 100) if total > 0 else 0
            print(f"{item_type:<12} {total:>8} {with_docs:>10} {with_ex:>10} {ex_pct:>7.1f}%")

    print("\n" + "-"*70)
    print("FILES NEEDING ATTENTION (public items without examples)")
    print("-"*70)

    # Group items by file
    files_needing_examples = defaultdict(list)
    for file_path, item_type, name, has_doc, has_example, _ in all_items:
        if not has_example:
            files_needing_examples[file_path].append((item_type, name))

    # Show files with most missing examples
    sorted_files = sorted(files_needing_examples.items(), key=lambda x: len(x[1]), reverse=True)
    for file_path, items in sorted_files[:15]:
        rel_path = file_path.relative_to(src_dir)
        print(f"\n{rel_path} ({len(items)} items without examples):")
        for item_type, name in items[:10]:
            print(f"  - {item_type} {name}")
        if len(items) > 10:
            print(f"  ... and {len(items) - 10} more")

    print("\n" + "="*70)


if __name__ == '__main__':
    main()

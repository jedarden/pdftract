#!/usr/bin/env python3
"""Assess rustdoc coverage for pdftract-core public API."""

import re
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass

@dataclass
class DocStats:
    total_items: int = 0
    with_docs: int = 0
    with_examples: int = 0
    items: list = None

    def __post_init__(self):
        if self.items is None:
            self.items = []

def extract_public_items(file_path: Path) -> DocStats:
    """Extract public items and their documentation status."""
    content = file_path.read_text()
    lines = content.split('\n')

    stats = DocStats()

    # Pattern to match public items
    patterns = {
        'pub fn': r'pub\s+fn\s+(\w+)',
        'pub struct': r'pub\s+struct\s+(\w+)',
        'pub enum': r'pub\s+enum\s+(\w+)',
        'pub trait': r'pub\s+trait\s+(\w+)',
        'pub const': r'pub\s+const\s+(\w+)',
        'pub type': r'pub\s+type\s+(\w+)',
        'pub mod': r'pub\s+mod\s+(\w+)',
    }

    for i, line in enumerate(lines):
        for item_type, pattern in patterns.items():
            match = re.search(pattern, line)
            if match:
                name = match.group(1)
                stats.total_items += 1

                # Check for doc comment above
                has_doc = False
                has_example = False

                # Look back for doc comments (/// or //!)
                j = i - 1
                doc_lines = []
                while j >= 0 and (lines[j].strip().startswith('///') or lines[j].strip().startswith('//!') or lines[j].strip() == ''):
                    if lines[j].strip().startswith('///') or lines[j].strip().startswith('//!'):
                        doc_lines.append(lines[j])
                    j -= 1

                has_doc = len(doc_lines) > 0
                has_example = any('```rust' in dl or '```no_run' in dl or '```ignore' in dl for dl in doc_lines)

                if has_doc:
                    stats.with_docs += 1
                if has_example:
                    stats.with_examples += 1

                stats.items.append({
                    'name': name,
                    'type': item_type,
                    'file': str(file_path),
                    'line': i + 1,
                    'has_doc': has_doc,
                    'has_example': has_example,
                })

    return stats

def main():
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')

    all_stats = DocStats()
    module_docs = {}

    for rs_file in src_dir.rglob('*.rs'):
        # Skip files in tests/ and examples/
        if 'tests' in rs_file.parts or 'examples' in rs_file.parts:
            continue

        stats = extract_public_items(rs_file)

        if stats.total_items > 0:
            module_name = rs_file.relative_to(src_dir)
            module_docs[module_name] = stats
            all_stats.total_items += stats.total_items
            all_stats.with_docs += stats.with_docs
            all_stats.with_examples += stats.with_examples

    print(f"Total public items: {all_stats.total_items}")
    print(f"With documentation: {all_stats.with_docs} ({all_stats.with_docs/all_stats.total_items*100:.1f}%)")
    print(f"With examples: {all_stats.with_examples} ({all_stats.with_examples/all_stats.total_items*100:.1f}%)")
    print()

    # Show modules with worst coverage
    print("Modules needing documentation (sorted by items without examples):")
    for module, stats in sorted(module_docs.items(), key=lambda x: x[1].total_items - x[1].with_examples, reverse=True):
        if stats.total_items > 0:
            coverage = stats.with_examples / stats.total_items * 100 if stats.total_items > 0 else 0
            print(f"  {module}: {stats.with_examples}/{stats.total_items} ({coverage:.0f}%)")

    # List items without docs
    print("\nItems WITHOUT any documentation:")
    for module, stats in module_docs.items():
        for item in stats.items:
            if not item['has_doc']:
                print(f"  {module}:{item['line']} - {item['type']} {item['name']}")

if __name__ == '__main__':
    main()

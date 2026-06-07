#!/usr/bin/env python3
"""
Measure rustdoc coverage for pdftract-core.

This script scans all .rs files and counts:
- Public items (pub fn/struct/enum/trait/type/mod/const)
- Items with documentation (/// or /*!)
- Items with worked examples (```rust blocks in doc comments)
"""

import os
import re
from pathlib import Path
from dataclasses import dataclass
from typing import Dict, List

@dataclass
class FileStats:
    """Statistics for a single source file."""
    path: str
    pub_items: int
    with_doc: int
    with_example: int
    items: List[Dict]

def extract_public_items(content: str, filepath: str) -> List[Dict]:
    """Extract public items from Rust source code.

    Returns a list of dicts with keys: kind, name, has_doc, has_example, line
    """
    items = []
    lines = content.split('\n')

    # Patterns for public items
    patterns = [
        (r'pub\s+(?:async\s+)?fn\s+(\w+)', 'fn'),
        (r'pub\s+struct\s+(\w+)', 'struct'),
        (r'pub\s+enum\s+(\w+)', 'enum'),
        (r'pub\s+trait\s+(\w+)', 'trait'),
        (r'pub\s+type\s+(\w+)', 'type'),
        (r'pub\s+mod\s+(\w+)', 'mod'),
        (r'pub\s+(?:const|static)\s+(\w+)', 'const'),
        (r'pub\s+use\s+(?:(\w+)|.*\s+as\s+(\w+))', 'use'),  # pub use X as Y
        (r'impl\s+(\w+)\s*\{', 'impl'),  # impl blocks (inherent impls)
    ]

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        # Skip lines that are just comments or empty
        if stripped.startswith('//') or not stripped:
            i += 1
            continue

        # Check if this line declares a public item
        matched = False
        for pattern, kind in patterns:
            match = re.search(pattern, line)
            if match:
                # Get the name (handle both groups for pub use case)
                name = match.group(1) or match.group(2) if match.lastindex >= 2 else match.group(1)
                if name:
                    # Look back for documentation comments
                    has_doc = False
                    has_example = False
                    doc_lines = []

                    j = i - 1
                    while j >= 0:
                        prev_line = lines[j].strip()
                        if prev_line.startswith('///') or prev_line.startswith('//!'):
                            has_doc = True
                            doc_lines.insert(0, prev_line[3:])
                            # Check for example blocks
                            if '```' in prev_line:
                                has_example = True
                        elif prev_line.startswith('/**') or prev_line.startswith('/*!'):
                            has_doc = True
                            # Multi-line comment - scan forward
                            k = j
                            while k < len(lines):
                                curr = lines[k].strip()
                                if '```' in curr:
                                    has_example = True
                                if curr.endswith('*/') or curr.endswith('*/)'):
                                    break
                                k += 1
                            break
                        elif prev_line and not prev_line.startswith('//'):
                            # Non-comment, non-empty line - stop looking back
                            break
                        j -= 1

                    items.append({
                        'kind': kind,
                        'name': name,
                        'line': i + 1,
                        'has_doc': has_doc,
                        'has_example': has_example,
                        'doc_lines': doc_lines
                    })
                    matched = True
                    break

        # Special handling for re-exports that span multiple lines
        if not matched and 'pub use' in line:
            # This might be a multi-line pub use - skip for now
            pass

        i += 1

    return items

def scan_directory(src_dir: Path) -> Dict[str, FileStats]:
    """Scan all .rs files in the source directory."""
    stats = {}

    for rs_file in src_dir.rglob('*.rs'):
        # Skip tests and benchmarks directories
        if 'tests' in rs_file.parts or 'benches' in rs_file.parts:
            continue

        try:
            with open(rs_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
        except Exception as e:
            print(f"Warning: Could not read {rs_file}: {e}")
            continue

        relative_path = rs_file.relative_to(src_dir.parent)
        items = extract_public_items(content, str(rs_file))

        if items:
            with_doc = sum(1 for it in items if it['has_doc'])
            with_example = sum(1 for it in items if it['has_example'])

            stats[str(relative_path)] = FileStats(
                path=str(relative_path),
                pub_items=len(items),
                with_doc=with_doc,
                with_example=with_example,
                items=items
            )

    return stats

def print_summary(stats: Dict[str, FileStats]):
    """Print summary statistics."""
    total_items = sum(s.pub_items for s in stats.values())
    total_with_doc = sum(s.with_doc for s in stats.values())
    total_with_example = sum(s.with_example for s in stats.values())

    doc_coverage = (total_with_doc / total_items * 100) if total_items > 0 else 0
    example_coverage = (total_with_example / total_items * 100) if total_items > 0 else 0

    print("=" * 70)
    print("RUSTDOC COVERAGE SUMMARY")
    print("=" * 70)
    print(f"\nTotal public items: {total_items}")
    print(f"With documentation: {total_with_doc} ({doc_coverage:.1f}%)")
    print(f"With examples: {total_with_example} ({example_coverage:.1f}%)")
    print()

    # Files with low example coverage
    print("Files with lowest example coverage (top 10):")
    print("-" * 70)
    sorted_files = sorted(
        stats.items(),
        key=lambda x: (x[1].pub_items - x[1].with_example) if x[1].pub_items > 0 else 0,
        reverse=True
    )

    for i, (path, stat) in enumerate(sorted_files[:10]):
        if stat.pub_items > 0:
            cov = (stat.with_example / stat.pub_items * 100) if stat.pub_items > 0 else 0
            print(f"{i+1:2d}. {path:50s} {stat.with_example:3d}/{stat.pub_items:3d} ({cov:5.1f}%)")

    print()

    # Files lacking documentation entirely
    no_doc_files = [(p, s) for p, s in stats.items() if s.with_doc == 0 and s.pub_items > 0]
    if no_doc_files:
        print("Files with NO documentation:")
        print("-" * 70)
        for path, stat in no_doc_files[:10]:
            print(f"  {path}: {stat.pub_items} undocumented items")
        print()

    # Specific items without documentation
    undocumented = []
    for path, stat in stats.items():
        for item in stat.items:
            if not item['has_doc']:
                undocumented.append((path, item))

    if undocumented:
        print(f"Undocumented items (showing first 20 of {len(undocumented)}):")
        print("-" * 70)
        for i, (path, item) in enumerate(undocumented[:20]):
            print(f"{i+1:2d}. {path:45s} {item['kind']:8s} {item['name']}")
        print()

    # Items without examples
    no_example = []
    for path, stat in stats.items():
        for item in stat.items:
            if not item['has_example'] and item['kind'] in ('fn', 'struct', 'enum', 'trait'):
                no_example.append((path, item))

    if no_example:
        print(f"Items without examples (showing first 30 of {len(no_example)}):")
        print("-" * 70)
        for i, (path, item) in enumerate(no_example[:30]):
            print(f"{i+1:2d}. {path:45s} {item['kind']:8s} {item['name']}")
        print()

def main():
    src_dir = Path(__file__).parent / 'src'

    if not src_dir.exists():
        print(f"Error: Source directory not found: {src_dir}")
        return 1

    print(f"Scanning {src_dir}...")
    stats = scan_directory(src_dir)
    print_summary(stats)

    # Return non-zero if example coverage < 80%
    total_items = sum(s.pub_items for s in stats.values())
    total_with_example = sum(s.with_example for s in stats.values())
    coverage = (total_with_example / total_items * 100) if total_items > 0 else 0

    print("=" * 70)
    if coverage >= 80:
        print(f"✓ PASS: Example coverage {coverage:.1f}% >= 80%")
        return 0
    else:
        print(f"✗ FAIL: Example coverage {coverage:.1f}% < 80%")
        return 1

if __name__ == '__main__':
    exit(main())

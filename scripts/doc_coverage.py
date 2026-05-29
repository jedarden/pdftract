#!/usr/bin/env python3
"""Measure rustdoc coverage for pdftract-core public API."""

import os
import re
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple

RUST_KEYWORDS = {
    'where', 'let', 'mut', 'if', 'else', 'for', 'while', 'loop', 'match',
    'return', 'break', 'continue', 'impl', 'struct', 'enum', 'trait',
    'type', 'fn', 'const', 'static', 'mod', 'use', 'crate', 'super',
    'self', 'Self', 'extern', 'unsafe', 'async', 'await', 'move',
    'ref', 'True', 'False', 'Some', 'None', 'Ok', 'Err', 'Vec',
    'String', 'Box', 'Result', 'Option', 'u8', 'u16', 'u32', 'u64',
    'i8', 'i16', 'i32', 'i64', 'f32', 'f64', 'bool', 'usize', 'isize'
}


def extract_items_from_file(filepath: Path) -> List[Tuple[str, str, int, bool]]:
    """Extract public items from a Rust source file.

    Returns: List of (name, kind, line_number, has_example) tuples.
    """
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    items = []
    lines = content.split('\n')

    # Track current doc comment for next item
    pending_doc = None

    for i, line in enumerate(lines, 1):
        stripped = line.strip()

        # Skip empty lines and non-doc comments
        if not stripped or stripped.startswith('//') and not stripped.startswith('///'):
            if stripped.startswith('//') and not stripped.startswith('///'):
                pending_doc = None
            continue

        # Track doc comments
        if stripped.startswith('///'):
            if pending_doc is None:
                pending_doc = []
            pending_doc.append(stripped)
            continue

        # Check for attribute lines (cfg, derive, etc.) - don't reset doc
        if stripped.startswith('#['):
            continue

        # Check for pub items
        if stripped.startswith('pub '):
            # Extract item kind and name
            kind_match = re.search(r'pub (fn|struct|enum|trait|type|const|mod|use)\s+(\w+)', stripped)
            if not kind_match:
                # Handle complex cases like `pub use foo::Bar;`
                use_match = re.search(r'pub use\s+(.+?);', stripped)
                if use_match:
                    item_name = use_match.group(1).split('::')[-1].rstrip(';')
                    kind = 'use'
                else:
                    continue
            else:
                kind = kind_match.group(1)
                item_name = kind_match.group(2)

            # Skip known items that are re-exports
            if item_name in RUST_KEYWORDS:
                pending_doc = None
                continue

            # Check if doc has examples
            has_example = False
            if pending_doc:
                doc_text = '\n'.join(pending_doc)
                has_example = '```rust' in doc_text or '```no_run' in doc_text

            items.append((item_name, kind, i, has_example))
            pending_doc = None

        # Reset doc if we encounter something else
        elif stripped and not stripped.startswith('#') and not stripped.startswith('use'):
            pending_doc = None

    return items


def scan_directory(src_dir: Path) -> Dict[str, List[Tuple[str, str, int, bool]]]:
    """Scan all Rust files in a directory."""
    all_items = {}

    for rust_file in src_dir.rglob('*.rs'):
        # Skip test files and tests modules
        if 'tests.rs' in rust_file.name or 'test_' in rust_file.name:
            continue
        if any(p.startswith('test') or p == 'benches' for p in rust_file.parts):
            continue

        relative = rust_file.relative_to(src_dir)
        module_path = str(relative.with_suffix(''))

        items = extract_items_from_file(rust_file)
        if items:
            all_items[module_path] = items

    return all_items


def print_report(all_items: Dict[str, List[Tuple[str, str, int, bool]]]):
    """Print coverage report."""
    total = 0
    with_examples = 0
    by_kind = defaultdict(lambda: [0, 0])  # kind -> [total, with_examples]

    print("=" * 80)
    print("RUSTDOC COVERAGE REPORT")
    print("=" * 80)

    for module_path in sorted(all_items.keys()):
        items = all_items[module_path]
        if not items:
            continue

        module_total = len(items)
        module_with = sum(1 for _, _, _, has_ex in items if has_ex)
        module_pct = (module_with / module_total * 100) if module_total else 0

        print(f"\n{module_path}:")
        print(f"  {module_with}/{module_total} items with examples ({module_pct:.1f}%)")

        # List missing examples
        missing = [name for name, kind, _, has_ex in items if not has_ex and kind in ('fn', 'struct', 'enum', 'trait', 'type')]
        if missing:
            print(f"  Missing examples: {', '.join(missing[:10])}", end='')
            if len(missing) > 10:
                print(f" ... and {len(missing) - 10} more")
            else:
                print()

        total += module_total
        with_examples += module_with

        for _, kind, _, has_ex in items:
            by_kind[kind][0] += 1
            if has_ex:
                by_kind[kind][1] += 1

    overall_pct = (with_examples / total * 100) if total else 0
    print("\n" + "=" * 80)
    print(f"OVERALL: {with_examples}/{total} items with examples ({overall_pct:.1f}%)")
    print("=" * 80)

    print("\nBy kind:")
    for kind in sorted(by_kind.keys()):
        t, w = by_kind[kind]
        pct = (w / t * 100) if t else 0
        print(f"  {kind:10s}: {w:4d}/{t:4d} ({pct:5.1f}%)")

    # Threshold check
    print("\n" + "=" * 80)
    if overall_pct >= 80:
        print("PASS: Meets 80% threshold")
    else:
        print(f"FAIL: Below 80% threshold (need {int((0.8 * total) - with_examples)} more examples)")
    print("=" * 80)


if __name__ == '__main__':
    src_dir = Path('/home/coding/pdftract/crates/pdftract-core/src')
    all_items = scan_directory(src_dir)
    print_report(all_items)

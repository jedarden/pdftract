#!/usr/bin/env python3
"""Count public items in pdftract-core and measure documentation coverage."""

import subprocess
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple

def run_cargo_doc() -> str:
    """Run cargo doc and capture output."""
    result = subprocess.run(
        ["cargo", "doc", "--no-deps", "--all-features", "-p", "pdftract-core"],
        cwd=Path("/home/coding/pdftract"),
        capture_output=True,
        text=True
    )
    return result.stdout + result.stderr

def has_example(doc: str) -> bool:
    """Check if documentation contains a code example."""
    if not doc:
        return False
    # Look for ```rust, ```no_run, ```ignore, etc.
    return bool(re.search(r'```rust', doc))

def extract_docs_from_file(file_path: Path) -> List[Tuple[str, str, bool, str]]:
    """Extract public items and their docs from a Rust file."""
    items = []

    content = file_path.read_text()
    lines = content.split('\n')

    # Track current doc comment being built
    current_doc = []
    doc_line_start = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Check for doc comments
        if stripped.startswith("///"):
            current_doc.append(stripped[3:].strip())
            if not doc_line_start:
                doc_line_start = i + 1
        elif stripped.startswith("//!"):
            # Module-level doc - skip for item-level tracking
            pass
        elif stripped.startswith("//"):
            # Regular comment - skip
            pass
        else:
            # Check if this is a public item declaration
            if current_doc:
                pub_match = re.match(r'pub\b\s*(fn|struct|enum|trait|type|const|static|mod)\b\s*(\w+)?', stripped)
                if pub_match:
                    item_type = pub_match.group(1)
                    item_name = pub_match.group(2) or f"anon_{i}"
                    doc_text = "\n".join(current_doc)
                    items.append((item_type, item_name, has_example(doc_text), file_path.name))
                current_doc = []
                doc_line_start = 0

    return items

def main():
    """Main entry point."""
    print("Checking pdftract-core documentation coverage...\n")

    # First, run cargo doc to check for warnings
    print("Running cargo doc --no-deps --all-features...")
    result = subprocess.run(
        ["cargo", "doc", "--no-deps", "--all-features", "-p", "pdftract-core"],
        cwd=Path("/home/coding/pdftract"),
        capture_output=True,
        text=True
    )

    has_warnings = "warning:" in result.stdout or "warning:" in result.stderr
    has_missing_docs = "missing documentation" in result.stdout or "missing documentation" in result.stderr

    if has_warnings:
        print("⚠️  Warnings found:")
        for line in (result.stdout + result.stderr).split('\n'):
            if 'warning:' in line or 'warning:' in line.lower():
                print(f"  {line.strip()}")
    elif has_missing_docs:
        print("❌ Missing documentation warnings found")
    else:
        print("✅ No warnings - cargo doc passes!")

    print("\nScanning source files for public items with examples...")

    src_dir = Path("/home/coding/pdftract/crates/pdftract-core/src")
    all_items: List[Tuple[str, str, bool, str]] = []

    for rs_file in src_dir.rglob("*.rs"):
        if rs_file.name == "lib.rs":
            continue  # Already well-documented
        items = extract_docs_from_file(rs_file)
        all_items.extend(items)

    # Count by category
    total_items = len(all_items)
    items_with_examples = sum(1 for _, _, has_ex, _ in all_items if has_ex)
    coverage = (items_with_examples / total_items * 100) if total_items > 0 else 0

    print(f"\n📊 Documentation Coverage:")
    print(f"  Total public items: {total_items}")
    print(f"  With examples: {items_with_examples}")
    print(f"  Coverage: {coverage:.1f}%")

    # Show items without examples by type
    by_type: Dict[str, List[Tuple[str, bool, str]]] = {}
    for item_type, item_name, has_ex, file_name in all_items:
        if item_type not in by_type:
            by_type[item_type] = []
        by_type[item_type].append((item_name, has_ex, file_name))

    print(f"\n📋 By item type:")
    for item_type, items in sorted(by_type.items()):
        with_ex = sum(1 for _, h, _ in items if h)
        total = len(items)
        cov = (with_ex / total * 100) if total > 0 else 0
        print(f"  {item_type}: {with_ex}/{total} ({cov:.0f}%)")

    # Find high-value modules needing examples
    print(f"\n🔍 High-value modules needing examples:")
    high_value_modules = [
        "extract.rs", "document.rs", "parser/mod.rs", "span/mod.rs",
        "table/mod.rs", "layout/mod.rs", "output/mod.rs"
    ]
    for mod_name in high_value_modules:
        mod_items = [(t, n, h) for t, n, h, f in all_items if f == mod_name]
        if mod_items:
            with_ex = sum(1 for _, _, h in mod_items if h)
            total = len(mod_items)
            cov = (with_ex / total * 100) if total > 0 else 0
            if cov < 80:
                print(f"  {mod_name}: {with_ex}/{total} ({cov:.0f}%)")

    # Check against threshold
    if coverage >= 80:
        print(f"\n✅ PASS: {coverage:.1f}% >= 80% threshold")
        return 0
    else:
        print(f"\n❌ FAIL: {coverage:.1f}% < 80% threshold")
        print(f"   Need {int((80 - coverage) / 100 * total_items)} more items with examples")
        return 1

if __name__ == "__main__":
    exit(main())

#!/usr/bin/env python3
"""
Script to analyze rustdoc coverage in pdftract-core.

Measures:
- Total public items (pub fn, pub struct, pub enum, pub trait, pub type)
- Public items with documentation
- Public items with worked examples (```rust blocks)
"""
import subprocess
import re
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, List

@dataclass
class ModuleStats:
    total: int = 0
    with_doc: int = 0
    with_example: int = 0
    items: List[str] = None

    def __post_init__(self):
        if self.items is None:
            self.items = []

def run_rg(pattern: str, path: Path) -> str:
    """Run ripgrep and return output."""
    result = subprocess.run(
        ["rg", pattern, str(path), "-n", "-A", "10", "--type", "rust"],
        capture_output=True,
        text=True,
        cwd="/home/coding/pdftract"
    )
    return result.stdout

def analyze_module(module_path: Path) -> ModuleStats:
    """Analyze a single module file for rustdoc coverage."""
    stats = ModuleStats()

    content = module_path.read_text()
    lines = content.split("\n")

    # Track public items
    for i, line in enumerate(lines):
        # Look for pub items
        for pattern in [
            r"pub\s+fn\s+(\w+)",
            r"pub\s+struct\s+(\w+)",
            r"pub\s+enum\s+(\w+)",
            r"pub\s+trait\s+(\w+)",
            r"pub\s+type\s+(\w+)",
            r"pub\s+mod\s+(\w+)",
        ]:
            match = re.search(pattern, line)
            if match:
                item_name = match.group(1)
                stats.total += 1
                stats.items.append(f"{line.strip()}:{i+1}")

                # Check for documentation above
                has_doc = False
                has_example = False

                # Look back up to 20 lines for doc comments
                for j in range(max(0, i - 20), i):
                    prev_line = lines[j].strip()
                    if prev_line.startswith("///") or prev_line.startswith("//!"):
                        has_doc = True
                        # Check for example within doc
                        if "```rust" in prev_line or "```rust,no_run" in prev_line or "```ignore" in prev_line:
                            has_example = True
                        # Also check a few lines after the doc start
                        for k in range(j+1, min(j+10, i)):
                            if "```rust" in lines[k]:
                                has_example = True
                    elif not prev_line.startswith("//") and prev_line and not prev_line.startswith("#"):
                        # Stop if we hit something that's not a comment
                        if j < i - 1 and lines[j+1].strip().startswith("#"):
                            continue
                        if j < i - 2:
                            break

                if has_doc:
                    stats.with_doc += 1
                if has_example:
                    stats.with_example += 1

    return stats

def main():
    """Main analysis function."""
    src_dir = Path("/home/coding/pdftract/crates/pdftract-core/src")

    print(f"Analyzing rustdoc coverage for pdftract-core")
    print(f"=" * 60)

    total_stats = ModuleStats()
    module_stats: Dict[str, ModuleStats] = {}

    # Analyze each module
    for rs_file in sorted(src_dir.rglob("*.rs")):
        # Skip main.rs and test files
        if "tests" in str(rs_file) or rs_file.name == "main.rs":
            continue

        # Get module name from path
        rel_path = rs_file.relative_to(src_dir)
        if str(rel_path) == "lib.rs":
            continue

        module_name = str(rel_path).replace("/", "::").replace(".rs", "")
        stats = analyze_module(rs_file)

        if stats.total > 0:
            module_stats[module_name] = stats
            total_stats.total += stats.total
            total_stats.with_doc += stats.with_doc
            total_stats.with_example += stats.with_example

    # Print report
    print(f"\nOverall Coverage:")
    print(f"  Total public items: {total_stats.total}")
    print(f"  With documentation: {total_stats.with_doc} ({100*total_stats.with_doc/total_stats.total:.1f}%)")
    print(f"  With examples: {total_stats.with_example} ({100*total_stats.with_example/total_stats.total:.1f}%)")
    print()

    print(f"Top modules by public items:")
    sorted_modules = sorted(module_stats.items(), key=lambda x: x[1].total, reverse=True)[:15]
    for name, stats in sorted_modules:
        doc_pct = 100 * stats.with_doc / stats.total if stats.total > 0 else 0
        ex_pct = 100 * stats.with_example / stats.total if stats.total > 0 else 0
        print(f"  {name:50s} items:{stats.total:3d} docs:{doc_pct:5.1f}% examples:{ex_pct:5.1f}%")

if __name__ == "__main__":
    main()

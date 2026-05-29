#!/usr/bin/env python3
"""
Measure rustdoc coverage for pdftract-core public API.
Counts public items and tracks which have doc comments with examples.
"""

import os
import re
from pathlib import Path
from dataclasses import dataclass
from typing import List, Set, Dict

@dataclass
class DocStats:
    """Statistics for documentation coverage."""
    total_items: int = 0
    documented_items: int = 0
    with_examples: int = 0
    items_with_examples: List[str] = None
    
    def __post_init__(self):
        if self.items_with_examples is None:
            self.items_with_examples = []

def extract_rust_items(content: str, filename: str) -> List[tuple]:
    """
    Extract public items from Rust source code.
    Returns list of (item_type, name, line_number, has_doc, has_example) tuples.
    """
    items = []
    lines = content.split('\n')
    i = 0
    in_doc_block = False
    doc_lines = []
    
    # Patterns for public items
    patterns = {
        'pub fn': re.compile(r'pub\s+(?:async\s+)?fn\s+(\w+)'),
        'pub struct': re.compile(r'pub\s+struct\s+(\w+)'),
        'pub enum': re.compile(r'pub\s+enum\s+(\w+)'),
        'pub trait': re.compile(r'pub\s+trait\s+(\w+)'),
        'pub const': re.compile(r'pub\s+const\s+(\w+)'),
        'pub type': re.compile(r'pub\s+type\s+(\w+)'),
        'pub mod': re.compile(r'pub\s+mod\s+(\w+)'),
        'impl': re.compile(r'impl\s+(\w+)'),  # For trait impls
    }
    
    while i < len(lines):
        line = lines[i].strip()
        
        # Track doc comments
        if line.startswith('///') or line.startswith('//!'):
            in_doc_block = True
            doc_lines.append(line)
        elif line.startswith('/*!') or line.startswith('/**!'):
            # Block doc start
            in_doc_block = True
            doc_lines.append(line)
        elif in_doc_block and (line.startswith('*/') or line.startswith('/*!') or line.startswith('/**!')):
            # End of block doc
            doc_lines.append(line)
        elif in_doc_block and not (line.startswith('/*') or line.startswith('*') or not line):
            # Still in doc block or continuation
            if line.startswith('*') or line.startswith('/*') or line.startswith('*/'):
                doc_lines.append(line)
            else:
                in_doc_block = False
        else:
            # Check for public items
            for item_type, pattern in patterns.items():
                match = pattern.search(line)
                if match:
                    name = match.group(1)
                    has_doc = len(doc_lines) > 0
                    has_example = any('```' in dl for dl in doc_lines)
                    
                    # Only count if it's actually public (not `pub(crate)` etc)
                    if 'pub(' not in lines[i][max(0, lines[i].find('pub')-10):lines[i].find('pub')+20]:
                        items.append((item_type, name, i + 1, has_doc, has_example, filename))
                    
                    doc_lines = []
                    break
            else:
                # No match found, reset doc tracking
                if not line.startswith('*') and not line.startswith('/*') and line and not line.startswith('//'):
                    doc_lines = []
                in_doc_block = False
        
        i += 1
    
    return items

def scan_directory(src_dir: Path) -> Dict[str, DocStats]:
    """Scan all Rust files in src directory."""
    all_items = []
    
    for rs_file in src_dir.rglob('*.rs'):
        if 'tests' in str(rs_file) or 'examples' in str(rs_file):
            continue
            
        content = rs_file.read_text(encoding='utf-8', errors='ignore')
        items = extract_rust_items(content, str(rs_file))
        all_items.extend(items)
    
    stats = DocStats()
    stats.total_items = len(all_items)
    stats.documented_items = sum(1 for item in all_items if item[3])
    stats.with_examples = sum(1 for item in all_items if item[4])
    stats.items_with_examples = [f"{item[0]} {item[1]} ({item[5]}:{item[2]})" for item in all_items if item[4]]
    
    return stats, all_items

def main():
    src_dir = Path('crates/pdftract-core/src')
    
    print("Scanning pdftract-core for public API items...")
    stats, all_items = scan_directory(src_dir)
    
    print(f"\n=== Documentation Coverage Report ===")
    print(f"Total public items: {stats.total_items}")
    print(f"Documented items: {stats.documented_items} ({stats.documented_items/max(1,stats.total_items)*100:.1f}%)")
    print(f"With examples: {stats.with_examples} ({stats.with_examples/max(1,stats.total_items)*100:.1f}%)")
    print(f"\nTarget: 80% coverage")
    print(f"Current: {stats.with_examples/max(1,stats.total_items)*100:.1f}%")
    print(f"Gap: {max(0, 0.8 * stats.total_items - stats.with_examples):.0f} items need examples")
    
    # Show items by type
    from collections import defaultdict
    by_type = defaultdict(list)
    for item in all_items:
        by_type[item[0]].append(item)
    
    print(f"\n=== Breakdown by type ===")
    for item_type, items in sorted(by_type.items()):
        total = len(items)
        with_ex = sum(1 for i in items if i[4])
        print(f"{item_type}: {with_ex}/{total} ({with_ex/max(1,total)*100:.0f}%)")
    
    # Show undocumented items
    undocumented = [item for item in all_items if not item[3]]
    if undocumented:
        print(f"\n=== Undocumented items ({len(undocumented)}) ===")
        for item in sorted(undocumented, key=lambda x: (x[5], x[2]))[:50]:
            print(f"  {item[0]} {item[1]} at {item[5]}:{item[2]}")
        if len(undocumented) > 50:
            print(f"  ... and {len(undocumented) - 50} more")
    
    # Show documented without examples
    doc_no_ex = [item for item in all_items if item[3] and not item[4]]
    if doc_no_ex:
        print(f"\n=== Documented but without examples ({len(doc_no_ex)}) ===")
        for item in sorted(doc_no_ex, key=lambda x: (x[5], x[2]))[:50]:
            print(f"  {item[0]} {item[1]} at {item[5]}:{item[2]}")
        if len(doc_no_ex) > 50:
            print(f"  ... and {len(doc_no_ex) - 50} more")

if __name__ == '__main__':
    main()

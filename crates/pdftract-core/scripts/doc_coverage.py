#!/usr/bin/env python3
"""Analyze rustdoc coverage for pdftract-core."""

import os
import re
from pathlib import Path
from collections import defaultdict

# Patterns for public API items
PUB_PATTERNS = {
    'function': re.compile(r'^pub\s+(?:async\s+)?fn\s+(\w+)'),
    'struct': re.compile(r'^pub\s+struct\s+(\w+)'),
    'enum': re.compile(r'^pub\s+enum\s+(\w+)'),
    'trait': re.compile(r'^pub\s+trait\s+(\w+)'),
    'type': re.compile(r'^pub\s+type\s+(\w+)'),
    'module': re.compile(r'^pub\s+mod\s+(\w+)'),
    'const': re.compile(r'^pub\s+(?:const|static)\s+(\w+)'),
}

# Pattern for doc comments with examples
DOC_WITH_EXAMPLE = re.compile(r'```rust[^`]*```', re.DOTALL)

def count_items_and_examples(content: str) -> dict:
    """Count public items and those with examples."""
    counts = defaultdict(lambda: {'total': 0, 'with_examples': 0})
    
    lines = content.split('\n')
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Check each pattern
        for item_type, pattern in PUB_PATTERNS.items():
            match = pattern.match(line)
            if match:
                counts[item_type]['total'] += 1
                
                # Look backwards for doc comments
                doc_lines = []
                j = i - 1
                while j >= 0 and (lines[j].strip().startswith('///') or 
                                 lines[j].strip().startswith('//!') or
                                 not lines[j].strip()):
                    if lines[j].strip().startswith('///') or lines[j].strip().startswith('//!'):
                        doc_lines.append(lines[j])
                    j -= 1
                
                # Check for examples
                doc_text = '\n'.join(reversed(doc_lines))
                if DOC_WITH_EXAMPLE.search(doc_text):
                    counts[item_type]['with_examples'] += 1
                
                break
        i += 1
    
    return dict(counts)

def main():
    src_dir = Path('crates/pdftract-core/src')
    
    total_counts = defaultdict(lambda: {'total': 0, 'with_examples': 0})
    module_docs = []
    
    for rs_file in src_dir.rglob('*.rs'):
        content = rs_file.read_text()
        counts = count_items_and_examples(content)
        
        for item_type, counts_data in counts.items():
            for key in ['total', 'with_examples']:
                total_counts[item_type][key] += counts_data[key]
        
        # Track modules with doc comments
        if 'pub mod' in content or (rs_file.name == 'mod.rs' or rs_file.name == 'lib.rs'):
            has_module_doc = '//!' in content[:500]  # Check beginning of file
            module_name = rs_file.relative_to(src_dir)
            module_docs.append((str(module_name), has_module_doc))
    
    # Print results
    print("=" * 60)
    print("PDFTRACT-CORE RUSTDOC COVERAGE REPORT")
    print("=" * 60)
    print()
    
    total_items = sum(data['total'] for data in total_counts.values())
    total_with_examples = sum(data['with_examples'] for data in total_counts.values())
    coverage = (total_with_examples / total_items * 100) if total_items > 0 else 0
    
    print(f"Total public items: {total_items}")
    print(f"With examples: {total_with_examples}")
    print(f"Coverage: {coverage:.1f}%")
    print()
    
    print("By item type:")
    for item_type in ['function', 'struct', 'enum', 'trait', 'type', 'module', 'const']:
        if item_type in total_counts:
            data = total_counts[item_type]
            pct = (data['with_examples'] / data['total'] * 100) if data['total'] > 0 else 0
            print(f"  {item_type:10s}: {data['with_examples']:3d}/{data['total']:3d} ({pct:5.1f}%)")
    
    print()
    print("Modules with/without module-level docs (//!):")
    modules_without_doc = [name for name, has_doc in module_docs if not has_doc]
    print(f"  Modules checked: {len(module_docs)}")
    print(f"  Without module docs: {len(modules_without_doc)}")
    
    if modules_without_doc and len(modules_without_doc) <= 20:
        print("  Examples needing module docs:")
        for name in modules_without_doc[:10]:
            print(f"    - {name}")
    
    print()
    print("=" * 60)
    
    # Exit with error if coverage < 80%
    if coverage < 80:
        print(f"ERROR: Coverage {coverage:.1f}% is below 80% threshold")
        exit(1)
    else:
        print(f"SUCCESS: Coverage {coverage:.1f}% meets 80% threshold")
        exit(0)

if __name__ == '__main__':
    main()

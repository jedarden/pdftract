import re
import os
from pathlib import Path

def count_public_items(file_path):
    with open(file_path, 'r') as f:
        lines = f.readlines()
    
    items = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Check for public items
        if re.match(r'^pub (fn|struct|enum|trait|type|const|static)', line):
            item = {'line': i + 1, 'type': line.strip(), 'has_doc': False}
            # Check for doc comments in the 3 lines before
            j = max(0, i - 3)
            while j < i:
                if lines[j].strip().startswith('///'):
                    item['has_doc'] = True
                    break
                j += 1
            items.append(item)
        i += 1
    
    return items

src_dir = Path('crates/pdftract-core/src')
all_items = []
for rs_file in src_dir.rglob('*.rs'):
    items = count_public_items(rs_file)
    all_items.extend(items)

total = len(all_items)
with_docs = sum(1 for item in all_items if item['has_doc'])

print(f"Total public items: {total}")
print(f"Items with docs: {with_docs}")
print(f"Coverage: {with_docs/total*100:.1f}%")

# Show which modules need work
modules = {}
for item in all_items:
    module = item.get('module', 'unknown')
    if module not in modules:
        modules[module] = {'total': 0, 'with_docs': 0}
    modules[module]['total'] += 1
    if item['has_doc']:
        modules[module]['with_docs'] += 1

print("\nModules needing work:")
for mod, counts in sorted(modules.items(), key=lambda x: x[1]['total'] - x[1]['with_docs'], reverse=True):
    if counts['total'] > 0:
        coverage = counts['with_docs']/counts['total']*100
        if coverage < 80:
            print(f"  {mod}: {coverage:.0f}% ({counts['with_docs']}/{counts['total']})")

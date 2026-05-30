import re
from pathlib import Path

def count_items_with_examples(file_path):
    with open(file_path, 'r') as f:
        content = f.read()
        lines = f.readlines()
    
    items = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Check for public items
        if re.match(r'^pub (fn|struct|enum|trait|type|const|static)', line):
            item = {'line': i + 1, 'type': line.strip(), 'has_doc': False, 'has_example': False}
            # Look back up to 10 lines for doc comments
            j = max(0, i - 10)
            doc_lines = []
            while j < i:
                if lines[j].strip().startswith('///'):
                    doc_lines.append(lines[j])
                elif not lines[j].strip().startswith('///') and doc_lines:
                    # Non-doc comment breaks the doc block
                    break
                j += 1
            
            if doc_lines:
                item['has_doc'] = True
                # Check for example in doc (```rust)
                doc_text = '\n'.join(doc_lines)
                if '```rust' in doc_text:
                    item['has_example'] = True
            
            items.append(item)
        i += 1
    
    return items

src_dir = Path('crates/pdftract-core/src')
all_items = []
for rs_file in src_dir.rglob('*.rs'):
    items = count_items_with_examples(rs_file)
    all_items.extend(items)

total = len(all_items)
with_docs = sum(1 for item in all_items if item['has_doc'])
with_examples = sum(1 for item in all_items if item['has_example'])

print(f"Total public items: {total}")
print(f"Items with docs: {with_docs} ({with_docs/total*100:.1f}%)")
print(f"Items with examples: {with_examples} ({with_examples/total*100:.1f}%)")

# Show items missing docs
print("\nItems missing documentation:")
for item in sorted(all_items, key=lambda x: x['line']):
    if not item['has_doc']:
        print(f"  {item['type']}")

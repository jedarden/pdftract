#!/usr/bin/env python3
"""
Measure rustdoc coverage for the actual public API (re-exported items only).

This focuses on items users can access via pdftract_core::, not internal pub items.
"""
import re
import subprocess
from pathlib import Path
from typing import Dict, List, Set

def get_public_api_items() -> Set[str]:
    """
    Get the list of public API items by parsing rustdoc output.
    These are items accessible via pdftract_core:: prefix.
    """
    # Run cargo doc and capture the JSON output
    result = subprocess.run(
        ['cargo', 'doc', '--no-deps', '-p', 'pdftract-core', '--open', '--no-deps'],
        cwd=Path(__file__).parent.parent,
        capture_output=True,
        text=True,
        timeout=300
    )
    # For now, parse lib.rs re-exports
    lib_rs = Path(__file__).parent.parent / 'src' / 'lib.rs'
    content = lib_rs.read_text()

    items = set()

    # Parse pub use statements
    for line in content.split('\n'):
        # Match: pub use module::{item1, item2, ...};
        match = re.search(r'pub\s+use\s+(\w+)\s*::\s*\{([^}]+)\}', line)
        if match:
            module = match.group(1)
            items_list = match.group(2)
            for item in items_list.split(','):
                item = item.strip()
                if item and not item.startswith('_'):
                    items.add(f"{module}::{item}")

        # Match: pub use module::item;
        match = re.search(r'pub\s+use\s+(\w+)::(\w+)', line)
        if match:
            module = match.group(1)
            item = match.group(2)
            if not item.startswith('_'):
                items.add(f"{module}::{item}")

    # Parse module declarations (pub mod foo;)
    for line in content.split('\n'):
        match = re.search(r'pub\s+mod\s+(\w+)', line)
        if match:
            items.add(match.group(1))

    return items

def check_item_has_example(item_path: str, src_dir: Path) -> bool:
    """Check if an item has a worked example in its documentation."""
    # Convert item_path to file path
    # e.g., "extract::extract_pdf" -> "src/extract.rs"
    # or "document::Document" -> "src/document.rs"

    parts = item_path.split('::')
    if len(parts) < 2:
        return False

    module_name = parts[0]
    item_name = parts[-1]

    # Find the module file
    module_file = src_dir / f"{module_name}.rs"
    if not module_file.exists():
        # Check if it's a mod directory
        mod_dir = src_dir / module_name
        if mod_dir.is_dir():
            # Look for mod.rs or lib.rs in the directory
            for potential in [mod_dir / 'mod.rs', mod_dir / 'lib.rs']:
                if potential.exists():
                    module_file = potential
                    break

    if not module_file.exists():
        return False

    content = module_file.read_text()

    # Look for the item and check if it has a doc with example
    # Simple regex search for the item declaration
    pattern = rf'pub\s+(?:fn|struct|enum|trait|type|const)\s+{re.escape(item_name)}\b'

    # Find the position of the item
    match = re.search(pattern, content)
    if not match:
        return False

    # Look backwards from the match for doc comments
    pos = match.start()
    doc_content = content[:pos]

    # Check if there's a doc comment with an example
    return '```rust' in doc_content or '```no_run' in doc_content

def main():
    script_dir = Path(__file__).parent
    src_dir = script_dir.parent / 'src'

    # Get public API items from lib.rs re-exports
    lib_rs = src_dir / 'lib.rs'
    content = lib_rs.read_text()

    public_items = []
    for line in content.split('\n'):
        # Parse pub use statements
        matches = re.finditer(r'pub\s+use\s+([^;]+);', line)
        for match in matches:
            use_stmt = match.group(1)
            # Handle "module::{items}" format
            brace_match = re.search(r'(\w+)::\s*\{([^}]+)\}', use_stmt)
            if brace_match:
                module = brace_match.group(1)
                items = brace_match.group(2)
                for item in items.split(','):
                    item = item.strip()
                    if item and not item.startswith('_') and 'as' not in item:
                        public_items.append((module, item))
            else:
                # Handle "module::item" format
                item_match = re.search(r'(\w+)::(\w+)', use_stmt)
                if item_match:
                    module = item_match.group(1)
                    item = item_match.group(2)
                    if not item.startswith('_'):
                        public_items.append((module, item))

    # Also count pub mod declarations
    for line in content.split('\n'):
        matches = re.finditer(r'pub\s+mod\s+(\w+)', line)
        for match in matches:
            public_items.append((match.group(1), '<module>'))

    print(f"Found {len(public_items)} public API items (re-exports)")

    # Check which ones have examples
    with_examples = 0
    with_docs = 0
    items_without = []

    for module, item in public_items:
        if item == '<module>':
            # Module-level docs
            module_file = src_dir / f"{module}.rs"
            if not module_file.exists():
                mod_dir = src_dir / module
                if mod_dir.is_dir():
                    for potential in [mod_dir / 'mod.rs', mod_dir / 'lib.rs']:
                        if potential.exists():
                            module_file = potential
                            break
            if module_file.exists():
                content = module_file.read_text()
                has_doc = content.lstrip().startswith('//!')
                has_example = '```rust' in content[:500] or '```no_run' in content[:500]
                if has_doc:
                    with_docs += 1
                if has_example:
                    with_examples += 1
                else:
                    items_without.append((module, item, has_doc))
        else:
            # Item-level docs
            has_ex, has_doc = check_item_for_docs(module, item, src_dir)
            if has_doc:
                with_docs += 1
            if has_ex:
                with_examples += 1
            else:
                items_without.append((module, item, has_doc))

    total = len(public_items)
    coverage = (with_examples / total * 100) if total > 0 else 0
    doc_coverage = (with_docs / total * 100) if total > 0 else 0

    print(f"\n{'='*50}")
    print(f"Public API Rustdoc Coverage")
    print(f"{'='*50}")
    print(f"Total public API items: {total}")
    print(f"With documentation: {with_docs} ({doc_coverage:.1f}%)")
    print(f"With worked examples: {with_examples} ({coverage:.1f}%)")
    print(f"\nTarget: 80% example coverage")
    print(f"Status: {'✓ PASS' if coverage >= 80 else '✗ FAIL'}")

    if items_without:
        print(f"\n--- Items lacking examples ({len(items_without)}) ---")
        for module, item, has_doc in items_without[:20]:
            doc_marker = '📄' if has_doc else '❌'
            print(f"  {doc_marker} {module}::{item}")
        if len(items_without) > 20:
            print(f"  ... and {len(items_without) - 20} more")

    return 0 if coverage >= 80 else 1

def check_item_for_docs(module: str, item: str, src_dir: Path) -> tuple:
    """Check if an item has documentation and/or examples."""
    # Find the module file
    module_file = src_dir / f"{module}.rs"
    if not module_file.exists():
        mod_dir = src_dir / module
        if mod_dir.is_dir():
            for potential in [mod_dir / 'mod.rs', mod_dir / 'lib.rs']:
                if potential.exists():
                    module_file = potential
                    break

    if not module_file.exists():
        return False, False

    content = module_file.read_text()

    # Look for the item
    patterns = [
        rf'pub\s+fn\s+{re.escape(item)}\b',
        rf'pub\s+struct\s+{re.escape(item)}\b',
        rf'pub\s+enum\s+{re.escape(item)}\b',
        rf'pub\s+trait\s+{re.escape(item)}\b',
        rf'pub\s+type\s+{re.escape(item)}\b',
        rf'impl\s+(?:<[^>]*>\s+)?{re.escape(item)}\s*\{{[^}}]*\bpub\s+fn\s+(\w+)',
    ]

    for pattern in patterns:
        match = re.search(pattern, content)
        if match:
            pos = match.start()
            doc_content = content[:pos]
            has_doc = '///' in doc_content or '/**' in doc_content
            has_example = '```rust' in doc_content or '```no_run' in doc_content
            return has_example, has_doc

    return False, False

if __name__ == '__main__':
    exit(main())

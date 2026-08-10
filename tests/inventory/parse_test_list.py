#!/usr/bin/env python3
"""Parse cargo test --list output and create a JSON inventory."""
import json
import re
from pathlib import Path
from collections import defaultdict

def parse_test_list(file_path: str) -> dict:
    """Parse the cargo test --list output file."""
    with open(file_path, 'r') as f:
        content = f.read()

    # Parse test lines (format: "module::test_name: test")
    test_pattern = re.compile(r'^([\w:]+)::tests::([\w_]+):\s+test$')

    # Track tests by module
    inventory = {
        "tests": [],
        "by_module": defaultdict(list),
        "summary": {}
    }

    total_tests = 0
    modules = set()

    for line in content.split('\n'):
        line = line.strip()
        if not line or line.startswith(('Derived', 'running', 'Caused')) or 'test' not in line:
            continue

        # Extract test name and module
        match = test_pattern.match(line)
        if match:
            module, test_name = match.groups()
            full_name = f"{module}::tests::{test_name}"

            inventory["tests"].append({
                "name": test_name,
                "module": module,
                "full_name": full_name,
                "type": "unit"
            })

            inventory["by_module"][module].append(test_name)
            modules.add(module)
            total_tests += 1

    # Build summary
    inventory["summary"] = {
        "total_tests": total_tests,
        "total_modules": len(modules),
        "modules": sorted(modules)
    }

    # Convert defaultdict to regular dict for JSON serialization
    inventory["by_module"] = dict(inventory["by_module"])

    return inventory

def main():
    """Main entry point."""
    input_file = Path(__file__).parent / "cargo-test-list.txt"
    output_file = Path(__file__).parent / "cargo-test-inventory.json"

    print(f"Parsing {input_file}...")
    inventory = parse_test_list(input_file)

    print(f"Writing {output_file}...")
    with open(output_file, 'w') as f:
        json.dump(inventory, f, indent=2, sort_keys=True)

    print(f"✓ Total tests: {inventory['summary']['total_tests']}")
    print(f"✓ Total modules: {inventory['summary']['total_modules']}")
    print(f"✓ Inventory written to {output_file}")

if __name__ == "__main__":
    main()
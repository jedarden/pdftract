#!/usr/bin/env python3
"""
Check for false-positive #[test] attributes in Rust code.
A function with #[test] is a false positive if it's called by other functions.
"""

import re
import os
from collections import defaultdict

def extract_test_functions(file_path):
    """Extract all functions with #[test] attribute from a Rust file."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return []

    test_pattern = r'#\[test\]\s*\n?\s*(?:#\[.*?\]\s*\n?\s*)*(?:async\s+)?fn\s+(\w+)\s*\('
    matches = re.finditer(test_pattern, content, re.MULTILINE | re.DOTALL)
    functions = []
    for match in matches:
        func_name = match.group(1)
        line_num = content[:match.start()].count('\n') + 1
        functions.append((func_name, line_num, content))
    
    return functions

def find_function_calls(content, func_name):
    """Check if a function is called elsewhere in the same file."""
    # Look for calls to this function excluding the function definition itself
    call_pattern = rf'{re.escape(func_name)}\s*\('
    matches = list(re.finditer(call_pattern, content))
    
    # Filter out the definition itself
    call_count = 0
    for match in matches:
        # Check if this looks like a function call (not just the definition)
        before = content[max(0, match.start() - 100):match.start()]
        after = content[match.end():match.end() + 50]
        
        # Skip if this is the function definition
        if 'fn ' in before and '#[test]' in before[:max(1, len(before)-50)]:
            continue
        
        # This looks like a function call
        call_count += 1
    
    return call_count > 0

def analyze_project(root_dir):
    """Analyze all Rust files for false-positive test attributes."""
    results = []
    all_tests = []

    for root, dirs, files in os.walk(root_dir):
        if 'target' in root or '.claude' in root:
            continue
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                test_funcs = extract_test_functions(file_path)
                for func_name, line_num, content in test_funcs:
                    all_tests.append((file_path, func_name, line_num))
                    
                    # Check if this test function is called by other functions
                    if find_function_calls(content, func_name):
                        rel_path = os.path.relpath(file_path, root_dir)
                        results.append({
                            'file': rel_path,
                            'function': func_name,
                            'line': line_num
                        })

    return results, all_tests

def main():
    root_dir = '/home/coding/pdftract'

    print("Analyzing Rust files for false-positive #[test] attributes...")
    print("Looking for test functions that are called by other functions...")
    print("=" * 70)

    results, all_tests = analyze_project(root_dir)

    print(f"\nTotal functions with #[test]: {len(all_tests)}")
    print(f"Potential false positives (test functions called by other code): {len(results)}")

    if results:
        print("\n" + "=" * 70)
        print("FALSE-POSITIVE #[test] ATTRIBUTES")
        print("Test functions that appear to be called by other code:")
        print("=" * 70)

        for item in sorted(results, key=lambda x: (x['file'], x['line'])):
            print(f"\nFile: {item['file']}:{item['line']}")
            print(f"  Function: {item['function']}")
    else:
        print("\nNo false-positive test attributes found.")
        print("All #[test] functions appear to be standalone tests.")

    return results

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
Check for false-positive #[test] attributes in Rust code.
Identifies helper functions incorrectly marked with #[test].
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

    # Pattern to match #[test] followed by function definition
    # Matches: #[test] followed by optional whitespace/newlines, then fn <name>(
    test_pattern = r'#\[test\]\s*\n?\s*(?:#\[.*?\]\s*\n?\s*)*(?:async\s+)?fn\s+(\w+)\s*\('

    matches = re.finditer(test_pattern, content, re.MULTILINE | re.DOTALL)
    functions = []
    for match in matches:
        func_name = match.group(1)
        line_num = content[:match.start()].count('\n') + 1
        functions.append((func_name, line_num))

    return functions

def is_likely_helper_function(func_name, file_path):
    """Determine if a function with #[test] is likely a helper function."""
    helper_patterns = [
        r'^helper_',
        r'^test_helper_',
        r'^setup_',
        r'^teardown_',
        r'^init_',
        r'^cleanup_',
        r'^fixture_',
        r'^mock_',
        r'^stub_',
        r'^provide_',
        r'^create_.*_data$',
        r'^get_.*_fixture$',
        r'^make_.*_mock$',
    ]

    for pattern in helper_patterns:
        if re.match(pattern, func_name):
            return True, f"Matches helper pattern: {pattern}"

    file_name = os.path.basename(file_path)
    if file_name.startswith('generate_') or file_name.startswith('gen_'):
        return True, "In generator file"

    return False, None

def check_if_function_is_called(func_name, file_path):
    """Check if a function is called by other functions."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return False

    call_pattern = rf'(?<!fn\s){re.escape(func_name)}\s*\('
    matches = list(re.finditer(call_pattern, content))

    if len(matches) > 1:
        for match in matches:
            before = content[max(0, match.start() - 200):match.start()]
            if 'fn ' not in before or '#[test]' not in before:
                return True

    return False

def analyze_project(root_dir):
    """Analyze all Rust files in the project for false-positive test attributes."""
    results = defaultdict(list)
    all_tests = []

    for root, dirs, files in os.walk(root_dir):
        if 'target' in root:
            continue
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                test_funcs = extract_test_functions(file_path)
                for func_name, line_num in test_funcs:
                    all_tests.append((file_path, func_name, line_num))

    print(f"Found {len(all_tests)} functions with #[test] attribute")

    for file_path, func_name, line_num in all_tests:
        is_helper, reason = is_likely_helper_function(func_name, file_path)

        if not is_helper:
            is_called = check_if_function_is_called(func_name, file_path)
            if is_called:
                is_helper = True
                reason = "Function is called by other functions (likely helper)"

        if is_helper:
            rel_path = os.path.relpath(file_path, root_dir)
            results['helpers'].append({
                'file': rel_path,
                'function': func_name,
                'line': line_num,
                'reason': reason
            })

    return results, all_tests

def main():
    root_dir = '/home/coding/pdftract'

    print("Analyzing Rust files for false-positive #[test] attributes...")
    print("=" * 70)

    results, all_tests = analyze_project(root_dir)

    print(f"\nTotal functions with #[test]: {len(all_tests)}")
    print(f"Potential false positives (helper functions): {len(results['helpers'])}")

    if results['helpers']:
        print("\n" + "=" * 70)
        print("FALSE-POSITIVE #[test] ATTRIBUTES (Helper Functions)")
        print("=" * 70)

        for helper in sorted(results['helpers'], key=lambda x: (x['file'], x['line'])):
            print(f"\nFile: {helper['file']}:{helper['line']}")
            print(f"  Function: {helper['function']}")
            print(f"  Reason: {helper['reason']}")
    else:
        print("\nNo false-positive test attributes found.")

    return results

if __name__ == '__main__':
    main()

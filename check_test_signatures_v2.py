#!/usr/bin/env python3
"""
Verify test function signatures in the pdftract codebase.

Checks:
1. Test functions (fn test_*) have #[test] or similar attribute
2. Helper functions not starting with test_ should NOT be marked with #[test]
3. Test functions have valid signatures (no parameters, or specific patterns)
"""

import os
import re
from pathlib import Path
from typing import List, Tuple, Set

# Test-related attributes
TEST_ATTRS = {
    '#[test]',
    '#[tokio::test]',
    '#[async_test]',
}

# Known helper function patterns that take parameters
KNOWN_HELPER_PATTERNS = [
    'test_fixture',
    'test_encoding_fixture',
    'test_cjk_fixture',
    'test_fixture_pair',
]

# Known non-test function names that might have #[test] for other reasons
BENCHMARK_PATTERNS = [
    'benchmark_',
]

PROPTEST_PATTERNS = [
    'prop_',
    'proptest_',
    'fuzz_',
]

def extract_tests_and_attrs(content: str, filepath: str) -> List[Tuple[str, str, int, str]]:
    """
    Extract test functions and their attributes.
    Returns list of (function_name, attributes, line_number, signature)
    """
    results = []
    lines = content.split('\n')

    for i, line in enumerate(lines, 1):
        # Look for function definitions starting with test_
        match = re.search(r'fn\s+(test_[a-zA-Z_][a-zA-Z0-9_]*)\s*\(', line)
        if match:
            func_name = match.group(1)
            signature = line.strip()

            # Look backwards for attributes (up to 10 lines)
            attrs = []
            for j in range(max(0, i-11), i):
                check_line = lines[j-1].strip()
                if check_line.startswith('#['):
                    attrs.append(check_line)
                elif check_line and not check_line.startswith('#') and check_line != '' and check_line != '}' and check_line != '{':
                    # Non-attribute, non-empty line - stop looking (but allow braces)
                    break

            results.append((func_name, '\n'.join(attrs), i, signature))

    return results


def extract_false_positives(content: str, filepath: str) -> List[Tuple[str, str, int, str]]:
    """
    Extract non-test functions marked with #[test] attribute.
    Returns list of (function_name, attributes, line_number, signature)
    """
    results = []
    lines = content.split('\n')

    i = 0
    while i < len(lines):
        line = lines[i]

        # Found #[test] or similar attribute
        if any(attr in line for attr in TEST_ATTRS):
            # Look ahead for the function definition
            func_line = None
            func_attrs = [line.strip()]
            func_line_num = i + 1

            for j in range(i+1, min(i+6, len(lines))):
                next_line = lines[j].strip()
                if next_line.startswith('#['):
                    func_attrs.append(next_line)
                elif next_line.startswith('fn '):
                    func_line = next_line
                    func_line_num = j + 1
                    break
                elif next_line and not next_line.startswith('#'):
                    # Non-attribute, non-empty line - stop
                    break

            if func_line:
                match = re.search(r'fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(', func_line)
                if match:
                    func_name = match.group(1)
                    # Check if it's NOT a test function
                    if not func_name.startswith('test_'):
                        results.append((func_name, '\n'.join(func_attrs), func_line_num, func_line))
                        i = j  # Skip to function definition
                        continue

        i += 1

    return results


def is_known_test_helper(func_name: str) -> bool:
    """Check if this is a known test helper function."""
    return any(pattern in func_name for pattern in KNOWN_HELPER_PATTERNS)


def is_benchmark_or_proptest(func_name: str) -> bool:
    """Check if this is a benchmark or property test."""
    return (any(pattern in func_name for pattern in BENCHMARK_PATTERNS) or
            any(pattern in func_name for pattern in PROPTEST_PATTERNS))


def check_test_signature(func_name: str, signature: str) -> Tuple[bool, str]:
    """
    Check if a test function has a valid signature.
    """
    # Known test helpers with parameters are OK
    if is_known_test_helper(func_name):
        return True, "Known test helper function with parameters"

    # Extract parameters from signature
    match = re.search(r'fn\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\(([^)]*)\)', signature)
    if not match:
        return False, "Cannot parse function signature"

    params = match.group(1).strip()

    # Test functions should generally have no parameters
    # But we allow some flexibility for property tests, etc.
    if params and params != '':
        # Check if it's a property test (has `in` keyword or special proptest syntax)
        if 'in ' in params or 'data in' in params:
            return True, "Property test with parameters"
        
        # Allow simple parameter patterns
        if any(pattern in signature for pattern in ['prop_', 'proptest_', 'fuzz_']):
            return True, "Property/fuzz test with parameters"
            
        return False, f"Test function has parameters: {params}"

    return True, "Valid test signature"


def main():
    """Main verification function."""
    issues = []
    stats = {
        'total_test_functions': 0,
        'missing_test_attr': 0,
        'invalid_signature': 0,
        'false_positive_test_attr': 0,
    }

    # Find all Rust files
    rust_files = []
    for root, dirs, files in os.walk('/home/coding/pdftract'):
        # Skip target and build directories
        dirs[:] = [d for d in dirs if d not in ['target', '.git', '.beads', 'node_modules', '.claude']]

        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))

    print(f"Scanning {len(rust_files)} Rust files...\n")

    # Check each file
    for filepath in rust_files:
        rel_path = os.path.relpath(filepath, '/home/coding/pdftract')

        try:
            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
        except Exception as e:
            continue

        # Check test functions for missing #[test] attribute
        tests = extract_tests_and_attrs(content, filepath)
        for func_name, attrs, line_num, signature in tests:
            stats['total_test_functions'] += 1
            
            has_test_attr = any(attr in attrs for attr in TEST_ATTRS)
            
            if not has_test_attr:
                stats['missing_test_attr'] += 1
                issues.append({
                    'file': rel_path,
                    'line': line_num,
                    'function': func_name,
                    'issue': 'Missing #[test] attribute',
                    'signature': signature,
                    'attrs': attrs
                })

            # Check signature validity (only for functions that look like actual tests)
            valid, msg = check_test_signature(func_name, signature)
            if not valid:
                stats['invalid_signature'] += 1
                issues.append({
                    'file': rel_path,
                    'line': line_num,
                    'function': func_name,
                    'issue': f'Invalid signature: {msg}',
                    'signature': signature,
                    'attrs': attrs
                })

        # Check for false positives (non-test functions with #[test])
        false_positives = extract_false_positives(content, filepath)
        for func_name, attrs, line_num, signature in false_positives:
            # Skip if it's a benchmark or proptest - those might legitimately use #[test]
            if is_benchmark_or_proptest(func_name):
                continue
                
            stats['false_positive_test_attr'] += 1
            issues.append({
                'file': rel_path,
                'line': line_num,
                'function': func_name,
                'issue': 'Helper function marked with #[test]',
                'signature': signature,
                'attrs': attrs
            })

    # Report results
    print(f"Statistics:")
    print(f"  Total test functions found: {stats['total_test_functions']}")
    print(f"  Missing #[test] attribute: {stats['missing_test_attr']}")
    print(f"  Invalid signatures: {stats['invalid_signature']}")
    print(f"  False positive #[test] on helpers: {stats['false_positive_test_attr']}")
    print()

    if issues:
        print(f"❌ Found {len(issues)} issues:\n")

        # Group by issue type
        missing_test = [i for i in issues if 'Missing #[test]' in i['issue']]
        invalid_sig = [i for i in issues if 'Invalid signature' in i['issue']]
        false_pos = [i for i in issues if 'Helper function' in i['issue']]

        if missing_test:
            print(f"## Missing #[test] attribute ({len(missing_test)} cases)")
            print("Showing first 10:")
            for issue in missing_test[:10]:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                print(f"    Attributes: {issue['attrs'] or 'none'}")
            if len(missing_test) > 10:
                print(f"  ... and {len(missing_test) - 10} more")
            print()

        if invalid_sig:
            print(f"## Invalid signatures ({len(invalid_sig)} cases)")
            print("Showing first 10:")
            for issue in invalid_sig[:10]:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                print(f"    {issue['issue']}")
            if len(invalid_sig) > 10:
                print(f"  ... and {len(invalid_sig) - 10} more")
            print()

        if false_pos:
            print(f"## Helper functions marked with #[test] ({len(false_pos)} cases)")
            print("Showing first 10:")
            for issue in false_pos[:10]:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                print(f"    Signature: {issue['signature']}")
            if len(false_pos) > 10:
                print(f"  ... and {len(false_pos) - 10} more")
            print()

        return 1
    else:
        print("✅ All test signatures are valid!")
        print("  - All test functions (fn test_*) have #[test] attribute")
        print("  - No invalid test signatures found")
        print("  - No helper functions incorrectly marked with #[test]")
        return 0


if __name__ == '__main__':
    exit(main())

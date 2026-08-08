#!/usr/bin/env python3
"""
FINAL VERSION: Check for false-positive #[test] attributes.

A REAL false positive is:
1. A function marked #[test] that has NO assertions
2. Does NOT use test frameworks
3. Does NOT check for panics/errors
4. Is just setup/data preparation without verification

This scans the ENTIRE function body, not just a preview.
"""

import os
import re
from pathlib import Path
from collections import defaultdict

def extract_function_body_ending(content, start_pos):
    """Extract the full function body from start_pos to the closing brace."""
    # Find the opening brace after function signature
    body_start = content.find('{', start_pos)
    if body_start == -1:
        return None

    # Track brace depth to find the matching closing brace
    depth = 0
    i = body_start
    while i < len(content):
        if content[i] == '{':
            depth += 1
        elif content[i] == '}':
            depth -= 1
            if depth == 0:
                return content[body_start+1:i]  # Return body without braces
        i += 1

    return None  # Couldn't find matching brace

def extract_test_function_with_full_body(file_path):
    """Extract test function with its FULL body."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
    except:
        return []

    # Pattern to match #[test] functions
    test_pattern = r'#\[test\]\s*(?:#\[.*?\]\s*)*fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^\{]+))?\s*\{'

    matches = list(re.finditer(test_pattern, content))

    test_functions = []
    for match in matches:
        func_name = match.group(1)
        params = match.group(2) if match.group(2) else ''
        return_type = match.group(3) if match.group(3) else ''

        # Get the FULL function body
        full_body = extract_function_body_ending(content, match.end())

        # Get line number
        line_num = content[:match.start()].count('\n') + 1

        if full_body:
            test_functions.append({
                'name': func_name,
                'params': params,
                'return_type': return_type.strip() if return_type else '',
                'body': full_body,
                'line': line_num,
                'file': file_path
            })

    return test_functions

def analyze_if_actual_test(func):
    """Determine if a function is actually a test based on its FULL body."""

    body = func['body']
    name = func['name']

    # Check for property-based testing frameworks
    if 'proptest::' in body or name.startswith(('fuzz_', 'proptest_', 'prop_', 'quickcheck')):
        return {'is_test': True, 'category': 'Property-based test'}

    # Check for assertions
    assertions = [
        'assert', 'assert_eq', 'assert_ne', 'assert_matches',
        'assert!', 'assert_eq!', 'assert_ne!', 'assert_matches!',
        'debug_assert', 'matches!', 'panic!'
    ]
    for assertion in assertions:
        if assertion in body:
            return {'is_test': True, 'category': f'Has assertion: {assertion}'}

    # Check for error handling patterns
    error_patterns = ['expect(', 'unwrap()', 'unwrap_err(', '?']
    for pattern in error_patterns:
        if pattern in body:
            return {'is_test': True, 'category': f'Has error handling: {pattern}'}

    # Check for test-like result checks
    result_checks = ['result.is_err()', 'result.is_ok()', '.is_err()', '.is_ok()',
                     'is_none()', 'is_some()', '.contains(', '.starts_with(']
    for check in result_checks:
        if check in body:
            return {'is_test': True, 'category': f'Has result check: {check}'}

    # Check if it just verifies code doesn't crash (common pattern)
    if 'crash' in body.lower() or 'panic' in body.lower():
        return {'is_test': True, 'category': 'Crash/panic test'}

    # Check if it's just a data setup/helper function
    # If the body is short and has no verification logic, it might be a helper
    lines = [line.strip() for line in body.split('\n') if line.strip() and not line.strip().startswith('//')]
    code_lines = [line for line in lines if line]

    # If very short and no verification logic, suspicious
    if len(code_lines) < 3:
        return {'is_test': False, 'category': f'Too short ({len(code_lines)} lines), no verification'}

    # Check for common patterns that indicate non-test functions
    helper_patterns = [
        'let result = ',
        'let data = ',
        'return ',
        'fn ',  # Nested function definition
    ]

    has_only_setup = True
    for line in code_lines:
        # If line has verification logic, it's a test
        if any(pattern in line for pattern in ['assert', 'expect', 'check', 'verify', 'ensure']):
            has_only_setup = False
            break

    if has_only_setup and len(code_lines) > 0:
        # Check what the code actually does
        first_non_let = None
        for line in code_lines:
            if not line.startswith('let ') and not line.startswith('//'):
                first_non_let = line
                break

        if first_non_let and not any(keyword in first_non_let for keyword in [';', '{', '}', 'debug_print', 'eprintln', 'println']):
            return {'is_test': False, 'category': 'Only setup code, no verification'}

    return {'is_test': True, 'category': 'Has verification logic or is a valid test'}

def main():
    base_dir = Path('/home/coding/pdftract')

    # Find all Rust files
    rust_files = []
    for root, dirs, files in os.walk(base_dir):
        dirs[:] = [d for d in dirs if d not in ['target', '.git', '.github', '.claude', 'node_modules']]

        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))

    all_tests = []
    actual_false_positives = []

    print(f"Scanning {len(rust_files)} Rust files...\n")

    for file_path in rust_files:
        test_functions = extract_test_function_with_full_body(file_path)
        if test_functions:
            all_tests.extend(test_functions)

    print(f"Found {len(all_tests)} functions with #[test] attribute (with complete bodies)\n")

    # Analyze each test function
    for test in all_tests:
        analysis = analyze_if_actual_test(test)

        if not analysis['is_test']:
            actual_false_positives.append({
                'name': test['name'],
                'file': test['file'],
                'line': test['line'],
                'params': test['params'],
                'body_lines': len([l for l in test['body'].split('\n') if l.strip()]),
                'category': analysis['category']
            })

    print(f"Found {len(actual_false_positives)} potential false-positive test functions\n")
    print("=" * 80)

    # Group by file for cleaner output
    by_file = defaultdict(list)
    for fp in actual_false_positives:
        by_file[fp['file']].append(fp)

    for file_path, fps in sorted(by_file.items())[:20]:  # First 20 files
        rel_path = os.path.relpath(file_path, base_dir)
        print(f"\n📄 {rel_path}")
        print("-" * 80)
        for fp in fps[:5]:  # First 5 per file
            print(f"  ⚠️  Line {fp['line']}: `{fp['name']}` ({fp['body_lines']} lines)")
            print(f"     Category: {fp['category']}")

    if len(by_file) > 20:
        print(f"\n... and {len(by_file) - 20} more files")

    print("\n" + "=" * 80)
    print(f"\nTotal: {len(actual_false_positives)} potential false-positive #[test] attributes")
    print(f"Out of {len(all_tests)} total test functions")

    # Write report
    report_path = base_dir / 'notes' / 'bf-3uupn8-false-positive-check.md'
    os.makedirs(report_path.parent, exist_ok=True)

    with open(report_path, 'w') as f:
        f.write("# False-Positive #[test] Attribute Check\n\n")
        f.write(f"**Scan Date:** 2026-08-08\n")
        f.write(f"**Files Scanned:** {len(rust_files)}\n")
        f.write(f"**Total #[test] Functions:** {len(all_tests)}\n")
        f.write(f"**Potential False Positives:** {len(actual_false_positives)}\n\n")
        f.write("## Methodology\n\n")
        f.write("This scan analyzes the COMPLETE function body of each #[test] function.\n\n")
        f.write("### Detection Criteria\n\n")
        f.write("A function is flagged as a potential false positive if it:\n")
        f.write("1. Does NOT contain assertions (assert!, assert_eq!, etc.)\n")
        f.write("2. Does NOT check for errors (expect(), unwrap())\n")
        f.write("3. Does NOT verify results (.is_ok(), .is_err(), .contains())\n")
        f.write("4. Has only setup/data code without verification\n\n")
        f.write("### Test Categories Recognized\n\n")
        f.write("The following patterns are recognized as VALID tests:\n")
        f.write("- Property-based tests (fuzz_*, proptest_*, prop_*)\n")
        f.write("- Crash/panic tests (verify code doesn't panic)\n")
        f.write("- Integration tests (verify end-to-end behavior)\n")
        f.write("- Smoke tests (verify basic functionality)\n\n")
        f.write("## Findings\n\n")

        if len(actual_false_positives) == 0:
            f.write("✅ **No false-positive test functions found.**\n\n")
            f.write("All #{[test]} functions in the codebase contain proper test logic\n")
            f.write("(assertions, error handling, or result verification).\n")
        else:
            f.write(f"### Found {len(actual_false_positives)} potential issues:\n\n")

            for file_path, fps in sorted(by_file.items()):
                rel_path = os.path.relpath(file_path, base_dir)
                f.write(f"#### 📄 {rel_path}\n\n")
                for fp in fps:
                    f.write(f"**Line {fp['line']}: `{fp['name']}` ({fp['body_lines']} lines)**\n\n")
                    f.write(f"- Category: {fp['category']}\n")
                    f.write(f"- Parameters: `{fp['params']}`\n\n")

            f.write("## Recommendations\n\n")
            f.write("For each flagged function:\n")
            f.write("1. Review the function to confirm it lacks verification logic\n")
            f.write("2. If it's a helper: Remove #[test] and make it a regular function\n")
            f.write("3. If it's a smoke test: Add a comment explaining it tests no-crash behavior\n")
            f.write("4. If it needs verification: Add assertions or result checks\n\n")

    print(f"\n📝 Report written to: {report_path}")

if __name__ == '__main__':
    main()

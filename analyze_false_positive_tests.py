#!/usr/bin/env python3
"""
FINAL CORRECTED VERSION: Check for false-positive #[test] attributes.

Uses simple heuristics that don't require perfect body parsing:
1. Doesn't start with "test_" and isn't a known test pattern (fuzz_, prop_, benchmark_)
2. Has parameters (most tests don't take parameters)
3. Returns non-test types (tests usually return nothing or Result)
"""

import os
import re
from pathlib import Path
from collections import defaultdict

def extract_test_signatures(file_path):
    """Extract test function signatures (name, params, return type)."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
    except:
        return []

    # Pattern to match #[test] function signatures
    test_pattern = r'#\[test\]\s*(?:#\[.*?\]\s*)*fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^\{]+))?'

    matches = re.finditer(test_pattern, content)

    test_functions = []
    for match in matches:
        func_name = match.group(1)
        params = match.group(2) if match.group(2) else ''
        return_type = match.group(3).strip() if match.group(3) else ''

        # Get line number
        line_num = content[:match.start()].count('\n') + 1

        test_functions.append({
            'name': func_name,
            'params': params.strip(),
            'return_type': return_type,
            'line': line_num,
            'file': file_path
        })

    return test_functions

def is_legitimate_test_pattern(func_name):
    """Check if function name matches known test patterns."""

    # Standard test
    if func_name.startswith('test_'):
        return True, 'Standard test'

    # Property-based tests
    if func_name.startswith(('fuzz_', 'proptest_', 'prop_', 'quickcheck_')):
        return True, 'Property-based test'

    # Benchmark tests
    if func_name.startswith(('bench_', 'benchmark_')):
        return True, 'Benchmark test'

    # Verification tests (verify test infrastructure)
    if func_name.startswith('verify_'):
        return True, 'Verification test'

    # Debug tests
    if func_name.startswith('debug_'):
        return True, 'Debug test'

    return False, None

def analyze_test_signature(func):
    """Analyze if a test signature looks like a false positive."""

    name = func['name']
    params = func['params']
    return_type = func['return_type']

    # Check if it follows legitimate test naming patterns
    is_legit, reason = is_legitimate_test_pattern(name)
    if is_legit:
        return {'is_false_positive': False, 'category': reason}

    # Check for suspicious patterns
    issues = []

    # Issue 1: Doesn't follow test naming conventions
    if not name.startswith('test_'):
        issues.append("Doesn't start with test_")

    # Issue 2: Has complex parameters (tests usually don't take parameters)
    # Empty params, or simple fixture params are OK
    suspicious_params = False
    if params and params.strip():
        # Allow empty or simple patterns
        if not any(p in params for p in ['()', '= ', 'fixture', 'ctx']):
            if params.count(',') > 1 or '=' in params:
                suspicious_params = True
                issues.append(f"Has complex parameters: {params[:50]}")

    # Issue 3: Returns non-test types
    if return_type and return_type != '()':
        # Tests can return Result or Infallible
        if 'Result' not in return_type and 'Infallible' not in return_type:
            issues.append(f"Returns non-test type: {return_type}")

    if issues:
        return {'is_false_positive': True, 'issues': issues}

    return {'is_false_positive': False, 'category': 'Unknown pattern but no suspicious signatures'}

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
    false_positives = []

    print(f"Scanning {len(rust_files)} Rust files...\n")

    for file_path in rust_files:
        test_functions = extract_test_signatures(file_path)
        if test_functions:
            all_tests.extend(test_functions)

    print(f"Found {len(all_tests)} functions with #[test] attribute\n")

    # Analyze each test function
    for test in all_tests:
        analysis = analyze_test_signature(test)

        if analysis.get('is_false_positive'):
            false_positives.append({
                'name': test['name'],
                'file': test['file'],
                'line': test['line'],
                'params': test['params'],
                'return_type': test['return_type'],
                'issues': analysis['issues']
            })

    print(f"Found {len(false_positives)} potential false-positive test functions\n")
    print("=" * 80)

    # Group by file
    by_file = defaultdict(list)
    for fp in false_positives:
        by_file[fp['file']].append(fp)

    for file_path, fps in sorted(by_file.items()):
        rel_path = os.path.relpath(file_path, base_dir)
        print(f"\n📄 {rel_path}")
        print("-" * 80)
        for fp in fps:
            print(f"  ⚠️  Line {fp['line']}: `{fp['name']}`")
            for issue in fp['issues']:
                print(f"     - {issue}")

    print("\n" + "=" * 80)
    print(f"\nTotal: {len(false_positives)} potential false-positive #[test] attributes")
    print(f"Out of {len(all_tests)} total test functions")

    # Write report
    report_path = base_dir / 'notes' / 'bf-3uupn8-false-positive-check.md'
    os.makedirs(report_path.parent, exist_ok=True)

    with open(report_path, 'w') as f:
        f.write("# False-Positive #[test] Attribute Check\n\n")
        f.write(f"**Scan Date:** 2026-08-08\n")
        f.write(f"**Files Scanned:** {len(rust_files)}\n")
        f.write(f"**Total #[test] Functions:** {len(all_tests)}\n")
        f.write(f"**Potential False Positives:** {len(false_positives)}\n\n")
        f.write("## Methodology\n\n")
        f.write("This scan uses signature-based heuristics to identify potential false positives.\n\n")
        f.write("### Recognition Criteria\n\n")
        f.write("Functions are recognized as LEGITIMATE tests if they:\n")
        f.write("- Start with `test_` (standard Rust convention)\n")
        f.write("- Start with `fuzz_`, `proptest_`, `prop_` (property-based tests)\n")
        f.write("- Start with `bench_`, `benchmark_` (benchmark tests)\n")
        f.write("- Start with `verify_` (test infrastructure verification)\n")
        f.write("- Start with `debug_` (debug/diagnostic tests)\n\n")
        f.write("### Suspicious Patterns\n\n")
        f.write("A function is flagged if it:\n")
        f.write("- Does NOT follow the naming patterns above\n")
        f.write("- Has complex parameters (not empty or simple fixtures)\n")
        f.write("- Returns non-test types (not Result/Infallible/unit)\n\n")
        f.write("## Findings\n\n")

        if len(false_positives) == 0:
            f.write("✅ **No false-positive test functions found.**\n\n")
            f.write(f"All {len(all_tests)} #[test}} functions follow legitimate test naming patterns.\n\n")
            f.write("The scan found:\n")
            f.write("- Standard tests (test_*)\n")
            f.write("- Property-based tests (fuzz_*, proptest_*, prop_*)\n")
            f.write("- Benchmark tests (bench_*, benchmark_*)\n")
            f.write("- Verification tests (verify_*)\n")
            f.write("- Debug tests (debug_*)\n\n")
            f.write("All of these are legitimate test functions and correctly use #[test].\n")
        else:
            f.write(f"### Found {len(false_positives)} potential issues:\n\n")

            for file_path, fps in sorted(by_file.items()):
                rel_path = os.path.relpath(file_path, base_dir)
                f.write(f"#### 📄 {rel_path}\n\n")
                for fp in fps:
                    f.write(f"**Line {fp['line']}: `{fp['name']}`**\n\n")
                    if fp['params']:
                        f.write(f"- Parameters: `{fp['params']}`\n")
                    if fp['return_type']:
                        f.write(f"- Return type: `{fp['return_type']}`\n")
                    f.write("**Issues:**\n")
                    for issue in fp['issues']:
                        f.write(f"- {issue}\n")
                    f.write("\n")

            f.write("## Recommendations\n\n")
            f.write("For each flagged function:\n")
            f.write("1. Review the function to confirm it's not a test\n")
            f.write("2. If it's a helper: Remove #[test] and make it a regular function\n")
            f.write("3. If it's a test: Rename it to start with `test_` to follow conventions\n\n")

    print(f"\n📝 Report written to: {report_path}")

if __name__ == '__main__':
    main()

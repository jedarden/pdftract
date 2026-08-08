#!/usr/bin/env python3
"""
Verify test function signatures in the pdftract codebase - FINAL VERSION.

Checks:
1. Functions marked with #[test] have valid test signatures
2. Functions starting with test_ that are NOT marked with #[test]
3. Helper functions with parameters marked as tests (likely false positives)
"""

import os
import re
from typing import List, Tuple, Dict

# Test-related attributes
TEST_ATTRS = {
    '#[test]',
    '#[tokio::test]',
    '#[async_test]',
    '#[actix_web::test]',
}

# Known patterns for property tests and benchmarks
PROPTEST_PATTERNS = [
    'prop_',
    'proptest_',
    'fuzz_',
    'benchmark_',
]

def find_function_with_attrs(content: str, line_num: int) -> Dict:
    """
    Find a function and its attributes by line number.
    Returns dict with function info and attributes.
    """
    lines = content.split('\n')
    
    # The line is 1-indexed
    idx = line_num - 1
    if idx >= len(lines):
        return None
    
    # Get the function line
    func_line = lines[idx].strip()
    
    # Look backwards for attributes
    attrs = []
    for i in range(idx - 1, max(-1, idx - 15), -1):
        if i < 0:
            break
        line = lines[i].strip()
        if line.startswith('#['):
            attrs.insert(0, line)  # Keep in order
        elif line.startswith('//') or line == '':
            # Skip comments and blank lines
            continue
        else:
            # Stop at first non-attribute, non-comment, non-blank line
            break
    
    return {
        'line_num': line_num,
        'function_line': func_line,
        'attributes': attrs,
    }

def check_test_functions(filepath: str) -> List[Dict]:
    """
    Check all test functions in a file.
    Returns list of issues found.
    """
    issues = []
    
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
    except:
        return issues
    
    lines = content.split('\n')
    
    # First pass: find all functions and their attributes
    functions = []
    for i, line in enumerate(lines, 1):
        # Match function definitions
        match = re.search(r'fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(', line)
        if match:
            func_name = match.group(1)
            info = find_function_with_attrs(content, i)
            if info:
                info['function_name'] = func_name
                functions.append(info)
    
    # Second pass: check for issues
    for func in functions:
        func_name = func['function_name']
        attrs = func['attributes']
        line_num = func['line_num']
        
        # Check if this function has a test attribute
        has_test_attr = any(any(test_attr in attr for test_attr in TEST_ATTRS) for attr in attrs)
        
        # Issue 1: Function starts with test_ but has no #[test] attribute
        if func_name.startswith('test_') and not has_test_attr:
            issues.append({
                'file': filepath,
                'line': line_num,
                'function': func_name,
                'issue_type': 'missing_test_attr',
                'message': f"Function '{func_name}' starts with 'test_' but has no #[test] attribute",
                'attributes': attrs,
            })
        
        # Issue 2: Function has #[test] but might not be a test (doesn't start with test_ and isn't a known pattern)
        # Skip this check - function names don't need to start with test_
        
        # Issue 3: Check if function with #[test] has parameters (property tests are OK)
        if has_test_attr:
            # Extract parameters
            match = re.search(r'fn\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\(([^)]*)\)', func['function_line'])
            if match:
                params = match.group(1).strip()
                
                # Check if it's a property test or benchmark (allowed to have params)
                is_prop_test = any(pattern in func_name for pattern in PROPTEST_PATTERNS)
                has_special_syntax = ' in ' in params or 'data in' in params
                
                if params and not is_prop_test and not has_special_syntax:
                    # This might be a helper function incorrectly marked as a test
                    issues.append({
                        'file': filepath,
                        'line': line_num,
                        'function': func_name,
                        'issue_type': 'possible_helper',
                        'message': f"Function '{func_name}' has #[test] but takes parameters: {params[:50]}",
                        'attributes': attrs,
                        'params': params,
                    })
    
    return issues

def main():
    """Main verification function."""
    all_issues = []
    rust_files = []
    
    # Find all Rust files
    for root, dirs, files in os.walk('/home/coding/pdftract'):
        dirs[:] = [d for d in dirs if d not in ['target', '.git', '.beads', 'node_modules', '.claude', 'worktrees']]
        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))
    
    print(f"Scanning {len(rust_files)} Rust files...\n")
    
    # Check each file
    for filepath in rust_files:
        rel_path = os.path.relpath(filepath, '/home/coding/pdftract')
        issues = check_test_functions(filepath)
        for issue in issues:
            issue['file'] = rel_path
            all_issues.append(issue)
    
    # Group by issue type
    missing_test = [i for i in all_issues if i['issue_type'] == 'missing_test_attr']
    possible_helpers = [i for i in all_issues if i['issue_type'] == 'possible_helper']
    
    print(f"Results:")
    print(f"  Functions starting with 'test_' but missing #[test]: {len(missing_test)}")
    print(f"  Functions with #[test] that might be helpers: {len(possible_helpers)}")
    print()
    
    if all_issues:
        print("❌ Issues found:\n")
        
        if missing_test:
            print(f"## Functions missing #[test] attribute ({len(missing_test)} cases)")
            print("Showing first 15:")
            for issue in missing_test[:15]:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                last_attr = issue['attributes'][-1] if issue['attributes'] else 'none'
                print(f"    Last attribute: {last_attr}")
            if len(missing_test) > 15:
                print(f"  ... and {len(missing_test) - 15} more")
            print()
        
        if possible_helpers:
            print(f"## Possible helper functions with #[test] ({len(possible_helpers)} cases)")
            print("Showing first 15:")
            for issue in possible_helpers[:15]:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                print(f"    Parameters: {issue['params'][:60]}")
            if len(possible_helpers) > 15:
                print(f"  ... and {len(possible_helpers) - 15} more")
            print()
        
        return 1
    else:
        print("✅ All test signatures are valid!")
        print("  - Functions with #[test] have correct signatures")
        print("  - Functions starting with 'test_' have #[test] attribute")
        print("  - No helper functions incorrectly marked as tests")
        return 0

if __name__ == '__main__':
    exit(main())

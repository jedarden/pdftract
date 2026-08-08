#!/usr/bin/env python3
"""
Verify test function signatures in the pdftract codebase - FINAL CORRECTED VERSION.

Checks:
1. Functions marked with #[test] or variants have valid test signatures
2. Functions starting with test_ that appear to BE tests but are missing #[test]
3. Helper functions (that return values) should NOT have #[test]
"""

import os
import re
from typing import List, Dict

# Test-related attributes (including conditional and modifier attributes)
TEST_ATTRS = {
    '#[test]',
    '#[tokio::test]',
    '#[async_test]',
    '#[actix_web::test]',
    '#[cfg_attr',
    '#[ignore',
    '#[should_panic',
}

def is_return_statement(line: str) -> bool:
    """Check if a line contains a return statement."""
    return 'return ' in line or line.strip().startswith('return ')

def analyze_function_body(content: str, start_line: int) -> Dict:
    """Analyze a function body to determine if it's a test or helper."""
    lines = content.split('\n')
    body = []
    brace_count = 0
    found_open_brace = False
    
    # Start from the function definition line
    for i in range(start_line - 1, len(lines)):
        line = lines[i]
        
        if '{' in line:
            brace_count += line.count('{')
            found_open_brace = True
        
        if found_open_brace:
            body.append(line)
        
        if '}' in line:
            brace_count -= line.count('}')
            if brace_count == 0 and found_open_brace:
                break
    
    body_text = '\n'.join(body)
    
    # Check for patterns indicating helper vs test
    has_return_value = 'return ' in body_text and 'return;' not in body_text
    has_assertions = 'assert_' in body_text
    has_expected_panics = '#[should_panic]' in body_text
    
    return {
        'has_return_value': has_return_value,
        'has_assertions': has_assertions,
        'has_expected_panics': has_expected_panics,
        'is_helper': has_return_value and not has_assertions,
    }

def check_test_functions(filepath: str) -> List[Dict]:
    """Check all functions in a file for test signature issues."""
    issues = []
    
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
    except:
        return issues
    
    lines = content.split('\n')
    
    # Find all function definitions
    for i, line in enumerate(lines, 1):
        match = re.search(r'fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(', line)
        if not match:
            continue
        
        func_name = match.group(1)
        
        # Look backwards for attributes
        attrs = []
        for j in range(i - 2, max(-1, i - 15), -1):
            if j < 0:
                break
            check_line = lines[j].strip()
            if check_line.startswith('#['):
                attrs.insert(0, check_line)
            elif check_line and not check_line.startswith('//') and check_line != '':
                break
        
        # Check if has test attribute
        has_test_attr = any(any(test_attr in attr for test_attr in TEST_ATTRS) for attr in attrs)
        
        # Analyze function body
        func_analysis = analyze_function_body(content, i)
        
        # Issue 1: Function looks like a test (starts with test_, has assertions) but no #[test]
        if func_name.startswith('test_') and not has_test_attr:
            if func_analysis['has_assertions'] or func_analysis['has_expected_panics']:
                issues.append({
                    'file': filepath,
                    'line': i,
                    'function': func_name,
                    'issue_type': 'missing_test_attr',
                    'message': f"Function '{func_name}' appears to be a test but has no #[test] attribute",
                    'attributes': attrs,
                    'has_assertions': func_analysis['has_assertions'],
                })
        
        # Issue 2: Function has #[test] but is actually a helper (returns value, no assertions)
        if has_test_attr and func_analysis['has_return_value'] and not func_analysis['has_assertions']:
            # This might be OK - could be a parametric test or setup function
            # Only flag if it explicitly returns a non-() value
            func_sig = line.strip()
            if re.search(r'->\s*(!|[A-Z]\w+|&\w+|\w+<[^>]+>)', func_sig):
                issues.append({
                    'file': filepath,
                    'line': i,
                    'function': func_name,
                    'issue_type': 'possible_helper_with_test',
                    'message': f"Function '{func_name}' has #[test] but returns a value (helper function?)",
                    'attributes': attrs,
                    'signature': func_sig,
                })
    
    return issues

def main():
    """Main verification function."""
    all_issues = []
    rust_files = []
    
    # Find all Rust files
    for root, dirs, files in os.walk('/home/coding/pdftract'):
        dirs[:] = [d for d in dirs if d not in ['target', '.git', '.beads', 'node_modules', '.claude']]
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
    helpers_with_test = [i for i in all_issues if i['issue_type'] == 'possible_helper_with_test']
    
    print(f"Results:")
    print(f"  Functions that appear to be tests but missing #[test]: {len(missing_test)}")
    print(f"  Possible helper functions with #[test]: {len(helpers_with_test)}")
    print()
    
    if all_issues:
        print("❌ Issues found:\n")
        
        if missing_test:
            print(f"## Functions missing #[test] attribute ({len(missing_test)} cases)")
            for issue in missing_test:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                last_attr = issue['attributes'][-1] if issue['attributes'] else 'none'
                print(f"    Last attribute: {last_attr}")
            print()
        
        if helpers_with_test:
            print(f"## Possible helper functions with #[test] ({len(helpers_with_test)} cases)")
            for issue in helpers_with_test:
                print(f"  {issue['file']}:{issue['line']} - {issue['function']}")
                print(f"    Signature: {issue['signature'][:80]}")
            print()
        
        return 1
    else:
        print("✅ All test signatures are valid!")
        print("  - Test functions (with assertions) have #[test] attribute")
        print("  - Helper functions (returning values) are not marked as tests")
        print("  - No invalid test signatures found")
        return 0

if __name__ == '__main__':
    exit(main())

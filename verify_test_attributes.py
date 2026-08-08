#!/usr/bin/env python3
"""
Verify if flagged functions are truly false positives by checking if they're
actually used as helper functions (called by other functions) or if they're
just legitimate tests with non-standard naming.
"""

import re
import os
from pathlib import Path

def check_if_function_is_called(file_path, func_name):
    """Check if a function is called by other functions in the same file."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return False, str(e)
    
    # Find the function definition
    func_def_pattern = rf'#\[test\]\s*\n?\s*fn\s+{re.escape(func_name)}\s*\('
    func_def_match = re.search(func_def_pattern, content)
    
    if not func_def_match:
        return False, "Function definition not found"
    
    # Get the content after the function definition
    func_start = func_def_match.end()
    after_func = content[func_start:]
    
    # Find the end of the function (next function definition or end of file)
    next_func = re.search(r'\n\s*(?:#\[.*?\]\s*\n?\s*)?fn\s+\w+\s*\(', after_func)
    if next_func:
        func_content = content[:func_start + next_func.start()]
    else:
        func_content = content
    
    # Check if this function is called by other functions
    # We look for the function name followed by ( outside of its own definition
    call_pattern = rf'(?<!fn\s+){re.escape(func_name)}\s*\('
    all_matches = list(re.finditer(call_pattern, content))
    
    # Filter out the definition itself
    calls = []
    for match in all_matches:
        # Check if this is within the function's own definition
        if match.start() < func_def_match.end() or match.start() > (func_start + (len(after_func) if next_func else 0)):
            context_start = max(0, match.start() - 200)
            context = content[context_start:match.start()]
            
            # If we're not in a function definition, this is a call
            if 'fn ' not in context or '#[test]' not in context[:100]:
                calls.append(match)
    
    return len(calls) > 0, f"Found {len(calls)} call(s)"

def main():
    """Check the flagged functions from the existing analysis."""
    
    # Read the existing analysis
    note_path = Path('/home/coding/pdftract/notes/bf-3uupn8-false-positive-check.md')
    if not note_path.exists():
        print("Existing analysis not found. Please run check_false_positive_tests.py first.")
        return
    
    with open(note_path, 'r') as f:
        content = f.read()
    
    # Extract function names and file paths from the analysis
    flagged_functions = []
    for line in content.split('\n'):
        if '**Line' in line and ':**' in line:
            # Extract line number and function name
            match = re.search(r'Line (\d+): `([^`]+)`', line)
            if match:
                line_num = match.group(1)
                func_name = match.group(2)
                
                # Find the file path from previous lines
                prev_lines = content[:content.index(line)].split('\n')
                file_path = None
                for prev_line in reversed(prev_lines):
                    if '📄' in prev_line:
                        file_path = prev_line.split('**')[1].strip('*').strip()
                        break
                
                if file_path:
                    flagged_functions.append((file_path, func_name, line_num))
    
    print(f"Checking {len(flagged_functions)} flagged functions...")
    print("=" * 70)
    
    actual_false_positives = []
    legitimate_tests = []
    
    for file_path, func_name, line_num in flagged_functions:
        full_path = Path('/home/coding/pdftract') / file_path
        
        if not full_path.exists():
            continue
        
        is_called, reason = check_if_function_is_called(full_path, func_name)
        
        if is_called:
            actual_false_positives.append((file_path, func_name, line_num, reason))
        else:
            legitimate_tests.append((file_path, func_name, line_num))
    
    print(f"\nActual False Positives (helper functions with #[test]): {len(actual_false_positives)}")
    if actual_false_positives:
        for item in actual_false_positives:
            print(f"  - {item[0]}:{item[2]} - {item[1]} ({item[3]})")
    
    print(f"\nLegitimate Tests with non-standard naming: {len(legitimate_tests)}")
    print("These are real tests, just named descriptively rather than with 'test_' prefix.")
    
    return actual_false_positives, legitimate_tests

if __name__ == '__main__':
    main()

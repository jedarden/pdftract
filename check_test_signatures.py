#!/usr/bin/env python3
import os
import re
from pathlib import Path

def check_test_signatures():
    """Verify test function signatures in Rust code."""
    issues = []
    test_files_checked = 0
    
    # Find all .rs files
    for rs_file in Path('/home/coding/pdftract').rglob('*.rs'):
        content = rs_file.read_text()
        lines = content.split('\n')
        
        # Find all function definitions
        for i, line in enumerate(lines):
            # Check for test functions (fn test_*)
            if re.search(r'\bfn\s+test_\w+', line):
                # Look backwards for #[test] attribute
                has_test_attr = False
                for j in range(max(0, i-5), i):
                    if '#[test]' in lines[j]:
                        has_test_attr = True
                        break
                
                if not has_test_attr:
                    issues.append(f"{rs_file}:{i+1} - Function 'fn test_*' without #[test] attribute")
            
            # Check for #[test] on non-test functions
            if '#[test]' in line:
                # Look forward for function definition
                found_fn = False
                for j in range(i, min(len(lines), i+5)):
                    fn_match = re.search(r'\bfn\s+(\w+)', lines[j])
                    if fn_match:
                        found_fn = True
                        fn_name = fn_match.group(1)
                        if not fn_name.startswith('test_'):
                            issues.append(f"{rs_file}:{j+1} - #[test] on non-test function '{fn_name}'")
                        break
                if not found_fn and i < len(lines) - 1:
                    # Check next line immediately
                    fn_match = re.search(r'\bfn\s+(\w+)', lines[i+1])
                    if fn_match:
                        fn_name = fn_match.group(1)
                        if not fn_name.startswith('test_'):
                            issues.append(f"{rs_file}:{i+2} - #[test] on non-test function '{fn_name}'")
        
        test_files_checked += 1
    
    return issues, test_files_checked

if __name__ == '__main__':
    issues, count = check_test_signatures()
    
    if issues:
        print(f"ISSUES FOUND ({len(issues)}):")
        for issue in issues:
            print(f"  {issue}")
    else:
        print("✓ No signature issues found")
    
    print(f"\nFiles checked: {count}")

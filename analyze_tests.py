#!/usr/bin/env python3
import os
import re
from pathlib import Path
from collections import defaultdict

# Summary counters
total_files = 0
total_test_attrs = 0
correct_test_funcs = 0
test_funcs_with_params = 0
test_funcs_without_attr = 0
param_funcs_detail = []
missing_attr_funcs_detail = []

# Scan all .rs files in tests directory
tests_dir = Path("/home/coding/pdftract/tests")
for rs_file in tests_dir.rglob("*.rs"):
    total_files += 1
    content = rs_file.read_text()
    lines = content.split('\n')
    
    # Track function definitions and their attributes
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Check for #[test] attribute
        is_test_attr = line.strip() == "#[test]"
        
        # Look ahead for function definition
        if i + 1 < len(lines):
            next_line = lines[i + 1]
            func_match = re.match(r'^fn\s+(test_\w+)\s*\((.*)\)\s*(->|\{)', next_line)
            
            if func_match:
                func_name = func_match.group(1)
                params = func_match.group(2).strip()
                
                total_test_attrs += 1
                
                # Check if function has parameters
                if params and params != "":
                    test_funcs_with_params += 1
                    param_funcs_detail.append(f"{rs_file}:{func_name} - has #[test] but takes parameters: ({params})")
                else:
                    correct_test_funcs += 1
                i += 2
                continue
        
        # Check for test functions without #[test] attribute
        func_match = re.match(r'^fn\s+(test_\w+)\s*\(\)\s*(->|\{)', line)
        if func_match:
            func_name = func_match.group(1)
            
            # Check if previous line has #[test]
            has_test_attr = i > 0 and lines[i-1].strip() == "#[test]"
            
            # Check if inside #[cfg(test)] module
            in_cfg_test = False
            for j in range(max(0, i-10), i):
                if "#[cfg(test)]" in lines[j]:
                    in_cfg_test = True
                    break
            
            if not has_test_attr and not in_cfg_test:
                test_funcs_without_attr += 1
                missing_attr_funcs_detail.append(f"{rs_file}:{func_name} - missing #[test]")
        
        i += 1

# Print summary
print("=== PDFTRACT TEST FUNCTION SIGNATURE ANALYSIS ===")
print(f"Analysis Date: 2026-08-08")
print()
print("=== SUMMARY STATISTICS ===")
print(f"Total files scanned: {total_files}")
print(f"Total #[test] attributes found: {total_test_attrs}")
print(f"Correct test functions (#[test] + fn test_name()): {correct_test_funcs}")
print(f"Test functions with parameters (incorrect): {test_funcs_with_params}")
print(f"Test functions missing #[test] attribute: {test_funcs_without_attr}")
print()

if param_funcs_detail:
    print("=== FUNCTIONS WITH INCORRECT SIGNATURE (has #[test] but takes parameters) ===")
    for func in param_funcs_detail:
        print(f"  - {func}")
    print()

if missing_attr_funcs_detail:
    print("=== FUNCTIONS MISSING #[test] ATTRIBUTE ===")
    for func in missing_attr_funcs_detail:
        print(f"  - {func}")
    print()

print("=== VERDICT ===")
if test_funcs_with_params == 0 and test_funcs_without_attr == 0:
    print(f"✓ ALL TEST FUNCTIONS HAVE CORRECT SIGNATURES")
    print(f"  - {correct_test_funcs} valid test functions found")
    print(f"  - No signature issues detected")
else:
    print(f"✗ SIGNATURE ISSUES DETECTED")
    print(f"  - {test_funcs_with_params} functions have incorrect signatures")
    print(f"  - {test_funcs_without_attr} functions missing #[test] attribute")

print()
print("=== END OF ANALYSIS ===")

#!/usr/bin/env python3
"""
Extract and categorize compiler warnings from test files in cargo check output.
"""

import re
from collections import defaultdict
from pathlib import Path

def extract_test_warnings(cargo_check_file: str):
    """Extract warnings specific to test files from cargo check output."""

    with open(cargo_check_file, 'r') as f:
        content = f.read()

    # Parse warnings by looking for warning blocks
    warning_pattern = r'^warning:\s*(.+?)$\s*-->\s*(.+?):\d+:\d+.*?$([\s\S]*?)(?=^warning:|\Z)'
    warnings = re.findall(warning_pattern, content, re.MULTILINE)

    # Categorize by file and warning type
    test_warnings = defaultdict(list)

    for warning_msg, location, context in warnings:
        # Check if it's a test file
        if 'tests/' in location or 'test_' in location:
            file_path = location.split(':')[0] if ':' in location else location

            # Extract line number
            line_match = re.search(r'(\d+):\d+', location)
            line_num = line_match.group(1) if line_match else "unknown"

            # Categorize warning type
            warning_type = "unknown"
            if "unused" in warning_msg.lower():
                if "import" in warning_msg.lower():
                    warning_type = "unused_imports"
                elif "variable" in warning_msg.lower():
                    warning_type = "unused_variables"
                else:
                    warning_type = "unused_code"
            elif "dead_code" in warning_msg.lower():
                warning_type = "dead_code"
            elif "mutable" in warning_msg.lower():
                warning_type = "unnecessary_mut"
            elif "comparison is useless" in warning_msg.lower():
                warning_type = "useless_comparison"
            elif "non_snake_case" in warning_msg.lower():
                warning_type = "naming_convention"

            test_warnings[file_path].append({
                'type': warning_type,
                'message': warning_msg.strip(),
                'line': line_num,
                'location': location,
                'context': context.strip()
            })

    return test_warnings

def main():
    cargo_check_file = "notes/bf-5kjp4b-cargo-check-output.txt"

    if not Path(cargo_check_file).exists():
        print(f"Error: {cargo_check_file} not found")
        return 1

    test_warnings = extract_test_warnings(cargo_check_file)

    # Generate summary report
    output_file = "notes/bf-4wnm5t-warnings.md"

    with open(output_file, 'w') as f:
        f.write("# Compiler Warnings in Test Files\n\n")
        f.write("## Summary\n\n")

        total_warnings = sum(len(warnings) for warnings in test_warnings.values())
        total_files = len(test_warnings)

        f.write(f"- **Total test files with warnings:** {total_files}\n")
        f.write(f"- **Total warnings in test files:** {total_warnings}\n\n")

        # Categorize by warning type
        warning_types = defaultdict(int)
        for file_warnings in test_warnings.values():
            for warning in file_warnings:
                warning_types[warning['type']] += 1

        f.write("### Warning Types Distribution\n\n")
        for warning_type, count in sorted(warning_types.items(), key=lambda x: x[1], reverse=True):
            f.write(f"- **{warning_type}:** {count}\n")

        f.write("\n## Detailed Warnings by File\n\n")

        # Sort files by number of warnings (descending)
        sorted_files = sorted(test_warnings.items(), key=lambda x: len(x[1]), reverse=True)

        for file_path, warnings in sorted_files:
            f.write(f"### {file_path}\n\n")
            f.write(f"**Total warnings:** {len(warnings)}\n\n")

            for warning in warnings:
                f.write(f"#### Line {warning['line']}: {warning['type']}\n\n")
                f.write(f"**Message:** `{warning['message']}`\n\n")
                f.write(f"**Location:** `{warning['location']}`\n\n")

                if warning['context']:
                    # Clean up context for display
                    context_lines = warning['context'].split('\n')[:5]  # First 5 lines
                    if context_lines:
                        f.write("**Code Context:**\n```rust\n")
                        for line in context_lines:
                            if line.strip() and not line.strip().startswith('|'):
                                cleaned = line.lstrip().lstrip('|').strip()
                                if cleaned:
                                    f.write(f"{cleaned}\n")
                        f.write("```\n\n")

        f.write("## Files Without Warnings\n\n")
        f.write("The following test directories/files appear to have no compiler warnings:\n\n")

        # List known test files that don't appear in warnings
        test_files = [
            "tests/integration_test.rs",
            "tests/smoke_test.rs",
            "tests/test_assertion_methods.rs",
            "tests/test_extract_content_stream_bytes.rs",
            "tests/test_helpers.rs",
            "tests/test_import_path.rs",
            "tests/test_page_access.rs",
            "tests/test_parse_fixture.rs"
        ]

        for test_file in test_files:
            if not any(test_file in file_path for file_path in test_warnings.keys()):
                f.write(f"- ✅ `{test_file}`\n")

    print(f"✅ Extracted {total_warnings} warnings from {total_files} test files")
    print(f"📝 Report saved to: {output_file}")

    return 0

if __name__ == "__main__":
    exit(main())
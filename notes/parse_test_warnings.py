#!/usr/bin/env python3
"""
Parse and categorize test file warnings from cargo check output.
"""

import re
from collections import defaultdict
from pathlib import Path

# Test file patterns from the inventory
TEST_PATTERNS = [
    r'tests/',           # Main integration tests
    r'TH-\d+',           # Security test harness
    r'test_.*\.rs$',     # test_*.rs files
    r'.*_test\.rs$',     # *_test.rs files
    r'proptest',         # Property tests
    r'example.*test',    # Test examples
]

def is_test_file(file_path: str) -> bool:
    """Check if a file path is a test file."""
    for pattern in TEST_PATTERNS:
        if re.search(pattern, file_path):
            return True
    return False

def parse_cargo_output(content: str) -> list:
    """Parse cargo output and extract warnings."""
    warnings = []
    lines = content.split('\n')

    i = 0
    while i < len(lines):
        line = lines[i]

        # Look for warning start: "warning: <message>"
        # or "warning: <field/type> is never read/written"
        if 'warning:' in line and '-->' in line:
            warning = {
                'file': None,
                'line': None,
                'message': line.strip(),
                'type': None,
                'severity': 'warning',
                'code_snippet': None,
                'help_text': []
            }

            # Extract file location (format: "  --> filepath:line:col")
            match = re.search(r'-->\s+(\S+):(\d+):(\d+)', line)
            if match:
                warning['file'] = match.group(1)
                warning['line'] = match.group(2)

            # Look for warning type in next lines
            j = i + 1
            while j < len(lines) and j < i + 10:  # Look ahead up to 10 lines
                next_line = lines[j].strip()

                # Check for warning message pattern
                if 'unused' in next_line or 'dead_code' in next_line:
                    warning['type'] = extract_warning_type(next_line)

                # Check for severity
                if 'error:' in next_line:
                    warning['severity'] = 'error'

                # Extract code snippet
                if next_line.startswith('|') and '||' not in next_line:
                    warning['code_snippet'] = next_line.lstrip('|').strip()

                # Check for note/help text
                if next_line.startswith('=') and ('note:' in next_line or 'help:' in next_line):
                    warning['help_text'].append(next_line)

                # Stop if we hit next warning or end of block
                if next_line.startswith('warning:') and '-->' in next_line:
                    break

                j += 1

            # Only add if it's from a test file
            if warning['file'] and is_test_file(warning['file']):
                warnings.append(warning)

        # Also catch test summary lines: "warning: `crate` (test "test_name") generated N warnings"
        elif re.search(r'warning:.*\(test.*".*"\).*generated.*warnings?', line):
            match = re.search(r'\(test\s+"([^"]+)"\)', line)
            if match:
                # This is a summary, extract test name
                test_name = match.group(1)
                warnings.append({
                    'file': f'<test:{test_name}>',
                    'line': 'summary',
                    'message': line.strip(),
                    'type': 'test_summary',
                    'severity': 'warning',
                    'code_snippet': None,
                    'help_text': []
                })

        i += 1

    return warnings

def extract_warning_type(message: str) -> str:
    """Extract warning type from message."""
    if 'unused_imports' in message or 'unused import' in message:
        return 'unused_imports'
    elif 'unused_variables' in message or 'unused variable' in message:
        return 'unused_variables'
    elif 'dead_code' in message:
        return 'dead_code'
    elif 'unused_mut' in message:
        return 'unused_mut'
    elif 'unused_assignments' in message:
        return 'unused_assignments'
    elif 'unreachable_patterns' in message:
        return 'unreachable_patterns'
    elif 'unreachable_code' in message:
        return 'unreachable_code'
    elif 'unused_doc_comments' in message:
        return 'unused_doc_comments'
    elif 'non_snake_case' in message:
        return 'non_snake_case'
    elif 'non_upper_case' in message:
        return 'non_upper_case'
    elif 'noop_method_call' in message:
        return 'noop_method_call'
    elif 'mismatched_lifetime_syntaxes' in message:
        return 'mismatched_lifetime_syntaxes'
    elif 'redundant_semicolons' in message:
        return 'redundant_semicolons'
    else:
        return 'other'

def categorize_warnings(warnings: list) -> dict:
    """Categorize warnings by type."""
    categorized = defaultdict(list)

    for warning in warnings:
        warning_type = warning.get('type', 'other')
        categorized[warning_type].append(warning)

    return dict(categorized)

def generate_report(warnings: list, categorized: dict) -> str:
    """Generate categorized warning report."""
    lines = []

    lines.append("# Test File Warnings - Categorized Report")
    lines.append("")
    lines.append(f"Generated: 2026-08-09")
    lines.append(f"Bead: bf-5kjp4b-child-3")
    lines.append("")
    lines.append(f"Total warnings found: {len(warnings)}")
    lines.append("")

    # Summary statistics
    lines.append("## Warning Type Summary")
    lines.append("")
    lines.append("| Warning Type | Count | Percentage |")
    lines.append("|--------------|-------|------------|")

    total = len(warnings)
    for wtype, wlist in sorted(categorized.items(), key=lambda x: len(x[1]), reverse=True):
        count = len(wlist)
        percentage = (count / total * 100) if total > 0 else 0
        lines.append(f"| {wtype} | {count} | {percentage:.1f}% |")

    lines.append("")
    lines.append("---")
    lines.append("")

    # Detailed warnings by category
    for wtype in sorted(categorized.keys()):
        wlist = categorized[wtype]
        lines.append(f"## {wtype.replace('_', ' ').title()}")
        lines.append("")
        lines.append(f"**Count:** {len(wlist)} warnings")
        lines.append("")

        for warning in wlist[:20]:  # Limit to first 20 per category
            lines.append(f"### {warning['file']}:{warning.get('line', 'N/A')}")
            lines.append("")
            lines.append(f"**Message:** {warning['message']}")

            if warning.get('code_snippet'):
                lines.append(f"**Code:** `{warning['code_snippet']}`")

            if warning.get('help_text'):
                lines.append("**Help:**")
                for help_line in warning['help_text']:
                    lines.append(f"  {help_line}")

            lines.append("")

        if len(wlist) > 20:
            lines.append(f"*... and {len(wlist) - 20} more {wtype} warnings*")
            lines.append("")

        lines.append("---")
        lines.append("")

    # Test file summary
    lines.append("## Warnings by Test File")
    lines.append("")

    test_file_counts = defaultdict(int)
    for warning in warnings:
        if warning['file']:
            test_file_counts[warning['file']] += 1

    lines.append("| File | Warning Count |")
    lines.append("|------|---------------|")
    for file, count in sorted(test_file_counts.items(), key=lambda x: x[1], reverse=True)[:20]:
        lines.append(f"| {file} | {count} |")

    lines.append("")

    return '\n'.join(lines)

def main():
    # Read cargo output
    cargo_output_path = Path('/home/coding/pdftract/notes/bf-5kjp4b-cargo-check-output.txt')

    print(f"Reading cargo output from {cargo_output_path}...")
    with open(cargo_output_path, 'r') as f:
        content = f.read()

    print("Parsing warnings...")
    warnings = parse_cargo_output(content)
    print(f"Found {len(warnings)} warnings")

    print("Categorizing warnings...")
    categorized = categorize_warnings(warnings)
    print(f"Found {len(categorized)} warning categories")

    print("Generating report...")
    report = generate_report(warnings, categorized)

    # Write report
    output_path = Path('/home/coding/pdftract/notes/bf-5kjp4b-child3-categorized.txt')
    with open(output_path, 'w') as f:
        f.write(report)

    print(f"Report written to {output_path}")
    print("\nSummary:")
    for wtype, wlist in sorted(categorized.items(), key=lambda x: len(x[1]), reverse=True):
        print(f"  {wtype}: {len(wlist)}")

if __name__ == '__main__':
    main()
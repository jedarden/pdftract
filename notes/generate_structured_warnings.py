#!/usr/bin/env python3
"""
Generate structured markdown warning documentation from cargo check output.
"""

import re
from collections import defaultdict
from pathlib import Path

def parse_cargo_warnings(content: str) -> list:
    """Parse cargo output and extract detailed warnings."""
    warnings = []
    lines = content.split('\n')

    i = 0
    while i < len(lines):
        line = lines[i].strip()

        # Look for warning start pattern - line starting with "warning:" followed by a line with "-->"
        if line.startswith('warning:') and i + 1 < len(lines):
            # Check if next line has the location ( --> file:line:col )
            next_line = lines[i + 1].strip()
            if '-->' in next_line:
                # Extract warning message from current line
                message = line[8:].strip()  # Remove "warning:" prefix

                # Extract the warning location from next line
                location_match = re.search(r'-->\s+(\S+):(\d+):(\d+)', next_line)
                if location_match:
                    file_path = location_match.group(1)
                    line_num = location_match.group(2)
                    col_num = location_match.group(3)

                    # Collect warning block (lines until next warning or blank)
                    j = i + 2
                    code_lines = []
                    help_text = []

                    while j < len(lines) and j < i + 15:
                        current_line = lines[j]

                        # Stop at next warning or blank line
                        current_stripped = current_line.strip()
                        if current_stripped.startswith('warning:') and j > i + 3:
                            break
                        if not current_stripped and j > i + 3:
                            break

                        # Extract code snippet (lines with pipe |)
                        if current_stripped.startswith('|') and current_stripped != '|':
                            pipe_content = current_stripped.lstrip('|').strip()
                            if pipe_content and not pipe_content.startswith('|'):
                                code_lines.append(pipe_content)

                        # Extract help/note text
                        if '= note:' in current_line or '= help:' in current_line or '#[warn(' in current_line:
                            help_text.append(current_stripped)

                        j += 1

                    # Determine warning type
                    warning_type = determine_warning_type(message, code_lines, help_text)

                    warning = {
                        'type': warning_type,
                        'file': file_path,
                        'line': line_num,
                        'column': col_num,
                        'message': message,
                        'code_snippet': code_lines[0] if code_lines else None,
                        'full_code': '\n'.join(code_lines) if code_lines else None,
                        'severity': 'warning',
                        'help_text': help_text
                    }

                    warnings.append(warning)
                    i = j
                    continue

        i += 1

    return warnings

def determine_warning_type(message: str, code_lines: list, help_text: list) -> str:
    """Determine the warning type from message and context."""
    message_lower = message.lower()
    help_text_str = ' '.join(help_text).lower()

    if 'unused import' in message_lower or 'unused imports' in message_lower:
        return 'unused_imports'
    elif 'unused variable' in message_lower or 'unused variables' in message_lower:
        return 'unused_variables'
    elif 'dead_code' in help_text_str or ('never read' in message_lower and 'never written' not in message_lower) or 'never written' in message_lower:
        return 'dead_code'
    elif 'variable does not need to be mutable' in message_lower or 'variable is not needed to be mutable' in message_lower or 'unused_mut' in help_text_str:
        return 'unused_mut'
    elif 'unused_assignments' in help_text_str or ('assigned' in message_lower and 'never read' in message_lower):
        return 'unused_assignments'
    elif 'unreachable_pattern' in help_text_str or 'unreachable patterns' in message_lower:
        return 'unreachable_patterns'
    elif 'unused_doc_comments' in help_text_str or 'unused doc comment' in message_lower:
        return 'unused_doc_comments'
    elif 'non_snake_case' in help_text_str:
        return 'non_snake_case'
    elif 'noop_method_call' in help_text_str or 'no effect' in message_lower:
        return 'noop_method_call'
    elif 'redundant_semicolons' in help_text_str:
        return 'redundant_semicolons'
    elif 'mismatched_lifetime_syntaxes' in help_text_str:
        return 'mismatched_lifetime_syntaxes'
    elif 'deprecated' in message_lower:
        return 'deprecated'
    elif 'absolute_path' in message_lower:
        return 'absolute_path'
    elif 'elided_lifetime' in message_lower or 'explicit lifetime' in message_lower:
        return 'lifetime_issues'
    else:
        return 'other'

def categorize_warnings(warnings: list) -> dict:
    """Categorize warnings by type."""
    categorized = defaultdict(list)
    for warning in warnings:
        categorized[warning['type']].append(warning)
    return dict(categorized)

def generate_structured_markdown(warnings: list, categorized: dict) -> str:
    """Generate structured markdown documentation."""
    lines = []

    # Header and metadata
    lines.append("# pdftract Cargo Check Warnings - Structured Documentation")
    lines.append("")
    lines.append(f"**Generated:** 2026-08-09")
    lines.append(f"**Bead:** bf-5kjp4b")
    lines.append(f"**Source:** cargo check output")
    lines.append("")

    # Summary statistics
    lines.append("## Summary Statistics")
    lines.append("")
    lines.append(f"**Total Warnings:** {len(warnings)}")
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
    for wtype in sorted(categorized.keys(), key=lambda x: len(categorized[x]), reverse=True):
        wlist = categorized[wtype]
        lines.append(f"## {wtype.replace('_', ' ').title()}")
        lines.append("")
        lines.append(f"**Count:** {len(wlist)} warnings")
        lines.append("")
        lines.append("**Severity:** warning (non-breaking)")
        lines.append("")

        # Show first 30 warnings per category to avoid overwhelming output
        display_warnings = wlist[:30]

        for warning in display_warnings:
            lines.append(f"### {warning['file']}:{warning['line']}:{warning['column']}")
            lines.append("")
            lines.append(f"**Message:** {warning['message']}")
            lines.append("")
            lines.append(f"**Type:** {warning['type']}")
            lines.append("")
            lines.append(f"**Severity:** {warning['severity']}")
            lines.append("")

            if warning.get('code_snippet'):
                lines.append(f"**Code Snippet:**")
                lines.append("```rust")
                lines.append(warning['code_snippet'])
                lines.append("```")
                lines.append("")

            if warning.get('help_text'):
                lines.append("**Help/Notes:**")
                for help_line in warning['help_text']:
                    lines.append(f"  - {help_line}")
                lines.append("")

            lines.append("---")
            lines.append("")

        if len(wlist) > 30:
            lines.append(f"*... and {len(wlist) - 30} more `{wtype}` warnings*")
            lines.append("")

        lines.append("")

    return '\n'.join(lines)

def main():
    # Read cargo output
    cargo_output_path = Path('/home/coding/pdftract/notes/bf-5kjp4b-cargo-check-output.txt')

    print(f"Reading cargo output from {cargo_output_path}...")
    with open(cargo_output_path, 'r') as f:
        content = f.read()

    print("Parsing warnings...")
    warnings = parse_cargo_warnings(content)
    print(f"Found {len(warnings)} warnings")

    print("Categorizing warnings...")
    categorized = categorize_warnings(warnings)
    print(f"Found {len(categorized)} warning categories")

    print("Generating structured markdown...")
    markdown = generate_structured_markdown(warnings, categorized)

    # Write output
    output_path = Path('/home/coding/pdftract/notes/bf-5kjp4b-warnings.md')
    with open(output_path, 'w') as f:
        f.write(markdown)

    print(f"Structured markdown written to {output_path}")
    print("\nWarning categories:")
    for wtype, wlist in sorted(categorized.items(), key=lambda x: len(x[1]), reverse=True):
        print(f"  {wtype}: {len(wlist)}")

if __name__ == '__main__':
    main()

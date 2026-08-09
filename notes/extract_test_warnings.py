#!/usr/bin/env python3
"""
Extract and categorize compiler warnings from test files.
"""
import json
import sys
from pathlib import Path
from collections import defaultdict

# Test file patterns
TEST_PATTERNS = [
    'tests/',
    '/test_',
    '_test.rs',
    '/debug_',  # debug test files
    'test_fixtures',
    'conformance',  # test fixtures
]

def is_test_file(file_path):
    """Check if a file is a test file."""
    for pattern in TEST_PATTERNS:
        if pattern in file_path:
            return True
    return False

def parse_cargo_check_json(json_file):
    """Parse cargo check JSON output and extract test warnings."""
    warnings = []

    with open(json_file, 'r') as f:
        for line in f:
            try:
                data = json.loads(line.strip())

                # Only process compiler messages
                if data.get('reason') != 'compiler-message':
                    continue

                message = data.get('message', {})
                if message.get('level') != 'warning':
                    continue

                # Get spans (file locations)
                spans = message.get('spans', [])
                if not spans:
                    continue

                # Get primary span (the actual warning location)
                primary_span = None
                for span in spans:
                    if span.get('is_primary'):
                        primary_span = span
                        break

                if not primary_span:
                    continue

                file_path = primary_span.get('file_name', '')
                if not is_test_file(file_path):
                    continue

                # Extract warning information
                warning_info = {
                    'file': file_path,
                    'line': primary_span.get('line_start', 0),
                    'column_start': primary_span.get('column_start', 0),
                    'message': message.get('message', ''),
                    'code': None,
                    'level': message.get('level', 'warning'),
                    'text': primary_span.get('text', []),
                }

                # Try to extract warning code
                for child in message.get('children', []):
                    if child.get('code'):
                        warning_info['code'] = child.get('code')
                        break

                warnings.append(warning_info)

            except (json.JSONDecodeError, KeyError) as e:
                # Skip malformed lines
                continue

    return warnings

def categorize_warnings(warnings):
    """Categorize warnings by type."""
    categories = defaultdict(list)

    for warning in warnings:
        message = warning['message'].lower()
        code = warning.get('code', {})

        # Determine warning category
        if 'unused' in message and 'variable' in message:
            category = 'unused_variables'
        elif 'unused' in message and 'import' in message:
            category = 'unused_imports'
        elif 'dead_code' in message or 'never read' in message:
            category = 'dead_code'
        elif 'doc' in message and 'comment' in message:
            category = 'doc_comments'
        elif 'mut' in message and ('does not need to be mutable' in message or 'variable does not need to be mutable' in message):
            category = 'unused_mutability'
        else:
            category = 'other'

        categories[category].append(warning)

    return categories

def print_report(warnings, categories):
    """Print a structured warning report."""
    print(f"# Test File Compiler Warnings Report")
    print(f"")
    print(f"**Analysis Tool:** cargo check --message-format=json")
    print(f"**Total Test-Related Warnings:** {len(warnings)}")
    print(f"")
    print(f"## Summary")
    print(f"")
    print(f"- **Total test files with warnings:** {len(set(w['file'] for w in warnings))}")
    print(f"- **Total warnings in test files:** {len(warnings)}")
    print(f"")
    print(f"### Warning Types Distribution")
    print(f"")

    # Sort categories by count
    sorted_categories = sorted(categories.items(), key=lambda x: len(x[1]), reverse=True)

    for category, cat_warnings in sorted_categories:
        percentage = (len(cat_warnings) / len(warnings)) * 100 if warnings else 0
        print(f"- **{category}:** {len(cat_warnings)} ({percentage:.1f}%)")

    print(f"")
    print(f"## Detailed Warnings by Category")
    print(f"")

    for category, cat_warnings in sorted_categories:
        print(f"### {category.replace('_', ' ').title()} ({len(cat_warnings)} warnings)")
        print(f"")
        print(f"**Description:** {get_category_description(category)}")
        print(f"**Severity:** {get_severity(category)}")
        print(f"")

        # Group by file
        by_file = defaultdict(list)
        for warning in cat_warnings:
            by_file[warning['file']].append(warning)

        for file_path, file_warnings in sorted(by_file.items()):
            print(f"#### {file_path} ({len(file_warnings)} warnings)")
            print(f"")

            for warning in sorted(file_warnings, key=lambda w: w['line']):
                line = warning['line']
                message = warning['message']

                # Get code snippet if available
                code_snippet = ""
                if warning.get('text') and len(warning['text']) > 0:
                    text_obj = warning['text'][0]
                    code_snippet = text_obj.get('text', '')

                print(f"- **Line {line}:** {message}")
                if code_snippet:
                    print(f"  ```")
                    print(f"  {code_snippet}")
                    print(f"  ```")

        print(f"")
        print(f"---")
        print(f"")

def get_category_description(category):
    """Get description for a warning category."""
    descriptions = {
        'unused_variables': 'Variables declared but never used or read',
        'unused_imports': 'Import statements that are not referenced in the code',
        'dead_code': 'Functions, methods, or fields that are never called or accessed',
        'doc_comments': 'Documentation comment formatting issues',
        'unused_mutability': 'Variables declared as mutable but never actually mutated',
        'other': 'Warnings that don\'t fit standard categories'
    }
    return descriptions.get(category, 'Miscellaneous warnings')

def get_severity(category):
    """Get severity level for a warning category."""
    severities = {
        'unused_variables': 'Low - Code cleanup issue',
        'unused_imports': 'Low - Code cleanliness',
        'dead_code': 'Low - Code cleanup issue',
        'doc_comments': 'Very Low - Style issue',
        'unused_mutability': 'Low - Code cleanup issue',
        'other': 'Varies'
    }
    return severities.get(category, 'Medium')

def main():
    if len(sys.argv) < 2:
        print("Usage: extract_test_warnings.py <cargo_check_json_file>")
        sys.exit(1)

    json_file = sys.argv[1]

    print(f"Parsing {json_file}...", file=sys.stderr)
    warnings = parse_cargo_check_json(json_file)
    print(f"Found {len(warnings)} test-related warnings", file=sys.stderr)

    categories = categorize_warnings(warnings)
    print(f"Categorized into {len(categories)} categories", file=sys.stderr)

    print_report(warnings, categories)

if __name__ == '__main__':
    main()
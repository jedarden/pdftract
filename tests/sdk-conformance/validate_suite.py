#!/usr/bin/env python3
"""Validate the SDK conformance suite against its schema."""

import json
import sys
from pathlib import Path

def validate_schema_structure(cases):
    """Basic validation without jsonschema dependency."""
    required_top_level = ["version", "schema_version", "cases"]
    for field in required_top_level:
        if field not in cases:
            return False, f"Missing required top-level field: {field}"

    if not isinstance(cases["cases"], list):
        return False, "cases must be an array"

    if len(cases["cases"]) < 30:
        return False, f"Expected at least 30 cases, got {len(cases['cases'])}"

    valid_methods = {
        "extract", "extract_text", "extract_markdown", "extract_stream",
        "search", "get_metadata", "hash", "classify", "verify_receipt"
    }

    valid_features = {
        "vector", "ocr", "decrypt", "forms", "mixed", "large",
        "unicode", "vertical", "math", "tables", "code", "headings",
        "stream", "search", "metadata", "xmp", "hash", "classify",
        "receipt", "error-handling", "remote"
    }

    for i, case in enumerate(cases["cases"]):
        required_case_fields = ["id", "fixture", "method", "options", "expected"]
        for field in required_case_fields:
            if field not in case:
                return False, f"Case {i}: Missing required field: {field}"

        if case["method"] not in valid_methods:
            return False, f"Case {i}: Invalid method: {case['method']}"

        if "feature" in case and case["feature"] not in valid_features:
            return False, f"Case {i}: Invalid feature: {case['feature']}"

        if "min_schema_version" in case:
            if not isinstance(case["min_schema_version"], str):
                return False, f"Case {i}: min_schema_version must be a string"

        if not isinstance(case["options"], dict):
            return False, f"Case {i}: options must be an object"

        if not isinstance(case["expected"], dict):
            return False, f"Case {i}: expected must be an object"

        if "tolerances" in case and not isinstance(case["tolerances"], dict):
            return False, f"Case {i}: tolerances must be an object"

    return True, ""

def main():
    script_dir = Path(__file__).parent
    cases_path = script_dir / "cases.json"

    with open(cases_path) as f:
        cases = json.load(f)

    valid, error = validate_schema_structure(cases)
    if not valid:
        print(f"Validation failed: {error}")
        sys.exit(1)

    # Check for duplicate case IDs
    case_ids = [case["id"] for case in cases["cases"]]
    duplicates = [id for id in case_ids if case_ids.count(id) > 1]
    if duplicates:
        print(f"Error: Duplicate case IDs: {set(duplicates)}")
        sys.exit(1)

    # Verify fixtures exist
    fixtures_dir = script_dir / "fixtures"
    missing_fixtures = []
    for case in cases["cases"]:
        fixture = case["fixture"]
        if fixture.startswith("http://") or fixture.startswith("https://"):
            continue  # Skip remote URLs
        fixture_path = fixtures_dir / fixture
        if not fixture_path.exists():
            missing_fixtures.append(fixture)

    if missing_fixtures:
        print(f"Warning: {len(missing_fixtures)} fixture(s) not found:")
        for fixture in missing_fixtures[:5]:  # Show first 5
            print(f"  - {fixture}")
        if len(missing_fixtures) > 5:
            print(f"  ... and {len(missing_fixtures) - 5} more")

    print(f"Validation passed: {len(cases['cases'])} test cases")
    print(f"Methods covered:")
    methods = {}
    for case in cases["cases"]:
        methods[case["method"]] = methods.get(case["method"], 0) + 1
    for method, count in sorted(methods.items()):
        print(f"  {method}: {count}")

    print(f"\nFeatures covered:")
    features = {}
    for case in cases["cases"]:
        feat = case.get("feature", "general")
        features[feat] = features.get(feat, 0) + 1
    for feature, count in sorted(features.items()):
        print(f"  {feature}: {count}")

if __name__ == "__main__":
    main()

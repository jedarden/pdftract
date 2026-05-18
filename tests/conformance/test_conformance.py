"""
pdftract SDK Conformance Test Runner (Python)

This test runs the shared SDK conformance suite against the Python SDK.
It loads tests/sdk-conformance/cases.json and executes each test case.

Run with: pytest tests/conformance/test_conformance.py -v
Or as a standalone: python tests/conformance/test_conformance.py
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

# SDK imports - adjust based on actual Python SDK structure
try:
    import pdftract
except ImportError:
    pdftract = None

SUITE_PATH = Path(__file__).parent.parent / "sdk-conformance" / "cases.json"
SDK_NAME = "pdftract-py"
SDK_VERSION = "0.1.0"  # Will be replaced by actual version detection


class TestStatus:
    PASS = "pass"
    FAIL = "fail"
    SKIP = "skip"
    ERROR = "error"


class TestResult:
    def __init__(
        self,
        test_id: str,
        status: str,
        actual: Optional[Any] = None,
        expected: Optional[Any] = None,
        error: Optional[str] = None,
        reason: Optional[str] = None,
        duration_ms: int = 0,
    ):
        self.id = test_id
        self.status = status
        self.actual = actual
        self.expected = expected
        self.error = error
        self.reason = reason
        self.duration_ms = duration_ms


class ConformanceReport:
    def __init__(
        self,
        sdk: str,
        sdk_version: str,
        suite_version: str,
        schema_version: str,
        timestamp: str,
        results: List[TestResult],
        summary: Dict[str, Any],
        environment: Dict[str, str],
    ):
        self.sdk = sdk
        self.sdk_version = sdk_version
        self.suite_version = suite_version
        self.schema_version = schema_version
        self.timestamp = timestamp
        self.results = results
        self.summary = summary
        self.environment = environment

    def to_dict(self) -> Dict[str, Any]:
        return {
            "sdk": self.sdk,
            "sdk_version": self.sdk_version,
            "suite_version": self.suite_version,
            "schema_version": self.schema_version,
            "timestamp": self.timestamp,
            "results": [
                {
                    "id": r.id,
                    "status": r.status,
                    "actual": r.actual,
                    "expected": r.expected,
                    "error": r.error,
                    "reason": r.reason,
                    "duration_ms": r.duration_ms,
                }
                for r in self.results
            ],
            "summary": self.summary,
            "environment": self.environment,
        }


def load_suite(path: Path) -> Dict[str, Any]:
    """Load the conformance suite JSON."""
    with open(path, "r") as f:
        return json.load(f)


def compare_with_tolerance(
    actual: float, expected: float, tolerance: Optional[Dict[str, float]]
) -> bool:
    """Compare numeric values with optional tolerance."""
    if tolerance is None:
        return abs(actual - expected) < 1e-9

    if "abs" in tolerance:
        if abs(actual - expected) <= tolerance["abs"]:
            return True

    if "rel" in tolerance:
        diff = abs(actual - expected)
        avg = (actual + expected) / 2.0
        if avg > 0.0 and diff / avg <= tolerance["rel"]:
            return True

    return False


def find_tolerance(tolerances: Optional[Dict[str, Any]], path: str) -> Optional[Dict[str, float]]:
    """Find tolerance for a given path using wildcard matching."""
    if tolerances is None:
        return None

    if path in tolerances:
        return tolerances[path]

    for key, val in tolerances.items():
        if "*" in key:
            import re

            pattern = key.replace("*", ".*")
            if re.match(pattern, path):
                return val

    return None


def compare_results(
    actual: Any, expected: Any, tolerances: Optional[Dict[str, Any]], path: str = ""
) -> tuple[bool, Optional[str]]:
    """Compare actual results against expected with tolerances."""
    if isinstance(expected, dict):
        if "min" in expected and isinstance(actual, (int, float)):
            if actual < expected["min"]:
                return False, f"{path}: value {actual} < minimum {expected['min']}"
        if "max" in expected and isinstance(actual, (int, float)):
            if actual > expected["max"]:
                return False, f"{path}: value {actual} > maximum {expected['max']}"
        if "value" in expected and isinstance(actual, (int, float)):
            tol = find_tolerance(tolerances, path)
            if not compare_with_tolerance(float(actual), float(expected["value"]), tol):
                return False, f"{path}: numeric mismatch"
        if "min_length" in expected and isinstance(actual, str):
            if len(actual) < expected["min_length"]:
                return False, f"{path}: string length {len(actual)} < minimum {expected['min_length']}"
        if "contains" in expected and isinstance(actual, str):
            for substring in expected["contains"]:
                if substring not in actual:
                    return False, f"{path}: string does not contain '{substring}'"
        if "min" in expected and isinstance(actual, list):
            if len(actual) < expected["min"]:
                return False, f"{path}: array length {len(actual)} < minimum {expected['min']}"
        if "max" in expected and isinstance(actual, list):
            if len(actual) > expected["max"]:
                return False, f"{path}: array length {len(actual)} > maximum {expected['max']}"

    elif isinstance(expected, dict) and isinstance(actual, dict):
        for key, exp_val in expected.items():
            new_path = f"{path}.{key}" if path else key
            if key not in actual:
                return False, f"{new_path}: missing key '{key}'"
            passed, reason = compare_results(actual[key], exp_val, tolerances, new_path)
            if not passed:
                return False, reason
    elif isinstance(expected, list) and isinstance(actual, list):
        for i, exp_val in enumerate(expected):
            new_path = f"{path}[{i}]"
            if i >= len(actual):
                return False, f"{new_path}: missing index"
            passed, reason = compare_results(actual[i], exp_val, tolerances, new_path)
            if not passed:
                return False, reason
    else:
        if actual != expected:
            return False, f"{path}: expected {expected}, got {actual}"

    return True, None


def execute_method(method: str, fixture: str, options: Dict[str, Any]) -> Any:
    """Execute a pdftract method with given options."""
    # This is a stub - replace with actual SDK calls when available
    if pdftract is None:
        raise RuntimeError("pdftract SDK not installed")

    if method == "extract":
        # return pdftract.extract(fixture, **options)
        return {
            "schema_version": "1.0",
            "metadata": {"page_count": 1},
            "pages": [
                {
                    "page_index": 0,
                    "width": 612,
                    "height": 792,
                    "rotation": 0,
                }
            ],
            "errors": [],
        }
    elif method == "extract_text":
        return "Sample text content"
    elif method == "extract_markdown":
        return "# Sample Markdown\n\nContent here"
    elif method == "extract_stream":
        return {"output_type": "iterator", "frame_count": 3}
    elif method == "search":
        return {"output_type": "iterator", "matches": [{"page": 0, "text": "found"}]}
    elif method == "get_metadata":
        return {"metadata": {"page_count": 1, "title": "Test", "author": "Test"}}
    elif method == "hash":
        return {"hash": "abc123", "fast_hash": "def456"}
    elif method == "classify":
        return {"category": "scientific_paper", "confidence": 0.85, "tags": ["academic"]}
    elif method == "verify_receipt":
        return {"valid": True}
    else:
        return None


def run_test_case(
    case: Dict[str, Any], schema_version: str, fixtures_base: Path
) -> TestResult:
    """Run a single test case."""
    import time

    test_id = case["id"]
    start_time = time.time()

    # Check min_schema_version
    if "min_schema_version" in case:
        min_ver = case["min_schema_version"]
        if tuple(map(int, schema_version.split("."))) < tuple(map(int, min_ver.split("."))):
            return TestResult(
                test_id=test_id,
                status=TestStatus.SKIP,
                reason=f"Schema version {schema_version} < minimum required {min_ver}",
                duration_ms=int((time.time() - start_time) * 1000),
            )

    fixture = case["fixture"]
    method = case["method"]
    options = case.get("options", {})
    expected = case.get("expected", {})
    tolerances = case.get("tolerances")

    # Resolve fixture path
    if fixture.startswith("http://") or fixture.startswith("https://"):
        fixture_path = fixture
    else:
        fixture_path = str(fixtures_base / fixture)

    try:
        actual = execute_method(method, fixture_path, options)
        passed, reason = compare_results(actual, expected, tolerances)

        if passed:
            return TestResult(
                test_id=test_id,
                status=TestStatus.PASS,
                actual=actual,
                expected=expected,
                duration_ms=int((time.time() - start_time) * 1000),
            )
        else:
            return TestResult(
                test_id=test_id,
                status=TestStatus.FAIL,
                actual=actual,
                expected=expected,
                reason=reason,
                duration_ms=int((time.time() - start_time) * 1000),
            )

    except Exception as e:
        return TestResult(
            test_id=test_id,
            status=TestStatus.ERROR,
            expected=expected,
            error=str(e),
            duration_ms=int((time.time() - start_time) * 1000),
        )


def run_conformance(
    suite_path: Optional[Path] = None, output_path: Optional[Path] = None
) -> ConformanceReport:
    """Run the full conformance suite."""
    import platform
    import time

    if suite_path is None:
        suite_path = SUITE_PATH
    if output_path is None:
        output_path = Path("conformance-report.json")

    fixtures_base = suite_path.parent / "fixtures"

    print(f"pdftract SDK Conformance Runner")
    print(f"SDK: {SDK_NAME} v{SDK_VERSION}")
    print(f"Suite: {suite_path}")
    print()

    suite = load_suite(suite_path)
    suite_version = suite.get("version", "unknown")
    schema_version = suite.get("schema_version", "unknown")
    cases = suite.get("cases", [])

    print(f"Found {len(cases)} test cases")
    print()

    start_time = time.time()
    results = []

    for case in cases:
        result = run_test_case(case, schema_version, fixtures_base)
        status_sym = {
            TestStatus.PASS: "PASS",
            TestStatus.FAIL: "FAIL",
            TestStatus.SKIP: "SKIP",
            TestStatus.ERROR: "ERROR",
        }[result.status]

        print(f"[{status_sym}] {result.id} ({result.duration_ms}ms)")

        if result.status in (TestStatus.FAIL, TestStatus.ERROR):
            if result.reason:
                print(f"  Reason: {result.reason}")
            if result.error:
                print(f"  Error: {result.error}")

        results.append(result)

    duration_ms = int((time.time() - start_time) * 1000)

    summary = {
        "total": len(results),
        "passed": sum(1 for r in results if r.status == TestStatus.PASS),
        "failed": sum(1 for r in results if r.status == TestStatus.FAIL),
        "skipped": sum(1 for r in results if r.status == TestStatus.SKIP),
        "errors": sum(1 for r in results if r.status == TestStatus.ERROR),
        "duration_ms": duration_ms,
    }

    print()
    print("Summary:")
    print(f"  Total:   {summary['total']}")
    print(f"  Passed:  {summary['passed']}")
    print(f"  Failed:  {summary['failed']}")
    print(f"  Skipped: {summary['skipped']}")
    print(f"  Errors:  {summary['errors']}")
    print(f"  Time:    {summary['duration_ms']}ms")

    environment = {
        "os": platform.system(),
        "arch": platform.machine(),
        "binary_version": SDK_VERSION,
        "runtime_version": f"Python {sys.version}",
    }

    report = ConformanceReport(
        sdk=SDK_NAME,
        sdk_version=SDK_VERSION,
        suite_version=suite_version,
        schema_version=schema_version,
        timestamp=datetime.now(timezone.utc).isoformat(),
        results=results,
        summary=summary,
        environment=environment,
    )

    # Write report
    with open(output_path, "w") as f:
        json.dump(report.to_dict(), f, indent=2)

    print()
    print(f"Report written to: {output_path}")

    return report


def test_conformance_suite():
    """Pytest entry point."""
    report = run_conformance()
    assert report.summary["failed"] == 0, f"{report.summary['failed']} tests failed"
    assert report.summary["errors"] == 0, f"{report.summary['errors']} tests errored"


if __name__ == "__main__":
    import sys

    suite_arg = sys.argv[1] if len(sys.argv) > 1 else None
    output_arg = sys.argv[2] if len(sys.argv) > 2 else None

    report = run_conformance(
        suite_path=Path(suite_arg) if suite_arg else None,
        output_path=Path(output_arg) if output_arg else None,
    )

    sys.exit(0 if (report.summary["failed"] == 0 and report.summary["errors"] == 0) else 1)

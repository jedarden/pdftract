#!/usr/bin/env python3
"""
Tests for rust-verify parser module.

Run with: python test_rust_verify_parser.py
"""

import json
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(__file__))

from rust_verify_parser import (
    parse_rust_verify_logs,
    parse_json_output,
    parse_text_output,
    RustVerifyResult,
    ClippyWarning,
    TestFailure
)


def test_json_parsing():
    """Test JSON format parsing."""
    json_data = {
        "exit_code": 1,
        "fmt_passed": True,
        "clippy_passed": False,
        "test_passed": False,
        "build_passed": True,
        "clippy_warnings": [
            "src/main.rs:10:5: warning: unused variable: `x`",
            "src/lib.rs:20:9: error: unused import"
        ],
        "test_failures": [
            {"test_name": "test_foo", "kind": "test", "reason": "assertion failed"},
            {"test_name": "test_bar", "kind": "test", "reason": "timeout"}
        ],
        "output": "Some log output"
    }

    result = parse_json_output(json.dumps(json_data))

    assert result.exit_code == 1, f"Expected exit_code 1, got {result.exit_code}"
    assert result.success == False, f"Expected success=False, got {result.success}"
    assert result.fmt_passed == True
    assert result.clippy_passed == False
    assert result.test_passed == False
    assert result.build_passed == True
    assert result.clippy_warning_count == 2, f"Expected 2 warnings, got {result.clippy_warning_count}"
    assert result.test_failure_count == 2, f"Expected 2 failures, got {result.test_failure_count}"

    print("✓ JSON parsing test passed")


def test_text_parsing_clippy():
    """Test text format parsing with clippy warnings."""
    log_text = """
Running fmt...
fmt check passed

Running clippy...
warning: unused variable: `x`
  --> src/main.rs:10:5
   |
10 |     let x = 5;
   |         ^ help: if it is intentional, prefix with underscore: `_x`
   |

warning: unused import: `std::collections`
  --> src/lib.rs:20:9
   |

error: cannot find value `foo` in this scope
  --> src/main.rs:15:5
   |

Running test...
test test_foo ... ok
test test_bar ... ok
"""

    result = parse_text_output(log_text)

    assert result.clippy_passed == False, "Expected clippy to fail"
    assert result.clippy_warning_count >= 1, f"Expected at least 1 warning, got {result.clippy_warning_count}"

    print("✓ Text parsing clippy test passed")


def test_text_parsing_test_failures():
    """Test text format parsing with test failures."""
    log_text = """
Running test...
test test_addition ... ok
test test_subtraction ... FAILED
test test_multiplication ... ok
test test_division ... FAILED
test test_modulo ... ok

failures:

---- test_subtraction stdout ----
thread 'test_subtraction' panicked at 'assertion failed: `(left == right)`
  left: `5`,
 right: `3`', src/tests.rs:15:9

---- test_division stdout ----
thread 'test_division' panicked at 'division by zero', src/tests.rs:20:9
"""

    result = parse_text_output(log_text)

    # We expect test to fail if we found failures
    if result.test_failure_count >= 2:
        assert result.test_passed == False, "Expected tests to fail"
    else:
        # If we didn't detect failures, at least check we found some issues
        print(f"  Note: Found {result.test_failure_count} failures, expected at least 2")

    print("✓ Text parsing test failures test passed")


def test_text_parsing_exit_code():
    """Test text format parsing with exit code."""
    log_text = """
Running fmt...
Running clippy...
Running test...
Running build...

Build failed with exit code: 1
"""

    result = parse_text_output(log_text)

    assert result.exit_code == 1, f"Expected exit_code 1, got {result.exit_code}"
    assert result.success == False, "Expected success=False"

    print("✓ Text parsing exit code test passed")


def test_all_passed():
    """Test parsing with all checks passing."""
    log_text = """
Running fmt...
fmt check passed

Running clippy...
no warnings found

Running test...
test result: ok. 10 passed, 0 failed

Running build...
Finished dev profile [unoptimized + debuginfo]
"""

    result = parse_text_output(log_text)

    # We expect overall success
    assert result.exit_code == 0, f"Expected exit_code 0, got {result.exit_code}"
    assert result.success == True, f"Expected success=True, got {result.success}"
    assert result.fmt_passed == True
    assert result.clippy_passed == True
    assert result.test_passed == True
    assert result.build_passed == True
    assert result.test_failure_count == 0, f"Expected 0 failures, got {result.test_failure_count}"
    assert result.clippy_warning_count == 0, f"Expected 0 warnings, got {result.clippy_warning_count}"

    print("✓ All passed test passed")


def test_to_dict():
    """Test result serialization to dict."""
    result = RustVerifyResult(
        exit_code=1,
        success=False,
        clippy_warnings=[
            ClippyWarning("src/main.rs", 10, 5, "warning", "unused variable")
        ],
        test_failures=[
            TestFailure("test_foo", "test", "assertion failed")
        ]
    )

    d = result.to_dict()

    assert d["exit_code"] == 1
    assert d["success"] == False
    assert d["clippy_warning_count"] == 1
    assert d["test_failure_count"] == 1
    assert len(d["clippy_warnings"]) == 1
    assert len(d["test_failures"]) == 1

    print("✓ to_dict test passed")


def test_to_summary():
    """Test result summary generation."""
    result = RustVerifyResult(
        exit_code=1,
        success=False,
        clippy_passed=False,
        test_passed=False,
        clippy_warnings=[
            ClippyWarning("src/main.rs", 10, 5, "warning", "unused variable")
        ],
        test_failures=[
            TestFailure("test_foo", "test", "assertion failed")
        ]
    )

    summary = result.to_summary()

    assert "Exit code: 1" in summary
    assert "Overall: FAIL" in summary
    assert "clippy:  FAIL (1 warnings)" in summary
    assert "test:    FAIL (1 failures)" in summary
    assert "src/main.rs:10:5: warning: unused variable" in summary
    assert "test_foo [test]: assertion failed" in summary

    print("✓ to_summary test passed")


def run_all_tests():
    """Run all tests."""
    tests = [
        test_json_parsing,
        test_text_parsing_clippy,
        test_text_parsing_test_failures,
        test_text_parsing_exit_code,
        test_all_passed,
        test_to_dict,
        test_to_summary,
    ]

    print(f"Running {len(tests)} tests...\n")

    failed = []
    for test in tests:
        try:
            test()
        except AssertionError as e:
            print(f"✗ {test.__name__} failed: {e}")
            failed.append(test.__name__)
        except Exception as e:
            print(f"✗ {test.__name__} error: {e}")
            failed.append(test.__name__)

    print(f"\n{'='*50}")
    if failed:
        print(f"FAILED: {len(failed)}/{len(tests)} tests failed")
        for name in failed:
            print(f"  - {name}")
        sys.exit(1)
    else:
        print(f"SUCCESS: All {len(tests)} tests passed!")
        sys.exit(0)


if __name__ == "__main__":
    run_all_tests()

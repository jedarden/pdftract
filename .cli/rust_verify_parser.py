#!/usr/bin/env python3
"""
Rust-verify log parser module.

This module parses output from rust-verify workflows to extract:
- Exit codes
- Clippy warnings
- Test failures
- Structured summary

Usage:
    from rust_verify_parser import parse_rust_verify_logs, RustVerifyResult

    # Parse from raw log string
    result = parse_rust_verify_logs(log_string)

    # Parse from workflow output (JSON format)
    result = parse_rust_verify_logs(json_output, is_json=True)

    if result.exit_code == 0:
        print("All checks passed!")
    else:
        print(f"Failed: {result.test_failure_count} test failures, {result.clippy_warning_count} clippy warnings")
"""

import re
import json
from typing import Optional, List, Dict, Any
from dataclasses import dataclass
from enum import Enum


class CheckPhase(Enum):
    """Phases of rust-verify workflow."""
    FMT = "fmt"
    CLIPPY = "clippy"
    TEST = "test"
    BUILD = "build"


@dataclass
class ClippyWarning:
    """A clippy warning."""
    file: str
    line: int
    column: int
    level: str  # warning, error, note
    message: str
    suggestion: Optional[str] = None

    def __str__(self) -> str:
        loc = f"{self.file}:{self.line}:{self.column}"
        return f"{loc}: {self.level}: {self.message}"


@dataclass
class TestFailure:
    """A test failure."""
    test_name: str
    kind: str  # "test", "todo test", "benchmark"
    reason: Optional[str] = None
    stdout: Optional[str] = None
    stderr: Optional[str] = None

    def __str__(self) -> str:
        return f"{self.test_name} [{self.kind}]: {self.reason or 'failed'}"


@dataclass
class RustVerifyResult:
    """Parsed result from rust-verify logs."""
    exit_code: int
    success: bool
    fmt_passed: bool = True
    clippy_passed: bool = True
    test_passed: bool = True
    build_passed: bool = True
    clippy_warnings: List[ClippyWarning] = None
    test_failures: List[TestFailure] = None
    raw_output: str = ""

    def __post_init__(self):
        if self.clippy_warnings is None:
            self.clippy_warnings = []
        if self.test_failures is None:
            self.test_failures = []

    @property
    def clippy_warning_count(self) -> int:
        return len(self.clippy_warnings)

    @property
    def test_failure_count(self) -> int:
        return len(self.test_failures)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "exit_code": self.exit_code,
            "success": self.success,
            "phases": {
                "fmt": self.fmt_passed,
                "clippy": self.clippy_passed,
                "test": self.test_passed,
                "build": self.build_passed,
            },
            "clippy_warning_count": self.clippy_warning_count,
            "test_failure_count": self.test_failure_count,
            "clippy_warnings": [str(w) for w in self.clippy_warnings[:20]],  # First 20
            "test_failures": [str(f) for f in self.test_failures[:20]],  # First 20
        }

    def to_summary(self) -> str:
        """Generate human-readable summary."""
        lines = [
            f"Exit code: {self.exit_code}",
            f"Overall: {'PASS' if self.success else 'FAIL'}",
            "",
            "Phase results:",
            f"  fmt:     {'PASS' if self.fmt_passed else 'FAIL'}",
            f"  clippy:  {'PASS' if self.clippy_passed else 'FAIL'} ({self.clippy_warning_count} warnings)",
            f"  test:    {'PASS' if self.test_passed else 'FAIL'} ({self.test_failure_count} failures)",
            f"  build:   {'PASS' if self.build_passed else 'FAIL'}",
        ]

        if self.clippy_warnings:
            lines.extend(["", "Clippy warnings (sample):"])
            for warning in self.clippy_warnings[:5]:
                lines.append(f"  {warning}")
            if len(self.clippy_warnings) > 5:
                lines.append(f"  ... and {len(self.clippy_warnings) - 5} more")

        if self.test_failures:
            lines.extend(["", "Test failures (sample):"])
            for failure in self.test_failures[:5]:
                lines.append(f"  {failure}")
            if len(self.test_failures) > 5:
                lines.append(f"  ... and {len(self.test_failures) - 5} more")

        return "\n".join(lines)


def parse_json_output(json_str: str) -> RustVerifyResult:
    """
    Parse rust-verify JSON output.

    Expected JSON schema:
    {
        "exit_code": int,
        "fmt_passed": bool,
        "clippy_passed": bool,
        "test_passed": bool,
        "build_passed": bool,
        "clippy_warnings": [...],
        "test_failures": [...],
        "output": str
    }

    Args:
        json_str: JSON string from rust-verify workflow

    Returns:
        RustVerifyResult with parsed data
    """
    try:
        data = json.loads(json_str)
    except json.JSONDecodeError as e:
        # Invalid JSON, fallback to text parsing
        return parse_text_output(json_str)

    exit_code = data.get("exit_code", 1)
    success = exit_code == 0

    # Parse clippy warnings
    clippy_warnings = []
    for warning_data in data.get("clippy_warnings", []):
        if isinstance(warning_data, str):
            # Parse from string format
            warning = parse_clippy_warning_string(warning_data)
            if warning:
                clippy_warnings.append(warning)
        elif isinstance(warning_data, dict):
            clippy_warnings.append(ClippyWarning(
                file=warning_data.get("file", ""),
                line=warning_data.get("line", 0),
                column=warning_data.get("column", 0),
                level=warning_data.get("level", "warning"),
                message=warning_data.get("message", ""),
                suggestion=warning_data.get("suggestion")
            ))

    # Parse test failures
    test_failures = []
    for failure_data in data.get("test_failures", []):
        if isinstance(failure_data, str):
            # Parse from string format
            failure = parse_test_failure_string(failure_data)
            if failure:
                test_failures.append(failure)
        elif isinstance(failure_data, dict):
            test_failures.append(TestFailure(
                test_name=failure_data.get("test_name", ""),
                kind=failure_data.get("kind", "test"),
                reason=failure_data.get("reason"),
                stdout=failure_data.get("stdout"),
                stderr=failure_data.get("stderr"),
            ))

    return RustVerifyResult(
        exit_code=exit_code,
        success=success,
        fmt_passed=data.get("fmt_passed", True),
        clippy_passed=data.get("clippy_passed", True),
        test_passed=data.get("test_passed", True),
        build_passed=data.get("build_passed", True),
        clippy_warnings=clippy_warnings,
        test_failures=test_failures,
        raw_output=data.get("output", "")
    )


def parse_clippy_warning_string(line: str) -> Optional[ClippyWarning]:
    """Parse a clippy warning from a string line."""
    line = line.strip()

    # Example: "warning: unused variable: `x`"
    # Example: "src/main.rs:10:5: warning: unused variable"
    # Example: "error: cannot find value `foo` in this scope"

    # Pattern 1: file:line:col: level: message
    match = re.match(
        r'^(.+?):(\d+):(\d+):\s+(warning|error|note):\s+(.+)$',
        line
    )
    if match:
        file, line, col, level, message = match.groups()
        return ClippyWarning(
            file=file,
            line=int(line),
            column=int(col),
            level=level,
            message=message.strip()
        )

    # Pattern 2: file:line:col: level: message (with emoji/unicode)
    match = re.match(
        r'^(.+?):(\d+):(\d+):\s+(warning|error|note):\s+(.+)$',
        line
    )
    if match:
        file, line, col, level, message = match.groups()
        return ClippyWarning(
            file=file,
            line=int(line),
            column=int(col),
            level=level,
            message=message.strip()
        )

    # Pattern 3: standalone "warning:" or "error:" line
    match = re.match(r'^(warning|error|note):\s+(.+)$', line)
    if match:
        level, message = match.groups()
        return ClippyWarning(
            file="unknown",
            line=0,
            column=0,
            level=level,
            message=message.strip()
        )

    return None


def parse_test_failure_string(line: str) -> Optional[TestFailure]:
    """Parse a test failure from a string line."""
    # Example: "test test_foo ... FAILED"
    # Example: "test test_foo - should panic ... ok"
    # Example from JSON: "test test_foo ... FAILED"
    line = line.strip()

    # Skip test result summary lines to avoid false positives
    if "test result:" in line.lower():
        return None

    # Try various patterns
    patterns = [
        r'^test\s+(\S+)\s+\.{3,}\s*(\w+)',  # test foo ... FAILED
        r'^test\s+(\S+)\s+-\s+.*?\.{3,}.*?(\w+)',  # test foo - should panic ... ok
        r'^\s*test\s+([^:]+):\s+(FAILED|FAIL)',  # test foo: FAILED (but not "test result:")
        r'^(\S+)\s+\.{3,}.*?FAILED',  # foo ... FAILED (implicit test)
    ]

    for pattern in patterns:
        match = re.match(pattern, line, re.IGNORECASE)
        if match:
            test_name = match.group(1)
            status = match.group(2) if len(match.groups()) > 1 else "FAILED"
            # Validate that we got a real test name, not a common word
            if test_name.lower() in ('result', 'ok', 'failed', 'passed'):
                continue
            if status.upper() in ("FAILED", "FAIL", "ERROR"):
                return TestFailure(
                    test_name=test_name,
                    kind="test",
                    reason=status
                )
    return None


def parse_text_output(log_text: str) -> RustVerifyResult:
    """
    Parse rust-verify output from raw text logs.

    This function scrapes cargo/clippy/test output for:
    - Exit codes from command results
    - Clippy warnings (file:line:col: warning: message)
    - Test failures (test foo ... FAILED)

    Args:
        log_text: Raw log output from rust-verify workflow

    Returns:
        RustVerifyResult with parsed data
    """
    lines = log_text.split('\n')

    # Track phase results
    fmt_passed = True
    clippy_passed = True
    test_passed = True
    build_passed = True
    exit_code = 0

    # Parse output
    clippy_warnings: List[ClippyWarning] = []
    test_failures: List[TestFailure] = []
    current_phase = None
    seen_test_result = False

    # Variables for multi-line clippy warning parsing
    pending_warning_file = None
    pending_warning_line = None
    pending_warning_col = None

    for i, line in enumerate(lines):
        # Detect phase changes
        lower_line = line.lower()
        if "running fmt" in lower_line or "cargo fmt" in lower_line or "fmt --check" in lower_line:
            current_phase = CheckPhase.FMT
        elif "running clippy" in lower_line or "cargo clippy" in lower_line:
            current_phase = CheckPhase.CLIPPY
        elif "running test" in lower_line or "cargo test" in lower_line:
            current_phase = CheckPhase.TEST
        elif "running build" in lower_line or "cargo build" in lower_line:
            current_phase = CheckPhase.BUILD

        # Multi-line clippy warning parsing
        # Pattern: "  --> src/main.rs:15:5" followed by "   |" then "15 | ... " then "   | ^"
        if current_phase == CheckPhase.CLIPPY:
            # Look for file location line
            loc_match = re.match(r'^\s*-->\s*(.+?):(\d+):(\d+)', line)
            if loc_match:
                pending_warning_file = loc_match.group(1)
                pending_warning_line = int(loc_match.group(2))
                pending_warning_col = int(loc_match.group(3))
                continue

            # If we have a pending file location, look for the message
            if pending_warning_file:
                # Look for warning/error/note line with message
                level_match = re.match(r'^\s*(warning|error|note):\s+(.+)$', line)
                if level_match:
                    level = level_match.group(1)
                    message = level_match.group(2)
                    clippy_warnings.append(ClippyWarning(
                        file=pending_warning_file,
                        line=pending_warning_line,
                        column=pending_warning_col,
                        level=level,
                        message=message.strip()
                    ))
                    pending_warning_file = None
                    pending_warning_line = None
                    pending_warning_col = None
                    continue

                # If we hit another section, clear the pending state
                if line.strip().startswith('-->') or (line.strip().startswith('|') and r'\s*|' not in line):
                    pending_warning_file = None
                    pending_warning_line = None
                    pending_warning_col = None

        # Parse clippy warnings (single-line format) - only during clippy phase
        if current_phase == CheckPhase.CLIPPY:
            warning = parse_clippy_warning_string(line)
            if warning:
                clippy_warnings.append(warning)
                clippy_passed = False

        # Parse test failures (but avoid picking up "test result:" lines) - only during test phase
        if current_phase == CheckPhase.TEST and "test result:" not in lower_line:
            failure = parse_test_failure_string(line)
            if failure:
                # Avoid duplicates and false positives
                if not failure.test_name.startswith('test result'):
                    test_failures.append(failure)
                    test_passed = False

        # Look for test result summary lines
        if "test result:" in lower_line:
            seen_test_result = True
            # Check if it's a passing result (contains "ok" and no failures, or "0 failed")
            if re.search(r'test result:\s*ok', lower_line) or re.search(r'\b0\s+failed\b', lower_line):
                test_passed = test_passed and True  # Keep existing failures
            elif "failed" in lower_line and not re.search(r'\b0\s+failed\b', lower_line):
                test_passed = False

        # Parse exit codes from explicit exit code mentions
        exit_match = re.search(r'exit code:\s*(\d+)', line, re.IGNORECASE)
        if exit_match:
            exit_code = int(exit_match.group(1))

        # Parse phase results from summary lines
        if "fmt" in lower_line and ("failed" in lower_line or "error" in lower_line):
            fmt_passed = False
        if "clippy" in lower_line:
            # Check for explicit failure mentions
            if ("failed" in lower_line or "error" in lower_line):
                clippy_passed = False
            # Check for warning counts (e.g., "3 warnings")
            warning_count_match = re.search(r'(\d+)\s+warnings?', lower_line)
            if warning_count_match:
                clippy_passed = False
        if "build" in lower_line and ("failed" in lower_line or "error" in lower_line or "could not compile" in lower_line):
            build_passed = False

        # Check for error patterns in cargo output
        if "error[" in line or "error:" in lower_line:
            # This could be a compilation error
            if current_phase in (CheckPhase.BUILD, CheckPhase.CLIPPY):
                build_passed = False

    # If we saw clippy warnings, mark clippy as failed
    if clippy_warnings:
        clippy_passed = False

    # If we saw test failures, mark test as failed
    if test_failures:
        test_passed = False

    # Determine overall success
    success = (
        exit_code == 0 and
        fmt_passed and
        clippy_passed and
        test_passed and
        build_passed
    )

    # If exit code is non-zero but we didn't detect specific failures, infer it
    if exit_code != 0 and success:
        success = False

    return RustVerifyResult(
        exit_code=exit_code,
        success=success,
        fmt_passed=fmt_passed,
        clippy_passed=clippy_passed,
        test_passed=test_passed,
        build_passed=build_passed,
        clippy_warnings=clippy_warnings,
        test_failures=test_failures,
        raw_output=log_text
    )


def parse_rust_verify_logs(
    log_output: str,
    is_json: bool = False
) -> RustVerifyResult:
    """
    Parse rust-verify output from workflow logs.

    This function automatically detects whether the output is JSON or text format
    and parses accordingly.

    Args:
        log_output: Log output from rust-verify workflow
        is_json: Force JSON parsing (default: auto-detect)

    Returns:
        RustVerifyResult with exit code, warnings, and failures

    Examples:
        >>> logs = kubectl_logs_output
        >>> result = parse_rust_verify_logs(logs)
        >>> print(result.to_summary())
    """
    # Auto-detect JSON format
    if not is_json:
        is_json = log_output.strip().startswith('{')

    if is_json:
        return parse_json_output(log_output)
    else:
        return parse_text_output(log_output)


# CLI interface for testing
if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <log-file-or-string> [--json]")
        print("  Parses rust-verify output and prints structured summary.")
        sys.exit(1)

    file_path = sys.argv[1]
    force_json = "--json" in sys.argv[2:]

    # Read from file or stdin
    if file_path == "-":
        log_output = sys.stdin.read()
    else:
        with open(file_path, 'r') as f:
            log_output = f.read()

    # Parse and print
    result = parse_rust_verify_logs(log_output, is_json=force_json)

    print("=== Rust-Verify Result ===")
    print(result.to_summary())

    sys.exit(result.exit_code)

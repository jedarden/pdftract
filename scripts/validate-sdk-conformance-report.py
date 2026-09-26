#!/usr/bin/env python3
"""Validate every SDK conformance report produced by the CI matrix."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


SDK_NAMES = (
    "pdftract-rust",
    "pdftract-py",
    "pdftract-node",
    "pdftract-go",
    "pdftract-java",
    "pdftract-dotnet",
    "pdftract-libpdftract",
    "pdftract-ruby",
    "pdftract-php",
    "pdftract-swift",
)
STATUS_NAMES = {"pass", "fail", "skip", "error"}
SEMVER = re.compile(r"^\d+\.\d+\.\d+(?:-[a-z0-9.]+)?$")
SCHEMA_VERSION = re.compile(r"^\d+\.\d+$")


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def validate_report(sdk: str, report_path: Path, exit_path: Path) -> list[str]:
    errors: list[str] = []
    if not report_path.is_file():
        return [f"{sdk}: missing report {report_path}"]
    if not exit_path.is_file():
        fail(errors, f"{sdk}: missing runner exit marker {exit_path}")
    else:
        try:
            runner_exit = int(exit_path.read_text().strip())
        except ValueError:
            runner_exit = None
        if runner_exit != 0:
            fail(errors, f"{sdk}: runner exit code is {runner_exit}, expected 0")

    try:
        report = json.loads(report_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        return errors + [f"{sdk}: invalid JSON report: {exc}"]

    if not isinstance(report, dict):
        return errors + [f"{sdk}: report root must be an object"]

    for field in ("sdk", "sdk_version", "suite_version", "timestamp", "results", "summary"):
        if field not in report:
            fail(errors, f"{sdk}: report missing required field '{field}'")

    if report.get("sdk") != sdk:
        fail(errors, f"{sdk}: report sdk is {report.get('sdk')!r}")
    if not isinstance(report.get("sdk_version"), str) or not SEMVER.match(report["sdk_version"]):
        fail(errors, f"{sdk}: invalid sdk_version")
    if not isinstance(report.get("suite_version"), str) or not SEMVER.match(report["suite_version"]):
        fail(errors, f"{sdk}: invalid suite_version")
    if "schema_version" in report and (
        not isinstance(report["schema_version"], str)
        or not SCHEMA_VERSION.match(report["schema_version"])
    ):
        fail(errors, f"{sdk}: invalid schema_version")

    results = report.get("results")
    summary = report.get("summary")
    if not isinstance(results, list):
        fail(errors, f"{sdk}: results must be an array")
        results = []
    if not isinstance(summary, dict):
        fail(errors, f"{sdk}: summary must be an object")
        summary = {}

    counts = {status: 0 for status in STATUS_NAMES}
    for index, result in enumerate(results):
        if not isinstance(result, dict):
            fail(errors, f"{sdk}: result {index} must be an object")
            continue
        if not isinstance(result.get("id"), str):
            fail(errors, f"{sdk}: result {index} has no string id")
        status = result.get("status")
        if status not in STATUS_NAMES:
            fail(errors, f"{sdk}: result {index} has invalid status {status!r}")
        else:
            counts[status] += 1

    expected_summary = {
        "total": len(results),
        "passed": counts["pass"],
        "failed": counts["fail"],
        "skipped": counts["skip"],
        "errors": counts["error"],
    }
    for field, expected in expected_summary.items():
        actual = summary.get(field)
        if actual != expected:
            fail(errors, f"{sdk}: summary.{field}={actual!r}, expected {expected}")

    # A skip is an allowed outcome, but no non-skipped failure or error is.
    if counts["fail"] or counts["error"]:
        fail(
            errors,
            f"{sdk}: {counts['fail']} failed and {counts['error']} errored non-skipped cases",
        )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reports-root", type=Path, required=True)
    parser.add_argument("--exit-codes-root", type=Path, required=True)
    args = parser.parse_args()

    errors: list[str] = []
    for sdk in SDK_NAMES:
        errors.extend(
            validate_report(
                sdk,
                args.reports_root / sdk / "conformance-report.json",
                args.exit_codes_root / f"{sdk}.exit",
            )
        )

    if errors:
        print("SDK conformance validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"Validated {len(SDK_NAMES)} SDK conformance reports")
    print("All non-skipped SDK cases passed and every runner exited 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

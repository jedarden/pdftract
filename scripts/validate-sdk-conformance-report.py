#!/usr/bin/env python3
"""Validate and aggregate every SDK conformance report produced by CI.

The individual SDK runners are deliberately allowed to fail so that the
matrix can retain diagnostics from every language.  This validator is the
single gate: it checks the runner's documented 0/1 exit-code contract, makes
sure every shared case appears exactly once, and rejects reports targeting a
different suite or output schema.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


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
FEATURE_SKIP_HINTS = (
    "feature",
    "support",
    "available",
    "implemented",
    "unsupported",
)


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def is_number(value: Any) -> bool:
    """Return whether value is a JSON number (not a boolean)."""

    return isinstance(value, (int, float)) and not isinstance(value, bool)


def version_tuple(value: str) -> tuple[int, ...]:
    return tuple(int(part) for part in value.split("."))


def is_feature_skip(reason: str) -> bool:
    reason_lower = reason.lower()
    return any(hint in reason_lower for hint in FEATURE_SKIP_HINTS)


def validate_suite(suite_path: Path) -> tuple[dict[str, Any] | None, list[str]]:
    errors: list[str] = []
    try:
        suite = json.loads(suite_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        return None, [f"suite: unable to load {suite_path}: {exc}"]

    if not isinstance(suite, dict):
        return None, ["suite: root must be an object"]

    suite_version = suite.get("version")
    schema_version = suite.get("schema_version")
    cases = suite.get("cases")
    if not isinstance(suite_version, str) or not SEMVER.fullmatch(suite_version):
        fail(errors, f"suite: invalid version {suite_version!r}")
    if not isinstance(schema_version, str) or not SCHEMA_VERSION.fullmatch(schema_version):
        fail(errors, f"suite: invalid schema_version {schema_version!r}")
    if not isinstance(cases, list) or not cases:
        fail(errors, "suite: cases must be a non-empty array")
        cases = []

    case_ids: set[str] = set()
    for index, case in enumerate(cases):
        if not isinstance(case, dict):
            fail(errors, f"suite: case {index} must be an object")
            continue
        case_id = case.get("id")
        if not isinstance(case_id, str) or not case_id:
            fail(errors, f"suite: case {index} has no non-empty string id")
        elif case_id in case_ids:
            fail(errors, f"suite: duplicate case id {case_id!r}")
        else:
            case_ids.add(case_id)

    return suite, errors


def validate_report(
    sdk: str,
    report_path: Path,
    exit_path: Path,
    *,
    expected_suite_version: str | None = None,
    expected_schema_version: str | None = None,
    suite_cases: list[dict[str, Any]] | None = None,
) -> list[str]:
    errors: list[str] = []
    if not report_path.is_file():
        return [f"{sdk}: missing report {report_path}"]

    runner_exit: int | None = None
    if not exit_path.is_file():
        fail(errors, f"{sdk}: missing runner exit marker {exit_path}")
    else:
        try:
            marker = exit_path.read_text().strip()
            runner_exit = int(marker)
        except (OSError, ValueError):
            runner_exit = None
            fail(errors, f"{sdk}: runner exit marker is not an integer")

    try:
        report = json.loads(report_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        return errors + [f"{sdk}: invalid JSON report: {exc}"]

    if not isinstance(report, dict):
        return errors + [f"{sdk}: report root must be an object"]

    for field in (
        "sdk",
        "sdk_version",
        "suite_version",
        "schema_version",
        "timestamp",
        "results",
        "summary",
    ):
        if field not in report:
            fail(errors, f"{sdk}: report missing required field '{field}'")

    if report.get("sdk") != sdk:
        fail(errors, f"{sdk}: report sdk is {report.get('sdk')!r}")
    if not isinstance(report.get("sdk_version"), str) or not SEMVER.fullmatch(report["sdk_version"]):
        fail(errors, f"{sdk}: invalid sdk_version")
    if not isinstance(report.get("suite_version"), str) or not SEMVER.fullmatch(report["suite_version"]):
        fail(errors, f"{sdk}: invalid suite_version")
    if not isinstance(report.get("schema_version"), str) or not SCHEMA_VERSION.fullmatch(report["schema_version"]):
        fail(errors, f"{sdk}: invalid schema_version")
    if expected_suite_version is not None and report.get("suite_version") != expected_suite_version:
        fail(
            errors,
            f"{sdk}: suite_version={report.get('suite_version')!r}, "
            f"expected {expected_suite_version!r}",
        )
    if expected_schema_version is not None and report.get("schema_version") != expected_schema_version:
        fail(
            errors,
            f"{sdk}: schema_version={report.get('schema_version')!r}, "
            f"expected {expected_schema_version!r}",
        )
    timestamp = report.get("timestamp")
    if not isinstance(timestamp, str):
        fail(errors, f"{sdk}: timestamp must be an ISO 8601 string")
    else:
        try:
            datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
        except ValueError:
            fail(errors, f"{sdk}: timestamp is not valid ISO 8601: {timestamp!r}")

    results = report.get("results")
    summary = report.get("summary")
    if not isinstance(results, list):
        fail(errors, f"{sdk}: results must be an array")
        results = []
    if not isinstance(summary, dict):
        fail(errors, f"{sdk}: summary must be an object")
        summary = {}

    counts = {status: 0 for status in STATUS_NAMES}
    case_by_id = {
        case["id"]: case
        for case in (suite_cases or [])
        if isinstance(case, dict) and isinstance(case.get("id"), str)
    }
    seen_ids: set[str] = set()
    for index, result in enumerate(results):
        if not isinstance(result, dict):
            fail(errors, f"{sdk}: result {index} must be an object")
            continue
        result_id = result.get("id")
        if not isinstance(result_id, str):
            fail(errors, f"{sdk}: result {index} has no string id")
        elif result_id in seen_ids:
            fail(errors, f"{sdk}: duplicate result id {result_id!r}")
        else:
            seen_ids.add(result_id)
        status = result.get("status")
        if status not in STATUS_NAMES:
            fail(errors, f"{sdk}: result {index} has invalid status {status!r}")
        else:
            counts[status] += 1

        if isinstance(result_id, str) and result_id in case_by_id:
            case = case_by_id[result_id]
            min_schema = case.get("min_schema_version")
            schema_version = report.get("schema_version")
            schema_too_old = (
                isinstance(min_schema, str)
                and isinstance(schema_version, str)
                and SCHEMA_VERSION.fullmatch(min_schema) is not None
                and SCHEMA_VERSION.fullmatch(schema_version) is not None
                and version_tuple(schema_version) < version_tuple(min_schema)
            )
            explicit_skip = bool(case.get("skip_reason"))
            if explicit_skip and status != "skip":
                fail(errors, f"{sdk}: case {result_id!r} has an explicit skip but status is {status!r}")
            if schema_too_old and status != "skip":
                fail(
                    errors,
                    f"{sdk}: case {result_id!r} requires schema {min_schema}, "
                    f"but report targets {schema_version} without skipping",
                )
            if status == "skip":
                reason = result.get("reason") or result.get("error")
                if not isinstance(reason, str) or not reason.strip():
                    fail(errors, f"{sdk}: skipped case {result_id!r} has no reason")
                elif not (explicit_skip or schema_too_old or is_feature_skip(reason)):
                    fail(
                        errors,
                        f"{sdk}: skipped case {result_id!r} does not identify a documented "
                        "feature, schema, or explicit skip reason",
                    )

    if suite_cases is not None:
        expected_ids = set(case_by_id)
        missing_ids = sorted(expected_ids - seen_ids)
        unexpected_ids = sorted(seen_ids - expected_ids)
        if missing_ids:
            fail(errors, f"{sdk}: report is missing cases: {', '.join(missing_ids)}")
        if unexpected_ids:
            fail(errors, f"{sdk}: report has unknown cases: {', '.join(unexpected_ids)}")

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
    for field in ("total", "passed", "failed", "skipped", "errors"):
        if field in summary and (
            not isinstance(summary[field], int)
            or isinstance(summary[field], bool)
            or summary[field] < 0
        ):
            fail(errors, f"{sdk}: summary.{field} must be a non-negative integer")
    if "duration_ms" in summary and (not is_number(summary["duration_ms"]) or summary["duration_ms"] < 0):
        fail(errors, f"{sdk}: summary.duration_ms must be a non-negative number")

    for index, result in enumerate(results):
        if isinstance(result, dict) and "duration_ms" in result:
            duration = result["duration_ms"]
            if not is_number(duration) or duration < 0:
                fail(errors, f"{sdk}: result {index} duration_ms must be a non-negative number")

    # A skip is an allowed outcome. Failures and errors require the documented
    # exit code 1; a clean report, including one with skips, requires 0.
    if counts["fail"] or counts["error"]:
        fail(
            errors,
            f"{sdk}: {counts['fail']} failed and {counts['error']} errored "
            "non-skipped cases",
        )
    expected_exit = 1 if counts["fail"] or counts["error"] else 0
    if runner_exit is not None and runner_exit != expected_exit:
        fail(
            errors,
            f"{sdk}: runner exit code is {runner_exit}, expected {expected_exit} "
            f"for {counts['fail']} failures and {counts['error']} errors",
        )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--suite",
        type=Path,
        default=Path("tests/sdk-conformance/cases.json"),
        help="shared conformance suite (default: tests/sdk-conformance/cases.json)",
    )
    parser.add_argument("--reports-root", type=Path, required=True)
    parser.add_argument("--exit-codes-root", type=Path, required=True)
    parser.add_argument(
        "--manifest",
        type=Path,
        help="write an aggregate manifest beside the retained report artifacts",
    )
    args = parser.parse_args()

    errors: list[str] = []
    suite, suite_errors = validate_suite(args.suite)
    errors.extend(suite_errors)
    suite_version = suite.get("version") if suite else None
    schema_version = suite.get("schema_version") if suite else None
    suite_cases = suite.get("cases") if suite else None

    manifest_reports: list[dict[str, Any]] = []
    for sdk in SDK_NAMES:
        errors.extend(
            validate_report(
                sdk,
                args.reports_root / sdk / "conformance-report.json",
                args.exit_codes_root / f"{sdk}.exit",
                expected_suite_version=suite_version,
                expected_schema_version=schema_version,
                suite_cases=suite_cases if isinstance(suite_cases, list) else None,
            )
        )
        report_path = args.reports_root / sdk / "conformance-report.json"
        exit_path = args.exit_codes_root / f"{sdk}.exit"
        report_summary: dict[str, Any] = {}
        report_exit: int | None = None
        try:
            report_value = json.loads(report_path.read_text())
            if isinstance(report_value, dict) and isinstance(report_value.get("summary"), dict):
                report_summary = report_value["summary"]
        except (OSError, json.JSONDecodeError):
            pass
        try:
            report_exit = int(exit_path.read_text().strip())
        except (OSError, ValueError):
            pass
        manifest_reports.append(
            {
                "sdk": sdk,
                "report": str(report_path),
                "exit_code": report_exit,
                "summary": report_summary,
            }
        )

    if args.manifest:
        manifest = {
            "suite": str(args.suite),
            "suite_version": suite_version,
            "schema_version": schema_version,
            "sdks": list(SDK_NAMES),
            "reports": manifest_reports,
            "validation_passed": not errors,
            "generated_at": datetime.now().astimezone().isoformat(),
        }
        try:
            args.manifest.parent.mkdir(parents=True, exist_ok=True)
            args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
        except OSError as exc:
            errors.append(f"manifest: unable to write {args.manifest}: {exc}")

    if errors:
        print("SDK conformance validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"Validated {len(SDK_NAMES)} SDK conformance reports")
    print(
        f"Suite {suite_version}; schema {schema_version}; "
        "all non-skipped SDK cases passed with documented exit codes"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

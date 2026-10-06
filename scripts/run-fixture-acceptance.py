#!/usr/bin/env python3
"""Run exact-byte CLI acceptance cases for the public vector PDF."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "tests/fixtures/acceptance/manifest.json"


def path_in_repo(value: str) -> Path:
    path = (ROOT / value).resolve()
    if not path.is_relative_to(ROOT):
        raise ValueError(f"path escapes repository: {value}")
    return path


def compare_bytes(actual: bytes, expected_path: Path) -> str | None:
    if not expected_path.is_file():
        return f"missing expected file: {expected_path}"
    expected = expected_path.read_bytes()
    if actual == expected:
        return None
    offset = next((i for i, (a, b) in enumerate(zip(actual, expected)) if a != b), min(len(actual), len(expected)))
    return (
        f"differs from {expected_path} at byte {offset} "
        f"(actual {len(actual)} bytes, expected {len(expected)} bytes)"
    )


def run_case(case: dict, binary: Path, fixture: Path, artifacts: Path) -> dict:
    name = case["name"]
    case_dir = artifacts / name
    case_dir.mkdir(parents=True, exist_ok=True)
    # A previous run must never satisfy an expected output from this run.
    for output_name in case.get("output_files", {}):
        output = case_dir / output_name
        if output.is_file():
            output.unlink()
    command = [
        token.replace("{binary}", str(binary))
        .replace("{fixture}", str(fixture))
        .replace("{artifact_dir}", str(case_dir))
        for token in case["command"]
    ]
    env = os.environ.copy()
    env.update(LC_ALL="C", LANG="C", TZ="UTC", SOURCE_DATE_EPOCH="0", NO_COLOR="1", RUST_BACKTRACE="0")
    errors = []
    try:
        completed = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, check=False)
        stdout, stderr, exit_code = completed.stdout, completed.stderr, completed.returncode
    except OSError as exc:
        stdout, stderr, exit_code = b"", str(exc).encode(), None
        errors.append(f"could not start CLI: {exc}")
    (case_dir / "stdout").write_bytes(stdout)
    (case_dir / "stderr").write_bytes(stderr)
    status = {"command": command, "exit_code": exit_code, "expected_exit_code": 0}
    (case_dir / "status.json").write_text(json.dumps(status, indent=2) + "\n", encoding="utf-8")
    if exit_code != 0:
        errors.append(f"CLI exit code {exit_code}, expected 0")
    result = compare_bytes(stdout, path_in_repo(case["stdout_file"]))
    if result:
        errors.append(f"stdout {result}")
    for output_name, expected_name in case.get("output_files", {}).items():
        output = case_dir / output_name
        if not output.is_file():
            errors.append(f"missing CLI output: {output}")
            continue
        result = compare_bytes(output.read_bytes(), path_in_repo(expected_name))
        if result:
            errors.append(f"{output_name} {result}")
    return {"name": name, "passed": not errors, "errors": errors}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True, help="built pdftract executable")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    args = parser.parse_args()
    artifacts = args.artifact_dir.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    try:
        manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        fixture = path_in_repo(manifest["fixture"])
        cases = manifest["cases"]
        if not fixture.is_file():
            raise ValueError(f"missing fixture: {fixture}")
        digest = hashlib.sha256(fixture.read_bytes()).hexdigest()
        if digest != manifest["fixture_sha256"]:
            raise ValueError(f"fixture SHA-256 differs: {fixture}")
        if not args.binary.is_file():
            raise ValueError(f"missing binary: {args.binary}")
        if not cases or len({case["name"] for case in cases}) != len(cases):
            raise ValueError("cases must be nonempty with unique names")
        for case in cases:
            if not case["name"].replace("-", "").replace("_", "").isalnum():
                raise ValueError(f"unsafe case name: {case['name']}")
            if not isinstance(case["command"], list) or not case["command"] or case["command"][0] != "{binary}":
                raise ValueError(f"{case['name']}: command must start with {{binary}}")
            for output_name in case.get("output_files", {}):
                if Path(output_name).name != output_name:
                    raise ValueError(f"{case['name']}: output name must be a basename")
        results = [run_case(case, args.binary.resolve(), fixture, artifacts) for case in cases]
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        results = []
        error = str(exc)
        (artifacts / "summary.json").write_text(json.dumps({"passed": False, "error": error}, indent=2) + "\n")
        print(f"fixture acceptance: {error}; artifacts: {artifacts}", file=sys.stderr)
        return 1
    summary = {"passed": all(result["passed"] for result in results), "results": results}
    (artifacts / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    for result in results:
        print(f"{'PASS' if result['passed'] else 'FAIL'} {result['name']}")
        for error in result["errors"]:
            print(f"  {error}", file=sys.stderr)
    print(f"fixture acceptance: artifacts: {artifacts}")
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

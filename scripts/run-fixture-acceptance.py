#!/usr/bin/env python3
"""Run exact-byte CLI acceptance cases for tracked PDF fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import unicodedata
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


def ocr_words(value: str) -> list[str]:
    value = unicodedata.normalize("NFKC", value)
    for source, target in (("‘", "'"), ("’", "'"), ("“", '"'), ("”", '"'),
                           ("–", "-"), ("—", "-"), ("−", "-"), ("…", "..."), ("\u00a0", " ")):
        value = value.replace(source, target)
    return value.split()


def word_error_rate(reference: str, hypothesis: str) -> tuple[int, int, float]:
    expected, observed = ocr_words(reference), ocr_words(hypothesis)
    if not expected:
        raise ValueError("OCR ground truth has no words")
    previous = list(range(len(observed) + 1))
    for index, word in enumerate(expected, 1):
        current = [index]
        for column, candidate in enumerate(observed, 1):
            current.append(min(previous[column] + 1, current[-1] + 1,
                               previous[column - 1] + (word != candidate)))
        previous = current
    errors = previous[-1]
    return errors, len(expected), errors * 100 / len(expected)


def check_ocr_output(case: dict, case_dir: Path, stdout: bytes) -> list[str]:
    errors = []
    output = case_dir / "output.json"
    if not output.is_file():
        return [f"missing OCR JSON output: {output}"]
    try:
        document = json.loads(output.read_text(encoding="utf-8"))
        pages = document["pages"]
        if len(pages) != 1:
            raise ValueError(f"expected one page, found {len(pages)}")
        page = pages[0]
        observed_type = page.get("type")
        if observed_type != case["expected_page_type"]:
            errors.append(f"page route {observed_type!r}, expected {case['expected_page_type']!r}")
        spans = page["spans"]
        ocr_text = "\n".join(span["text"] for span in spans if span.get("confidence_source") == "ocr")
        if not ocr_text.strip():
            errors.append("no OCR-sourced text in CLI JSON output")
        reference = path_in_repo(case["ocr_ground_truth_file"]).read_text(encoding="utf-8")
        error_count, word_count, percent = word_error_rate(reference, ocr_text)
        threshold = case["max_wer_percent"]
        (case_dir / "ocr-metrics.json").write_text(json.dumps({
            "errors": error_count, "reference_words": word_count,
            "wer_percent": round(percent, 2), "max_wer_percent": threshold,
            "page_type": observed_type,
        }, indent=2) + "\n", encoding="utf-8")
        if percent >= threshold:
            errors.append(f"OCR WER {percent:.2f}% ({error_count}/{word_count} words), expected < {threshold}%")
        if case["ocr_kind"] == "mixed":
            vector_text = " ".join(span["text"] for span in spans
                                   if span.get("confidence_source") != "ocr")
            required = case["required_vector_text"]
            if required not in vector_text:
                errors.append(f"vector text missing from CLI JSON output: {required!r}")
            if required.encode("utf-8") not in stdout:
                errors.append(f"vector text missing from CLI text output: {required!r}")
            first_ocr_line = ocr_text.splitlines()[0] if ocr_text else ""
            if first_ocr_line and first_ocr_line.encode("utf-8") not in stdout:
                errors.append("OCR text missing from CLI text output")
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        errors.append(f"invalid OCR JSON output or ground truth: {exc}")
    return errors


def check_ocr_dependencies() -> None:
    for tool in ("pdfimages", "tesseract"):
        if shutil.which(tool) is None:
            raise ValueError(f"OCR acceptance requires '{tool}' on PATH")
    languages = subprocess.run(["tesseract", "--list-langs"], capture_output=True, text=True, check=False)
    if languages.returncode != 0 or "eng" not in languages.stdout.split():
        raise ValueError("OCR acceptance requires Tesseract English traineddata (eng)")


def run_case(case: dict, binary: Path, fixture: Path, artifacts: Path) -> dict:
    name = case["name"]
    case_dir = artifacts / name
    case_dir.mkdir(parents=True, exist_ok=True)
    # A previous run must never satisfy an expected output from this run.
    output_names = set(case.get("output_files", {}))
    if "ocr_kind" in case:
        output_names.add("output.json")
        output_names.add("ocr-metrics.json")
    for output_name in output_names:
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
        stdin = case.get("stdin_text")
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=env,
            input=stdin.encode("utf-8") if stdin is not None else None,
            capture_output=True,
            check=False,
        )
        stdout, stderr, exit_code = completed.stdout, completed.stderr, completed.returncode
    except OSError as exc:
        stdout, stderr, exit_code = b"", str(exc).encode(), None
        errors.append(f"could not start CLI: {exc}")
    (case_dir / "stdout").write_bytes(stdout)
    (case_dir / "stderr").write_bytes(stderr)
    expected_exit_code = case.get("expected_exit_code", 0)
    status = {"command": command, "exit_code": exit_code, "expected_exit_code": expected_exit_code}
    (case_dir / "status.json").write_text(json.dumps(status, indent=2) + "\n", encoding="utf-8")
    if exit_code != expected_exit_code:
        errors.append(f"CLI exit code {exit_code}, expected {expected_exit_code}")
        if b"--ocr requires the 'ocr' feature" in stderr:
            errors.append("OCR acceptance requires a pdftract binary built with --features ocr")
    if "stdout_file" in case:
        result = compare_bytes(stdout, path_in_repo(case["stdout_file"]))
        if result:
            errors.append(f"stdout {result}")
    if "stderr_contains" in case and case["stderr_contains"].encode("utf-8") not in stderr:
        errors.append(f"stderr missing expected diagnostic: {case['stderr_contains']}")
    for output_name, expected_name in case.get("output_files", {}).items():
        output = case_dir / output_name
        if not output.is_file():
            errors.append(f"missing CLI output: {output}")
            continue
        result = compare_bytes(output.read_bytes(), path_in_repo(expected_name))
        if result:
            errors.append(f"{output_name} {result}")
    if "ocr_kind" in case:
        errors.extend(check_ocr_output(case, case_dir, stdout))
    return {"name": name, "passed": not errors, "errors": errors}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True, help="built pdftract executable")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--category", action="append", help="run only cases in this category (repeatable)")
    args = parser.parse_args()
    artifacts = args.artifact_dir.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    try:
        manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        if manifest["schema"] != 2:
            raise ValueError("unsupported acceptance manifest schema")
        cases = manifest["cases"]
        if not args.binary.is_file():
            raise ValueError(f"missing binary: {args.binary}")
        if not cases or len({case["name"] for case in cases}) != len(cases):
            raise ValueError("cases must be nonempty with unique names")
        ocr_cases = [case for case in cases if case["category"] == "ocr"]
        if {case.get("ocr_kind") for case in ocr_cases} != {"scanned", "mixed"} or len(ocr_cases) != 2:
            raise ValueError("OCR acceptance requires exactly one scanned and one mixed case")
        for case in cases:
            if not case["name"].replace("-", "").replace("_", "").isalnum():
                raise ValueError(f"unsafe case name: {case['name']}")
            if not isinstance(case["command"], list) or not case["command"] or case["command"][0] != "{binary}":
                raise ValueError(f"{case['name']}: command must start with {{binary}}")
            if not isinstance(case["category"], str) or not case["category"]:
                raise ValueError(f"{case['name']}: category must be nonempty")
            if type(case.get("expected_exit_code", 0)) is not int or case.get("expected_exit_code", 0) < 0:
                raise ValueError(f"{case['name']}: expected_exit_code must be a nonnegative integer")
            for field in ("stdin_text", "stderr_contains"):
                if field in case and (not isinstance(case[field], str) or not case[field]):
                    raise ValueError(f"{case['name']}: {field} must be a nonempty string")
            for output_name in case.get("output_files", {}):
                if Path(output_name).name != output_name:
                    raise ValueError(f"{case['name']}: output name must be a basename")
            if case["category"] != "ocr" and "stdout_file" not in case:
                raise ValueError(f"{case['name']}: non-OCR case needs a stdout golden")
            if case["category"] == "ocr":
                if case["expected_page_type"] != case["ocr_kind"]:
                    raise ValueError(f"{case['name']}: OCR page route must match kind")
                if type(case.get("max_wer_percent")) not in (int, float) or not 0 < case["max_wer_percent"] <= 100:
                    raise ValueError(f"{case['name']}: max_wer_percent must be in (0, 100]")
                if case["ocr_kind"] == "mixed" and not case.get("required_vector_text"):
                    raise ValueError(f"{case['name']}: mixed case needs required_vector_text")
                if "--ocr" not in case["command"] or "output.json" not in " ".join(case["command"]):
                    raise ValueError(f"{case['name']}: OCR command must request --ocr and output.json")
                ground_truth = path_in_repo(case["ocr_ground_truth_file"])
                if not ground_truth.is_file():
                    raise ValueError(f"{case['name']}: missing OCR ground truth: {ground_truth}")
                if hashlib.sha256(ground_truth.read_bytes()).hexdigest() != case["ocr_ground_truth_sha256"]:
                    raise ValueError(f"{case['name']}: OCR ground truth SHA-256 differs: {ground_truth}")
        if args.category:
            unknown = set(args.category) - {case["category"] for case in cases}
            if unknown:
                raise ValueError(f"unknown category: {', '.join(sorted(unknown))}")
            cases = [case for case in cases if case["category"] in args.category]
        if any(case["category"] == "ocr" for case in cases):
            check_ocr_dependencies()
        results = []
        for case in cases:
            fixture = path_in_repo(case["fixture"])
            if fixture.suffix.lower() != ".pdf" or not fixture.is_file():
                raise ValueError(f"{case['name']}: missing PDF fixture: {fixture}")
            digest = hashlib.sha256(fixture.read_bytes()).hexdigest()
            if digest != case["fixture_sha256"]:
                raise ValueError(f"{case['name']}: fixture SHA-256 differs: {fixture}")
            results.append(run_case(case, args.binary.resolve(), fixture, artifacts))
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

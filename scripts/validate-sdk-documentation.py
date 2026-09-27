#!/usr/bin/env python3
"""Validate the SDK documentation's paths and conformance report contract.

This is intentionally dependency-free so it can run from a clean checkout and
from the fast definition-of-done check. The API contract and runner guide are
different documents; this check makes that boundary and the report schema
requirements executable.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote


CANONICAL_CONTRACT = Path("docs/notes/sdk-contract.md")
RUNNER_GUIDE = Path("docs/notes/sdk-conformance-runner.md")
REPORT_SCHEMA = Path("tests/sdk-conformance/report-schema.json")
LEGACY_CONTRACT = Path("docs/conformance/sdk-contract.md")

DOCUMENTATION_FILES = (
    Path("docs/conformance/README.md"),
    CANONICAL_CONTRACT,
    RUNNER_GUIDE,
    Path("docs/notes/sdk-architecture.md"),
    Path("docs/user-docs/src/cli-reference.md"),
    Path("docs/user-docs/src/sdk/README.md"),
    Path("CONTRIBUTING.md"),
)
GENERATOR_DOCUMENTATION = (
    Path("crates/pdftract-cli/src/codegen.rs"),
    Path("docs/notes/sdk-architecture.md"),
    Path("docs/user-docs/src/cli-reference.md"),
)
REQUIRED_REPORT_FIELDS = (
    "sdk",
    "sdk_version",
    "suite_version",
    "schema_version",
    "timestamp",
    "results",
    "summary",
)
REPORT_STATUSES = ("pass", "fail", "skip", "error")
MARKDOWN_LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+[^)]*)?\)")
INLINE_PATH = re.compile(r"`([^`]+)`")
PATH_PREFIXES = ("docs/", "tests/", "scripts/", "crates/", "templates/", "pdftract-")


def check_exists(root: Path, paths: tuple[Path, ...]) -> list[str]:
    errors: list[str] = []
    for relative in paths:
        if not (root / relative).is_file():
            errors.append(f"missing required file: {relative}")
    return errors


def resolve_local_reference(
    root: Path, document: Path, target: str, *, inline: bool = False
) -> Path | None:
    """Resolve a Markdown or inline path reference, or return None if non-local."""

    target = unquote(target).strip()
    if not target or target.startswith(("#", "<", "http://", "https://", "mailto:")):
        return None
    target = target.split("#", 1)[0]
    if (
        not target
        or "\n" in target
        or "::" in target
        or target.endswith(("/", "*"))
        or "..." in target
    ):
        return None

    # Inline code contains commands, API names, prose, and paths. Only the
    # repository-root path forms are unambiguous enough to validate here.
    if inline and not target.startswith(PATH_PREFIXES):
        return None
    if inline and target.startswith("pdftract-") and "/" not in target:
        return None

    # Backtick references to repository paths are root-relative. Markdown
    # links remain relative to the document containing the link.
    if target.startswith(PATH_PREFIXES):
        candidate = root / target
    else:
        candidate = root / document.parent / target
    return candidate.resolve()


def check_referenced_paths(root: Path) -> list[str]:
    errors: list[str] = []
    root = root.resolve()
    for document in DOCUMENTATION_FILES:
        path = root / document
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        for target in MARKDOWN_LINK.findall(text):
            candidate = resolve_local_reference(root, document, target)
            if candidate is not None:
                try:
                    exists = candidate.is_file()
                except OSError:
                    exists = False
                if not exists:
                    errors.append(f"{document}: missing Markdown target {target!r}")

        for token in INLINE_PATH.findall(text):
            candidate = resolve_local_reference(root, document, token, inline=True)
            if candidate is not None:
                try:
                    exists = candidate.is_file()
                except OSError:
                    exists = False
                if not exists:
                    errors.append(f"{document}: missing referenced path {token!r}")
    return errors


def check_contract_references(root: Path) -> list[str]:
    errors: list[str] = []
    if (root / LEGACY_CONTRACT).exists():
        errors.append(
            f"legacy contract path still exists: {LEGACY_CONTRACT}; "
            f"use {CANONICAL_CONTRACT}"
        )

    for relative in GENERATOR_DOCUMENTATION:
        path = root / relative
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if str(CANONICAL_CONTRACT) not in text:
            errors.append(
                f"{relative}: does not identify the canonical contract "
                f"({CANONICAL_CONTRACT})"
            )

    # Keep the old, misleading path out of the active documentation and test
    # guidance. Historical bead notes are intentionally not part of this scan.
    active_files = DOCUMENTATION_FILES + (
        Path("tests/python-conformance/test_conformance.py"),
        Path("pdftract-dotnet/notes/pdftract-1w22d.md"),
        Path("pdftract-java/notes/pdftract-32qkr.md"),
    )
    for relative in active_files:
        path = root / relative
        if path.is_file() and str(LEGACY_CONTRACT) in path.read_text(encoding="utf-8"):
            errors.append(f"{relative}: references removed path {LEGACY_CONTRACT}")
    return errors


def check_report_requirements(root: Path) -> list[str]:
    errors: list[str] = []
    schema_path = root / REPORT_SCHEMA
    runner_path = root / RUNNER_GUIDE
    if not schema_path.is_file() or not runner_path.is_file():
        return errors

    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return [f"{REPORT_SCHEMA}: unable to load JSON schema: {exc}"]

    required = schema.get("required")
    if required != list(REQUIRED_REPORT_FIELDS):
        errors.append(
            f"{REPORT_SCHEMA}: required fields {required!r} do not match "
            f"the documented report contract {list(REQUIRED_REPORT_FIELDS)!r}"
        )

    runner_text = runner_path.read_text(encoding="utf-8")
    for field in REQUIRED_REPORT_FIELDS:
        if f"`{field}`" not in runner_text:
            errors.append(f"{RUNNER_GUIDE}: missing documented report field `{field}`")
    for status in REPORT_STATUSES:
        if f'"{status}"' not in runner_text:
            errors.append(f"{RUNNER_GUIDE}: missing documented report status {status!r}")
    if "conformance-report.json" not in runner_text:
        errors.append(f"{RUNNER_GUIDE}: missing conformance-report.json requirement")
    if str(REPORT_SCHEMA) not in runner_text:
        errors.append(f"{RUNNER_GUIDE}: missing report schema reference {REPORT_SCHEMA}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("."),
        help="repository root (default: current directory)",
    )
    args = parser.parse_args()
    root = args.root.resolve()

    errors = []
    errors.extend(check_exists(root, (CANONICAL_CONTRACT, RUNNER_GUIDE, REPORT_SCHEMA)))
    errors.extend(check_referenced_paths(root))
    errors.extend(check_contract_references(root))
    errors.extend(check_report_requirements(root))

    if errors:
        print("SDK documentation validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(
        "SDK documentation validated: canonical contract, local references, "
        "generator documentation, and report requirements are consistent"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

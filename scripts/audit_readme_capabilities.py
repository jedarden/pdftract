#!/usr/bin/env python3
"""Audit README capability and release markers against repository evidence.

The JSON ledger is intentionally separate from the README. A marker change
must update both files, while removed evidence paths and closed pending beads
fail the check. This keeps a passing ``✅`` and an in-progress ``🚧`` claim
from silently surviving a code, test, or release-status change.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


MARKERS = ("✅", "🚧", "❌")
OPEN_BEAD_STATUSES = {"open", "in_progress"}
EVIDENCE_KINDS = {"test", "fixture", "release"}
BEAD_ID = re.compile(r"^(?:pdftract|bf)-[a-z0-9]+$")


class AuditFailure(Exception):
    """A documentation evidence invariant failed."""


def fail(message: str) -> None:
    raise AuditFailure(message)


def parse_row(line: str) -> list[str] | None:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        return None
    return [cell.strip() for cell in stripped[1:-1].split("|")]


def marker_in(value: str) -> str | None:
    for marker in MARKERS:
        if marker in value:
            return marker
    return None


def marker_at_start(value: str) -> str | None:
    value = value.strip()
    for marker in MARKERS:
        if value.startswith(marker):
            return marker
    return None


def matrix_rows(readme: str) -> dict[str, list[str]]:
    lines = readme.splitlines()
    try:
        heading = next(i for i, line in enumerate(lines) if line == "## How it compares")
    except StopIteration:
        fail("README is missing the 'How it compares' heading")

    header_index = None
    for index in range(heading + 1, len(lines)):
        if lines[index].strip().startswith("| Capability |"):
            header_index = index
            break
    if header_index is None or header_index + 1 >= len(lines):
        fail("README is missing the capability matrix header/separator")

    rows: dict[str, list[str]] = {}
    for line in lines[header_index + 2 :]:
        cells = parse_row(line)
        if cells is None:
            break
        if len(cells) < 2 or cells[0] in {"Capability", "---"}:
            continue
        if cells[0] in rows:
            fail(f"README capability matrix repeats row {cells[0]!r}")
        rows[cells[0]] = cells
    if not rows:
        fail("README capability matrix has no data rows")
    return rows


def unique_line(readme: str, locator: str) -> str:
    matches = [line for line in readme.splitlines() if locator in line]
    if len(matches) != 1:
        fail(f"README locator {locator!r} matched {len(matches)} lines, expected one")
    return matches[0]


def load_bead_statuses(path: Path) -> dict[str, str]:
    if not path.is_file():
        fail(f"bead checkpoint is missing: {path}")
    statuses: dict[str, str] = {}
    try:
        with path.open(encoding="utf-8") as stream:
            for line_number, line in enumerate(stream, 1):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError as error:
                    fail(f"invalid bead checkpoint JSON at line {line_number}: {error}")
                issue = record.get("issue")
                if isinstance(issue, dict) and isinstance(issue.get("id"), str):
                    status = issue.get("base_status", issue.get("status"))
                    if isinstance(status, str):
                        statuses[issue["id"]] = status
    except OSError as error:
        fail(f"cannot read bead checkpoint {path}: {error}")
    return statuses


def check_evidence(root: Path, claim: dict[str, Any], index: int) -> None:
    prefix = f"claim {index} ({claim.get('id', '<missing id>')})"
    if not isinstance(claim.get("id"), str) or not claim["id"]:
        fail(f"{prefix}: id is required")
    marker = claim.get("marker")
    if marker not in (*MARKERS, None):
        fail(f"{prefix}: invalid marker {marker!r}")

    implementation = claim.get("implementation")
    if not isinstance(implementation, list) or not implementation:
        fail(f"{prefix}: at least one implementation path is required")
    for relative in implementation:
        if not isinstance(relative, str) or not (root / relative).exists():
            fail(f"{prefix}: implementation path is missing: {relative!r}")

    evidence = claim.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        fail(f"{prefix}: at least one evidence entry is required")
    evidence_kinds: set[str] = set()
    for entry in evidence:
        if not isinstance(entry, dict) or entry.get("kind") not in EVIDENCE_KINDS:
            fail(f"{prefix}: evidence kind must be one of {sorted(EVIDENCE_KINDS)}")
        relative = entry.get("path")
        if not isinstance(relative, str) or not (root / relative).exists():
            fail(f"{prefix}: evidence path is missing: {relative!r}")
        evidence_kinds.add(entry["kind"])

    open_beads = claim.get("open_beads")
    if not isinstance(open_beads, list) or len(open_beads) != len(set(open_beads)):
        fail(f"{prefix}: open_beads must be a list of unique bead IDs")
    statuses = claim["_bead_statuses"]
    for bead_id in open_beads:
        if not isinstance(bead_id, str) or not BEAD_ID.fullmatch(bead_id):
            fail(f"{prefix}: invalid bead ID {bead_id!r}")
        status = statuses.get(bead_id)
        if status not in OPEN_BEAD_STATUSES:
            detail = "missing" if status is None else f"status={status}"
            fail(f"{prefix}: referenced bead {bead_id} is not open ({detail})")

    mode = claim.get("mode", "capability")
    if mode not in {"capability", "release"}:
        fail(f"{prefix}: mode must be 'capability' or 'release'")
    if marker == "✅":
        if open_beads:
            fail(f"{prefix}: ✅ claim still has open beads: {', '.join(open_beads)}")
        if not {"test", "fixture"} & evidence_kinds:
            fail(f"{prefix}: ✅ claim needs test or fixture evidence")
    elif marker == "🚧":
        if not open_beads:
            fail(f"{prefix}: 🚧 claim needs an explicit open bead")
        if mode == "capability" and not {"test", "fixture"} & evidence_kinds:
            fail(f"{prefix}: 🚧 capability needs test or fixture evidence")
    elif marker == "❌" and mode == "capability":
        if not {"test", "fixture"} & evidence_kinds:
            fail(f"{prefix}: ❌ capability needs failure evidence")
        if not open_beads:
            fail(f"{prefix}: ❌ capability needs an explicit open bead")


def check_location(root: Path, readme: str, claim: dict[str, Any], index: int) -> None:
    location = claim.get("location")
    prefix = f"claim {index} ({claim.get('id', '<missing id>')})"
    if not isinstance(location, dict):
        fail(f"{prefix}: location is required")
    kind = location.get("kind")
    expected = claim.get("marker")
    if kind == "matrix":
        rows = matrix_rows(readme)
        label = location.get("label")
        if label not in rows:
            fail(f"{prefix}: README matrix row is missing: {label!r}")
        actual = marker_at_start(rows[label][1])
        if actual != expected:
            fail(f"{prefix}: README marker is {actual!r}, ledger expects {expected!r}")
        claim_text = claim.get("claim")
        if not isinstance(claim_text, str) or not claim_text:
            fail(f"{prefix}: claim text is required")
    elif kind == "line":
        locator = location.get("locator")
        if not isinstance(locator, str) or not locator:
            fail(f"{prefix}: line locator is required")
        line = unique_line(readme, locator)
        actual = marker_in(line)
        if actual != expected:
            fail(f"{prefix}: README marker is {actual!r}, ledger expects {expected!r}")
        claim_text = claim.get("claim")
        if not isinstance(claim_text, str) or not claim_text:
            fail(f"{prefix}: claim text is required")
        fragment = claim.get("fragment")
        if fragment is not None and fragment not in line:
            fail(f"{prefix}: README line no longer contains {fragment!r}")
    else:
        fail(f"{prefix}: location kind must be 'matrix' or 'line'")


def audit(root: Path, manifest_path: Path) -> int:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        readme_path = root / manifest["readme"]
        readme = readme_path.read_text(encoding="utf-8")
        claims = manifest["claims"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        fail(f"cannot load audit inputs: {error}")

    if manifest.get("schema_version") != 1:
        fail("unsupported evidence-ledger schema version")
    if not isinstance(claims, list) or not claims:
        fail("evidence ledger must contain claims")
    expected_date = manifest.get("status_snapshot")
    if not isinstance(expected_date, str) or f"evidence-audited {expected_date}" not in readme:
        fail(f"README status snapshot must say 'evidence-audited {expected_date}'")

    matrix = matrix_rows(readme)
    matrix_claims = {
        claim.get("location", {}).get("label")
        for claim in claims
        if isinstance(claim, dict) and claim.get("location", {}).get("kind") == "matrix"
    }
    if matrix_claims != set(matrix):
        missing = sorted(set(matrix) - matrix_claims)
        extra = sorted(matrix_claims - set(matrix))
        fail(f"matrix ledger mismatch; missing={missing!r} extra={extra!r}")

    checkpoint = root / manifest.get("checkpoint", ".beads/checkpoint/forensic.jsonl")
    statuses = load_bead_statuses(checkpoint)
    ids: set[str] = set()
    for index, claim in enumerate(claims, 1):
        if not isinstance(claim, dict):
            fail(f"claim {index} is not an object")
        if claim.get("id") in ids:
            fail(f"duplicate claim ID: {claim.get('id')!r}")
        ids.add(claim.get("id"))
        claim["_bead_statuses"] = statuses
        check_location(root, readme, claim, index)
        check_evidence(root, claim, index)

    print(f"README capability evidence audit passed: {len(claims)} claims checked")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root (default: parent of scripts/)",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        help="evidence ledger path (default: docs/readme-capability-evidence.json)",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    manifest = (args.manifest or root / "docs/readme-capability-evidence.json").resolve()
    try:
        return audit(root, manifest)
    except AuditFailure as error:
        print(f"README capability evidence audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

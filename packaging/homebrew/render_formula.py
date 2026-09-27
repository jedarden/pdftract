#!/usr/bin/env python3
"""Render and statically validate the pdftract Homebrew formula.

The renderer deliberately does not download releases, verify signatures, or
publish a tap.  Those are release-pipeline responsibilities.  It consumes the
already-resolved immutable release tuple documented in packaging/homebrew/README.md
and turns the tagged formula template into a deterministic formula.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Mapping, Sequence


REQUIRED_INPUTS = (
    "RELEASE_TAG",
    "VERSION",
    "SOURCE_ARCHIVE_URL",
    "SOURCE_ARCHIVE_SHA256",
    "SHA256SUMS_URL",
    "SHA256SUMS_SIG_URL",
    "SHA256SUMS_PEM_URL",
)

RELEASE_TAG_RE = re.compile(r"^v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
URL_RE = re.compile(r"https://[^\s\"']+")
PLACEHOLDER_RE = re.compile(r"<[A-Z][A-Z0-9_]*>")
ALLOWED_PLACEHOLDERS = {"<VERSION>", "<SHA256>"}

REPO = "jedarden/pdftract"
ARCHIVE_BASE = f"https://github.com/{REPO}/archive/refs/tags"
RELEASE_BASE = f"https://github.com/{REPO}/releases/download"


class RenderError(ValueError):
    """An invalid release tuple or formula template."""


def _required_string(inputs: Mapping[str, object], name: str) -> str:
    value = inputs.get(name)
    if not isinstance(value, str) or not value:
        raise RenderError(f"{name} must be a non-empty string")
    return value


def validate_inputs(raw_inputs: Mapping[str, object]) -> dict[str, str]:
    """Validate and normalize the immutable release tuple.

    The source archive hash is intentionally an explicit input.  The caller
    resolves and verifies the archive and signed SHA256SUMS before invoking
    this local renderer; this script must never silently hash a different
    artifact or fetch a moving ref.
    """

    missing = [name for name in REQUIRED_INPUTS if name not in raw_inputs]
    if missing:
        raise RenderError(f"missing required release input(s): {', '.join(missing)}")

    inputs = {name: _required_string(raw_inputs, name) for name in REQUIRED_INPUTS}
    tag = inputs["RELEASE_TAG"]
    version = inputs["VERSION"]

    if not RELEASE_TAG_RE.fullmatch(tag):
        raise RenderError(
            "RELEASE_TAG must be an immutable non-prerelease tag in vX.Y.Z form"
        )
    if version != tag[1:]:
        raise RenderError("VERSION must equal RELEASE_TAG without its leading 'v'")

    expected_archive_url = f"{ARCHIVE_BASE}/{tag}.tar.gz"
    if inputs["SOURCE_ARCHIVE_URL"] != expected_archive_url:
        raise RenderError(
            "SOURCE_ARCHIVE_URL must be the canonical versioned GitHub tag archive "
            f"({expected_archive_url})"
        )
    if not SHA256_RE.fullmatch(inputs["SOURCE_ARCHIVE_SHA256"]):
        raise RenderError("SOURCE_ARCHIVE_SHA256 must be exactly 64 lowercase hex characters")

    expected_release_urls = {
        "SHA256SUMS_URL": f"{RELEASE_BASE}/{tag}/SHA256SUMS",
        "SHA256SUMS_SIG_URL": f"{RELEASE_BASE}/{tag}/SHA256SUMS.sig",
        "SHA256SUMS_PEM_URL": f"{RELEASE_BASE}/{tag}/SHA256SUMS.pem",
    }
    for name, expected in expected_release_urls.items():
        if inputs[name] != expected:
            raise RenderError(f"{name} must be the canonical URL for {tag} ({expected})")

    return inputs


def validate_template(template: str) -> None:
    """Check that the template exposes exactly the supported substitutions."""

    placeholders = set(PLACEHOLDER_RE.findall(template))
    unknown = sorted(placeholders - ALLOWED_PLACEHOLDERS)
    if unknown:
        raise RenderError(f"unsupported placeholder(s) in template: {', '.join(unknown)}")
    for placeholder in sorted(ALLOWED_PLACEHOLDERS):
        if placeholder not in template:
            raise RenderError(f"template is missing required placeholder {placeholder}")


def validate_rendered_formula(formula: str, inputs: Mapping[str, str]) -> None:
    """Validate formula structure and immutable URL/checksum bindings."""

    leftovers = sorted(set(PLACEHOLDER_RE.findall(formula)))
    if leftovers:
        raise RenderError(f"unsubstituted placeholder(s) remain: {', '.join(leftovers)}")

    expected_url = inputs["SOURCE_ARCHIVE_URL"]
    expected_sha = inputs["SOURCE_ARCHIVE_SHA256"]
    url_lines = re.findall(r'^\s*url\s+"([^"]+)"\s*$', formula, re.MULTILINE)
    version_lines = re.findall(r'^\s*version\s+"([^"]+)"\s*$', formula, re.MULTILINE)
    sha_lines = re.findall(r'^\s*sha256\s+"([^"]+)"\s*$', formula, re.MULTILINE)

    if url_lines != [expected_url]:
        raise RenderError(
            "formula must contain exactly one source archive URL matching the release tuple"
        )
    if version_lines != [inputs["VERSION"]]:
        raise RenderError("formula must contain exactly one version matching RELEASE_TAG")
    if sha_lines != [expected_sha]:
        raise RenderError("formula must contain exactly one sha256 matching SOURCE_ARCHIVE_SHA256")

    required_fragments = (
        "class Pdftract < Formula",
        'depends_on "rust"',
        "test do",
        "pdftract --version",
    )
    for fragment in required_fragments:
        if fragment not in formula:
            raise RenderError(f"formula is missing required fragment: {fragment}")

    lowered = formula.lower()
    forbidden_ref = re.search(
        r":latest|/latest(?:[/?\"']|$)|refs/(?:heads|pull)/|\bmain\b|\bmaster\b",
        lowered,
    )
    if forbidden_ref:
        raise RenderError(f"formula contains a floating ref: {forbidden_ref.group(0)}")

    # A raw commit SHA is not an acceptable release input, even if it appears
    # in a comment or an unrelated URL accidentally added to the template.
    if re.search(r"(?<![0-9a-f])[0-9a-f]{40}(?![0-9a-f])", lowered):
        raise RenderError("formula contains a bare commit SHA")

    # Every URL in the rendered formula must remain free of query/fragment
    # overrides; the only download URL is the exact immutable archive above.
    for url in URL_RE.findall(formula):
        if "?" in url or "#" in url:
            raise RenderError(f"formula URL must not contain a query or fragment: {url}")


def render(template: str, inputs: Mapping[str, str]) -> str:
    validate_template(template)
    formula = template.replace("<VERSION>", inputs["VERSION"]).replace(
        "<SHA256>", inputs["SOURCE_ARCHIVE_SHA256"]
    )
    validate_rendered_formula(formula, inputs)
    return formula


def _ruby_check(path: Path, label: str) -> None:
    ruby = shutil.which("ruby")
    if ruby is None:
        print(
            f"warning: ruby is unavailable; skipped syntax validation for {label}",
            file=sys.stderr,
        )
        return
    result = subprocess.run([ruby, "-c", os.fspath(path)], text=True, capture_output=True)
    if result.returncode != 0:
        detail = (result.stdout + result.stderr).strip()
        raise RenderError(f"ruby -c failed for {label}: {detail}")
    print(f"validated: ruby -c {label}", file=sys.stderr)


def _load_inputs(args: argparse.Namespace) -> dict[str, str]:
    if args.inputs is not None:
        direct = (
            args.release_tag,
            args.version,
            args.source_archive_url,
            args.source_archive_sha256,
            args.sha256sums_url,
            args.sha256sums_sig_url,
            args.sha256sums_pem_url,
        )
        if any(value is not None for value in direct):
            raise RenderError("--inputs cannot be combined with direct release input options")
        try:
            with args.inputs.open(encoding="utf-8") as handle:
                raw = json.load(handle)
        except (OSError, json.JSONDecodeError) as exc:
            raise RenderError(f"cannot read --inputs JSON: {exc}") from exc
        if not isinstance(raw, dict):
            raise RenderError("--inputs JSON must contain an object")
        return validate_inputs(raw)

    names = {
        "RELEASE_TAG": args.release_tag,
        "VERSION": args.version,
        "SOURCE_ARCHIVE_URL": args.source_archive_url,
        "SOURCE_ARCHIVE_SHA256": args.source_archive_sha256,
        "SHA256SUMS_URL": args.sha256sums_url,
        "SHA256SUMS_SIG_URL": args.sha256sums_sig_url,
        "SHA256SUMS_PEM_URL": args.sha256sums_pem_url,
    }
    return validate_inputs(names)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--template",
        type=Path,
        default=Path(__file__).with_name("pdftract.rb.template"),
        help="formula template (default: pdftract.rb.template next to this script)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="write the rendered formula here; print it to stdout when omitted",
    )
    parser.add_argument(
        "--inputs",
        type=Path,
        help="JSON file containing the seven uppercase release-tuple fields",
    )
    parser.add_argument("--release-tag")
    parser.add_argument("--version")
    parser.add_argument("--source-archive-url")
    parser.add_argument("--source-archive-sha256")
    parser.add_argument("--sha256sums-url")
    parser.add_argument("--sha256sums-sig-url")
    parser.add_argument("--sha256sums-pem-url")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        inputs = _load_inputs(args)
        template = args.template.read_text(encoding="utf-8")
        formula = render(template, inputs)

        with tempfile.TemporaryDirectory(prefix="pdftract-formula-") as temp_dir:
            rendered_path = Path(temp_dir) / "pdftract.rb"
            rendered_path.write_text(formula, encoding="utf-8")
            _ruby_check(args.template, "template")
            _ruby_check(rendered_path, "rendered formula")

            if args.output is None:
                sys.stdout.write(formula)
            else:
                args.output.parent.mkdir(parents=True, exist_ok=True)
                args.output.write_text(formula, encoding="utf-8")
                print(f"rendered: {args.output}", file=sys.stderr)
    except (OSError, RenderError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

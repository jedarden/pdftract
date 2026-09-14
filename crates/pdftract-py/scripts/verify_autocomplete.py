#!/usr/bin/env python3
"""Headless autocomplete verification harness for the pdftract-py SDK.

Enumerates exactly what an editor's autocomplete popup would show for typed
expressions over the SDK, using jedi — the completion engine used by the
VSCode Python extension before Pylance and still used by several editors.
jedi completion queries are fully headless: no IDE, no GUI, no interactive
REPL, so this runs anywhere the SDK source and jedi are available.

Usage:
    python3 crates/pdftract-py/scripts/verify_autocomplete.py

Requires: jedi (e.g. `python3 -m pip install --user jedi`, or a venv under
~/scratch/). The harness points jedi at the typed SDK at ../python/ itself;
nothing is installed and no PDF is opened — jedi resolves the return-type
annotations statically.

Exit code 0 = sanity check passed and every pdftract query offered all
expected members. Non-zero = a query failed or an expected member was
missing.
"""

from __future__ import annotations

import sys
from pathlib import Path

import jedi

SDK_DIR = Path(__file__).resolve().parents[1] / "python"

# Point jedi's static analysis at the in-tree SDK. A live sys.path.insert
# does NOT work: jedi analyses in its own environment, whose search path is
# derived from the interpreter, not from the running process's sys.path.
_PROJECT = jedi.Project(path=str(Path(__file__).resolve().parent), added_sys_path=[str(SDK_DIR)])


def completions_at_end(code: str) -> list[str]:
    """Sorted unique completion names jedi offers at the end of ``code``.

    The snippet must end exactly where the editor cursor would sit (e.g. a
    trailing ``doc.`` with the cursor after the dot).
    """
    lines = code.splitlines()
    script = jedi.Script(code=code, path="snippet.py", project=_PROJECT)
    return sorted({c.name for c in script.complete(len(lines), len(lines[-1]))})


def check(label: str, code: str, expected: set[str]) -> list[str]:
    """Print the completion list for ``code``; return missing expected names."""
    names = completions_at_end(code)
    missing = sorted(expected - set(names))
    status = "PASS" if not missing else "FAIL"
    print(f"[{status}] {label} — {len(names)} completions")
    for name in names:
        print(f"    {name}")
    for name in missing:
        print(f"    MISSING expected member: {name}")
    return missing


def main() -> int:
    print(f"python : {sys.version.split()[0]} ({sys.executable})")
    print(f"jedi   : {jedi.__version__}")
    print(f"sdk    : {SDK_DIR}")
    print()

    failures: list[str] = []

    # Sanity check on a trivial expression: if jedi cannot complete `os.pa`
    # to `path`, any pdftract result below would be meaningless.
    missing = check(
        "sanity: os.pa",
        "import os\nos.pa",
        {"path"},
    )
    if missing:
        failures.append("sanity (os.pa -> path)")

    # Module surface: what an editor offers after `import pdftract` + `pdftract.`
    missing = check(
        "pdftract. (module API)",
        'import pdftract\npdftract.',
        {
            "extract",
            "extract_text",
            "extract_markdown",
            "extract_stream",
            "search",
            "get_metadata",
            "hash",
            "classify",
            "Document",
            "Page",
            "Span",
            "Block",
            "Match",
            "Fingerprint",
            "Classification",
            "Metadata",
        },
    )
    if missing:
        failures.append("pdftract. module API")

    # Document: extract() is annotated `-> Document`; jedi must infer that
    # statically and offer the dataclass fields.
    missing = check(
        'doc. where doc = pdftract.extract("x.pdf")',
        'import pdftract\ndoc = pdftract.extract("x.pdf")\ndoc.',
        {"pages", "schema_version", "metadata"},
    )
    if missing:
        failures.append("doc. (Document fields)")

    # Nested type: Document.pages is annotated List[Page].
    missing = check(
        "page. where page = doc.pages[0]",
        'import pdftract\ndoc = pdftract.extract("x.pdf")\npage = doc.pages[0]\npage.',
        {"page", "width", "height", "rotation", "spans", "blocks"},
    )
    if missing:
        failures.append("page. (Page fields)")

    # Metadata: get_metadata() is annotated `-> Metadata`.
    missing = check(
        'pdftract.get_metadata("x.pdf").',
        'import pdftract\npdftract.get_metadata("x.pdf").',
        {"page_count", "title", "author", "producer", "created", "modified"},
    )
    if missing:
        failures.append("get_metadata(...). (Metadata fields)")

    print()
    if failures:
        print(f"FAILED ({len(failures)}):")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("All autocomplete checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

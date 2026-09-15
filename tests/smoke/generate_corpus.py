#!/usr/bin/env python3
"""Generate the canonical platform-smoke corpus.

Writes the PDFs in ``tests/smoke/corpus/`` and the ``corpus.manifest``
(sha256 per fixture, ``sha256sum -c`` compatible) next to them.

The corpus is the stable input for the per-release manual platform smoke
test (docs/operations/manual-platform-smoke.md). It is deliberately tiny,
well-formed, and independent of ``tests/fixtures/`` so that the per-release
golden diff is never invalidated by unrelated test-fixture churn.

Determinism rules:
  * no wall-clock input: every date string is a fixed constant
  * stdlib only, no third-party deps
  * re-running over an unchanged generator reproduces byte-identical PDFs,
    which is what makes ``corpus.manifest`` stable across releases

Usage:
    python3 tests/smoke/generate_corpus.py            # write corpus/ + manifest
    python3 tests/smoke/generate_corpus.py --check    # verify manifest only
"""

import argparse
import hashlib
import pathlib
import zlib

SMOKE_DIR = pathlib.Path(__file__).resolve().parent
CORPUS_DIR = SMOKE_DIR / "corpus"
MANIFEST_PATH = SMOKE_DIR / "corpus.manifest"

# Fixed "generation date" baked into /Info so output bytes never vary.
FIXED_DATE = "D:20260101000000Z"
PRODUCER = "pdftract smoke corpus generator"


def _escape(text: str) -> str:
    """Escape a string for a PDF literal."""
    return text.replace("\\", r"\\").replace("(", r"\(").replace(")", r"\)")


def _content_stream(lines: list[str]) -> bytes:
    """One Helvetica text block per line, stacked down the page."""
    parts = ["BT", "/F1 12 Tf", "16 TL", "72 720 Td"]
    for i, line in enumerate(lines):
        if i > 0:
            parts.append("T*")
        parts.append(f"({_escape(line)}) Tj")
    parts.append("ET")
    return "\n".join(parts).encode("ascii")


def build_pdf(
    pages: list[list[str]],
    *,
    info: dict[str, str] | None = None,
    compress: bool = False,
) -> bytes:
    """Assemble a well-formed single-font PDF from per-page text lines.

    Object layout: 1 Catalog, 2 Pages, 3 Font, then per page a Page object
    followed by its (optionally Flate-compressed) content stream, then the
    optional /Info dictionary last.
    """
    body: list[tuple[int, bytes]] = []

    def add(obj_num: int, payload: bytes) -> None:
        body.append((obj_num, payload))

    page_count = len(pages)
    kids = []
    next_obj = 4
    for index in range(page_count):
        page_obj = next_obj
        stream_obj = next_obj + 1
        next_obj += 2
        kids.append(f"{page_obj} 0 R")

        stream = _content_stream(pages[index])
        if compress:
            stream = zlib.compress(stream)
            stream_dict = f"<< /Length {len(stream)} /Filter /FlateDecode >>"
        else:
            stream_dict = f"<< /Length {len(stream)} >>"
        add(
            page_obj,
            (
                f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
                f"/Contents {stream_obj} 0 R /Resources << /Font << /F1 3 0 R >> >> >>"
            ).encode("ascii"),
        )
        add(
            stream_obj,
            stream_dict.encode("ascii") + b"\nstream\n" + stream + b"\nendstream",
        )

    info_obj = None
    if info is not None:
        info_obj = next_obj
        entries = "".join(
            f"/{key} ({_escape(value)}) " for key, value in info.items()
        )
        add(info_obj, f"<< {entries}/Producer ({_escape(PRODUCER)}) >>".encode("ascii"))

    add(1, b"<< /Type /Catalog /Pages 2 0 R >>")
    add(
        2,
        f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {page_count} >>".encode(
            "ascii"
        ),
    )
    add(
        3,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    )

    # Serialise in object-number order; record exact offsets for the xref.
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets: dict[int, int] = {}
    for obj_num, payload in sorted(body):
        offsets[obj_num] = len(out)
        out += f"{obj_num} 0 obj\n".encode("ascii") + payload + b"\nendobj\n"

    xref_pos = len(out)
    size = max(offsets) + 1
    out += f"xref\n0 {size}\n".encode("ascii")
    out += b"0000000000 65535 f \n"
    for obj_num in range(1, size):
        out += f"{offsets[obj_num]:010d} 00000 n \n".encode("ascii")

    trailer = f"<< /Size {size} /Root 1 0 R "
    if info_obj is not None:
        trailer += f"/Info {info_obj} 0 R "
    trailer += ">>"
    out += trailer.encode("ascii")
    out += f"\nstartxref\n{xref_pos}\n%%EOF\n".encode("ascii")
    return bytes(out)


def build_corpus() -> dict[str, bytes]:
    """The canonical fixture set. Names are stable; treat changes as breaking."""
    corpus = {
        "minimal-text.pdf": build_pdf(
            [["Hello pdftract smoke corpus.", "Single page, uncompressed stream."]]
        ),
        "multipage-text.pdf": build_pdf(
            [
                ["Multipage fixture, page one.", "Three pages in total."],
                ["Multipage fixture, page two.", "Distinct text on every page."],
                ["Multipage fixture, page three.", "Last page of the set."],
            ]
        ),
        "metadata.pdf": build_pdf(
            [["Metadata fixture.", "Info dictionary carries fixed strings."]],
            info={
                "Title": "pdftract smoke metadata fixture",
                "Author": "pdftract release engineering",
                "Subject": "per-release platform smoke test",
                "CreationDate": FIXED_DATE,
                "ModDate": FIXED_DATE,
            },
        ),
        "flate-compressed.pdf": build_pdf(
            [["Compressed fixture.", "Content stream uses FlateDecode."]],
            compress=True,
        ),
    }
    return corpus


def write_corpus() -> None:
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    lines = []
    for name in sorted(build_corpus()):
        data = build_corpus()[name]
        (CORPUS_DIR / name).write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        lines.append(f"{digest}  {name}")
        print(f"wrote corpus/{name}  sha256={digest}")
    MANIFEST_PATH.write_text("\n".join(lines) + "\n", encoding="ascii")
    print(f"wrote corpus.manifest ({len(lines)} entries)")


def check_corpus() -> bool:
    if not MANIFEST_PATH.exists():
        print("FAIL: corpus.manifest missing")
        return False
    ok = True
    for line in MANIFEST_PATH.read_text(encoding="ascii").splitlines():
        if not line.strip():
            continue
        digest, name = line.split(None, 1)
        path = CORPUS_DIR / name.strip()
        if not path.exists():
            print(f"FAIL: corpus/{name.strip()} missing")
            ok = False
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != digest:
            print(f"FAIL: corpus/{name.strip()} hash mismatch")
            ok = False
    print("corpus.manifest OK" if ok else "corpus.manifest MISMATCH")
    return ok


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="verify corpus against the manifest"
    )
    args = parser.parse_args()
    if args.check:
        return 0 if check_corpus() else 1
    write_corpus()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

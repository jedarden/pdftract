#!/usr/bin/env python3
"""Generate the small PDF used to verify Markdown extraction.

The fixture deliberately uses only PDF base-14 Helvetica and a plain xref
table so it can be regenerated without third-party packages.  It contains a
large heading and a body line covered by a URI annotation.  The annotation is
what makes ``extract_markdown`` emit a Markdown link while ``extract_text``
keeps the same visible words as plain text.
"""

from __future__ import annotations

import argparse
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("markdown_verification.pdf")


def pdf_literal(value: str) -> bytes:
    """Encode an ASCII string as a PDF literal string."""

    return b"(" + value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)").encode("ascii") + b")"


def build_pdf() -> bytes:
    """Return a deterministic, one-page PDF with heading text and one link."""

    heading = pdf_literal("Markdown Verification Heading")
    body = pdf_literal("Read the verification link")
    content = (
        b"BT\n/F1 24 Tf\n72 680 Td\n"
        + heading
        + b" Tj\nET\n"
        b"BT\n/F1 12 Tf\n72 635 Td\n"
        + body
        + b" Tj\nET\n"
    )

    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources << /Font << /F1 4 0 R >> >> "
            b"/Contents 5 0 R /Annots [6 0 R] >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"<< /Length " + str(len(content)).encode("ascii") + b" >>\nstream\n" + content + b"endstream",
        (
            b"<< /Type /Annot /Subtype /Link /Rect [68 630 240 650] "
            b"/Border [0 0 0] /A << /S /URI /URI (https://example.com/verification) >> >>"
        ),
    ]

    pdf = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for object_number, obj in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf.extend(f"{object_number} 0 obj\n".encode("ascii"))
        pdf.extend(obj)
        pdf.extend(b"\nendobj\n")

    xref_offset = len(pdf)
    pdf.extend(f"xref\n0 {len(offsets)}\n".encode("ascii"))
    pdf.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        pdf.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    pdf.extend(
        (
            f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    return bytes(pdf)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(build_pdf())
    print(f"wrote {args.output} ({args.output.stat().st_size} bytes)")


if __name__ == "__main__":
    main()

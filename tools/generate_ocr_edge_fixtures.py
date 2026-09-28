#!/usr/bin/env python3
"""Generate deterministic OCR acceptance fixtures beyond clean scans.

The edge corpus is deliberately small and synthetic.  It exercises the OCR
gate with repeatable scan defects while keeping the source text and licensing
unambiguous.  The generated PDFs contain no timestamps and each fixture gets
a JSON sidecar consumed by ``scripts/verify-ocr-acceptance.sh``.

Usage:
    python3 tools/generate_ocr_edge_fixtures.py
    python3 tools/generate_ocr_edge_fixtures.py --out-root /tmp/ocr-corpus

The default output directory is ``tests/fixtures/scanned``.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import random
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont


PAGE_W_PT = 612
PAGE_H_PT = 792
DEFAULT_ROOT = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "scanned"
FONT_CANDIDATES = (
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/dejavu/DejaVuSerif.ttf",
)


FIXTURES = {
    "noisy-300dpi": {
        "directory": "noisy",
        "dpi": 300,
        "mode": "scanned",
        "target": 5,
        "kind": "stress",
    },
    "skewed-300dpi": {
        "directory": "skewed",
        "dpi": 300,
        "mode": "scanned",
        "target": 5,
        "kind": "stress",
    },
    "low-resolution-150dpi": {
        "directory": "low-resolution",
        "dpi": 150,
        "mode": "scanned",
        "target": 10,
        "kind": "stress",
    },
    "multi-column-300dpi": {
        "directory": "multi-column",
        "dpi": 300,
        "mode": "scanned",
        "target": 5,
        "kind": "stress",
    },
    "mixed-vector-scanned-300dpi": {
        "directory": "mixed-vector-scanned",
        "dpi": 300,
        "mode": "mixed",
        "target": 5,
        "kind": "stress",
    },
}


TEXT = {
    "noisy-300dpi": """NOISY SCAN ACCEPTANCE FIXTURE
The archive team tested a noisy paper scan for the OCR acceptance gate.
Random dust and faint background marks are present, but every sentence is
still recoverable from the page.
Document ID NOISE 300. Review date September 28 2026.
The expected extraction route is scanned OCR and the source is CC0.
""",
    "skewed-300dpi": """SKEWED SCAN ACCEPTANCE FIXTURE
This page is rotated by a small angle before it is stored as a PDF image.
Deskew handling should preserve the title, the body text, and the final line.
Document ID SKEW 300. Review date September 28 2026.
The expected extraction route is scanned OCR and the source is CC0.
""",
    "low-resolution-150dpi": """LOW RESOLUTION ACCEPTANCE FIXTURE
This document is intentionally rendered at one hundred fifty dots per inch.
Small but legible type checks that the OCR path remains useful at low input
resolution. Document ID LOW 150. Review date September 28 2026.
The expected extraction route is scanned OCR and the source is CC0.
""",
    "multi-column-300dpi": {
        "left": """LEFT COLUMN ONE
LEFT COLUMN ALPHA
LEFT COLUMN BRAVO
LEFT COLUMN CHARLIE
LEFT COLUMN DELTA
LEFT COLUMN ECHO
LEFT COLUMN FOXTROT
LEFT COLUMN GOLF
""",
        "right": """RIGHT COLUMN ONE
RIGHT COLUMN ALPHA
RIGHT COLUMN BRAVO
RIGHT COLUMN CHARLIE
RIGHT COLUMN DELTA
RIGHT COLUMN ECHO
RIGHT COLUMN FOXTROT
RIGHT COLUMN GOLF
""",
    },
    "mixed-vector-scanned-300dpi": """SCANNED BODY ACCEPTANCE FIXTURE
This body is raster content beneath a separate vector header and sidebar.
The OCR transcript covers only the scanned body so the vector layer can be
checked independently by the routing verifier.
Document ID MIXED 300. Review date September 28 2026.
The expected extraction route is mixed vector and scanned content.
""",
}


def font_path() -> str:
    override = os.environ.get("PDFTRACT_FIXTURE_FONT")
    if override and Path(override).is_file():
        return override
    for candidate in FONT_CANDIDATES:
        if Path(candidate).is_file():
            return candidate
    try:
        discovered = subprocess.check_output(
            ["fc-match", "-f", "%{file}", "serif"], text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        discovered = ""
    if discovered and Path(discovered).is_file():
        return discovered
    raise RuntimeError("DejaVuSerif.ttf not found; set PDFTRACT_FIXTURE_FONT")


def wrap_words(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.FreeTypeFont, width: int) -> list[str]:
    lines: list[str] = []
    for source_line in text.strip().splitlines():
        words = source_line.split()
        current = ""
        for word in words:
            candidate = word if not current else f"{current} {word}"
            if current and draw.textbbox((0, 0), candidate, font=font)[2] > width:
                lines.append(current)
                current = word
            else:
                current = candidate
        if current:
            lines.append(current)
    return lines


def render_page(text: str, dpi: int, *, columns: tuple[str, str] | None = None) -> Image.Image:
    width = round(8.5 * dpi)
    height = round(11 * dpi)
    image = Image.new("L", (width, height), 255)
    draw = ImageDraw.Draw(image)
    font_size = max(18, round(11 * dpi / 72))
    font = ImageFont.truetype(font_path(), font_size)
    margin = round(0.7 * dpi)
    line_gap = round(0.27 * dpi)

    if columns is None:
        lines = wrap_words(draw, text, font, width - 2 * margin)
        y = margin
        for index, line in enumerate(lines):
            if index == 0:
                heading_font = ImageFont.truetype(font_path(), round(14 * dpi / 72))
                draw.text((margin, y), line, font=heading_font, fill=0)
                y += round(0.42 * dpi)
            else:
                draw.text((margin, y), line, font=font, fill=0)
                y += line_gap
            if y >= height - margin:
                raise RuntimeError(f"fixture text overflowed {dpi} DPI page")
        return image

    column_gap = round(0.35 * dpi)
    column_width = (width - 2 * margin - column_gap) // 2
    for column_index, column_text in enumerate(columns):
        x = margin + column_index * (column_width + column_gap)
        lines = wrap_words(draw, column_text, font, column_width)
        y = margin
        for index, line in enumerate(lines):
            draw.text((x, y), line, font=font, fill=0)
            y += line_gap
            if y >= height - margin:
                raise RuntimeError(f"column {column_index} overflowed {dpi} DPI page")
    return image


def jpeg_bytes(image: Image.Image, dpi: int) -> bytes:
    output = io.BytesIO()
    image.convert("RGB").save(
        output,
        format="JPEG",
        quality=94,
        optimize=False,
        progressive=False,
        subsampling=0,
        dpi=(dpi, dpi),
    )
    return output.getvalue()


def pdf_object(number: int, body: bytes) -> bytes:
    return f"{number} 0 obj\n".encode() + body + b"\nendobj\n"


def build_image_pdf(image: Image.Image, output: Path, dpi: int) -> None:
    jpeg = jpeg_bytes(image, dpi)
    width, height = image.size
    contents = b"q %d 0 0 %d 0 0 cm /Im0 Do Q\n" % (PAGE_W_PT, PAGE_H_PT)
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>",
        b"<< /Length %d >>\nstream\n" % len(contents) + contents + b"endstream",
        b"<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length %d >>\nstream\n"
        % (width, height, len(jpeg))
        + jpeg
        + b"\nendstream",
    ]
    write_pdf_objects(objects, output)


def build_mixed_pdf(image: Image.Image, output: Path, dpi: int) -> None:
    jpeg = jpeg_bytes(image, dpi)
    width, height = image.size
    vector_lines = [
        "VECTOR HEADER - MIXED PAGE ROUTING",
        "The right column is real PDF text.",
        "The left column is scanned image content.",
        "Expected page type: mixed.",
    ]
    text_ops = ["BT /F1 13 Tf 330 742 Td (%s) Tj" % escape_pdf(vector_lines[0])]
    for line in vector_lines[1:]:
        text_ops.append("0 -30 Td (%s) Tj" % escape_pdf(line))
    text_ops.append("ET")
    contents = (
        "q 306 0 0 792 0 0 cm /Im0 Do Q\n" + "\n".join(text_ops) + "\n"
    ).encode("ascii")
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im0 5 0 R >> /Font << /F1 6 0 R >> >> /Contents 4 0 R >>",
        b"<< /Length %d >>\nstream\n" % len(contents) + contents + b"endstream",
        b"<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length %d >>\nstream\n"
        % (width, height, len(jpeg))
        + jpeg
        + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    write_pdf_objects(objects, output)


def escape_pdf(value: str) -> str:
    return value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")


def write_pdf_objects(objects: list[bytes], output: Path) -> None:
    data = bytearray(b"%PDF-1.4\n%\xc7\xec\x8f\xa2\n")
    offsets = [0]
    for number, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(pdf_object(number, body))
    xref = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n".encode())
    data.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        data.extend(f"{offset:010d} 00000 n \n".encode())
    data.extend(
        f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    )
    output.write_bytes(data)


def apply_transform(name: str, image: Image.Image) -> Image.Image:
    if name == "noisy-300dpi":
        randomizer = random.Random(300300)
        image = image.convert("L")
        pixels = image.load()
        width, height = image.size
        for _ in range(width * height // 2600):
            x = randomizer.randrange(width)
            y = randomizer.randrange(height)
            value = randomizer.randrange(125, 235)
            pixels[x, y] = value
        return image.filter(ImageFilter.GaussianBlur(radius=0.18))
    if name == "skewed-300dpi":
        return image.rotate(1.35, resample=Image.Resampling.BICUBIC, expand=False, fillcolor=255)
    if name == "low-resolution-150dpi":
        return image
    return image


def ground_truth(name: str) -> str:
    value = TEXT[name]
    if isinstance(value, str):
        return value.strip() + "\n"
    if name == "multi-column-300dpi":
        return value["left"].strip() + "\n" + value["right"].strip() + "\n"
    return value["left"].strip() + "\n"


def generate_one(root: Path, name: str, config: dict) -> None:
    directory = root / config["directory"]
    directory.mkdir(parents=True, exist_ok=True)
    gt_path = directory / f"{name}.txt"
    pdf_path = directory / f"{name}.pdf"
    metadata_path = directory / f"{name}.metadata.json"
    gt_path.write_text(ground_truth(name), encoding="utf-8")

    if name == "multi-column-300dpi":
        value = TEXT[name]
        image = render_page("", config["dpi"], columns=(value["left"], value["right"]))
    else:
        image = render_page(TEXT[name], config["dpi"])
    image = apply_transform(name, image)
    if name == "mixed-vector-scanned-300dpi":
        image = render_page("", config["dpi"], columns=(TEXT[name], "VECTOR SIDEBAR"))
        left = image.crop((0, 0, image.width // 2, image.height))
        build_mixed_pdf(left, pdf_path, config["dpi"])
    else:
        build_image_pdf(image, pdf_path, config["dpi"])

    metadata = {
        "fixture": name,
        "document_type": config["directory"],
        "expected_extraction_mode": config["mode"],
        "expected_page_type": "mixed" if config["mode"] == "mixed" else "scanned",
        "wer_class": config["kind"],
        "wer_target_percent": config["target"],
        "dpi": config["dpi"],
        "source": "Original synthetic composition created for the pdftract OCR acceptance corpus.",
        "license": "CC0-1.0",
        "generator": "tools/generate_ocr_edge_fixtures.py",
        "generation_method": {
            "base": "Pillow text rasterization with DejaVu Serif",
            "transform": {
                "noisy-300dpi": "deterministic sparse gray dust and slight blur",
                "skewed-300dpi": "deterministic 1.35 degree clockwise skew",
                "low-resolution-150dpi": "single 150 by 150 PPI raster",
                "multi-column-300dpi": "two-column page rasterized as one image",
                "mixed-vector-scanned-300dpi": "left scanned raster plus right vector text",
            }[name],
        },
    }
    metadata["pdf_sha256"] = hashlib.sha256(pdf_path.read_bytes()).hexdigest()
    metadata["ground_truth_sha256"] = hashlib.sha256(gt_path.read_bytes()).hexdigest()
    metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"generated {pdf_path} ({pdf_path.stat().st_size} bytes)")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("names", nargs="*", choices=sorted(FIXTURES))
    args = parser.parse_args()
    for name in args.names or FIXTURES:
        generate_one(args.out_root, name, FIXTURES[name])


if __name__ == "__main__":
    main()

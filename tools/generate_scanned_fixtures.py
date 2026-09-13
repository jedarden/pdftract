#!/usr/bin/env python3
"""
Generate scanned PDF fixtures from ground truth text files.

Creates a matched pair per fixture:
  - a text-embedded PDF rendered from the ground truth text
  - a scanned (image-only) PDF rasterized at the fixture DPI

Every ground-truth line is guaranteed to render fully inside the page
margins: lines wider than the text area are shrunk to fit (down to
FONT_FLOOR_PT), and only word-wrapped if they still do not fit. This
matters because the previous implementation drew each line at a fixed
size with no width check, so long lines overflowed the right margin and
were cut off at rasterize time (the letter fixture lost the right edge
of every long line that way).

All fixtures render in DejaVu Serif (a real TTF), never reportlab's
built-in Type1 core fonts. When poppler rasterizes the core-font
substitutes it mis-advances initial capitals, and tesseract splits them
off as separate tokens ("Johnson" -> "J ohnson"); on a probe line that
alone pushed WER to 91%. DejaVu Serif also has a serifed capital I,
which avoids the I -> "|" confusion of sans-serif fonts.

Usage:
    python3 generate_scanned_fixtures.py [--out-root DIR] [fixture-name ...]

Requirements:
    reportlab, Pillow, img2pdf (python), pdftoppm (poppler-utils) and a
    DejaVu Serif TTF (found via $PDFTRACT_FIXTURE_FONT or one of
    FONT_FILE_CANDIDATES below).

    With nix:
    nix-shell -p 'python3.withPackages(p: [p.reportlab p.pillow p.img2pdf])' \
               -p poppler-utils -p dejavu_fonts \
               --run 'python3 tools/generate_scanned_fixtures.py'

Pass --out-root to render into a scratch tree (verification runs) instead
of overwriting the committed fixtures.
"""

import glob
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from reportlab import rl_config
from reportlab.lib.pagesizes import letter
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

# Deterministic output: no timestamps in the produced PDFs.
rl_config.invariant = 1

FONT_NAME = "DejaVuSerif"
FONT_FILE_CANDIDATES = [
    "/nix/store/*dejavu-fonts*/share/fonts/truetype/DejaVuSerif.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/truetype/dejavu-serif/DejaVuSerif.ttf",
]

FONT_FLOOR_PT = 7.0

# Fixture configuration.
#
# gt            ground truth text (relative to tests/fixtures/scanned/)
# text_pdf      rendered text-embedded PDF (None: no text twin is kept)
# scan_pdf      rasterized image-only PDF -- the OCR fixture itself
# page_marker   GT lines starting with this pattern force a page break
#               ({N} is replaced by the 1-based page number)
FIXTURES = [
    {
        "name": "receipt-300dpi",
        "dir": "receipt",
        "gt": "receipt-300dpi.txt",
        "text_pdf": "receipt-300dpi.pdf",
        "scan_pdf": "receipt-300dpi-scanned.pdf",
        "font": FONT_NAME,
        "font_size": 10,
        "page_size": letter,
        "margins": {"left": 0.5 * inch, "top": 0.5 * inch, "right": 0.5 * inch, "bottom": 0.5 * inch},
        "line_spacing": 14,
        "dpi": 300,
    },
    {
        "name": "invoice-300dpi",
        "dir": "invoice",
        "gt": "invoice-300dpi-ground-truth.txt",
        "text_pdf": "invoice-300dpi-text-embedded.pdf",
        "scan_pdf": "invoice-300dpi.pdf",
        "font": FONT_NAME,
        "font_size": 11,
        "page_size": letter,
        "margins": {"left": 0.75 * inch, "top": 0.75 * inch, "right": 0.75 * inch, "bottom": 0.75 * inch},
        "line_spacing": 16,
        "dpi": 300,
    },
    {
        "name": "letter-300dpi",
        "dir": "letter",
        "gt": "letter-300dpi-ground-truth.txt",
        "text_pdf": "letter-300dpi-text-embedded.pdf",
        "scan_pdf": "letter-300dpi.pdf",
        "font": FONT_NAME,
        "font_size": 11,
        "page_size": letter,
        "margins": {"left": 1.0 * inch, "top": 1.0 * inch, "right": 1.0 * inch, "bottom": 1.0 * inch},
        "line_spacing": 16,
        "dpi": 300,
    },
    {
        "name": "form-300dpi",
        "dir": "form",
        "gt": "form-300dpi-ground-truth.txt",
        "text_pdf": "form-300dpi-text-embedded.pdf",
        "scan_pdf": "form-300dpi.pdf",
        "font": FONT_NAME,
        "font_size": 11,
        "page_size": letter,
        "margins": {"left": 0.75 * inch, "top": 0.75 * inch, "right": 0.75 * inch, "bottom": 0.75 * inch},
        "line_spacing": 18,
        "dpi": 300,
    },
    {
        "name": "report-300dpi",
        "dir": "multi-page",
        "gt": "report-300dpi-ground-truth.txt",
        "text_pdf": None,
        "scan_pdf": "report-300dpi.pdf",
        "font": FONT_NAME,
        "font_size": 12,
        "page_size": letter,
        "margins": {"left": 1.0 * inch, "top": 0.75 * inch, "right": 1.0 * inch, "bottom": 0.75 * inch},
        "line_spacing": 18,
        "page_marker": "Page {N}:",
        "dpi": 300,
    },
]


def fit_size(line, font, size, max_width):
    """Largest size <= the given one at which the line fits max_width."""
    while size > FONT_FLOOR_PT and pdfmetrics.stringWidth(line, font, size) > max_width:
        size -= 0.5
    return size


def wrap_line(line, font, size, max_width):
    """Word-wrap a line that does not fit even at FONT_FLOOR_PT.

    WER is computed on whitespace-split words, so wrapping is
    word-order preserving and WER-neutral.
    """
    words = line.split(" ")
    out, cur = [], ""
    for word in words:
        candidate = word if not cur else cur + " " + word
        if cur and pdfmetrics.stringWidth(candidate, font, size) > max_width:
            out.append(cur)
            cur = word
        else:
            cur = candidate
    if cur:
        out.append(cur)
    return out


def layout_line(line, font, size, max_width):
    """Fit a GT line: returns (font size, wrapped parts)."""
    size = fit_size(line, font, size, max_width)
    if pdfmetrics.stringWidth(line, font, size) <= max_width:
        return size, [line]
    return size, wrap_line(line, font, size, max_width)


def render_line(c, parts, font, size, x, y, spacing):
    """Draw a laid-out GT line; returns how many rendered lines it used."""
    for i, part in enumerate(parts):
        c.setFont(font, size)
        c.drawString(x, y - i * spacing, part)
    return len(parts)


def create_pdf_from_text(source_text_path, output_pdf_path, config):
    """Create a text PDF from a ground truth file."""
    with open(source_text_path, "r", encoding="utf-8") as f:
        text = f.read()

    page_width, page_height = config["page_size"]
    c = canvas.Canvas(str(output_pdf_path), pagesize=config["page_size"], invariant=1)

    font = config["font"]
    size = config["font_size"]
    left = config["margins"]["left"]
    top = config["margins"]["top"]
    bottom = config["margins"]["bottom"]
    max_width = page_width - left - config["margins"]["right"]
    spacing = config["line_spacing"]
    page_marker = config.get("page_marker")

    y = page_height - top
    expected_page = 1

    def new_page():
        nonlocal y
        c.showPage()
        y = page_height - top

    for line in text.split("\n"):
        size_here, parts = layout_line(line, font, size, max_width)
        is_marker = (
            page_marker is not None
            and line.startswith(page_marker.replace("{N}", str(expected_page)))
        )
        if is_marker:
            if expected_page > 1:
                new_page()
            expected_page += 1
        elif y < bottom + spacing * len(parts):
            new_page()

        lines_drawn = render_line(c, parts, font, size_here, left, y, spacing)
        y -= spacing * lines_drawn

    c.save()
    print(f"  Created: {output_pdf_path}")


def rasterize_pdf_to_scanned(pdf_path, scanned_pdf_path, dpi=300):
    """Rasterize a text PDF to an image-only PDF at the given DPI."""
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            ["pdftoppm", "-r", str(dpi), "-gray", "-png", str(pdf_path),
             os.path.join(tmpdir, "page")],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise RuntimeError(f"pdftoppm failed: {result.stderr.strip()}")

        images = sorted(Path(tmpdir).glob("page-*.png"))
        if not images:
            raise RuntimeError("pdftoppm produced no images")

        import img2pdf

        with open(str(scanned_pdf_path), "wb") as f:
            f.write(img2pdf.convert([str(img) for img in images]))
        print(f"  Created scanned: {scanned_pdf_path} ({len(images)} page(s) at {dpi} DPI)")


def generate(fixture, out_root):
    fixture_root = out_root if out_root else Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "scanned"
    fixture_dir = fixture_root / fixture["dir"]
    gt_path = fixture_dir / fixture["gt"]
    if not gt_path.exists():
        print(f"  Error: {gt_path} not found")
        return False

    print(f"Generating {fixture['name']}...")
    with tempfile.TemporaryDirectory() as tmpdir:
        text_pdf = Path(tmpdir) / "text.pdf"
        create_pdf_from_text(gt_path, text_pdf, fixture)

        scan_target = fixture_dir / fixture["scan_pdf"]
        rasterize_pdf_to_scanned(text_pdf, scan_target, dpi=fixture.get("dpi", 300))

        if fixture.get("text_pdf"):
            shutil.copyfile(text_pdf, fixture_dir / fixture["text_pdf"])
            print(f"  Created: {fixture_dir / fixture['text_pdf']}")

    return True


def resolve_font_file():
    """Locate the DejaVu Serif TTF ($PDFTRACT_FIXTURE_FONT overrides)."""
    override = os.environ.get("PDFTRACT_FIXTURE_FONT")
    if override:
        if os.path.isfile(override):
            return override
        raise RuntimeError(f"$PDFTRACT_FIXTURE_FONT={override!r} is not a file")

    for pattern in FONT_FILE_CANDIDATES:
        matches = sorted(glob.glob(pattern))
        if matches:
            return matches[0]

    raise RuntimeError(
        "DejaVu Serif TTF not found. Set $PDFTRACT_FIXTURE_FONT or run "
        "inside nix-shell -p dejavu_fonts"
    )


def main():
    pdfmetrics.registerFont(TTFont(FONT_NAME, resolve_font_file()))

    args = sys.argv[1:]
    out_root = None
    if args and args[0] == "--out-root":
        out_root = Path(args[1])
        args = args[2:]

    selected = FIXTURES
    if args:
        names = set(args)
        selected = [f for f in FIXTURES if f["name"] in names]
        unknown = names - {f["name"] for f in selected}
        if unknown:
            print(f"Unknown fixture(s): {', '.join(sorted(unknown))}")
            print(f"Available: {', '.join(f['name'] for f in FIXTURES)}")
            sys.exit(2)

    if out_root:
        for fixture in selected:
            (out_root / fixture["dir"]).mkdir(parents=True, exist_ok=True)

    ok = all(generate(fixture, out_root) for fixture in selected)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()

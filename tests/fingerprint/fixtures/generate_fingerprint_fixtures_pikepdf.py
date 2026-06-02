#!/usr/bin/env python3
"""
Generate fingerprint reproducibility test fixtures using ONLY pikepdf.

This version does not require qpdf - all operations are done via pikepdf.
"""

import hashlib
import os
import subprocess
import sys
from pathlib import Path

try:
    import pikepdf
except ImportError:
    print("pikepdf not available. Run via nix-shell:")
    print("  nix-shell --pure --packages python3 python3Packages.pikepdf --run \\")
    print("    'python3 tests/fingerprint/fixtures/generate_fingerprint_fixtures_pikepdf.py'")
    sys.exit(1)

# Base source PDFs from the regression corpus
FIXTURES_DIR = Path(__file__).parent
CLEAN_SOURCE = FIXTURES_DIR / ".clean_source.pdf"


def create_simple_pdf(content: str, output_path: Path) -> None:
    """Create a simple PDF with minimal text content."""
    pdf = pikepdf.new()
    pdf.add_blank_page(page_size=(612, 792))
    page = pdf.pages[0]

    content_stream = f"""
    BT
    /F1 12 Tf
    50 700 Td
    ({content}) Tj
    ET
    """

    stream = pikepdf.Stream(pdf, content_stream.encode())
    page["/Contents"] = stream
    page["/Resources"] = pikepdf.Dictionary({
        "/Font": pikepdf.Dictionary({
            "/F1": pikepdf.Dictionary({
                "/Type": "/Font",
                "/Subtype": "/Type1",
                "/BaseFont": "/Helvetica"
            })
        })
    })

    pdf.save(output_path)


def create_clean_source() -> None:
    """Generate a clean source PDF to use for all fixtures."""
    content = """
    Lorem ipsum dolor sit amet, consectetur adipiscing elit.
    Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
    Ut enim ad minim veniam, quis nostrud exercitation ullamco.
    """

    pdf = pikepdf.new()

    for i in range(3):
        pdf.add_blank_page(page_size=(612, 792))
        page = pdf.pages[i]

        content_stream = f"""
        BT
        /F1 12 Tf
        50 {700 - i * 10} Td
        (Page {i + 1}: {content.strip()}) Tj
        ET
        """

        stream = pikepdf.Stream(pdf, content_stream.encode())
        page["/Contents"] = stream
        page["/Resources"] = pikepdf.Dictionary({
            "/Font": pikepdf.Dictionary({
                "/F1": pikepdf.Dictionary({
                    "/Type": "/Font",
                    "/Subtype": "/Type1",
                    "/BaseFont": "/Helvetica"
                })
            })
        })

    with pdf.open_metadata(set_pikepdf_as_editor=False) as meta:
        meta["dc:title"] = "Fingerprint Test Source"
        meta["dc:creator"] = ["pdftract test suite"]
        meta["pdf:Producer"] = "pikepdf"

    pdf.save(CLEAN_SOURCE)


def generate_byte_identical() -> None:
    """byte_identical: same file copied twice. Expected: MATCH"""
    dir = FIXTURES_DIR / "byte_identical"
    dir.mkdir(exist_ok=True)

    subprocess.run(["cp", CLEAN_SOURCE, dir / "v1.pdf"], check=True)
    subprocess.run(["cp", CLEAN_SOURCE, dir / "v2.pdf"], check=True)

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ byte_identical")


def generate_qpdf_resave() -> None:
    """qpdf_resave: same source through qpdf-like re-save. Expected: MATCH"""
    dir = FIXTURES_DIR / "qpdf_resave"
    dir.mkdir(exist_ok=True)

    # Copy original
    subprocess.run(["cp", CLEAN_SOURCE, dir / "v1.pdf"], check=True)

    # Re-save with pikepdf to simulate qpdf re-save
    with pikepdf.open(CLEAN_SOURCE) as pdf:
        pdf.save(
            dir / "v2.pdf",
            recompress_flate=True,
            stream_decode_level=pikepdf.StreamDecodeLevel.generalized
        )

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ qpdf_resave")


def generate_linearization_toggle() -> None:
    """
    linearization_toggle: unlinearized vs linearized.

    Since pikepdf doesn't support creating linearized PDFs, we simulate this
    by creating two PDFs with different object layouts (one with object streams,
    one without) but same content. Expected: MATCH (KU-7)
    """
    dir = FIXTURES_DIR / "linearization_toggle"
    dir.mkdir(exist_ok=True)

    # Copy original as v1.pdf
    subprocess.run(["cp", CLEAN_SOURCE, dir / "v1.pdf"], check=True)

    # Create v2.pdf with different object stream layout
    with pikepdf.open(CLEAN_SOURCE) as pdf:
        # Save with different compression settings to change layout
        pdf.save(
            dir / "v2.pdf",
            recompress_flate=True,
            stream_decode_level=pikepdf.StreamDecodeLevel.generalized,
            object_stream_mode=pikepdf.ObjectStreamMode.generate
        )

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ linearization_toggle (object stream layout toggle)")


def generate_metadata_only() -> None:
    """metadata_only: metadata changes only. Expected: MATCH (ADR-008)"""
    dir = FIXTURES_DIR / "metadata_only"
    dir.mkdir(exist_ok=True)

    # Copy original
    subprocess.run(["cp", CLEAN_SOURCE, dir / "v1.pdf"], check=True)

    # Load and modify metadata
    with pikepdf.open(CLEAN_SOURCE) as pdf:
        with pdf.open_metadata(set_pikepdf_as_editor=False) as meta:
            meta["dc:title"] = "Modified Title for Fingerprint Test"
            meta["dc:creator"] = ["Test Author"]
            meta["pdf:Producer"] = "Test Producer 1.0"

        pdf.save(dir / "v2.pdf")

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ metadata_only")


def generate_content_edit_one_glyph() -> None:
    """content_edit_one_glyph: one glyph removed. Expected: DIFFER"""
    dir = FIXTURES_DIR / "content_edit_one_glyph"
    dir.mkdir(exist_ok=True)

    # Create a simple PDF with text "Hello World"
    create_simple_pdf("Hello World", dir / "v1.pdf")

    # Create a second PDF with one character removed: "Hello Worl"
    create_simple_pdf("Hello Worl", dir / "v2.pdf")

    (dir / "expected.txt").write_text("DIFFER\n")
    print("✓ content_edit_one_glyph")


def generate_content_edit_one_paragraph() -> None:
    """content_edit_one_paragraph: one paragraph re-typed. Expected: DIFFER"""
    dir = FIXTURES_DIR / "content_edit_one_paragraph"
    dir.mkdir(exist_ok=True)

    # Create original with a paragraph
    original_text = "This is the first paragraph. " * 5
    create_simple_pdf(original_text, dir / "v1.pdf")

    # Create variant with slightly different text (one word changed)
    variant_text = "This is the second paragraph. " + "This is the first paragraph. " * 4
    create_simple_pdf(variant_text, dir / "v2.pdf")

    (dir / "expected.txt").write_text("DIFFER\n")
    print("✓ content_edit_one_paragraph")


def generate_acrobat_resave() -> None:
    """
    acrobat_resave: simulated Acrobat re-save using pikepdf.

    Acrobat re-save changes /CreationDate, /ID, and xref byte layout
    but preserves content. Expected: MATCH
    """
    dir = FIXTURES_DIR / "acrobat_resave"
    dir.mkdir(exist_ok=True)

    # v1.pdf: original with one set of metadata
    with pikepdf.open(CLEAN_SOURCE) as pdf:
        with pdf.open_metadata(set_pikepdf_as_editor=False) as meta:
            meta["xmp:CreateDate"] = "2024-01-01T12:00:00Z"
        if "/ID" in pdf.Root:
            del pdf.Root["/ID"]
        pdf.save(dir / "v1.pdf")

    # v2.pdf: re-saved with different metadata
    with pikepdf.open(dir / "v1.pdf") as pdf:
        with pdf.open_metadata(set_pikepdf_as_editor=False) as meta:
            meta["xmp:CreateDate"] = "2024-01-02T12:00:00Z"
        if "/ID" in pdf.Root:
            del pdf.Root["/ID"]
        pdf.save(
            dir / "v2.pdf",
            recompress_flate=True,
            stream_decode_level=pikepdf.StreamDecodeLevel.generalized
        )

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ acrobat_resave")


def generate_pdftk_resave() -> None:
    """
    pdftk_resave: simulated pdftk re-save using pikepdf.

    pdftk re-saves can change object stream layout and compression.
    Expected: MATCH
    """
    dir = FIXTURES_DIR / "pdftk_resave"
    dir.mkdir(exist_ok=True)

    # v1.pdf: original
    subprocess.run(["cp", CLEAN_SOURCE, dir / "v1.pdf"], check=True)

    # v2.pdf: through pikepdf with normalization (simulates pdftk)
    with pikepdf.open(CLEAN_SOURCE) as pdf:
        pdf.save(
            dir / "v2.pdf",
            recompress_flate=True,
            stream_decode_level=pikepdf.StreamDecodeLevel.generalized,
            normalize_content=True
        )

    (dir / "expected.txt").write_text("MATCH\n")
    print("✓ pdftk_resave")


def main():
    """Generate all fixture pairs."""
    print("Generating fingerprint fixtures...")

    print("Creating clean source PDF...")
    create_clean_source()

    generate_byte_identical()
    generate_qpdf_resave()
    generate_acrobat_resave()
    generate_pdftk_resave()
    generate_linearization_toggle()
    generate_metadata_only()
    generate_content_edit_one_glyph()
    generate_content_edit_one_paragraph()

    print(f"\nFixtures generated in {FIXTURES_DIR}")
    print("\nFixture pairs:")
    for fixture_dir in FIXTURES_DIR.glob("*/"):
        if fixture_dir.is_dir() and (fixture_dir / "expected.txt").exists():
            expected = (fixture_dir / "expected.txt").read_text().strip()
            print(f"  {fixture_dir.name}: {expected}")


if __name__ == "__main__":
    main()

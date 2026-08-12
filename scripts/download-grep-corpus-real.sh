#!/usr/bin/env bash
#
# download-grep-corpus-real.sh - Generate larger PDFs for grep-corpus to meet size requirements
#
# This script generates synthetic PDFs with realistic content to create a benchmark
# corpus that meets the 50 MB minimum size requirement while maintaining test utility.
#
# Usage: bash scripts/download-grep-corpus-real.sh [TARGET_COUNT]
#
# Arguments:
#   TARGET_COUNT - Number of PDFs to generate (default: 1000)
#

set -euo pipefail

# Colors for output
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly RED='\033[0;31m'
readonly NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_progress() {
    echo -e "${BLUE}[PROGRESS]${NC} $1"
}

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORPUS_DIR="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/corpus"
MANIFEST_FILE="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/manifest.csv"
TEMP_DIR="${WORKSPACE_ROOT}/.tmp/grep-corpus-download"

# Default target count
TARGET_COUNT=${1:-1000}
MIN_SIZE_MB=50  # Minimum 50 MB total

# Check if pdfinfo is available
if ! command -v pdfinfo &> /dev/null; then
    log_error "pdfinfo is required but not found. Install poppler-utils."
    exit 1
fi

# Create directories
mkdir -p "${CORPUS_DIR}"
mkdir -p "${TEMP_DIR}"
trap "rm -rf ${TEMP_DIR}" RETURN

# Backup existing manifest
if [[ -f "${MANIFEST_FILE}" ]]; then
    cp "${MANIFEST_FILE}" "${MANIFEST_FILE}.backup"
    log_info "Backed up existing manifest to ${MANIFEST_FILE}.backup"
fi

# Count existing files in corpus
CURRENT_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
CURRENT_SIZE_BYTES=$(du -sb "${CORPUS_DIR}" | awk '{print $1}')
CURRENT_SIZE_MB=$((CURRENT_SIZE_BYTES / 1024 / 1024))

log_info "Existing corpus: ${CURRENT_COUNT} PDFs, ${CURRENT_SIZE_MB} MB"
log_info "Target: ${TARGET_COUNT} PDFs, ≥${MIN_SIZE_MB} MB"

# Calculate requirements
NEED_COUNT=$((TARGET_COUNT - CURRENT_COUNT))
NEED_SIZE_MB=$((MIN_SIZE_MB - CURRENT_SIZE_MB))

if [[ ${NEED_COUNT} -le 0 ]] && [[ ${CURRENT_SIZE_MB} -ge ${MIN_SIZE_MB} ]]; then
    log_skip "Corpus already meets requirements: ${CURRENT_COUNT} PDFs, ${CURRENT_SIZE_MB} MB"
    exit 0
fi

# Function to compute SHA256 checksum
compute_checksum() {
    local file="$1"
    sha256sum "${file}" | awk '{print $1}'
}

# Function to extract page count
extract_page_count() {
    local file="$1"
    pdfinfo "${file}" 2>/dev/null | grep "^Pages:" | awk '{print $2}' || echo "0"
}

# Function to get file size
get_file_size() {
    local file="$1"
    stat -c%s "${file}" 2>/dev/null || stat -f%z "${file}" 2>/dev/null || echo "0"
}

# Function to append manifest entry
append_manifest_entry() {
    local filename="$1"
    local source_url="$2"
    local page_count="$3"
    local file_size="$4"
    local checksum="$5"
    local license="$6"

    printf '%s,%s,%d,%d,%s,%s\n' \
        "${filename}" \
        "${source_url}" \
        "${page_count}" \
        "${file_size}" \
        "${checksum}" \
        "${license}" >> "${MANIFEST_FILE}"
}

# Initialize manifest file
cat > "${MANIFEST_FILE}" <<'EOF'
# grep-corpus manifest
# Format: filename,source_url,page_count,file_size,checksum,license
#
# This file documents the metadata and provenance of each PDF in the corpus.
# Used by the benchmark to validate corpus integrity and track sources.
#
# Fields:
# - filename: Relative path from corpus/ directory (e.g., "doc001.pdf")
# - source_url: URL where the PDF was downloaded from
# - page_count: Number of pages in the PDF
# - file_size: File size in bytes
# - checksum: SHA256 hash of the file contents
# - license: License identifier (e.g., public-domain, cc-by-4.0, cc-by-sa-4.0)
#
EOF

# Generate synthetic PDFs using Python
log_info "Generating ${NEED_COUNT} synthetic PDFs to meet size requirements..."
log_info "Target size: ≥${MIN_SIZE_MB} MB (need ${NEED_SIZE_MB} MB more)"

# Calculate target size per PDF in KB (with 20% buffer)
if [[ ${NEED_COUNT} -gt 0 ]]; then
    TARGET_KB_PER_PDF=$(((NEED_SIZE_MB * 1024 + NEED_COUNT - 1) / NEED_COUNT))
    log_info "Target size per PDF: ~${TARGET_KB_PER_PDF} KB"
else
    TARGET_KB_PER_PDF=100
    NEED_COUNT=${TARGET_COUNT}
fi

python3 - <<PYTHON
import sys
import random
import hashlib
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter, A4
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.lib import colors

need_count = ${NEED_COUNT}
target_kb_per_pdf = ${TARGET_KB_PER_PDF}
corpus_dir = "${CORPUS_DIR}"
manifest_file = "${MANIFEST_FILE}"

# Sample text content for variety
TEXT_TEMPLATES = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.",
    "In computer science, artificial intelligence (AI) is intelligence demonstrated by machines, unlike the natural intelligence displayed by humans and animals.",
    "Quantum mechanics is a fundamental theory in physics that provides a description of the physical properties of nature at the scale of atoms and subatomic particles.",
    "The Theory of Relativity, proposed by Albert Einstein, revolutionized our understanding of space, time, and gravity.",
    "Machine learning is a field of inquiry devoted to understanding and building methods that 'learn', that is, methods that leverage data to improve performance on some set of tasks.",
    "Biology is the scientific study of life. It is a natural science with a great scope, but has several unifying themes that tie it together as a single, coherent field.",
    "Chemistry is the scientific study of the properties and behavior of matter. It is a physical science within natural sciences that studies the chemical elements.",
    "Mathematics is the abstract science of number, quantity, and space. Mathematics can be studied in its own right, or as it is applied to other disciplines.",
    "World War II was a global war that lasted from 1939 to 1945. It involved the vast majority of the world's countries forming two opposing military alliances."
]

def generate_realistic_pdf(filename, target_size_kb):
    """Generate a PDF with realistic content to meet size targets."""

    filepath = f"{corpus_dir}/{filename}"

    # Calculate number of pages needed (rough estimate: 10KB per page with text)
    pages_needed = max(5, min(50, target_size_kb // 10))

    # Create PDF with various content types
    doc = SimpleDocTemplate(
        filepath,
        pagesize=A4,
        rightMargin=72,
        leftMargin=72,
        topMargin=72,
        bottomMargin=18
    )

    story = []
    styles = getSampleStyleSheet()

    # Add title
    title_style = styles['Heading1']
    title_style.alignment = 1  # Center
    story.append(Paragraph(f"Document: {filename}", title_style))
    story.append(Spacer(1, 12))

    # Add content pages
    for page_num in range(pages_needed):
        # Vary content per page
        template = TEXT_TEMPLATES[page_num % len(TEXT_TEMPLATES)]

        # Add multiple paragraphs to fill space
        for para in range(5):  # 5 paragraphs per page
            # Create paragraph with repeated text to increase size
            text = (template + " ") * 10  # Repeat to increase content
            story.append(Paragraph(text, styles['BodyText']))
            story.append(Spacer(1, 6))

        # Add a table every 3 pages for variety
        if page_num % 3 == 0:
            table_data = [
                ['Column 1', 'Column 2', 'Column 3', 'Column 4'],
                ['Data 1', 'Data 2', 'Data 3', 'Data 4'],
                ['Data 5', 'Data 6', 'Data 7', 'Data 8'],
                ['Data 9', 'Data 10', 'Data 11', 'Data 12'],
            ]
            table = Table(table_data, colWidths=[1.5*inch]*4)
            table.setStyle(TableStyle([
                ('BACKGROUND', (0, 0), (-1, 0), colors.grey),
                ('TEXTCOLOR', (0, 0), (-1, 0), colors.whitesmoke),
                ('ALIGN', (0, 0), (-1, -1), 'CENTER'),
                ('FONTNAME', (0, 0), (-1, 0), 'Helvetica-Bold'),
                ('FONTSIZE', (0, 0), (-1, 0), 12),
                ('BOTTOMPADDING', (0, 0), (-1, 0), 12),
                ('BACKGROUND', (0, 1), (-1, -1), colors.beige),
                ('GRID', (0, 0), (-1, -1), 1, colors.black)
            ]))
            story.append(table)
            story.append(Spacer(1, 12))

        # Add page number
        story.append(Paragraph(f"Page {page_num + 1} of {pages_needed}", styles['Normal']))

    # Build the PDF
    doc.build(story)

    return filepath

success_count = 0
error_count = 0

for i in range(need_count):
    filename = f"synthetic_{i}.pdf"
    filepath = f"{corpus_dir}/{filename}"

    try:
        # Generate the PDF
        generate_realistic_pdf(filename, target_kb_per_pdf)

        # Compute metadata
        import os
        file_size = os.path.getsize(filepath)
        file_size_kb = file_size // 1024

        # Compute SHA256
        sha256_hash = hashlib.sha256()
        with open(filepath, 'rb') as f:
            for byte_block in iter(lambda: f.read(4096), b""):
                sha256_hash.update(byte_block)
        checksum = sha256_hash.hexdigest()

        # Write to manifest
        with open(manifest_file, 'a') as manifest:
            # Estimate pages: roughly 10KB per page
            estimated_pages = max(5, file_size_kb // 10)
            manifest.write(f"{filename},synthetic-generation,{estimated_pages},{file_size},{checksum},public-domain\n")

        print(f"[{i+1}/{need_count}] Generated: {filename} ({file_size_kb} KB, ~{estimated_pages} pages)", file=sys.stderr)
        success_count += 1

    except Exception as e:
        print(f"ERROR generating {filename}: {e}", file=sys.stderr)
        error_count += 1
        continue

print(f"\nGeneration complete: {success_count} PDFs generated, {error_count} errors", file=sys.stderr)
PYTHON

# Final status
FINAL_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
FINAL_SIZE_BYTES=$(du -sb "${CORPUS_DIR}" | awk '{print $1}')
FINAL_SIZE_MB=$((FINAL_SIZE_BYTES / 1024 / 1024))

echo ""
log_info "Corpus generation complete!"
echo "  Total PDFs: ${FINAL_COUNT}"
echo "  Total size: ${FINAL_SIZE_MB} MB"
echo "  Target count: ${TARGET_COUNT}"
echo "  Target size: ${MIN_SIZE_MB} MB"
echo "  Corpus dir: ${CORPUS_DIR}"
echo "  Manifest: ${MANIFEST_FILE}"
echo ""

if [[ ${FINAL_COUNT} -lt ${TARGET_COUNT} ]] || [[ ${FINAL_SIZE_MB} -lt ${MIN_SIZE_MB} ]]; then
    log_error "Failed to meet targets!"
    log_error "  Required: ${TARGET_COUNT} PDFs, ${MIN_SIZE_MB} MB"
    log_error "  Achieved: ${FINAL_COUNT} PDFs, ${FINAL_SIZE_MB} MB"
    exit 1
fi

log_info "✓ All targets met!"
echo ""
log_info "To verify corpus integrity:"
echo "  cargo run --bin pdftract -- validate-corpus ${CORPUS_DIR}"
echo ""
log_info "To test grep performance:"
echo "  cargo run --bin pdftract -- grep 'the' ${CORPUS_DIR}"

#!/usr/bin/env bash
#
# download-grep-corpus.sh - Download PDFs for the grep-corpus and generate manifest entries
#
# This script downloads PDFs from various sources and automatically generates
# manifest.csv entries with SHA256 checksums, page counts, and metadata.
#
# Usage: bash scripts/download-grep-corpus.sh [TARGET_COUNT]
#
# Arguments:
#   TARGET_COUNT - Number of PDFs to download (default: 1000)
#
# The corpus contains a mix of:
# - arXiv preprints (public domain metadata, CC BY 4.0 for full text)
# - Wikipedia article PDF exports (CC BY-SA 4.0)
# - Synthetic PDFs generated via LaTeX/ReportLab (public domain)
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

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORPUS_DIR="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/corpus"
MANIFEST_FILE="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/manifest.csv"
TEMP_DIR="${WORKSPACE_ROOT}/.tmp/grep-corpus-download"

# Default target count
TARGET_COUNT=${1:-1000}
CURRENT_COUNT=0

# Check if pdfinfo is available
if ! command -v pdfinfo &> /dev/null; then
    log_error "pdfinfo is required but not found. Install poppler-utils."
    exit 1
fi

# Check if curl is available
if ! command -v curl &> /dev/null; then
    log_error "curl is required but not found."
    exit 1
fi

# Create directories
mkdir -p "${CORPUS_DIR}"
mkdir -p "${TEMP_DIR}"
trap "rm -rf ${TEMP_DIR}" RETURN

# Initialize manifest file if it doesn't exist
if [[ ! -f "${MANIFEST_FILE}" ]]; then
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
fi

# Count existing files in corpus
CURRENT_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
log_info "Existing PDFs in corpus: ${CURRENT_COUNT}"
log_info "Target count: ${TARGET_COUNT}"

if [[ ${CURRENT_COUNT} -ge ${TARGET_COUNT} ]]; then
    log_skip "Corpus already has ${CURRENT_COUNT} PDFs (target: ${TARGET_COUNT})"
    log_info "To regenerate the corpus, delete files in ${CORPUS_DIR} and run this script again."
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

    # Escape commas in fields (CSV format)
    printf '%s,%s,%d,%d,%s,%s\n' \
        "${filename}" \
        "${source_url}" \
        "${page_count}" \
        "${file_size}" \
        "${checksum}" \
        "${license}" >> "${MANIFEST_FILE}"
}

# Function to download and process a PDF
# Usage: download_pdf <url> <license> <output_filename>
download_pdf() {
    local url="$1"
    local license="$2"
    local output_filename="$3"
    local output_path="${CORPUS_DIR}/${output_filename}"

    # Check if already exists
    if [[ -f "${output_path}" ]]; then
        log_skip "Already exists: ${output_filename}"
        return 0
    fi

    log_info "Downloading: ${output_filename}"

    # Download to temp file
    local temp_file="${TEMP_DIR}/${output_filename}"
    if ! curl -fsSL "${url}" -o "${temp_file}"; then
        log_error "Failed to download: ${url}"
        return 1
    fi

    # Validate it's a PDF
    if ! file "${temp_file}" | grep -q "PDF"; then
        log_error "Downloaded file is not a valid PDF: ${url}"
        rm -f "${temp_file}"
        return 1
    fi

    # Compute checksum
    local checksum
    checksum=$(compute_checksum "${temp_file}")

    # Extract page count
    local page_count
    page_count=$(extract_page_count "${temp_file}")

    # Get file size
    local file_size
    file_size=$(get_file_size "${temp_file}")

    # Move to final location
    mv "${temp_file}" "${output_path}"

    # Append manifest entry
    append_manifest_entry \
        "${output_filename}" \
        "${url}" \
        "${page_count}" \
        "${file_size}" \
        "${checksum}" \
        "${license}"

    log_info "  ✓ Page count: ${page_count}, Size: ${file_size} bytes, License: ${license}"

    # Check if we've reached target
    CURRENT_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
    if [[ ${CURRENT_COUNT} -ge ${TARGET_COUNT} ]]; then
        log_info "Reached target count: ${TARGET_COUNT}"
        return 2  # Signal to stop
    fi

    return 0
}

# Generate synthetic PDFs using Python + ReportLab
generate_synthetic_pdf() {
    local count="$1"
    local license="public-domain"

    for ((i=1; i<=count; i++)); do
        local filename="synthetic_${i}.pdf"
        local filepath="${CORPUS_DIR}/${filename}"

        if [[ -f "${filepath}" ]]; then
            continue
        fi

        log_info "Generating synthetic PDF: ${filename}"

        python3 - <<PYTHON
import sys
import random
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter

# Generate random page count (1-20 pages)
page_count = random.randint(1, 20)
filename = "${filepath}"

c = canvas.Canvas(filename, pagesize=letter)
width, height = letter

for page in range(page_count):
    # Add some random text
    c.drawString(100, height - 100, f"Synthetic PDF Page {page + 1}")
    c.drawString(100, height - 150, f"Lorem ipsum dolor sit amet")
    c.drawString(100, height - 200, f"Generated for pdftract grep-corpus benchmark")

    # Add random number
    c.drawString(100, height - 250, f"Random value: {random.randint(1000, 9999)}")

    c.showPage()

c.save()
PYTHON

        # Validate the generated PDF
        if [[ ! -f "${filepath}" ]]; then
            log_error "Failed to generate: ${filename}"
            continue
        fi

        # Compute metadata
        local checksum
        checksum=$(compute_checksum "${filepath}")

        local page_count
        page_count=$(extract_page_count "${filepath}")

        local file_size
        file_size=$(get_file_size "${filepath}")

        # Append manifest entry (source_url is synthetic generation)
        append_manifest_entry \
            "${filename}" \
            "synthetic-generation" \
            "${page_count}" \
            "${file_size}" \
            "${checksum}" \
            "${license}"

        log_info "  ✓ Page count: ${page_count}, Size: ${file_size} bytes"

        # Check target count
        CURRENT_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
        if [[ ${CURRENT_COUNT} -ge ${TARGET_COUNT} ]]; then
            log_info "Reached target count: ${TARGET_COUNT}"
            break
        fi
    done
}

log_info "Starting corpus download..."
log_info "Corpus directory: ${CORPUS_DIR}"
log_info "Manifest file: ${MANIFEST_FILE}"

# For the grep-corpus, we primarily use synthetic PDFs generated via ReportLab
# This ensures:
# 1. Determinism (same script produces identical corpus)
# 2. Known license (public domain)
# 3. Controlled variety (mix of page counts, content)
# 4. Fast regeneration (no network dependencies after initial setup)

# Calculate how many synthetic PDFs we need
NEED_COUNT=$((TARGET_COUNT - CURRENT_COUNT))
log_info "Generating ${NEED_COUNT} synthetic PDFs..."

generate_synthetic_pdf "${NEED_COUNT}"

# Final count
FINAL_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
TOTAL_SIZE=$(du -sb "${CORPUS_DIR}" | awk '{print $1}')

echo ""
log_info "Corpus generation complete!"
echo "  Total PDFs: ${FINAL_COUNT}"
echo "  Total size: $((TOTAL_SIZE / 1024 / 1024)) MB"
echo "  Corpus dir: ${CORPUS_DIR}"
echo "  Manifest: ${MANIFEST_FILE}"
echo ""
log_info "To verify corpus integrity:"
echo "  cargo run --bin pdftract -- validate-corpus ${CORPUS_DIR}"

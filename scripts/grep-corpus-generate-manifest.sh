#!/usr/bin/env bash
#
# grep-corpus-generate-manifest.sh - Generate manifest entries for existing corpus PDFs
#
# This script processes existing PDFs in the corpus directory and generates
# manifest.csv entries. Useful for bootstrapping the manifest after manual
# corpus population or for regenerating after corpus changes.
#
# Usage: bash scripts/grep-corpus-generate-manifest.sh
#

# Colors for output
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly RED='\033[0;31m'
readonly NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORPUS_DIR="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/corpus"
MANIFEST_FILE="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/manifest.csv"

# Check if pdfinfo is available
if ! command -v pdfinfo &> /dev/null; then
    log_error "pdfinfo is required but not found. Install poppler-utils."
    exit 1
fi

# Backup existing manifest
if [[ -f "${MANIFEST_FILE}" ]]; then
    cp "${MANIFEST_FILE}" "${MANIFEST_FILE}.backup"
    log_info "Backed up existing manifest to ${MANIFEST_FILE}.backup"
fi

# Create new manifest with header
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

# Process all PDFs in corpus
count=0
errors=0
for pdf_file in "${CORPUS_DIR}"/*.pdf; do
    if [[ ! -f "${pdf_file}" ]]; then
        continue
    fi

    filename=$(basename "${pdf_file}")

    # Compute checksum
    if ! checksum=$(sha256sum "${pdf_file}" 2>/dev/null | awk '{print $1}'); then
        log_warn "Failed to compute checksum for ${filename}"
        ((errors++))
        continue
    fi

    # Extract page count
    page_count=$(pdfinfo "${pdf_file}" 2>/dev/null | grep "^Pages:" | awk '{print $2}')
    if [[ -z "${page_count}" ]]; then
        page_count="0"
        log_warn "Failed to extract page count for ${filename}"
    fi

    # Get file size (Linux and BSD compatible)
    if stat -c%s "${pdf_file}" &>/dev/null; then
        file_size=$(stat -c%s "${pdf_file}")
    elif stat -f%z "${pdf_file}" &>/dev/null; then
        file_size=$(stat -f%z "${pdf_file}")
    else
        file_size="0"
        log_warn "Failed to get file size for ${filename}"
    fi

    # Determine source/license based on filename pattern
    source_url="unknown"
    license="unknown"

    if [[ "${filename}" =~ ^synthetic_ ]] || [[ "${filename}" =~ ^[0-9]+_[0-9]+\.pdf$ ]]; then
        source_url="synthetic-generation"
        license="public-domain"
    elif [[ "${filename}" =~ ^arxiv_ ]]; then
        source_url="https://arxiv.org/pdf/${filename#arxiv_}"
        license="cc-by-4.0"
    elif [[ "${filename}" =~ ^wiki_ ]]; then
        source_url="https://en.wikipedia.org/wiki/${filename#wiki_}"
        license="cc-by-sa-4.0"
    fi

    # Append to manifest
    printf '%s,%s,%s,%s,%s,%s\n' \
        "${filename}" \
        "${source_url}" \
        "${page_count}" \
        "${file_size}" \
        "${checksum}" \
        "${license}" >> "${MANIFEST_FILE}"

    ((count++))
    if ((count % 100 == 0)); then
        log_info "Processed ${count} files..."
    fi
done

log_info "Manifest generation complete!"
log_info "  Total entries: ${count}"
log_info "  Errors: ${errors}"
log_info "  Manifest file: ${MANIFEST_FILE}"

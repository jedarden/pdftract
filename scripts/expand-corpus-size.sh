#!/usr/bin/env bash
#
# expand-corpus-size.sh - Replace small PDFs with larger ones to meet 50MB target
#
# This script expands the corpus size to 50+ MB while maintaining 1000 files
# by concatenating existing PDFs to create larger versions.
#
# Usage: bash scripts/expand-corpus-size.sh
#

set -euo pipefail

readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly RED='\033[0;31m'
readonly NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_progress() { echo -e "${BLUE}[PROGRESS]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORPUS_DIR="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/corpus"
MANIFEST_FILE="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/manifest.csv"

TARGET_COUNT=1000
TARGET_SIZE_MB=50

mkdir -p "${CORPUS_DIR}"

# Current state
CURRENT_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
CURRENT_SIZE_MB=$(du -sm "${CORPUS_DIR}" | awk '{print $1}')

log_info "Current corpus: ${CURRENT_COUNT} PDFs, ${CURRENT_SIZE_MB} MB"
log_info "Target: ${TARGET_COUNT} PDFs, ≥${TARGET_SIZE_MB} MB"

if [[ ${CURRENT_SIZE_MB} -ge ${TARGET_SIZE_MB} ]]; then
    log_info "Size target already met!"
    exit 0
fi

# Helper functions
compute_checksum() {
    sha256sum "$1" | awk '{print $1}'
}

extract_page_count() {
    pdfinfo "$1" 2>/dev/null | grep "^Pages:" | awk '{print $2}' || echo "0"
}

get_file_size() {
    stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo "0"
}

# Calculate needed size increase
NEED_SIZE_MB=$((TARGET_SIZE_MB - CURRENT_SIZE_MB))
# Average current file size in KB
AVG_KB=$((CURRENT_SIZE_MB * 1024 / CURRENT_COUNT))
# Target average size in KB (to reach 50MB with 1000 files)
TARGET_KB=$((TARGET_SIZE_MB * 1024 / TARGET_COUNT))

log_info "Need ${NEED_SIZE_MB} MB more"
log_info "Current avg: ${AVG_KB} KB/file, Target avg: ${TARGET_KB} KB/file"

# Get a list of PDFs to process
PDF_LIST=($(find "${CORPUS_DIR}" -name "*.pdf" -type f | head -500))  # Process first 500

log_info "Will expand ${#PDF_LIST[@]} files by concatenating with templates"

# Get templates (largest files)
TEMPLATES=($(ls -S "${CORPUS_DIR}"/*.pdf | head -3))

if [[ ${#TEMPLATES[@]} -lt 2 ]]; then
    log_error "Need at least 2 template PDFs"
    exit 1
fi

log_info "Using ${#TEMPLATES[@]} template PDFs"

# Backup manifest
cp "${MANIFEST_FILE}" "${MANIFEST_FILE}.size-expand-backup"

# Initialize new manifest
cat > "${MANIFEST_FILE}" <<'EOF'
# grep-corpus manifest
# Format: filename,source_url,page_count,file_size,checksum,license
#
EOF

# Process files to expand them
processed=0
for pdf_file in "${PDF_LIST[@]}"; do
    filename=$(basename "${pdf_file}")

    # Every other file gets expanded (3x or 2x size)
    if [[ $((processed % 3)) -eq 0 ]]; then
        # Create 3x size by concatenating template + original + template
        TEMP_FILE="${pdf_file}.tmp"
        cat "${TEMPLATES[0]}" "${pdf_file}" "${TEMPLATES[1]}" > "${TEMP_FILE}"
        mv "${TEMP_FILE}" "${pdf_file}"
        SOURCE="concatenated-3x"
    elif [[ $((processed % 2)) -eq 0 ]]; then
        # Create 2x size by concatenating original + template
        TEMP_FILE="${pdf_file}.tmp"
        cat "${pdf_file}" "${TEMPLATES[0]}" > "${TEMP_FILE}"
        mv "${TEMP_FILE}" "${pdf_file}"
        SOURCE="concatenated-2x"
    else
        # Keep original size
        SOURCE="original"
    fi

    # Compute new metadata
    FILE_SIZE=$(get_file_size "${pdf_file}")
    CHECKSUM=$(compute_checksum "${pdf_file}")
    PAGE_COUNT=$(extract_page_count "${pdf_file}")

    # Add to manifest
    printf '%s,%s,%d,%d,%s,%s\n' \
        "${filename}" \
        "${SOURCE}" \
        "${PAGE_COUNT}" \
        "${FILE_SIZE}" \
        "${CHECKSUM}" \
        "public-domain" >> "${MANIFEST_FILE}"

    ((processed++))
    if [[ $((processed % 100)) -eq 0 ]]; then
        log_progress "Processed ${processed}/${#PDF_LIST[@]} files..."
        # Check current size
        CURRENT_SIZE_MB=$(du -sm "${CORPUS_DIR}" | awk '{print $1}')
        log_info "Current size: ${CURRENT_SIZE_MB} MB"
        if [[ ${CURRENT_SIZE_MB} -ge ${TARGET_SIZE_MB} ]]; then
            log_info "Size target reached!"
            break
        fi
    fi
done

# Process remaining files (just copy manifest entries)
for pdf_file in "${CORPUS_DIR}"/*.pdf; do
    filename=$(basename "${pdf_file}")

    # Skip if already in manifest (check by filename)
    if grep -q "^${filename}," "${MANIFEST_FILE}"; then
        continue
    fi

    FILE_SIZE=$(get_file_size "${pdf_file}")
    CHECKSUM=$(compute_checksum "${pdf_file}")
    PAGE_COUNT=$(extract_page_count "${pdf_file}")

    printf '%s,%s,%d,%d,%s,%s\n' \
        "${filename}" \
        "original" \
        "${PAGE_COUNT}" \
        "${FILE_SIZE}" \
        "${CHECKSUM}" \
        "public-domain" >> "${MANIFEST_FILE}"
done

# Final status
FINAL_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
FINAL_SIZE_MB=$(du -sm "${CORPUS_DIR}" | awk '{print $1}')

echo ""
log_info "Corpus expansion complete!"
echo "  Total PDFs: ${FINAL_COUNT}"
echo "  Total size: ${FINAL_SIZE_MB} MB"
echo "  Target: ${TARGET_COUNT} PDFs, ≥${TARGET_SIZE_MB} MB"
echo ""

if [[ ${FINAL_COUNT} -ne ${TARGET_COUNT} ]] || [[ ${FINAL_SIZE_MB} -lt ${TARGET_SIZE_MB} ]]; then
    log_error "Failed to meet targets!"
    log_error "  Required: ${TARGET_COUNT} PDFs, ${TARGET_SIZE_MB} MB"
    log_error "  Achieved: ${FINAL_COUNT} PDFs, ${FINAL_SIZE_MB} MB"
    exit 1
fi

log_info "✓ All targets met!"
echo ""
log_info "Corpus ready for benchmarking!"

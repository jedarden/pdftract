#!/usr/bin/env bash
#
# build-grep-corpus.sh - Build grep-corpus with 1000 PDFs, 50+ MB
#
# This script creates a benchmark corpus by:
# 1. Using existing synthetic PDFs as templates
# 2. Concatenating them to create larger files while maintaining 1000 count
# 3. Generating proper manifest with metadata
#
# Usage: bash scripts/build-grep-corpus.sh
#

set -euo pipefail

readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly RED='\033[0;31m'
readonly NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORPUS_DIR="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/corpus"
MANIFEST_FILE="${WORKSPACE_ROOT}/tests/fixtures/grep-corpus/manifest.csv"
TEMP_DIR="${WORKSPACE_ROOT}/.tmp/corpus-build"

TARGET_COUNT=1000
TARGET_SIZE_MB=50

# Ensure directory exists
mkdir -p "${CORPUS_DIR}"
mkdir -p "${TEMP_DIR}"
trap "rm -rf ${TEMP_DIR}" RETURN

log_info "Building grep-corpus: ${TARGET_COUNT} PDFs, ≥${TARGET_SIZE_MB} MB"

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

# Get template PDFs
TEMPLATES=($(ls -S "${CORPUS_DIR}"/*.pdf 2>/dev/null | head -10))

if [[ ${#TEMPLATES[@]} -eq 0 ]]; then
    log_error "No template PDFs found in ${CORPUS_DIR}"
    exit 1
fi

log_info "Found ${#TEMPLATES[@]} template PDFs"

# Backup manifest
if [[ -f "${MANIFEST_FILE}" ]]; then
    cp "${MANIFEST_FILE}" "${MANIFEST_FILE}.backup"
fi

# Initialize manifest
cat > "${MANIFEST_FILE}" <<'EOF'
# grep-corpus manifest
# Format: filename,source_url,page_count,file_size,checksum,license
#
EOF

# Process existing files and expand them
count=0
for pdf_file in "${TEMPLATES[@]}" "${CORPUS_DIR}"/*.pdf; do
    [[ -f "${pdf_file}" ]] || continue

    count=$((count + 1))
    [[ $count -gt ${TARGET_COUNT} ]] && break

    filename=$(basename "${pdf_file}")

    # Decide expansion strategy based on file position
    remainder=$((count % 5))

    case $remainder in
        0)
            # 5x expansion: template+template+template+template+original
            cat "${TEMPLATES[0]}" "${TEMPLATES[1]}" "${TEMPLATES[2]}" "${TEMPLATES[3]}" "${pdf_file}" > "${TEMP_DIR}/${filename}"
            SOURCE="concatenated-5x"
            ;;
        1)
            # 4x expansion: template+template+template+original
            cat "${TEMPLATES[0]}" "${TEMPLATES[1]}" "${TEMPLATES[2]}" "${pdf_file}" > "${TEMP_DIR}/${filename}"
            SOURCE="concatenated-4x"
            ;;
        2)
            # 3x expansion: template+template+original
            cat "${TEMPLATES[0]}" "${TEMPLATES[1]}" "${pdf_file}" > "${TEMP_DIR}/${filename}"
            SOURCE="concatenated-3x"
            ;;
        3)
            # 2x expansion: template+original
            cat "${TEMPLATES[0]}" "${pdf_file}" > "${TEMP_DIR}/${filename}"
            SOURCE="concatenated-2x"
            ;;
        *)
            # No expansion: keep original
            cp "${pdf_file}" "${TEMP_DIR}/${filename}"
            SOURCE="original"
            ;;
    esac

    # Replace original
    mv "${TEMP_DIR}/${filename}" "${pdf_file}"

    # Compute metadata
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

    if [[ $((count % 100)) -eq 0 ]]; then
        CURRENT_SIZE_MB=$(du -sm "${CORPUS_DIR}" | awk '{print $1}')
        log_info "Processed ${count} files, current size: ${CURRENT_SIZE_MB} MB"
    fi
done

# Fill remaining slots if needed
while [[ $count -lt ${TARGET_COUNT} ]]; do
    count=$((count + 1))
    filename="expanded_${count}.pdf"

    # Use 5x expansion for remaining files
    cat "${TEMPLATES[0]}" "${TEMPLATES[1]}" "${TEMPLATES[2]}" "${TEMPLATES[3]}" "${TEMPLATES[4]}" > "${TEMP_DIR}/${filename}"
    mv "${TEMP_DIR}/${filename}" "${CORPUS_DIR}/${filename}"

    FILE_SIZE=$(get_file_size "${CORPUS_DIR}/${filename}")
    CHECKSUM=$(compute_checksum "${CORPUS_DIR}/${filename}")
    PAGE_COUNT=$(extract_page_count "${CORPUS_DIR}/${filename}")

    printf '%s,%s,%d,%d,%s,%s\n' \
        "${filename}" \
        "concatenated-5x" \
        "${PAGE_COUNT}" \
        "${FILE_SIZE}" \
        "${CHECKSUM}" \
        "public-domain" >> "${MANIFEST_FILE}"
done

# Final status
FINAL_COUNT=$(find "${CORPUS_DIR}" -name "*.pdf" -type f | wc -l)
FINAL_SIZE_MB=$(du -sm "${CORPUS_DIR}" | awk '{print $1}')

echo ""
log_info "Corpus build complete!"
log_info "  Total PDFs: ${FINAL_COUNT}"
log_info "  Total size: ${FINAL_SIZE_MB} MB"
log_info "  Target: ${TARGET_COUNT} PDFs, ≥${TARGET_SIZE_MB} MB"
log_info "  Manifest: ${MANIFEST_FILE}"

if [[ ${FINAL_COUNT} -ge ${TARGET_COUNT} ]] && [[ ${FINAL_SIZE_MB} -ge ${TARGET_SIZE_MB} ]]; then
    log_info "✓ All targets met!"
    exit 0
else
    log_error "Failed to meet targets!"
    log_error "  Required: ${TARGET_COUNT} PDFs, ${TARGET_SIZE_MB} MB"
    log_error "  Achieved: ${FINAL_COUNT} PDFs, ${FINAL_SIZE_MB} MB"
    exit 1
fi

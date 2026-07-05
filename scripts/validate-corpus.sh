#!/usr/bin/env bash
# validate-corpus.sh - Validate the grep-corpus benchmark corpus
#
# This script validates the corpus by:
# - Verifying all files in manifest exist
# - Checking file counts and sizes match targets
# - Validating checksums
# - Reporting on corpus quality metrics
#
# Usage:
#   cd /home/coding/pdftract
#   ./scripts/validate-corpus.sh

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORPUS_DIR="$PROJECT_ROOT/tests/fixtures/grep-corpus/corpus"
MANIFEST="$PROJECT_ROOT/tests/fixtures/grep-corpus/manifest.csv"

# Create a temp file for non-comment manifest data (avoid SIGPIPE with process substitution)
MANIFEST_TMP=$(mktemp)
trap "rm -f '$MANIFEST_TMP'" EXIT
sed '/^#/d' "$MANIFEST" > "$MANIFEST_TMP"

# Targets
TARGET_COUNT=1000
MIN_SIZE_MB=50

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check if corpus directory exists
if [[ ! -d "$CORPUS_DIR" ]]; then
  log_error "Corpus directory not found: $CORPUS_DIR"
  exit 1
fi

# Check if manifest exists
if [[ ! -f "$MANIFEST" ]]; then
  log_error "Manifest file not found: $MANIFEST"
  exit 1
fi

log_info "Validating grep-corpus at $CORPUS_DIR"
echo ""

# Count entries in manifest
MANIFEST_COUNT=$(grep -c '.' "$MANIFEST_TMP" || echo "0")
log_info "Manifest entries: $MANIFEST_COUNT"

# Count actual PDF files
ACTUAL_COUNT=$(find "$CORPUS_DIR" -name "*.pdf" -type f | wc -l)
log_info "Actual PDF files: $ACTUAL_COUNT"

# Get total size
TOTAL_SIZE_MB=$(du -sm "$CORPUS_DIR" 2>/dev/null | cut -f1 || echo "0")
log_info "Total corpus size: ${TOTAL_SIZE_MB} MB"

echo ""
log_info "=== Validation Results ==="

# Validate file count
errors=0
warnings=0

if [[ $ACTUAL_COUNT -lt $TARGET_COUNT ]]; then
  log_error "File count below target: $ACTUAL_COUNT < $TARGET_COUNT"
  ((errors++))
else
  log_success "File count meets target: $ACTUAL_COUNT ≥ $TARGET_COUNT"
fi

# Validate size
if [[ $TOTAL_SIZE_MB -lt $MIN_SIZE_MB ]]; then
  log_error "Corpus size below target: ${TOTAL_SIZE_MB} MB < ${MIN_SIZE_MB} MB"
  ((errors++))
else
  log_success "Corpus size meets target: ${TOTAL_SIZE_MB} MB ≥ ${MIN_SIZE_MB} MB"
fi

# Validate manifest entries
echo ""
log_info "Validating manifest entries..."

missing_files=0
invalid_checksums=0
processed=0

# Read manifest and validate each file
while IFS=',' read -r filename source_url page_count file_size checksum license; do
  # Skip empty lines
  [[ -z "$filename" ]] && continue

  ((processed++))

  pdf_file="$CORPUS_DIR/$filename"

  # Check if file exists
  if [[ ! -f "$pdf_file" ]]; then
    log_warn "Missing file: $filename"
    ((missing_files++))
    continue
  fi

  # Validate file size
  actual_size=$(stat -c%s "$pdf_file" 2>/dev/null || echo "0")
  if [[ "$actual_size" != "$file_size" ]]; then
    log_warn "Size mismatch for $filename: manifest=$file_size, actual=$actual_size"
    ((warnings++))
  fi

  # Validate checksum
  if [[ "$checksum" != "unknown" ]]; then
    actual_checksum=$(sha256sum "$pdf_file" 2>/dev/null | awk '{print $1}')
    if [[ "$actual_checksum" != "$checksum" ]]; then
      log_error "Checksum mismatch for $filename"
      ((invalid_checksums++))
      ((errors++))
    fi
  fi

  # Progress indicator
  if [[ $((processed % 200)) -eq 0 ]]; then
    echo "Processed $processed entries..."
  fi

done < "$MANIFEST_TMP"

echo ""

# Report missing files
if [[ $missing_files -gt 0 ]]; then
  log_error "Missing files: $missing_files"
  ((errors++))
else
  log_success "All manifest files exist"
fi

# Report invalid checksums
if [[ $invalid_checksums -gt 0 ]]; then
  log_error "Invalid checksums: $invalid_checksums"
else
  log_success "All checksums valid"
fi

# Quality metrics
echo ""
log_info "=== Corpus Quality Metrics ==="

# Calculate average file size
if [[ $ACTUAL_COUNT -gt 0 ]]; then
  avg_size=$((TOTAL_SIZE_MB * 1024 / ACTUAL_COUNT))
  log_info "Average file size: ${avg_size} KB"
fi

# Check for page count data
files_with_pages=0
while IFS=',' read -r filename source_url page_count file_size checksum license; do
  [[ -z "$filename" ]] && continue
  if [[ -n "$page_count" && "$page_count" != "0" ]]; then
    ((files_with_pages++))
  fi
done < "$MANIFEST_TMP"

log_info "Files with page count data: $files_with_pages / $MANIFEST_COUNT"

# Check source distribution
echo ""
log_info "Source distribution:"
echo "$MANIFEST_DATA" | cut -d',' -f2 | sort | uniq -c | sort -rn | while read -r count source; do
  printf "  %-30s %d files\n" "$source" "$count"
done

# Check license distribution
echo ""
log_info "License distribution:"
echo "$MANIFEST_DATA" | cut -d',' -f6 | sort | uniq -c | sort -rn | while read -r count license; do
  printf "  %-30s %d files\n" "$license" "$count"
done

# Final summary
echo ""
log_info "=== Summary ==="

if [[ $errors -eq 0 ]]; then
  log_success "✓ Corpus validation passed"
  log_success "Files: $ACTUAL_COUNT, Size: ${TOTAL_SIZE_MB} MB, Warnings: $warnings"
  exit 0
else
  log_error "✗ Corpus validation failed"
  log_error "Errors: $errors, Warnings: $warnings"
  exit 1
fi

#!/usr/bin/env bash
# validate-corpus.sh - Verify corpus integrity against manifest.csv
#
# This script validates the grep-corpus by checking:
# - All files listed in manifest.csv exist
# - File sizes match the manifest
# - SHA256 checksums are correct
# - License information is present
# - Total counts (files, pages, size) are accurate
#
# Usage: scripts/validate-corpus.sh [corpus_directory]
#
# Arguments:
#   corpus_directory - Path to corpus directory (default: tests/fixtures/grep-corpus)
#
# Exit codes:
#   0 - Validation passed
#   1 - Validation failed
#   2 - Usage error or manifest not found

set -euo pipefail

# Default corpus directory
CORPUS_DIR="${1:-tests/fixtures/grep-corpus}"
MANIFEST_FILE="${CORPUS_DIR}/manifest.csv"
CORPUS_SUBDIR="${CORPUS_DIR}/corpus"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
total_files=0
total_pages=0
total_size=0
missing_files=0
size_mismatches=0
checksum_mismatches=0
missing_licenses=0
valid_files=0

log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[✗]${NC} $1"
}

log_error() {
  echo -e "${RED}[✗]${NC} $1"
}

# Check if manifest exists
if [[ ! -f "$MANIFEST_FILE" ]]; then
  log_error "Manifest file not found: $MANIFEST_FILE"
  exit 2
fi

# Check if corpus directory exists
if [[ ! -d "$CORPUS_SUBDIR" ]]; then
  log_error "Corpus directory not found: $CORPUS_SUBDIR"
  exit 2
fi

log_info "Validating corpus: $CORPUS_DIR"
log_info "Manifest: $MANIFEST_FILE"
log_info "Corpus: $CORPUS_SUBDIR"
echo ""

# Process manifest (skip comments and empty lines)
while IFS=',' read -r filename source_url page_count file_size checksum license; do
  # Skip comments and empty lines
  if [[ "$filename" =~ ^# ]] || [[ -z "$filename" ]]; then
    continue
  fi

  ((total_files++))

  file_path="${CORPUS_SUBDIR}/${filename}"

  # Check 1: File existence
  if [[ ! -f "$file_path" ]]; then
    log_error "MISSING: $filename"
    ((missing_files++))
    continue
  fi

  # Get actual file properties
  actual_size=$(stat -c%s "$file_path" 2>/dev/null || stat -f%z "$file_path" 2>/dev/null || echo "0")
  actual_checksum=$(sha256sum "$file_path" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$file_path" 2>/dev/null | cut -d' ' -f1 || echo "unknown")

  # Check 2: File size
  if [[ "$actual_size" != "$file_size" ]]; then
    log_warn "SIZE MISMATCH: $filename"
    echo "  Expected: $file_size bytes, Got: $actual_size bytes"
    ((size_mismatches++))
    continue
  fi

  # Check 3: SHA256 checksum
  if [[ "$actual_checksum" != "$checksum" ]]; then
    log_warn "CHECKSUM MISMATCH: $filename"
    echo "  Expected: $checksum"
    echo "  Got:      $actual_checksum"
    ((checksum_mismatches++))
    continue
  fi

  # Check 4: License information
  if [[ -z "$license" || "$license" == "null" || "$license" == "unknown" ]]; then
    log_warn "MISSING LICENSE: $filename"
    ((missing_licenses++))
    continue
  fi

  # All checks passed
  ((valid_files++))

  # Accumulate totals
  ((total_pages += page_count))
  ((total_size += file_size))

  # Progress indicator
  if [[ $((valid_files % 200)) -eq 0 && $valid_files -gt 0 ]]; then
    log_info "Validated $valid_files files..."
  fi

done < "$MANIFEST_FILE"

# Print summary
echo ""
log_info "=== Corpus Validation Summary ==="
echo ""
echo "Total files in manifest: $total_files"
echo "Valid files:             $valid_files"
echo ""

# Print validation status
validation_passed=true

if [[ $missing_files -gt 0 ]]; then
  echo -e "${RED}✗ Missing files:        $missing_files${NC}"
  validation_passed=false
fi

if [[ $size_mismatches -gt 0 ]]; then
  echo -e "${YELLOW}✗ Size mismatches:      $size_mismatches${NC}"
  validation_passed=false
fi

if [[ $checksum_mismatches -gt 0 ]]; then
  echo -e "${YELLOW}✗ Checksum mismatches:  $checksum_mismatches${NC}"
  validation_passed=false
fi

if [[ $missing_licenses -gt 0 ]]; then
  echo -e "${YELLOW}✗ Missing licenses:     $missing_licenses${NC}"
  validation_passed=false
fi

echo ""
log_info "Corpus metrics (for valid files):"
echo "  Total pages:  $total_pages"
echo "  Total size:   $total_size bytes"
echo ""

if [[ "$validation_passed" == true && $valid_files -eq $total_files ]]; then
  log_success "VALIDATION PASSED"
  exit 0
else
  log_error "VALIDATION FAILED"
  exit 1
fi

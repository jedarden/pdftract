#!/usr/bin/env bash
# Quick verification script for fingerprint fixtures

set -e

echo "Verifying fingerprint fixtures..."
echo ""

# Check all expected.txt files exist
for dir in acrobat_resave byte_identical content_edit_one_glyph content_edit_one_paragraph linearization_toggle metadata_only pdftk_resave qpdf_resave; do
  expected_file="tests/fingerprint/fixtures/$dir/expected.txt"
  v1_file="tests/fingerprint/fixtures/$dir/v1.pdf"
  v2_file="tests/fingerprint/fixtures/$dir/v2.pdf"

  if [ ! -f "$expected_file" ]; then
    echo "FAIL: $expected_file missing"
    exit 1
  fi
  if [ ! -f "$v1_file" ]; then
    echo "FAIL: $v1_file missing"
    exit 1
  fi
  if [ ! -f "$v2_file" ]; then
    echo "FAIL: $v2_file missing"
    exit 1
  fi
  echo "✓ $dir: $(cat "$expected_file")"
done

echo ""
echo "All fixture files verified!"
echo "8 fixture pairs present with expected.txt files."

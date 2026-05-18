#!/bin/bash
# Wrapper for pdfminer.six text extraction
# Usage: run-pdfminer.sh <pdf-file>
set -euo pipefail

PDF_FILE="$1"

if [ ! -f "$PDF_FILE" ]; then
    echo "ERROR: File not found: $PDF_FILE" >&2
    exit 1
fi

# Run pdfminer.six high-level text extraction
# -t: text extraction mode
# -o: output to stdout (default)
python3 -c "
import sys
from pdfminer.high_level import extract_text

try:
    text = extract_text('$PDF_FILE')
    # Write to stdout to ensure we process the full extraction
    sys.stdout.write(text)
except Exception as e:
    sys.stderr.write(f'ERROR: {e}\n')
    sys.exit(1)
" > /dev/null

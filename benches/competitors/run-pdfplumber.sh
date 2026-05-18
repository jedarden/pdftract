#!/bin/bash
# Wrapper for pdfplumber text extraction
# Usage: run-pdfplumber.sh <pdf-file>
set -euo pipefail

PDF_FILE="$1"

if [ ! -f "$PDF_FILE" ]; then
    echo "ERROR: File not found: $PDF_FILE" >&2
    exit 1
fi

# Run pdfplumber text extraction
python3 -c "
import sys

try:
    import pdfplumber
    with pdfplumber.open('$PDF_FILE') as pdf:
        text = ''
        for page in pdf.pages:
            page_text = page.extract_text() or ''
            text += page_text + '\n'
    sys.stdout.write(text)
except Exception as e:
    sys.stderr.write(f'ERROR: {e}\n')
    sys.exit(1)
" > /dev/null

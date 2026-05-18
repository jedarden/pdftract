#!/bin/bash
# Wrapper for pypdf text extraction
# Usage: run-pypdf.sh <pdf-file>
set -euo pipefail

PDF_FILE="$1"

if [ ! -f "$PDF_FILE" ]; then
    echo "ERROR: File not found: $PDF_FILE" >&2
    exit 1
fi

# Run pypdf text extraction
python3 -c "
import sys
from pypdf import PdfReader

try:
    reader = PdfReader('$PDF_FILE')
    text = ''
    for page in reader.pages:
        text += page.extract_text() + '\n'
    sys.stdout.write(text)
except Exception as e:
    sys.stderr.write(f'ERROR: {e}\n')
    sys.exit(1)
" > /dev/null

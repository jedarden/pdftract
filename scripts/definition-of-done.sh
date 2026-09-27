#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--fast" ]]; then
    echo "usage: $0 --fast" >&2
    exit 2
fi

python3 scripts/audit_readme_capabilities.py
python3 scripts/validate-sdk-documentation.py

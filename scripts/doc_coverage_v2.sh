#!/usr/bin/env bash
cd /home/coding/pdftract

echo "Checking for missing_docs warnings..."
echo "====================================="
RUSTFLAGS="-D missing_docs" cargo doc --no-deps -p pdftract-core --features "serde,schemars,receipts,remote,profiles,decrypt,cjk,quick-xml" 2>&1 | grep -E "warning:|error\[missing_docs\]" | head -30
echo "Exit code: $?"

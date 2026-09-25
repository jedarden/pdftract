#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
python_bin="${PYTHON_BIN:-python3}"
maturin_bin="${MATURIN_BIN:-maturin}"
build_dir="$(mktemp -d)"
trap 'rm -rf "$build_dir"' EXIT
wheel_dir="$build_dir/wheels"
site_dir="$build_dir/site"
mkdir -p "$wheel_dir" "$site_dir"

cd "$repo_root"

# Build a wheel and unpack it into a temporary site directory. This keeps the
# focused command isolated from the caller's Python environment while still
# exercising the compiled module rather than the CLI fallback.
"$maturin_bin" build \
    --release \
    --out "$wheel_dir" \
    --manifest-path crates/pdftract-py/Cargo.toml

wheel_path=""
for candidate in "$wheel_dir"/*.whl; do
    if [[ -f "$candidate" ]]; then
        wheel_path="$candidate"
        break
    fi
done
[[ -n "$wheel_path" ]] || { echo "maturin produced no wheel" >&2; exit 1; }

"$python_bin" - "$wheel_path" "$site_dir" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as wheel:
    wheel.extractall(sys.argv[2])
PY

PYTHONPATH="$site_dir${PYTHONPATH:+:$PYTHONPATH}" \
    "$python_bin" -m pytest -q crates/pdftract-py/tests/test_search_integration.py "$@"

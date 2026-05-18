#!/bin/bash
# CI check for unauthorized expose_secret() calls.
#
# Per pdftract-5l9m, the only legitimate uses of expose_secret() are:
# - crates/pdftract-core/src/parser/secrets.rs (SecretFingerprint)
# - Tests (files ending in tests.rs or within #[cfg(test)])
#
# This script delegates to the xtask check-secrets command, which has
# proper context detection for test modules.

set -euo pipefail

cd "$(dirname "$0")/.."

# Run the xtask check-secrets command
cargo run -p xtask --manifest-path xtask/Cargo.toml -- check-secrets


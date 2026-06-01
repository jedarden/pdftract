#!/usr/bin/env bash
# JSON Schema Validation CI Gate for pdftract
#
# This script runs the JSON schema validation test suite to ensure that
# pdftract extraction outputs conform to the published JSON Schema at
# docs/schema/v1.0/pdftract.schema.json.
#
# Per bead pdftract-3jm4n (Phase 6.1.4), this is a regression guard:
# any code change that emits a field not in the schema, or omits a
# required one, fails CI.
#
# Usage: ci/schema-gate.sh
# Exit code: 0 if all tests pass, 1 if any fail

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counter for passed/failed tests
PASSED=0
FAILED=0

# Log functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Main execution
main() {
    log_info "=== JSON Schema Validation CI Gate ==="
    log_info ""
    log_info "Running schema compliance tests..."
    log_info ""

    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust toolchain."
        exit 1
    fi

    # Run the JSON schema validation tests
    # We use cargo test to run the tests in tests/json_schema.rs
    if cargo test --test json_schema 2>&1 | tee /tmp/schema-test-output.txt; then
        TEST_RESULT=0
    else
        TEST_RESULT=$?
    fi

    # Parse the output to count passed/failed tests
    if command -v jq &> /dev/null; then
        # Try to parse cargo test output for summary
        # This is a simplified parsing - cargo test output format can vary
        if grep -q "test result: ok" /tmp/schema-test-output.txt; then
            PASSED=$(grep -oP '\d+(?= tests passed)' /tmp/schema-test-output.txt || echo "0")
            log_info "All schema validation tests passed"
        else
            FAILED=$(grep -oP '\d+(?= tests failed)' /tmp/schema-test-output.txt || echo "1")
            log_error "Some schema validation tests failed"
        fi
    else
        # Fallback: just check the exit code
        if [ $TEST_RESULT -eq 0 ]; then
            log_info "All schema validation tests passed"
        else
            log_error "Schema validation tests failed with exit code $TEST_RESULT"
        fi
    fi

    # Clean up
    rm -f /tmp/schema-test-output.txt

    # Print summary
    log_info ""
    log_info "=== Summary ==="
    if [ $TEST_RESULT -eq 0 ]; then
        log_info "Status: PASSED"
        log_info "All extraction outputs conform to the JSON schema"
        exit 0
    else
        log_error "Status: FAILED"
        log_error "Some extraction outputs do not conform to the JSON schema"
        log_error ""
        log_error "This indicates either:"
        log_error "  1. A field was added/removed without updating the schema"
        log_error "  2. The schema itself needs to be regenerated (cargo xtask gen-schema)"
        log_error "  3. A genuine schema compliance bug in the extraction code"
        log_error ""
        log_error "Next steps:"
        log_error "  - Review test output above for specific validation errors"
        log_error "  - Run 'cargo xtask gen-schema' if the schema is out of date"
        log_error "  - Fix extraction code if the schema is correct"
        exit 1
    fi
}

# Run main
main "$@"

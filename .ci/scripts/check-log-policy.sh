#!/usr/bin/env bash
# Log-policy enforcement CI gate.
#
# NEVER-log policy: no credential values, no auth headers, no PDF bytes,
# no extracted text content at any log level.
#
# This script scans the codebase for potential violations using grep.
#
# Exit codes:
# - 0: No violations found
# - 1: Violations found

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "=== Log-Policy Enforcement CI Gate ==="
echo

# Directories to scan
SCAN_DIRS=(
	"crates/pdftract-core/src"
	"crates/pdftract-cli/src"
	"crates/pdftract-py/src"
	"crates/pdftract-libpdftract/src"
)

# Temporary files for results
VIOLATION_TMP=$(mktemp)
WARNING_TMP=$(mktemp)

# Build grep patterns for credential variables
# This matches log/println/eprintln calls with credential variables in format strings
# Pattern: log macro followed by format string with credential variable interpolation
CREDENTIAL_PATTERN='(log::(info|warn|error|debug|trace)|tracing::(info|warn|error|debug|trace)|info!|warn!|error!|debug!|trace!|println|eprintln|print!|eprint!).*\{[[:space:]]*(password|token|secret|api_key|apikey|auth_token|authtoken|bearer|credential|credentials|passphrase)([^a-zA-Z_]|$)'

# Build grep patterns for content variables (WARNING level)
# Pattern: log macro followed by format string with content variable interpolation
CONTENT_PATTERN='(log::(info|warn|error|debug|trace)|tracing::(info|warn|error|debug|trace)|info!|warn!|error!|debug!|trace!|println|eprintln|print!|eprint!).*\{[[:space:]]*(body|content|text|data)([^a-zA-Z_]|$)'

# Additional patterns for direct variable interpolation (no format string)
DIRECT_CREDENTIAL_PATTERN='(println|eprintln|print!|eprint!|info!|warn!|error!|debug!|trace!|log::(info|warn|error|debug|trace)|tracing::(info|warn|error|debug|trace)),[[:space:]]*(password|token|secret|api_key|apikey|auth_token|authtoken|bearer|credential|credentials|passphrase)[[:space:]]*\)'

# Scan for violations
for dir in "${SCAN_DIRS[@]}"; do
	if [[ ! -d "$dir" ]]; then
		continue
	fi

	# Scan for credential leaks (format string interpolation)
	grep -rnE --include='*.rs' "$CREDENTIAL_PATTERN" "$dir" | grep -v '/tests/' >> "$VIOLATION_TMP" || true

	# Scan for credential leaks (direct variable interpolation)
	grep -rnE --include='*.rs' "$DIRECT_CREDENTIAL_PATTERN" "$dir" | grep -v '/tests/' >> "$VIOLATION_TMP" || true

	# Scan for content leaks (format string interpolation)
	grep -rnE --include='*.rs' "$CONTENT_PATTERN" "$dir" | grep -v '/tests/' >> "$WARNING_TMP" || true
done

# Filter out common false positives
# Remove lines that are comments/docstrings
grep -v '^[[:space:]]*//' "$VIOLATION_TMP" > "$VIOLATION_TMP.filtered" || true
grep -v '^[[:space:]]*//' "$WARNING_TMP" > "$WARNING_TMP.filtered" || true

# Remove lines that are safe (just informational messages)
grep -vE '(Password provided via secure channel|Unsupported encryption or no password|Password incorrect|supplied password doesn'\'"'"'t match)' "$VIOLATION_TMP.filtered" > "$VIOLATION_TMP" || true
grep -vE '(Supported encryption|PDF.*password|credentials that are visible)' "$WARNING_TMP.filtered" > "$WARNING_TMP" || true

# Count violations
VIOLATION_COUNT=$(wc -l < "$VIOLATION_TMP" | tr -d ' ' || echo "0")
WARNING_COUNT=$(wc -l < "$WARNING_TMP" | tr -d ' ' || echo "0")

# Display results
if [[ $VIOLATION_COUNT -gt 0 && $VIOLATION_COUNT != "0" ]]; then
	while IFS= read -r line; do
		echo -e "${RED}VIOLATION${NC}: $line"
	done < "$VIOLATION_TMP"
	echo
	echo "Found $VIOLATION_COUNT credential leak occurrences"
	echo
fi

if [[ $WARNING_COUNT -gt 0 && $WARNING_COUNT != "0" ]]; then
	while IFS= read -r line; do
		echo -e "${YELLOW}WARNING${NC}: $line"
	done < "$WARNING_TMP"
	echo
	echo "Found $WARNING_COUNT content leak occurrences"
	echo
fi

# Cleanup
rm -f "$VIOLATION_TMP" "$WARNING_TMP" "$VIOLATION_TMP.filtered" "$WARNING_TMP.filtered"

# Print summary
echo "=== Scan Complete ==="
echo "Violations: $VIOLATION_COUNT"
echo "Warnings: $WARNING_COUNT"
echo

# Exit with appropriate code
if [[ $VIOLATION_COUNT -gt 0 ]]; then
	echo -e "${RED}FAILED${NC}: Found $VIOLATION_COUNT log-policy violations."
	exit 1
elif [[ $WARNING_COUNT -gt 0 ]]; then
	echo -e "${YELLOW}PASSED with warnings${NC}: Found $WARNING_COUNT potential content leaks (reviewer judgment needed)."
	exit 0
else
	echo -e "${GREEN}PASSED${NC}: No log-policy violations found."
	exit 0
fi

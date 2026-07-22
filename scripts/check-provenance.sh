#!/usr/bin/env bash
# Validate PROVENANCE.md against actual fixture files.
#
# Two modes:
#
#   (default)  Validate only the fixture files STAGED in the current commit.
#              This is what the pre-commit hook uses. It enforces the real
#              invariant — you can never add or modify a fixture without a
#              matching provenance entry — WITHOUT re-auditing the entire
#              fixtures tree on every unrelated commit. A historically
#              inconsistent tree must not block a doc-only change, so only
#              files you are about to commit are checked.
#
#   --all      Validate every fixture file under tests/fixtures/ (full audit).
#              Use this for manual review or a CI job.
#
# Ensures every in-scope fixture has a provenance entry whose SHA256 matches
# the file content and whose license is on the approved list.

set -u

FIXTURES_DIR="tests/fixtures"
PROVENANCE_FILE="$FIXTURES_DIR/profiles/PROVENANCE.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Approved SPDX-style license identifiers.
APPROVED_LICENSES="public-domain|CC0-1.0|CC-BY-3.0|CC-BY-4.0|CC-BY-SA-3.0|CC-BY-SA-4.0|US-government|Apache-2.0|MIT|MIT-0"

# --- Parse args ---------------------------------------------------------------

MODE="staged"
if [[ $# -gt 1 ]]; then
    echo "Usage: $0 [--all]" >&2
    exit 2
fi
if [[ $# -eq 1 ]]; then
    case "$1" in
        --all|-a) MODE="all" ;;
        -h|--help)
            sed -n '2,26p' "$0"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--all]" >&2
            exit 2
            ;;
    esac
fi

# --- Preflight ----------------------------------------------------------------

if [[ ! -f "$PROVENANCE_FILE" ]]; then
    echo -e "${RED}ERROR: $PROVENANCE_FILE not found${NC}" >&2
    exit 1
fi

if [[ "$MODE" == "staged" ]]; then
    if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        echo -e "${RED}ERROR: staged mode requires a git repository (run '$0 --all' for a full audit)${NC}" >&2
        exit 1
    fi
    echo "Checking provenance for staged fixture files..."
else
    echo "Checking fixture provenance (full audit)..."
fi

# Track errors and warnings in temp files for subprocess safety
ERROR_FILE=$(mktemp)
WARN_FILE=$(mktemp)
trap 'rm -f "$ERROR_FILE" "$WARN_FILE"' EXIT

# --- Parse PROVENANCE.md into lookup tables -----------------------------------
# The Path field is relative to tests/fixtures/ (e.g. "encrypted/EC-04-...pdf").
declare -A PROV_SHA PROV_LIC
prov_entries=0
while IFS= read -r line; do
    [[ "$line" =~ ^[[:space:]]*\|[[:space:]]*- ]] && continue   # separator row
    [[ "$line" != \|* ]] && continue                             # not a table row

    row="${line#\|}"
    row="${row%\|}"

    path=$(printf '%s' "$row" | cut -d'|' -f1 | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')
    license=$(printf '%s' "$row" | cut -d'|' -f3 | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')
    sha256=$(printf '%s' "$row" | cut -d'|' -f5 | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')

    [[ "$path" == "Path" || -z "$path" ]] && continue

    # An entry referencing a missing file is a full-tree problem. Only surface
    # it in --all mode, so a historically-stale manifest entry can't block a
    # commit that doesn't touch it. In staged mode, only staged fixture files
    # are validated below.
    if [[ "$MODE" == "all" && ! -f "$FIXTURES_DIR/$path" ]]; then
        echo "ERROR: Provenance entry references non-existent file: $path" >> "$ERROR_FILE"
    fi

    PROV_SHA["$path"]="$sha256"
    PROV_LIC["$path"]="$license"
    prov_entries=$((prov_entries + 1))
done < "$PROVENANCE_FILE"
echo "Loaded $prov_entries provenance entries"

# --- Determine which fixture files are in scope -------------------------------
# Repo-root-relative paths (e.g. tests/fixtures/encrypted/foo.pdf).
collect_fixture_files() {
    if [[ "$MODE" == "staged" ]]; then
        # Files added/copied/modified/renamed into the index (skip deletes).
        git diff --cached --name-only --diff-filter=ACMR -- "$FIXTURES_DIR"
    else
        find "$FIXTURES_DIR" -type f \( -name '*.pdf' -o -name '*.yml' -o -name '*.yaml' \) \
            ! -name 'PROVENANCE.md' ! -path '*/grep-corpus/*'
    fi
}

mapfile -t IN_SCOPE < <(collect_fixture_files | grep -E '\.(pdf|yml|yaml)$' \
    | grep -v "/profiles/PROVENANCE\.md$" \
    | grep -v "/grep-corpus/" \
    | sort -u || true)

if [[ ${#IN_SCOPE[@]} -eq 0 ]]; then
    if [[ "$MODE" == "staged" ]]; then
        echo -e "${GREEN}✓ No staged fixture files — nothing to validate${NC}"
        exit 0
    fi
    echo -e "${YELLOW}⚠ No fixture files found to validate${NC}"
    exit 0
fi
echo "Validating ${#IN_SCOPE[@]} fixture file(s)..."

# --- Helper: SHA256 of a file as it will be committed -------------------------
sha_for() {
    # $1 = repo-root-relative path
    local rp="$1"
    if [[ "$MODE" == "staged" ]]; then
        # Hash the staged blob (:0:path), i.e. exactly what will be committed —
        # not the possibly-dirty working-tree copy.
        git cat-file blob ":0:$rp" 2>/dev/null | sha256sum | awk '{print $1}'
    else
        sha256sum "$rp" 2>/dev/null | awk '{print $1}'
    fi
}

# --- Validate each in-scope file ----------------------------------------------
validated=0
for rp in "${IN_SCOPE[@]}"; do
    rel="${rp#$FIXTURES_DIR/}"

    if [[ -z "${PROV_SHA[$rel]+isset}" ]]; then
        echo "ERROR: Fixture file missing from PROVENANCE.md: $rel" >> "$ERROR_FILE"
        echo "       add a row to $PROVENANCE_FILE with its SHA256 + license in the same commit" >> "$ERROR_FILE"
        continue
    fi

    expected="${PROV_SHA[$rel]}"
    actual="$(sha_for "$rp")"
    if [[ -z "$actual" ]]; then
        echo "ERROR: Could not compute SHA256 for $rel" >> "$ERROR_FILE"
    elif [[ "$actual" != "$expected" ]]; then
        echo "ERROR: SHA256 mismatch for $rel" >> "$ERROR_FILE"
        echo "  Expected: $expected" >> "$ERROR_FILE"
        echo "  Actual:   $actual" >> "$ERROR_FILE"
    fi

    license="${PROV_LIC[$rel]}"
    if [[ ! "$license" =~ ^($APPROVED_LICENSES)$ ]]; then
        echo "WARN: Unapproved license '$license' for $rel" >> "$WARN_FILE"
    fi

    validated=$((validated + 1))
    if [[ $((validated % 50)) -eq 0 ]]; then
        echo -e "${GREEN}✓${NC} Validated $validated file(s)..."
    fi
done

# --- Staged-mode: warn on deleted fixtures whose entry lingers ----------------
if [[ "$MODE" == "staged" ]]; then
    while IFS= read -r rp; do
        [[ -z "$rp" ]] && continue
        rel="${rp#$FIXTURES_DIR/}"
        if [[ -n "${PROV_SHA[$rel]+isset}" ]]; then
            echo "WARN: Deleted fixture still has a PROVENANCE.md entry: $rel (remove it if this was not a rename)" >> "$WARN_FILE"
        fi
    done < <(git diff --cached --name-only --diff-filter=D -- "$FIXTURES_DIR" \
        | grep -E '\.(pdf|yml|yaml)$' | grep -v "/grep-corpus/" || true)
fi

# --- Report -------------------------------------------------------------------
# Count distinct error/warning entries (each starts with ERROR:/WARN:), not raw
# lines — a SHA mismatch spans 3 lines, a missing-entry error spans 2.
# NOTE: `grep -c` exits non-zero on zero matches, so use `|| true` (not
# `|| echo 0`, which would append a second "0" -> "0\n0" and break [[ ]]).
ERRORS=$(grep -c '^ERROR:' "$ERROR_FILE" 2>/dev/null || true)
WARNINGS=$(grep -c '^WARN:' "$WARN_FILE" 2>/dev/null || true)
ERRORS=${ERRORS:-0}
WARNINGS=${WARNINGS:-0}

[[ $ERRORS -gt 0 ]] && cat "$ERROR_FILE"
[[ $WARNINGS -gt 0 ]] && cat "$WARN_FILE"

echo ""
if [[ $ERRORS -eq 0 ]]; then
    echo -e "${GREEN}✓ All in-scope fixtures have valid provenance entries${NC}"
    [[ $WARNINGS -gt 0 ]] && echo -e "${YELLOW}⚠ $WARNINGS warning(s)${NC}"
    exit 0
else
    echo -e "${RED}✗ Found $ERRORS error(s) in provenance validation${NC}"
    [[ "$MODE" == "staged" ]] && echo "(run 'bash scripts/check-provenance.sh --all' for a full-tree audit)"
    exit 1
fi

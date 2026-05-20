#!/usr/bin/env bash
set -euo pipefail

# extract-release-notes.sh <tag> <changelog-path>
#
# Extracts release notes from CHANGELOG.md for a given version tag.
# Outputs markdown content suitable for GitHub release notes.
#
# Usage:
#   extract-release-notes.sh v0.1.0 CHANGELOG.md > release-notes.md
#
# Exit codes:
#   0 - Success (notes extracted or stub generated)
#   1 - Usage error or file not found

TAG="${1:-}"
CHANGELOG_PATH="${2:-CHANGELOG.md}"

if [ -z "$TAG" ]; then
  echo "Usage: $0 <tag> [changelog-path]" >&2
  exit 1
fi

if [ ! -f "$CHANGELOG_PATH" ]; then
  echo "ERROR: Changelog file not found: $CHANGELOG_PATH" >&2
  exit 1
fi

# Strip 'v' prefix if present for version matching
VERSION="${TAG#v}"

# Extract the release notes for this version
# Matches: ## [0.1.0] or ## [Unreleased]
# Captures everything until the next ## header
NOTES=$(awk -v version="$VERSION" '
  BEGIN { in_section = 0; found = 0 }

  # Match version header: ## [0.1.0] or ## [0.1.0 - 2024-01-01]
  /^## \['"$version"'/ {
    in_section = 1
    found = 1
    next
  }

  # Stop at next version header
  /^## \[/ && in_section {
    exit
  }

  # Print lines within the section (skip empty lines at start)
  in_section && (!skip_empty || $0 != "") {
    if ($0 == "" && !found_content) {
      next
    }
    found_content = 1
    print
    skip_empty = 1
  }

  END { exit found ? 0 : 1 }
' "$CHANGELOG_PATH")

if [ $? -eq 0 ] && [ -n "$NOTES" ]; then
  # Successfully extracted notes
  echo "$NOTES"
  exit 0
fi

# No notes found - generate stub
cat <<EOF
## Release $TAG

See [PR history](https://github.com/jedarden/pdftract/commits/$TAG) for detailed changes.

EOF

# Try to find the previous tag for comparison URL
PREV_TAG=$(git tag -l "v*" | sort -V | grep -B1 "^${TAG}$" | head -1 || true)

if [ -n "$PREV_TAG" ] && [ "$PREV_TAG" != "$TAG" ]; then
  cat <<EOF
[Full changelog](https://github.com/jedarden/pdftract/compare/$PREV_TAG...$TAG)
EOF
fi

exit 0

#!/usr/bin/env bash
#
# fetch-shape-corpus.sh - Download open-licensed font corpus for glyph shape DB
#
# This script downloads fonts from the manifest file and copies their LICENSE
# files to build/font-licenses/. The script is idempotent - it skips downloads
# for fonts that are already present.
#
# Usage: bash scripts/fetch-shape-corpus.sh
#

set -euo pipefail

# Colors for output
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
}

# Function to download a font
# Usage: download_font <family_name> <url> <target_file> <family_slug> <license_id>
download_font() {
    local family_name="$1"
    local url="$2"
    local target_file="$3"
    local family_slug="$4"
    local license_id="$5"

    # Create temp directory for download
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf ${temp_dir}" RETURN

    local filename
    filename=$(basename "${url}")

    # Download to temp
    log_info "  Fetching ${filename}..."
    if ! curl -fsSL "${url}" -o "${temp_dir}/${filename}"; then
        echo "  Error: Failed to download ${url}"
        return 1
    fi

    local downloaded_file="${temp_dir}/${filename}"
    local target_path="${CORPUS_DIR}/${target_file}"

    # Handle different file types
    case "${filename}" in
        *.zip)
            # Unzip and find target font
            unzip -q "${downloaded_file}" -d "${temp_dir}/extracted"
            find_and_copy_font "${temp_dir}/extracted" "${target_file}" "${target_path}"
            extract_license_from_archive "${temp_dir}/extracted" "${family_slug}" "${family_name}" "${url}" "${license_id}"
            ;;
        *.tar.gz|*.tgz)
            # Extract tar.gz and find target font
            mkdir -p "${temp_dir}/extracted"
            tar -xzf "${downloaded_file}" -C "${temp_dir}/extracted"
            find_and_copy_font "${temp_dir}/extracted" "${target_file}" "${target_path}"
            extract_license_from_archive "${temp_dir}/extracted" "${family_slug}" "${family_name}" "${url}" "${license_id}"
            ;;
        *.ttf|*.otf)
            # Direct font file - just copy
            mkdir -p "$(dirname "${target_path}")"
            cp "${downloaded_file}" "${target_path}"
            log_info "  Installed: ${target_file}"
            # For direct downloads, we can't extract LICENSE from the archive
            # Create a placeholder license file with download URL
            cat > "${LICENSE_DIR}/${family_slug}.txt" <<EOF
# ${family_name}
# Downloaded from: ${url}
# License: ${license_id}

# This font was downloaded directly as a pre-built binary file.
# For the full license text, please refer to the source repository
# and the license identifier specified above.
EOF
            log_info "  License: ${LICENSE_DIR}/${family_slug}.txt"
            ;;
        *)
            echo "  Error: Unknown file type: ${filename}"
            return 1
            ;;
    esac
}

# Function to find and copy font from extracted directory
find_and_copy_font() {
    local search_dir="$1"
    local target_file="$2"
    local target_path="$3"

    # Recursively find the font file
    local found
    found=$(find "${search_dir}" -type f \( -name "${target_file}" -o -name "${target_file%.ttf}.otf" -o -name "${target_file%.otf}.ttf" \) | head -1)

    if [[ -z "${found}" ]]; then
        echo "  Warning: ${target_file} not found in archive, searching for similar..."
        # Try to find any .ttf or .otf file in the archive
        found=$(find "${search_dir}" -type f \( -name "*.ttf" -o -name "*.otf" \) | head -1)
        if [[ -z "${found}" ]]; then
            echo "  Error: No font files found in archive"
            return 1
        fi
        echo "  Using alternative: $(basename "${found}")"
    fi

    # Create target directory if needed
    mkdir -p "$(dirname "${target_path}")"
    cp "${found}" "${target_path}"
    log_info "  Installed: ${target_file}"
}

# Function to extract LICENSE from archive
extract_license_from_archive() {
    local search_dir="$1"
    local family_slug="$2"
    local family_name="$3"
    local url="$4"
    local license_id="$5"

    # Look for common license file names
    local license_file
    license_file=$(find "${search_dir}" -type f \( -name "LICENSE" -o -name "LICENSE.txt" -o -name "OFL.txt" -o -name "OFL-*.txt" \) | head -1)

    if [[ -n "${license_file}" ]]; then
        cp "${license_file}" "${LICENSE_DIR}/${family_slug}.txt"
        log_info "  License: ${LICENSE_DIR}/${family_slug}.txt"
    else
        # Create a placeholder if no license found
        cat > "${LICENSE_DIR}/${family_slug}.txt" <<EOF
# ${family_name}
# Downloaded from: ${url}
# License: ${license_id}

# License file not found in archive. Please refer to the source repository
# for the full license text corresponding to: ${license_id}
EOF
        log_info "  License: Placeholder (${license_id})"
    fi
}

# Function to extract license from already-present font file
# This is used when skipping downloads
extract_license() {
    local target_file="$1"
    local family_slug="$2"
    local family_name="$3"
    local license_id="$4"

    # Check if license already exists
    if [[ -f "${LICENSE_DIR}/${family_slug}.txt" ]]; then
        return 0
    fi

    # Create a placeholder license file
    cat > "${LICENSE_DIR}/${family_slug}.txt" <<EOF
# ${family_name}
# Source: ${target_file}
# License: ${license_id}

# This font file was already present in the corpus directory.
# For the full license text, please refer to the source repository
# and the license identifier specified above.
EOF
}

# Main script
# =============

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MANIFEST_FILE="${WORKSPACE_ROOT}/build/shape-corpus-manifest.txt"
CORPUS_DIR="${WORKSPACE_ROOT}/build/shape-corpus"
LICENSE_DIR="${WORKSPACE_ROOT}/build/font-licenses"

# Create directories
mkdir -p "${CORPUS_DIR}"
mkdir -p "${LICENSE_DIR}"

# Check if manifest exists
if [[ ! -f "${MANIFEST_FILE}" ]]; then
    echo "Error: Manifest file not found: ${MANIFEST_FILE}"
    exit 1
fi

# Read manifest and download fonts
# Skip comments and empty lines
while IFS='|' read -r family_name url license_id target_file; do
    # Skip comments and empty lines
    [[ "${family_name}" =~ ^#.*$ ]] && continue
    [[ -z "${family_name}" ]] && continue

    # Normalize family name for filename (replace spaces with underscores)
    family_slug=$(echo "${family_name}" | tr ' ' '_' | tr -cd '[:alnum:]_')
    target_path="${CORPUS_DIR}/${target_file}"

    # Skip if already downloaded
    if [[ -f "${target_path}" ]]; then
        log_skip "${family_name} - already present"
        # Still copy LICENSE if missing
        if [[ ! -f "${LICENSE_DIR}/${family_slug}.txt" ]]; then
            log_info "Extracting LICENSE for ${family_name}..."
            extract_license "${target_path}" "${family_slug}" "${family_name}" "${license_id}" || true
        fi
        continue
    fi

    log_info "Downloading ${family_name}..."
    download_font "${family_name}" "${url}" "${target_file}" "${family_slug}" "${license_id}"

done < "${MANIFEST_FILE}"

echo ""
log_info "Font corpus download complete!"
echo "  Corpus dir: ${CORPUS_DIR}"
echo "  License dir: ${LICENSE_DIR}"
echo ""
log_info "To generate the shape database, run:"
echo "  cargo xtask gen-shape-db ${CORPUS_DIR}"

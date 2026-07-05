#!/usr/bin/env bash
# Convert text-embedded PDFs to scanned image-based PDFs at 300 DPI
# This creates proper OCR test fixtures from text-embedded source PDFs

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

convert_to_scanned() {
    local input_pdf="$1"
    local output_pdf="$2"
    local dpi="${3:-300}"

    echo "Converting $input_pdf to scanned PDF at ${dpi} DPI..."

    # Create temporary directory for images
    local tmpdir
    tmpdir="$(mktemp -d)"

    # Convert PDF to PPM images at specified DPI
    pdftoppm -r "$dpi" "$input_pdf" "$tmpdir/page"

    # Check if images were created
    if ! ls "$tmpdir"/page-*.ppm 1>/dev/null 2>&1; then
        echo "Error: No images generated from $input_pdf"
        rm -rf "$tmpdir"
        return 1
    fi

    # Convert images back to PDF using ImageMagick via nix-shell
    nix-shell -p imagemagick --run "convert $tmpdir/page-*.ppm \"$output_pdf\""

    # Cleanup temporary directory
    rm -rf "$tmpdir"

    echo "Created: $output_pdf"
}

# Main processing
main() {
    local fixtures=(
        "invoice/invoice-300dpi.pdf"
        "letter/letter-300dpi.pdf"
        "form/form-300dpi.pdf"
    )

    cd "$SCRIPT_DIR"

    for fixture in "${fixtures[@]}"; do
        local input="$fixture"
        local backup="${fixture%.pdf}-text-embedded.pdf"
        local output="$fixture"

        # Skip if this is already a backup file
        if [[ "$input" == *"-text-embedded.pdf" ]]; then
            continue
        fi

        # Only process if the input exists
        if [ -f "$input" ]; then
            # Backup the original text-embedded version
            cp "$input" "$backup"
            echo "Backed up $input to $backup"

            # Convert to scanned version
            convert_to_scanned "$backup" "$output" 300
        else
            echo "Warning: $input not found, skipping"
        fi
    done

    echo ""
    echo "Conversion complete!"
    echo "Original text-embedded versions backed up as *-text-embedded.pdf"
}

# Run main function
main "$@"

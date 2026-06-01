#!/run/current-system/sw/bin/bash
# Wrapper script to run generate_scanned_fixtures.py with nix-shell dependencies

nix shell nixpkgs#python3Packages.reportlab nixpkgs#python3Packages.pillow nixpkgs#python3Packages.img2pdf nixpkgs#poppler_utils --command bash -c '
  cd "$(dirname "$0")"
  python3 generate_scanned_fixtures.py "$@"
'

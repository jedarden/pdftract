#!/usr/bin/env bash
# Verify the non-WER invariants of the OCR acceptance corpus.
#
# This is intentionally separate from measure-wer.sh: WER measures text
# quality, while this gate checks that each PDF is routed to the expected
# extraction mode and that the edge fixtures retain verifiable provenance.
# Live dependencies are pdfinfo, pdfimages, pdftotext, sha256sum, and python3.

set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SELF_DIR/.." && pwd)"
CORPUS="${OCR_ACCEPTANCE_CORPUS:-$PROJECT_ROOT/tests/fixtures/scanned}"
MANIFEST_SCRIPT="$PROJECT_ROOT/scripts/measure-wer.sh"

die() { printf 'verify-ocr-acceptance: error: %s\n' "$*" >&2; exit 2; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || die "missing dependency '$1'"; }

[[ -d "$CORPUS" ]] || die "corpus directory not found: $CORPUS"
for command_name in pdfinfo pdfimages pdftotext sha256sum python3; do
    require_cmd "$command_name"
done

printf '=== OCR acceptance routing and provenance ===\n'
printf 'corpus: %s\n' "$CORPUS"
printf '\n%-34s %-9s %-8s %-8s %-8s %-8s\n' FIXTURE MODE PAGES IMAGES DPI PROVENANCE

manifest_started=0
verified=0
while IFS='|' read -r name class mode target scan_rel gt_rel rec_rel metadata_rel; do
    [[ -z "$name" ]] && continue
    scan="$CORPUS/$scan_rel"
    gt="$CORPUS/$gt_rel"
    [[ -f "$scan" ]] || die "$name: scan missing: $scan"
    [[ -f "$gt" ]] || die "$name: ground truth missing: $gt"

    pages="$(pdfinfo "$scan" | awk '/^Pages:/ { print $2; exit }')"
    images="$(pdfimages -list "$scan" | awk 'NR > 2 && $1 ~ /^[0-9]+$/ { count++ } END { print count + 0 }')"
    [[ "$pages" =~ ^[0-9]+$ && "$pages" -gt 0 ]] || die "$name: invalid page count"
    [[ "$images" =~ ^[0-9]+$ && "$images" -gt 0 ]] || die "$name: no embedded image XObject"

    if [[ "$mode" == "scanned" ]]; then
        [[ "$images" -eq "$pages" ]] || die "$name: scanned route requires one embedded image per page (pages=$pages images=$images)"
        vector_text="$(pdftotext "$scan" - 2>/dev/null | tr -d '[:space:]')"
        [[ -z "$vector_text" ]] || die "$name: scanned route unexpectedly contains vector text"
    elif [[ "$mode" == "mixed" ]]; then
        [[ "$images" -ge 1 ]] || die "$name: mixed route needs an image layer"
        vector_text="$(pdftotext "$scan" - 2>/dev/null | tr -d '[:space:]')"
        [[ -n "$vector_text" ]] || die "$name: mixed route needs a vector text layer"
    else
        die "$name: unsupported extraction mode '$mode'"
    fi

    dpi="n/a"
    provenance="legacy"
    if [[ "$metadata_rel" != "-" ]]; then
        metadata="$CORPUS/$metadata_rel"
        [[ -f "$metadata" ]] || die "$name: provenance sidecar missing: $metadata"
        readarray -t fields < <(python3 - "$metadata" "$name" "$mode" "$target" "$scan" "$gt" <<'PY'
import hashlib
import json
import sys

metadata_path, expected_name, expected_mode, expected_target, pdf_path, gt_path = sys.argv[1:]
with open(metadata_path, encoding="utf-8") as stream:
    data = json.load(stream)
if data.get("fixture") != expected_name:
    raise SystemExit("metadata fixture does not match manifest")
if data.get("expected_extraction_mode") != expected_mode:
    raise SystemExit("metadata extraction mode does not match manifest")
if str(data.get("wer_target_percent")) != expected_target:
    raise SystemExit("metadata WER target does not match manifest")
for key in ("source", "license", "generator", "generation_method"):
    if not data.get(key):
        raise SystemExit(f"metadata field {key!r} is empty")
for path, key in ((pdf_path, "pdf_sha256"), (gt_path, "ground_truth_sha256")):
    actual = hashlib.sha256(open(path, "rb").read()).hexdigest()
    if data.get(key) != actual:
        raise SystemExit(f"metadata {key} does not match current file")
print(data["dpi"])
print(data["expected_page_type"])
print(data["license"])
PY
        ) || die "$name: invalid provenance sidecar"
        dpi="${fields[0]}"
        [[ "${fields[1]}" == "$mode" ]] || die "$name: expected page type does not match mode"
        provenance="${fields[2]}"
        [[ "$dpi" =~ ^[0-9]+$ && "$dpi" -gt 0 ]] || die "$name: invalid provenance DPI"
        image_dpi="$(pdfimages -list "$scan" | awk 'NR > 2 && $1 ~ /^[0-9]+$/ { print $13; exit }')"
        [[ "$image_dpi" == "$dpi" ]] || die "$name: embedded image DPI $image_dpi does not match metadata DPI $dpi"
    fi

    printf '%-34s %-9s %-8s %-8s %-8s %-8s\n' "$name" "$mode" "$pages" "$images" "$dpi" "$provenance"
    verified=$((verified + 1))
done < <(
    awk '
        /^read -r -d/ && /FIXTURE_MANIFEST/ { active=1; next }
        active && /^EOF$/ { exit }
        active && /^[^#[:space:]].*\|/ { print }
    ' "$MANIFEST_SCRIPT"
)

(( verified > 0 )) || die "manifest contains no fixtures"
printf '\nROUTING/PROVENANCE: PASS — %s manifested fixtures verified\n' "$verified"

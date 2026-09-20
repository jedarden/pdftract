#!/usr/bin/env bash
# measure-wer.sh — scanned-corpus Word Error Rate measurement and quality gate.
#
# Measures the OCR quality of the scanned fixture corpus in
# tests/fixtures/scanned/ against its ground-truth transcripts and enforces
# the Tier-1 OCR accuracy gate: every clean 300 DPI fixture must measure
# WER <= 3%. The intentionally degraded 200 DPI fixture is measured and
# reported on its own line with its own soft target; it never contributes to
# the gate (a high degraded-fixture WER is expected, not a failure).
#
# OCR command/dependency contract
# -------------------------------
# Live mode produces OCR text exactly the way the committed reference outputs
# were generated (see tests/fixtures/scanned/GEN_MANIFEST.md, "Regeneration"):
#
#   pdfimages -png <scan.pdf> <tmp>/page          # poppler-utils
#   tesseract <tmp>/page-NNN.png stdout -l eng    # tesseract 5.x, once per
#                                                 # page, concatenated in
#                                                 # page order
#
# Fixture contract: every gated scan is an image-only PDF with exactly one
# embedded image per page (verified at runtime against `pdfinfo`), rasterized
# at 300 DPI (200 DPI for the degraded fixture).
#
# Dependencies, live mode: tesseract 5.x with the `eng` traineddata,
# poppler-utils (pdfimages + pdfinfo), python3 (stdlib only — no pip
# packages). On this machine the OCR tools come from nix:
#
#   nix-shell -p tesseract poppler-utils python3 --run 'scripts/measure-wer.sh'
#
# --recorded skips the OCR step and measures the committed *-ocr.txt reference
# outputs instead. That verifies the gate arithmetic and the recorded state of
# the corpus — it does NOT measure live OCR quality — and needs only python3.
#
# Normalization contract (applied identically to both sides before the
# word-level Levenshtein comparison): Unicode NFKC, curly quotes/apostrophes
# folded to ASCII, en/em dashes and the minus sign folded to "-", ellipsis
# folded to "...", non-breaking space folded to space, then whitespace
# tokenization. Comparison is case-sensitive. WER = (S + D + I) / reference
# words.
#
# Usage
# -----
#   scripts/measure-wer.sh                   corpus mode, live OCR (the gate)
#   scripts/measure-wer.sh --recorded        corpus mode on committed *-ocr.txt
#   scripts/measure-wer.sh --corpus DIR      alternate corpus root
#   scripts/measure-wer.sh --fixture NAME    measure one manifested fixture only
#   scripts/measure-wer.sh --threshold PCT   clean gate threshold (default 3)
#   scripts/measure-wer.sh --self-test       built-in acceptance checks
#   scripts/measure-wer.sh OCR.txt GT.txt    legacy two-file comparison
#   scripts/measure-wer.sh -h | --help
#
# Exit codes
#   0  gate passed — every clean fixture measured WER <= threshold
#   1  gate failed — a clean fixture measured WER > threshold, or --self-test
#      found a broken invariant
#   2  usage or environment error (missing files, missing OCR dependencies)
#
# Environment: VERBOSE=1 prints word-level substitution/deletion/insertion
# detail for fixtures that fail the gate.

set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SELF="$SELF_DIR/$(basename "${BASH_SOURCE[0]}")"
PROJECT_ROOT="$(cd "$SELF_DIR/.." && pwd)"
DEFAULT_CORPUS="$PROJECT_ROOT/tests/fixtures/scanned"

THRESHOLD_PCT="3"
DEGRADED_TARGET_PCT="10"   # informational soft target, matches GEN_MANIFEST.md
RECORDED=0
CORPUS=""
ONLY_FIXTURE=""
SELF_TEST=0
MODE=""                    # "corpus" | "twofile"
TWOFILE_OCR=""
TWOFILE_GT=""

die() { local code="$1"; shift; printf 'measure-wer: error: %s\n' "$*" >&2; exit "$code"; }

# Shared temp workspace; set (globally, so the EXIT trap can still see it)
# immediately before the trap is (re)armed in corpus/self-test mode.
WORK_DIR=""
cleanup() { [[ -n "$WORK_DIR" ]] && rm -rf -- "$WORK_DIR"; return 0; }

usage() { awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$SELF"; }

# ---------------------------------------------------------------------------
# Manifest: name|class|scan.pdf|ground-truth.txt|reference-ocr.txt
# Paths are relative to the corpus root. `clean` fixtures carry the gate;
# `degraded` fixtures are reported but never gate. To add a fixture: generate
# the scan + ground truth per tests/fixtures/scanned/README.md ("Adding New
# Fixtures"), produce the reference OCR, measure it, then add a row here and
# to README.md / GEN_MANIFEST.md.
# ---------------------------------------------------------------------------
read -r -d '' FIXTURE_MANIFEST <<'EOF' || true
receipt-300dpi|clean|receipt/receipt-300dpi-scanned.pdf|receipt/receipt-300dpi.txt|receipt/receipt-300dpi-ocr.txt
invoice-300dpi|clean|invoice/invoice-300dpi.pdf|invoice/invoice-300dpi-ground-truth.txt|invoice/invoice-300dpi-ocr.txt
letter-300dpi|clean|letter/letter-300dpi.pdf|letter/letter-300dpi-ground-truth.txt|letter/letter-300dpi-ocr.txt
form-300dpi|clean|form/form-300dpi.pdf|form/form-300dpi-ground-truth.txt|form/form-300dpi-ocr.txt
report-300dpi|clean|multi-page/report-300dpi.pdf|multi-page/report-300dpi-ground-truth.txt|multi-page/report-300dpi-ocr.txt
degraded-200dpi|degraded|low-quality/degraded-200dpi.pdf|low-quality/degraded-200dpi.txt|low-quality/degraded-200dpi-ocr.txt
EOF

# ---------------------------------------------------------------------------
# WER engine: normalize both sides identically, then word-level Levenshtein.
# Emits "SUMMARY<TAB>ref_words<TAB>subs<TAB>dels<TAB>ins<TAB>errors<TAB>wer_pct"
# and, when detail=1, up to 15 "SUB/DEL/INS" lines for the worst offenders.
# ---------------------------------------------------------------------------
wer_compute() { # $1 = ground-truth path, $2 = hypothesis path, $3 = detail 0|1
    python3 - "$1" "$2" "$3" <<'PY'
import sys, unicodedata

gt_path, hyp_path, detail = sys.argv[1], sys.argv[2], sys.argv[3] == "1"

def tokens(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
    text = unicodedata.normalize("NFKC", text)
    for src, dst in (("‘", "'"), ("’", "'"), ("“", '"'),
                     ("”", '"'), ("–", "-"), ("—", "-"),
                     ("−", "-"), ("…", "..."), (" ", " ")):
        text = text.replace(src, dst)
    return text.split()

ref, hyp = tokens(gt_path), tokens(hyp_path)
m, n = len(ref), len(hyp)

dp = [[0] * (n + 1) for _ in range(m + 1)]
for i in range(m + 1):
    dp[i][0] = i
for j in range(n + 1):
    dp[0][j] = j
for i in range(1, m + 1):
    ri = ref[i - 1]
    row, prev = dp[i], dp[i - 1]
    for j in range(1, n + 1):
        dp[i][j] = prev[j - 1] if ri == hyp[j - 1] else 1 + min(
            prev[j], row[j - 1], prev[j - 1])

ops = []
i, j = m, n
while i > 0 or j > 0:
    if i > 0 and j > 0 and ref[i - 1] == hyp[j - 1]:
        i, j = i - 1, j - 1
    elif i > 0 and j > 0 and dp[i][j] == dp[i - 1][j - 1] + 1:
        ops.append(("SUB", ref[i - 1], hyp[j - 1])); i, j = i - 1, j - 1
    elif i > 0 and dp[i][j] == dp[i - 1][j] + 1:
        ops.append(("DEL", ref[i - 1], "")); i -= 1
    else:
        ops.append(("INS", "", hyp[j - 1])); j -= 1
ops.reverse()

errors = dp[m][n]
pct = (0.0 if n == 0 else 100.0) if m == 0 else errors * 100.0 / m
subs = sum(1 for op in ops if op[0] == "SUB")
dels = sum(1 for op in ops if op[0] == "DEL")
ins = sum(1 for op in ops if op[0] == "INS")
print(f"SUMMARY\t{m}\t{subs}\t{dels}\t{ins}\t{errors}\t{pct:.2f}")
if detail:
    for op, r, h in ops[:15]:
        if op == "SUB":
            print(f"SUB\t{r!r} -> {h!r}")
        elif op == "DEL":
            print(f"DEL\t{r!r} missing from OCR")
        else:
            print(f"INS\t{h!r} not in ground truth")
PY
}

# Float compare: wer_le WER THRESHOLD -> exit 0 when WER <= THRESHOLD
wer_le() { awk -v w="$1" -v t="$2" 'BEGIN { exit !(w <= t) }'; }

require_cmd() { # $1 = command, $2 = hint
    command -v "$1" >/dev/null 2>&1 || die 2 "missing dependency: '$1'. $2"
}

# Guard against file operands that begin with "-" (path-safety).
dash_guard() { [[ "$1" == -* ]] && printf './%s' "$1" || printf '%s' "$1"; }

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
positionals=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --recorded) RECORDED=1 ;;
        --self-test) SELF_TEST=1 ;;
        --corpus) [[ $# -ge 2 ]] || die 2 "--corpus requires a directory argument"
                  CORPUS="$2"; shift ;;
        --fixture) [[ $# -ge 2 ]] || die 2 "--fixture requires a fixture name argument"
                   ONLY_FIXTURE="$2"; shift ;;
        --threshold) [[ $# -ge 2 ]] || die 2 "--threshold requires a percentage argument"
                     [[ "$2" =~ ^[0-9]+([.][0-9]+)?$ ]] || die 2 "--threshold must be a non-negative number (percent), got '$2'"
                     THRESHOLD_PCT="$2"; shift ;;
        --) while [[ $# -gt 1 ]]; do positionals+=("$2"); shift; done; break ;;
        -*) die 2 "unknown option: $1 (see --help)" ;;
        *) positionals+=("$1") ;;
    esac
    shift
done

if (( SELF_TEST )); then
    [[ ${#positionals[@]} -eq 0 ]] || die 2 "--self-test takes no positional arguments"
    MODE="selftest"
elif [[ ${#positionals[@]} -eq 2 ]]; then
    MODE="twofile"
    TWOFILE_OCR="${positionals[0]}"
    TWOFILE_GT="${positionals[1]}"
elif [[ ${#positionals[@]} -eq 0 ]]; then
    MODE="corpus"
    [[ -n "$CORPUS" ]] || CORPUS="$DEFAULT_CORPUS"
else
    die 2 "expected exactly two file arguments for comparison mode, got ${#positionals[@]} (see --help)"
fi

# ---------------------------------------------------------------------------
# Two-file comparison mode (legacy entry point: <ocr.txt> <ground-truth.txt>)
# ---------------------------------------------------------------------------
run_twofile() {
    local ocr gt out detail
    ocr="$(dash_guard "$TWOFILE_OCR")"
    gt="$(dash_guard "$TWOFILE_GT")"
    [[ -f "$ocr" ]] || die 2 "OCR output file not found: $TWOFILE_OCR"
    [[ -f "$gt" ]] || die 2 "ground-truth file not found: $TWOFILE_GT"
    detail=0
    if [[ "${VERBOSE:-0}" == "1" ]]; then detail=1; fi
    out="$(wer_compute "$gt" "$ocr" "$detail")" || die 2 "WER computation failed"
    parse_summary "$out"
    printf 'OCR: %s\nGround truth: %s\n' "$TWOFILE_OCR" "$TWOFILE_GT"
    printf 'WER: %s%% (%s errors / %s reference words)  [S=%s D=%s I=%s]\n' \
        "$S_WER" "$S_ERRORS" "$S_WORDS" "$S_SUBS" "$S_DELS" "$S_INS"
    if [[ "$detail" == 1 ]]; then
        grep -v '^SUMMARY' <<<"$out" | sed 's/^/  /' || true
    fi
    if wer_le "$S_WER" "3"; then
        printf 'threshold: 3.00%% -> PASS (exit 0)\n'
        return 0
    fi
    printf 'threshold: 3.00%% -> FAIL (exit 1)\n'
    return 1
}

# Split a SUMMARY line into S_* globals
parse_summary() {
    local line
    line="$(grep '^SUMMARY' <<<"$1")" || die 2 "WER engine produced no summary (python3 failure?)"
    IFS=$'\t' read -r _ S_WORDS S_SUBS S_DELS S_INS S_ERRORS S_WER <<<"$line"
}

# ---------------------------------------------------------------------------
# Live OCR of one fixture: scan.pdf -> concatenated tesseract stdout per page
# Sets PAGES. Requires: $1 scan, $2 output txt, $3 stderr log, $4 page-img dir.
# ---------------------------------------------------------------------------
ocr_fixture() {
    local scan out log imgdir img pages images
    scan="$1"; out="$2"; log="$3"; imgdir="$4"
    mkdir -p "$imgdir"
    pdfimages -png "$scan" "$imgdir/page" 2>>"$log" || die 2 "pdfimages failed on $scan (see $log)"
    mapfile -d '' images < <(find "$imgdir" -maxdepth 1 -name 'page-*.png' -print0 | LC_ALL=C sort -z)
    pages="$(pdfinfo "$scan" 2>>"$log" | awk '/^Pages:/ { print $2 }')"
    PAGES="$pages"
    [[ "$pages" =~ ^[0-9]+$ ]] || die 2 "could not read page count from pdfinfo for $scan"
    [[ ${#images[@]} -eq "$pages" ]] || die 2 "$scan has ${#images[@]} embedded images for $pages pages; the fixture contract is exactly one image per page (see scripts/measure-wer.sh header and tests/fixtures/scanned/GEN_MANIFEST.md)"
    : > "$out"
    for img in "${images[@]}"; do
        timeout 120 tesseract "$(dash_guard "$img")" stdout -l eng >> "$out" 2>>"$log" \
            || die 2 "tesseract failed on $(basename "$img") of $scan (see $log)"
    done
}

# ---------------------------------------------------------------------------
# Corpus mode
# ---------------------------------------------------------------------------
run_corpus() {
    [[ -d "$CORPUS" ]] || die 2 "corpus directory not found: $CORPUS"
    CORPUS="$(cd "$CORPUS" && pwd)"
    require_cmd python3 "WER arithmetic needs python3 on PATH."

    if (( ! RECORDED )); then
        require_cmd pdfimages "live OCR rasterizes pages with poppler's pdfimages (nix: nix-shell -p poppler-utils)."
        require_cmd pdfinfo   "live OCR cross-checks page counts with poppler's pdfinfo (nix: nix-shell -p poppler-utils)."
        require_cmd tesseract "live OCR needs tesseract 5.x with the 'eng' traineddata (nix: nix-shell -p tesseract poppler-utils)."
    fi

    local -a lines=()
    mapfile -t lines <<<"$FIXTURE_MANIFEST"

    if [[ -n "$ONLY_FIXTURE" ]]; then
        local found=""
        for spec in "${lines[@]}"; do
            [[ "${spec%%|*}" == "$ONLY_FIXTURE" ]] && found=1
        done
        [[ -n "$found" ]] || die 2 "unknown fixture '$ONLY_FIXTURE' (manifested: $(echo "$FIXTURE_MANIFEST" | cut -d'|' -f1 | paste -sd, -))"
    fi

    local work
    work="$(mktemp -d "${TMPDIR:-/tmp}/measure-wer.XXXXXXXX")" \
        || die 2 "cannot create temporary directory"
    WORK_DIR="$work"
    trap cleanup EXIT

    local gate_fail=0
    local agg_errors=0 agg_words=0 clean_count=0
    local -a table=() failed_fixtures=()

    printf '=== scanned-corpus WER measurement ===\n'
    printf 'corpus:     %s\n' "$CORPUS"
    printf 'threshold:  clean fixtures <= %s%% (degraded reported separately, non-gating)\n' "$THRESHOLD_PCT"
    if (( RECORDED )); then
        printf 'mode:       recorded — committed *-ocr.txt reference outputs (NOT live OCR)\n'
    else
        printf 'mode:       live OCR — pdfimages -png + tesseract stdout -l eng, concatenated per page\n'
        printf 'tesseract:  %s\n' "$(tesseract --version 2>/dev/null | head -1)"
    fi
    printf '\n%-18s %5s %6s %4s %4s %4s %8s  %s\n' \
        FIXTURE PAGES WORDS SUB DEL INS WER GATE

    local spec name class scan_rel gt_rel rec_rel scan gt rec
    for spec in "${lines[@]}"; do
        IFS='|' read -r name class scan_rel gt_rel rec_rel <<<"$spec"
        [[ -n "$ONLY_FIXTURE" && "$name" != "$ONLY_FIXTURE" ]] && continue
        scan="$(dash_guard "$CORPUS/$scan_rel")"
        gt="$(dash_guard "$CORPUS/$gt_rel")"
        rec="$(dash_guard "$CORPUS/$rec_rel")"
        [[ -f "$scan" ]] || die 2 "fixture '$name': scan PDF not found: $CORPUS/$scan_rel"
        [[ -f "$gt" ]] || die 2 "fixture '$name': ground-truth transcript not found: $CORPUS/$gt_rel"

        local hyp pages="?"
        if (( RECORDED )); then
            [[ -f "$rec" ]] || die 2 "fixture '$name': reference OCR output missing: $CORPUS/$rec_rel — regenerate per GEN_MANIFEST.md or run live OCR mode"
            hyp="$rec"
        else
            hyp="$work/$name.ocr.txt"
            ocr_fixture "$scan" "$hyp" "$work/$name.tesseract.log" "$work/$name.pages"
            pages="$PAGES"
        fi

        local out
        out="$(wer_compute "$gt" "$hyp" 0)" || die 2 "WER computation failed for $name"
        parse_summary "$out"

        local status gate_cell
        if [[ "$class" == "degraded" ]]; then
            gate_cell="non-gating"
            if wer_le "$S_WER" "$DEGRADED_TARGET_PCT"; then
                status="REPORTED (soft target <${DEGRADED_TARGET_PCT}%)"
            else
                status="REPORTED-OVER (soft target ${DEGRADED_TARGET_PCT}%)"
            fi
        else
            gate_cell="<=${THRESHOLD_PCT}%"
            if wer_le "$S_WER" "$THRESHOLD_PCT"; then
                status="PASS"
                clean_count=$((clean_count + 1))
                agg_errors=$((agg_errors + S_ERRORS))
                agg_words=$((agg_words + S_WORDS))
            else
                status="FAIL"
                gate_fail=1
                failed_fixtures+=("$name")
                agg_errors=$((agg_errors + S_ERRORS))
                agg_words=$((agg_words + S_WORDS))
            fi
        fi
        table+=("$(printf '%-18s %5s %6s %4s %4s %4s %8s  %s' \
            "$name" "$pages" "$S_WORDS" "$S_SUBS" "$S_DELS" "$S_INS" "${S_WER}%" "$status ($gate_cell)")")

        if [[ "$status" == FAIL* || "$status" == REPORTED-OVER* ]] && [[ "${VERBOSE:-0}" == "1" ]]; then
            printf -- '--- %s word-level detail (first 15) ---\n' "$name"
            wer_compute "$gt" "$hyp" 1 | grep -v '^SUMMARY' || true
        fi
    done

    local row
    for row in "${table[@]}"; do printf '%s\n' "$row"; done

    if (( agg_words > 0 )); then
        awk -v e="$agg_errors" -v w="$agg_words" -v t="$THRESHOLD_PCT" \
            'BEGIN { printf "\nclean aggregate: %d errors / %d reference words = %.2f%% (micro-average; threshold %.2f%%)\n", e, w, e*100/w, t }'
    fi

    note_unmanifested_dirs

    if (( gate_fail )); then
        local f
        printf 'GATE: FAIL — clean fixture(s) above %s%%:\n' "$THRESHOLD_PCT"
        for f in "${failed_fixtures[@]}"; do
            printf '  - %s: see the S/D/I columns (SUB=misread, DEL=dropped, INS=hallucinated words);\n' "$f"
            printf '    re-check preprocessing and the OCR-stability rules in tests/fixtures/scanned/GEN_MANIFEST.md\n'
        done
        return 1
    fi
    printf 'GATE: PASS — %s clean fixture(s) at or under %s%%\n' "$clean_count" "$THRESHOLD_PCT"
    return 0
}

# Directories under the corpus root that host no manifested fixture are
# called out so a newly added fixture directory cannot silently sit outside
# the gate. Informational only.
note_unmanifested_dirs() {
    local -a manifested_dirs=()
    local spec d base matched candidate
    while IFS='|' read -r _ _ scan_rel _ _; do
        manifested_dirs+=("$(dirname "$scan_rel")")
    done <<<"$FIXTURE_MANIFEST"
    while IFS= read -r -d '' d; do
        base="$(basename "$d")"
        matched=""
        for candidate in "${manifested_dirs[@]}"; do
            if [[ "$candidate" == "$base" ]]; then matched=1; break; fi
        done
        if [[ -z "$matched" ]] && compgen -G "$d/*.pdf" >/dev/null; then
            printf 'NOTE: corpus directory "%s/" hosts no manifested fixture; add it to FIXTURE_MANIFEST in scripts/measure-wer.sh if it should join the gate.\n' "$base" >&2
        fi
    done < <(find "$CORPUS" -mindepth 1 -maxdepth 1 -type d -print0 | LC_ALL=C sort -z)
}

# ---------------------------------------------------------------------------
# Self-test: exercises the gate's failure paths without OCR dependencies
# (recorded mode + forced corruptions on temp copies). Also asserts the run
# never modifies the fixture tree.
# ---------------------------------------------------------------------------
run_selftest() {
    local work pass=0 fail=0
    work="$(mktemp -d "${TMPDIR:-/tmp}/measure-wer-selftest.XXXXXXXX")" \
        || die 2 "cannot create temporary directory"
    WORK_DIR="$work"
    trap cleanup EXIT
    local corpus="$DEFAULT_CORPUS"
    [[ -n "$CORPUS" ]] && corpus="$CORPUS"
    [[ -d "$corpus" ]] || die 2 "corpus directory not found: $corpus"
    require_cmd python3 "--self-test needs python3 on PATH."
    require_cmd sha256sum "--self-test asserts fixture immutability with sha256sum."

    fingerprint() {
        find "$corpus" -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum | sha256sum | cut -d' ' -f1
    }

    check() { # $1 = label, $2 = expected ("zero"|"nonzero"), $3 = actual exit
        local ok="FAIL"
        if [[ "$2" == "zero" && "$3" -eq 0 ]] || [[ "$2" == "nonzero" && "$3" -ne 0 ]]; then
            ok="PASS"; pass=$((pass + 1))
        else
            fail=$((fail + 1))
        fi
        printf '  [%s] %-58s (expected %s, got exit %s)\n' "$ok" "$1" "$2" "$3"
    }

    local rc=0 before after
    before="$(fingerprint)"

    printf '=== measure-wer self-test (corpus: %s) ===\n' "$corpus"

    # 1. Identical texts: WER 0%, exit 0.
    printf 'ground truth copy\n' > "$work/same.txt"
    rc=0
    "$SELF" "$work/same.txt" "$work/same.txt" >/dev/null 2>&1 </dev/null || rc=$?
    check "two-file identical texts pass (exit 0)" zero "$rc"

    # 2. Forced clean-fixture failure: corrupt a copy of a clean fixture's
    #    ground truth in temp space; the gate must go nonzero.
    sed -E 's/\<(the|and|of|to|for|with|our|from)\>/zzz/gI' \
        "$corpus/letter/letter-300dpi-ground-truth.txt" > "$work/corrupted.txt" \
        || die 2 "cannot build corrupted copy (sed missing?)"
    rc=0
    "$SELF" "$corpus/letter/letter-300dpi-ocr.txt" "$work/corrupted.txt" >/dev/null 2>&1 </dev/null || rc=$?
    check "forced clean-fixture corruption fails (exit nonzero)" nonzero "$rc"

    # 3. Recorded corpus: the committed reference state must pass the gate.
    rc=0
    "$SELF" --recorded --corpus "$corpus" >/dev/null 2>&1 </dev/null || rc=$?
    check "recorded corpus gate passes (exit 0)" zero "$rc"

    # 4. The threshold is actually enforced: with a 0% threshold the same
    #    corpus must fail.
    rc=0
    "$SELF" --recorded --corpus "$corpus" --threshold 0 >/dev/null 2>&1 </dev/null || rc=$?
    check "threshold 0 forces corpus failure (exit nonzero)" nonzero "$rc"

    # 5. Unknown flags are usage errors (exit 2, not a crash).
    rc=0
    "$SELF" --frobnicate >/dev/null 2>&1 </dev/null || rc=$?
    check "unknown option rejected (exit 2)" zero "$(( rc == 2 ? 0 : 1 ))"

    # 6. Nothing under the corpus tree was modified by any of the above.
    after="$(fingerprint)"
    if [[ "$before" == "$after" ]]; then pass=$((pass + 1)); printf '  [PASS] %-58s\n' "fixture tree unchanged (sha256 fingerprint)"; else fail=$((fail + 1)); printf '  [FAIL] %-58s\n' "fixture tree UNCHANGED (sha256 fingerprint mismatch)"; fi

    printf '=== self-test: %s passed, %s failed ===\n' "$pass" "$fail"
    [[ "$fail" -eq 0 ]]
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------
case "$MODE" in
    twofile)  run_twofile ;;
    corpus)   run_corpus ;;
    selftest) run_selftest ;;
    *)        die 2 "internal dispatch error" ;;
esac

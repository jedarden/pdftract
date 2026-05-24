# Manual Platform Smoke Test (KU-12)

> **Purpose:** This runbook is the canonical smoke test executed before each milestone release on at least one physical macOS machine and one Windows VM. Per KU-12, Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release.

**Execution frequency:** Quarterly (per KU-12) or before each milestone release.

**Executor:** Release lead or designated QA engineer.

---

## Step 1: Validate the environment (pdftract doctor)

Before running any extractions, validate the deployment with:

```bash
pdftract doctor
```

### Expected output on a fully-provisioned host

```
Check                         Status  Detail
────────────────────────────────────────────────────────────────────────────────
pdftract binary               OK      0.1.0 (git: abc1234)
Features: OCR, FULL_RENDER, PROFILES, SERVE, MCP, INSPECT, GREP, CACHE, RECEIPTS, MARKDOWN
tesseract install             OK      tesseract 5.3.0 found (major >= 5)
tesseract languages           OK      All required languages present: ["eng", "osd"]
leptonica install             OK      leptonica 1.82.0 found (>= 1.79)
libtiff                       OK      libtiff 4.4.0 found
libopenjp2                    OK      libopenjp2 2.5.0 found
pdfium native lib             OK      pdfium 6555 found (loaded from /usr/lib/x86_64-linux-gnu/libpdfium.so)
network reachability          OK      Network reachable: 200 in 0.23s
cache directory               OK      Layout version 1 (current) at /home/user/.cache/pdftract
profile search path           OK      All 9 profile(s) valid at /home/user/.config/pdftract/profiles
ulimit -n                     OK      File descriptor limit: 65536
available RAM                 OK      16384 MiB available
system locale                 OK      Locale 'en_US.UTF-8' (UTF-8)
temp dir writable             OK      Temp dir writable at /tmp
────────────────────────────────────────────────────────────────────────────────
15 OK, 0 WARN, 0 FAIL
```

### Exit policy

- **Exit code 0:** All checks OK or WARN (no FAIL). Deployment proceeds.
- **Exit code 1:** At least one check reports FAIL. Deployment **blocked**; resolve FAIL rows before proceeding.

Any FAIL row blocks deployment. See the [troubleshooting table](#troubleshooting) below for each FAIL.

### For CI integration

Use `--json` for machine-consumable output:

```bash
pdftract doctor --json | jq -e '.summary.fail == 0' || exit 1
```

Example JSON output:

```json
{
  "summary": {
    "ok": 14,
    "warn": 1,
    "fail": 0,
    "total": 15
  },
  "checks": [
    {
      "name": "pdftract binary",
      "status": "ok",
      "detail": "0.1.0 (git: abc1234)\nFeatures: OCR, FULL_RENDER, ..."
    },
    ...
  ]
}
```

### Interpreting WARN rows

WARN does **not** block deployment but should be tracked. Recommended action: open a tracking ticket per WARN row for resolution in the next patch release. Common WARN scenarios:

- **tesseract install (WARN):** Tesseract 4.x detected. OCR results may have minor glyph errors. Plan upgrade to 5.x.
- **cache directory (WARN):** Low disk space (< 1 GiB free). Monitor cache growth; add storage if needed.
- **ulimit -n (WARN):** File descriptor limit between 512–1023. May hit limits with batch operations. Increase to 4096+.
- **available RAM (WARN):** Less than 256 MiB free. Risk of OOM with large PDFs. Close other processes or add RAM.
- **system locale (WARN):** Non-UTF-8 locale (e.g., `C` or `POSIX`). May cause encoding issues with non-ASCII text. Export `LANG=en_US.UTF-8`.

---

## Troubleshooting

| Check | Common cause | Fix |
|---|---|---|
| pdftract binary (FAIL) | Corrupted binary or build artifact | Reinstall: `cargo install pdftract --force` or `pip install --force-reinstall pdftract` |
| tesseract install (FAIL) | binary missing | `apt install tesseract-ocr` (Debian/Ubuntu) or `brew install tesseract` (macOS) |
| tesseract install (FAIL) | major <= 3 | Upgrade to Tesseract 5.x via package manager |
| tesseract languages (FAIL) | eng pack missing | `apt install tesseract-ocr-eng` (Debian/Ubuntu) or `brew install tesseract-lang` (macOS) |
| tesseract languages (WARN) | optional langs missing | Install requested langs: `apt install tesseract-ocr-<lang>` |
| leptonica install (FAIL) | dev headers missing | `apt install libleptonica-dev` (Debian/Ubuntu) or `brew install leptonica` (macOS) |
| leptonica install (WARN) | older version (< 1.79) | Upgrade via package manager; WARN may be acceptable for basic OCR |
| libtiff (FAIL) | CCITT decoding library missing | `apt install libtiff-dev` (Debian/Ubuntu) or `brew install libtiff` (macOS) |
| libopenjp2 (FAIL) | JPEG2000 decoding library missing | `apt install libopenjp2-7-dev` (Debian/Ubuntu) or `brew install openjpeg` (macOS) |
| pdfium native lib (FAIL) | PDFium library not found | Install pdfium-render dependencies or compile with bundled PDFium |
| pdfium native lib (WARN) | older version (< 6555) | Upgrade PDFium; WARN may be acceptable for basic rendering |
| network reachability (FAIL) | no internet or firewall blocking | Check network connectivity; ensure HTTPS outbound is allowed |
| network reachability (WARN) | slow response (> 5s) or 3xx redirect | Check proxy settings; 3xx may indicate redirect loop |
| cache directory (FAIL) | not writable or layout incompatible | Check permissions: `ls -ld ~/.cache/pdftract`; fix ownership or recreate cache |
| cache directory (WARN) | low disk space (< 1 GiB free) | Clear cache: `pdftract cache clear` or add disk space |
| profile search path (FAIL) | YAML parse errors or forbidden keys | Run `pdftract profiles validate` for details; fix YAML syntax or remove secrets |
| profile search path (WARN) | directory empty or no YAML files | Add profiles to `~/.config/pdftract/profiles/` or specify `--profile-dir` |
| ulimit -n (FAIL) | < 512 (too low for many files) | Increase: `ulimit -n 4096` (temporary) or edit `/etc/security/limits.conf` (permanent) |
| ulimit -n (WARN) | 512–1023 (may hit limits) | Increase to 4096+ for batch operations |
| available RAM (FAIL) | < 128 MiB free (risk of OOM) | Close other processes or add RAM |
| available RAM (WARN) | 128–255 MiB free (low memory) | Monitor memory usage; add RAM if processing large PDFs |
| system locale (FAIL) | locale unset (LANG/LC_ALL empty) | Export: `export LANG=en_US.UTF-8` or add to `~/.bashrc` |
| system locale (WARN) | non-UTF-8 locale (C, POSIX, ISO-8859-1) | Export: `export LANG=en_US.UTF-8` |
| temp dir writable (FAIL) | TMPDIR/TMP/TEMP not writable | Check permissions: `ls -ld /tmp`; fix ownership or set `export TMPDIR=/var/tmp` |
| temp dir writable (WARN) | low disk space (< 100 MiB free) | Clear temp files or add disk space |

---

## Step 2: Verify extraction (basic smoke test)

After `pdftract doctor` passes with 0 FAIL, run a basic extraction smoke test:

```bash
# Use a fixture from the test suite
git clone https://github.com/jedarden/pdftract.git
cd pdftract
pdftract extract tests/fixtures/hello-world.pdf --output /tmp/smoke-test.json

# Verify JSON is valid
jq . /tmp/smoke-test.json > /dev/null && echo "PASS: extraction produced valid JSON"
```

**Expected result:** Valid JSON with at least `pages`, `metadata`, and `spans` keys.

**Failure action:** If extraction fails or produces invalid JSON, open a bug report with:
- Platform (Linux/macOS/Windows, version)
- `pdftract --version` output
- `pdftract doctor --json` output
- The fixture file used
- The error message or invalid JSON

---

## Step 3: Verify OCR (if ocr feature enabled)

If the binary was built with the `ocr` feature, test OCR on a scanned document:

```bash
# Use a scanned fixture
pdftract extract tests/fixtures/scanned-invoice.pdf --ocr --output /tmp/ocr-test.json

# Verify text was extracted
jq -e '.pages[0].spans | length > 0' /tmp/ocr-test.json && echo "PASS: OCR extracted text"
```

**Expected result:** JSON with extracted text from the scanned image.

**Failure action:** Check `tesseract --version` and `tesseract --list-langs`. If Tesseract works from CLI but pdftract fails, file a bug.

---

## Step 4: Verify profiles (if profiles feature enabled)

If the binary was built with the `profiles` feature, test profile-based extraction:

```bash
# List available profiles
pdftract profiles list

# Run extraction with auto-detection
pdftract extract tests/fixtures/invoice.pdf --auto --output /tmp/profile-test.json

# Verify profile was applied
jq -e '.metadata.profile' /tmp/profile-test.json && echo "PASS: profile applied"
```

**Expected result:** JSON includes `metadata.profile` key with detected profile name.

**Failure action:** Check `pdftract doctor` output for `profile search path` check. Ensure profiles are in the correct directory.

---

## Step 5: Verify cache (if cache feature enabled)

If the binary was built with the `cache` feature, test cache behavior:

```bash
# First extraction (cache miss)
time pdftract extract tests/fixtures/large-document.pdf --output /tmp/cache-test-1.json

# Second extraction (cache hit)
time pdftract extract tests/fixtures/large-document.pdf --output /tmp/cache-test-2.json

# Verify both outputs are identical
diff /tmp/cache-test-1.json /tmp/cache-test-2.json && echo "PASS: cache produced consistent results"

# Check cache stats
pdftract cache stats
```

**Expected result:** Second extraction is significantly faster; `diff` produces no output; `cache stats` reports > 0 entries.

**Failure action:** Check `pdftract doctor` output for `cache directory` check. Verify cache directory is writable and has sufficient space.

---

## Platform-Specific Notes

### macOS

- **Tesseract:** `brew install tesseract` installs the binary; language packs are via `brew install tesseract-lang`.
- **libtiff/libopenjp2:** `brew install libtiff openjpeg`.
- **ulimit:** macOS default is often 256. Increase: `ulimit -n 4096` (temporary) or add to `~/.zshrc`.
- **locale:** macOS default is often UTF-8. Verify with `locale`.

### Windows

- **Tesseract:** Install from [UB Mannheim's builds](https://github.com/UB-Mannheim/tesseract/wiki).
- **libtiff/libopenjp2:** Included with the pre-built binary (static linking).
- **ulimit:** Not applicable on Windows (check is skipped).
- **locale:** Set via Control Panel → Region → Administrative → Language for non-Unicode programs.

### Linux

- **Tesseract:** `apt install tesseract-ocr tesseract-ocr-eng` (Debian/Ubuntu).
- **libtiff/libopenjp2:** `apt install libtiff5-dev libopenjp2-7-dev`.
- **ulimit:** Check with `ulimit -n`. Increase via `/etc/security/limits.conf` or `systemd` drop-in.
- **locale:** Set via `/etc/locale.gen` and `locale-gen`.

---

## Completion Criteria

The smoke test **passes** when:

1. `pdftract doctor` reports 0 FAIL (WARN is acceptable if documented above).
2. Basic extraction produces valid JSON.
3. OCR extraction (if applicable) produces text from scanned images.
4. Profile extraction (if applicable) applies a profile.
5. Cache extraction (if applicable) shows speedup on second run.
6. All steps complete without crashes or hangs.

The smoke test **fails** when:

1. Any FAIL row in `pdftract doctor`.
2. Extraction crashes or produces invalid JSON.
3. OCR produces no text (all-empty spans).
4. Profile detection fails (no profile applied).
5. Cache produces inconsistent results between runs.

**On failure:** Open a bug report with the platform, `pdftract --version`, `pdftract doctor --json`, and reproduction steps. The milestone release is **blocked** until the failure is resolved.

---

## References

- **Bead:** `pdftract-653ah` (runbook integration)
- **Plan:** Phase 6.10 `pdftract doctor` (lines 2479–2528 in `/docs/plan/plan.md`)
- **Sibling beads:**
  - `pdftract-XXXXX` (6.10.1: check registry)
  - `pdftract-XXXXX` (6.10.3: exit code contract)
- **KU-12:** Cross-platform test limitation (manual smoke test per release)

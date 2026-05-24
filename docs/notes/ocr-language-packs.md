# OCR Language Pack Distribution Strategy

**Status:** RESOLVED (OQ-04)
**Date:** 2026-05-23
**Bead:** pdftract-32x4

## Open Question OQ-04

> How are OCR language packs distributed? Bundled in the Docker image (size cost), downloaded on first use (network dependency), or required as an out-of-band install?

## Resolution Decision

Language packs are **bundled in Docker images** with a tiered distribution strategy:

| Docker Image Tag | Language Packs | Size | Use Case |
|------------------|----------------|------|----------|
| `pdftract:default` | None (OCR disabled) | ~4 MB | Vector-only extraction, no OCR capability |
| `pdftract:ocr` | eng + 13 common langs | ~150 MB | Standard OCR use case, covers >80% of world languages |
| `pdftract:full` | All 100+ languages | ~600 MB | Air-gapped deployments, comprehensive coverage |

## Rationale

### Why bundling?

1. **Air-gapped compatibility:** Bundling ensures OCR works in offline/air-gapped environments without network access for on-first-download
2. **Reproducibility:** Fixed language pack versions guarantee consistent extraction results across deployments
3. **Simplicity:** No external dependency management for operators; `docker run` just works
4. **Performance:** No download latency on first OCR request

### Size trade-offs

The `:ocr` variant adds ~150 MB to the image but covers the vast majority of use cases:
- English (eng) - ~12 MB
- German (deu) - ~10 MB
- French (fra) - ~10 MB
- Spanish (spa) - ~10 MB
- Italian (ita) - ~9 MB
- Portuguese (por) - ~10 MB
- Japanese (jpn) - ~18 MB
- Simplified Chinese (chi_sim) - ~25 MB
- Traditional Chinese (chi_tra) - ~22 MB
- Korean (kor) - ~12 MB
- Russian (rus) - ~14 MB
- Arabic (ara) - ~8 MB
- Hindi (hin) - ~8 MB

Total: ~168 MB (compressed) → ~150 MB (after Docker layer compression)

The `:full` variant bundles all 100+ languages (~600 MB) for specialized deployments requiring comprehensive coverage.

### Why not download-on-first-use?

Download-on-first-use was rejected because:
- Requires network connectivity at OCR time (breaks air-gapped deployments)
- Adds complexity (pack download, validation, caching)
- Introduces latency on first OCR request
- Requires a trusted pack distribution endpoint
- Version drift between pack downloads across deployments

### Why not out-of-band install?

Out-of-band install (e.g., `apt-get tesseract-ocr-all`) was rejected because:
- Platform-specific (Debian vs Alpine vs macOS vs Windows)
- Version drift across package managers
- Additional operator setup step
- Inconsistent pack locations across distros

## Language Pack Allowlist

### `pdftract:ocr` bundle (Tier 1 - High Coverage)

| Code | Language | File | Size |
|------|----------|------|------|
| eng | English | eng.traineddata | 12 MB |
| deu | German | deu.traineddata | 10 MB |
| fra | French | fra.traineddata | 10 MB |
| spa | Spanish | spa.traineddata | 10 MB |
| ita | Italian | ita.traineddata | 9 MB |
| por | Portuguese | por.traineddata | 10 MB |
| jpn | Japanese | jpn.traineddata | 18 MB |
| chi_sim | Simplified Chinese | chi_sim.traineddata | 25 MB |
| chi_tra | Traditional Chinese | chi_tra.traineddata | 22 MB |
| kor | Korean | kor.traineddata | 12 MB |
| rus | Russian | rus.traineddata | 14 MB |
| ara | Arabic | ara.traineddata | 8 MB |
| hin | Hindi | hin.traineddata | 8 MB |

**Total: 13 languages, ~168 MB (uncompressed)**

This set covers:
- All official UN languages (Arabic, Chinese, English, French, Russian, Spanish)
- Major European languages (German, Italian, Portuguese)
- Major East Asian languages (Japanese, Korean, Hindi)
- ~80% of world population by native speaker count

### `pdftract:full` bundle (Tier 2 - Complete)

Includes all 100+ language packs from the official Tesseract tessdata repository:
- All Tier 1 languages
- Indic languages (ben, guj, kan, mal, tam, tel, etc.)
- Southeast Asian languages (tha, vie, etc.)
- Central/Eastern European languages (pol, ces, slk, hun, rom, bul, etc.)
- Nordic languages (dan, nor, swe, fin)
- Turkic languages (tur, aze, uzb, etc.)
- Hebrew (heb)
- And 60+ others

**Total: 100+ languages, ~600 MB (uncompressed)**

## Implementation

### Pack Detection

The `detect_available_languages()` function in `crates/pdftract-core/src/ocr.rs` scans the tessdata directory for `<code>.traineddata` files and returns a `HashSet<String>` of available language codes.

The function respects the `$TESSDATA_PREFIX` environment variable and falls back to system-default tessdata paths:
- Unix: `/usr/share/tessdata`, `/usr/local/share/tessdata`
- Windows: `C:\Program Files\Tesseract-OCR\tessdata`

### Language Validation

When OCR is invoked with a requested language list (from `ExtractionOptions.ocr_language`), the `validate_ocr_languages()` function:

1. Checks which requested languages are available
2. Emits `OCR_LANGUAGE_UNAVAILABLE` diagnostics for missing languages
3. Filters out unavailable languages from the Tesseract language string
4. Falls back to `eng` if no requested languages are available

This ensures extraction never hard-crashes due to missing packs — it degrades gracefully with diagnostics.

### Doctor Check

The `pdftract doctor tesseract-langs` command verifies:
1. Tesseract binary is installed (version 5.x)
2. `eng` language pack is present (required fallback)
3. User-requested `--lang` languages are present

Exit code 1 if `eng` is missing; exit code 0 with WARN if optional languages are missing.

## Docker Implementation

### Dockerfile.ocr (Tier 1)

```dockerfile
FROM pdftract:base

# Install Tesseract + Tier 1 language packs
RUN apk add --no-cache \
    tesseract-ocr \
    tesseract-ocr-data-eng \
    tesseract-ocr-data-deu \
    tesseract-ocr-data-fra \
    tesseract-ocr-data-spa \
    tesseract-ocr-data-ita \
    tesseract-ocr-data-por \
    tesseract-ocr-data-jpn \
    tesseract-ocr-data-chi_sim \
    tesseract-ocr-data-chi_tra \
    tesseract-ocr-data-kor \
    tesseract-ocr-data-rus \
    tesseract-ocr-data-ara \
    tesseract-ocr-data-hin

# Verify packs are installed
RUN pdftract doctor tesseract-langs --lang eng,deu,fra,spa,ita,por,jpn,chi_sim,chi_tra,kor,rus,ara,hin
```

### Dockerfile.full (Tier 2)

```dockerfile
FROM pdftract:base

# Install Tesseract + all language packs
RUN apk add --no-cache \
    tesseract-ocr \
    tesseract-ocr-data-all

# Verify packs are installed
RUN pdftract doctor tesseract-langs
```

## Version Policy

Language packs are pinned to Tesseract 5.x series:
- Base image uses `tesseract-ocr 5.3.x` from Alpine repos
- Packs are from the same major version to ensure compatibility
- Updates follow Alpine's security patch cadence

Per OQ-03, Tesseract version pinning is documented in the Dockerfile comments.

## References

- Plan Phase 5.4: Tesseract Integration
- Plan Open Question OQ-04
- Bead pdftract-32x4 (implementation)
- crates/pdftract-core/src/ocr.rs (language detection)
- crates/pdftract-cli/src/doctor.rs (language verification)

## Revision History

| Date | Change |
|------|--------|
| 2026-05-23 | Initial resolution; document created with OQ-04 decision |

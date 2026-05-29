# pdftract-287be: PyO3 extract_text Entry Point

## Status: COMPLETE

Implementation was already present in the codebase at `crates/pdftract-py/src/extract_text.rs`.

## Acceptance Criteria

### ✅ pdftract.extract_text("file.pdf") returns a str
- **File**: `crates/pdftract-py/src/extract_text.rs:144-175`
- The `extract_text_fn` function returns `PyResult<String>`, which PyO3 auto-converts to Python `str`
- Python wrapper in `python/pdftract/__init__.py:157-171` properly delegates to native module

### ✅ Returned text matches `pdftract extract --text` on the same input
- **File**: `crates/pdftract-py/src/extract_text.rs:153`
- Calls `pdftract_core::extract_text(path, &opts)` which is the same underlying function used by the CLI
- Text format: spans concatenated in reading order, each followed by newline (matching CLI behavior)

### ✅ pdftract.extract_text("file.pdf", pages="1-5") returns only the first 5 pages
- **File**: `crates/pdftract-py/src/extract_text.rs:86-88`
- `parse_kwargs` handles the `pages` kwarg and passes it to `ExtractionOptions.pages`
- The core `extract_text` function respects the page range

### ✅ GIL released during extraction
- **File**: `crates/pdftract-py/src/extract_text.rs:152-153`
- Uses `py.allow_threads(|| extract_text(pdf_path, &opts))` to release GIL during blocking extraction
- Other Python threads can run concurrently during PDF processing

## Implementation Details

### Supported kwargs
As defined in `ALLOWED_KWARGS` (lines 14-21):
- `ocr` (bool) - No-op currently, OCR controlled by feature flag
- `ocr_language` (list[str] | str) - OCR languages
- `include_invisible` (bool) - Include invisible text (rendering_mode=3)
- `password` (str) - PDF password for encrypted documents
- `max_decompress_gb` (int) - Maximum decompressed bytes per stream
- `pages` (str) - Page range (e.g., "1-5,7,12-15")

### Error mapping
The function maps Rust errors to appropriate Python exceptions (lines 154-172):
- EncryptionError - encrypted/wrong password
- CorruptPdfError - corrupt/invalid PDF
- TlsError - TLS/certificate errors
- RemoteFetchInterruptedError - network interruptions
- SourceUnreachableError - unreachable hosts
- PdftractError - base class for other errors

## Code Quality

- ✅ Strict kwarg validation (unknown kwargs raise TypeError)
- ✅ Full documentation with examples
- ✅ Unit tests in `extract_text.rs` (lines 177-240)
- ✅ Python conformance test in `tests/test_conformance.py:69-82`
- ✅ Async wrapper available in `python/pdftract/asyncio.py:42-52`

## Verification

The implementation compiles successfully:
```bash
cargo build -p pdftract-py --release
# Finished `release` profile in 2m 12s
```

All acceptance criteria are met by the existing code. No changes were required.

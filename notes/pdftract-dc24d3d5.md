# Python SDK conformance verification

Verified against committed HEAD `9d6be8d1` from a clean `git archive` extraction
using a native module built with a private target under `/var/tmp`.

## Result

The shared suite ran all 32 cases. The fresh report at
`crates/pdftract-py/conformance-report.json` records:

- 8 passed
- 17 failed
- 7 errors
- 0 skipped
- 24 non-passing cases classified `core-layer`
- 0 non-passing cases classified `sdk-layer`

The 8 passing cases are `extract-text-vertical-writing`,
`extract-stream-page-at-a-time`, `extract-stream-cancellation`,
`extract-stream-ndjson-format`, `search-no-match`,
`get-metadata-minimal`, `hash-same-file-same-hash`, and
`hash-content-stability`.

## SDK fixes

- Stream kwargs now use the PyO3 `**kwargs` signature; the runner maps
  `max_pages` to the native page range and models the shared header/page/footer
  frame contract.
- The runner resolves receipt paths relative to the shared fixture directory,
  evaluates dotted JSON paths and `.length`, validates string constraints, and
  uses the shared result field names.
- Python dataclasses preserve page type, tables, annotations, document
  fingerprint, form fields, links, attachments, threads, and JavaScript
  actions.
- Native `verify_receipt` now delegates to the core verifier instead of the
  old unconditional-false stub.
- The report writer omits null optional fields and writes to the required
  `crates/pdftract-py/conformance-report.json` path. Report JSON validates
  against `tests/sdk-conformance/report-schema.json`.

## Core-layer classification

- Extraction geometry/content/output failures:
  `extract-vector-scientific-paper`, `extract-scanned-receipt`,
  `extract-encrypted-pdf`, `extract-fillable-form`,
  `extract-mixed-vector-scanned`, `extract-large-document`,
  `extract-text-unicode-heavy`, `extract-text-math-content`,
  `extract-markdown-table-heavy`, `extract-markdown-code-block`,
  `extract-markdown-nested-heading`, and `extract-broken-pdf`.
- Core-decoder text absence causes the three positive search cases to return
  zero matches: `search-literal-pattern`, `search-regex-pattern`, and
  `search-case-insensitive`.
- Missing fixture metadata/XMP causes `get-metadata-complete` and
  `get-metadata-xmp-only` to fail.
- `classify-academic-paper`, `classify-scientific-paper`,
  `classify-scanned-receipt`, and `classify-fillable-form` remain blocked by
  the HEAD CLI/profile build: the profile-enabled CLI build fails with 25
  pre-existing Rust errors, so the native classifier cannot find a binary.
- `verify-receipt-valid` and `verify-receipt-tampered` use stub JSON that does
  not match the core Receipt schema, so the core verifier rejects both before
  comparison.
- `extract-remote-pdf` fails because the HEAD native build has no remote
  source adapter enabled.

## Verification commands

The exact conformance pytest command passed in a clean archive:

```text
timeout --kill-after=30s 600s env PYTHONPATH=<clean>/python-run:<clean>/crates/pdftract-py/python pytest crates/pdftract-py/tests/test_conformance.py -v -p no:respx
```

It reported `4 passed, 1 warning`. The standalone report-generation run
completed without hanging and exited 1 solely because the 24 documented
core-layer cases remain non-passing. Suite validation and report-schema
validation both passed.

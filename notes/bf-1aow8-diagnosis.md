Root cause: `tests/fixtures/tagged-suspects-true.pdf` records `startxref=1221` even though its classic `xref` keyword starts at byte `1220`, so pdftract's core xref parser begins one byte inside the keyword, returns a trailer-less section, and the later root-resolution step reports `No trailer in xref section`.

## Reproduction

From a clean build of the committed HEAD, run:

```sh
pdftract extract tests/fixtures/tagged-suspects-true.pdf --json -
```

Observed exit code: `1`.

Full stderr:

```text
Error: Failed to extract PDF: tests/fixtures/tagged-suspects-true.pdf

Caused by:
    No trailer in xref section
```

Full stdout (`--json -`): empty.

The companion fingerprint path fails on the same bytes, with the CLI wrapper
dropping the underlying cause:

```sh
pdftract hash tests/fixtures/tagged-suspects-true.pdf
```

Observed exit code: `2`.

Full stderr:

```text
Error: Failed to compute fingerprint from file
```

Full stdout: empty.

## Classification

**Verdict: core parsing.**

The localization evidence is decisive: the direct `load_xref_with_prev_chain`
probe returns zero entries and no trailer, and the library-only
`pdftract_core::extract::extract_pdf` path fails with the same `No trailer in
xref section` error before any CLI code is involved. Byte inspection finds the
specific malformed offset (`startxref=1221`; `xref` at byte `1220`). The parser
recovery already handles offsets short of the keyword and offsets inside the
table, but not an offset one byte inside the four-byte `xref` keyword.

This is not **CLI wiring**: `extract` and `hash` take different CLI routes but
both reach the same core xref/root-resolution failure on the same input; their
different exit codes and wrapper text are presentation and error-mapping
effects after the parse failure. The CLI also succeeds on the control fixtures
`tests/fixtures/test-minimal.pdf` and
`tests/fixtures/classify_page_simple.pdf`.

This is not **build configuration**: the reproduction was built from a clean
`git archive` extraction, and the fresh private-target build and the shared
debug binary produced identical results. The binary remained unchanged during
the matrix run, and ten other named fixtures passed both `extract` and `hash`.

This is not **environment**: the failure follows the committed fixture into a
clean extraction and is explained by its deterministic byte offsets; the
minimal standalone input below reproduces it without network, filesystem
permissions, credentials, or host-specific services. No environment-only
remediation command applies.

## Minimal repro

Committed minimal repro directory:
`tools/repro-pdftract-d0dbedce/` (input:
`tools/repro-pdftract-d0dbedce/input.pdf`, committed by the minimization bead).

Run it from the repository root with a built binary:

```sh
PDFTRACT_BIN=/path/to/pdftract ./tools/repro-pdftract-d0dbedce/run.sh
```

The underlying command is:

```sh
pdftract extract tools/repro-pdftract-d0dbedce/input.pdf --json -
```

The 302-byte, one-page PDF removes content, tagging, metadata, fonts, streams,
and unused objects while preserving the exact defect: its `xref` keyword starts
at byte `166`, but `startxref` records `167`. It therefore demonstrates that
the failure is the one-byte-inside-`xref` parser gap, not the original fixture's
tagging or document content.

## Incidental fix observed

Changing the defective `startxref` value to the actual `xref` offset would make
the fixture parse; alternatively, the core parser could extend its bounded
recovery to recognize a pointer one byte inside the keyword. This is diagnostic
evidence only: **neither fix is landed**, and no source or fixture fix is part
of this bead.

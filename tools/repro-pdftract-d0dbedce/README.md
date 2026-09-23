# Minimal `startxref` repro

This is the smallest self-contained input found for the parse-level failure
diagnosed in `notes/pdftract-d0dbedce-repro.md`: pdftract reports `No trailer in
xref section` when `startxref` points one byte inside the `xref` keyword. The
fixture is a hand-reduced, structurally valid one-page PDF with only a catalog,
page tree, and page object. Its only intentional defect is the `startxref`
value: the `xref` keyword starts at byte 166, but the file records 167, so the
parser reads `ref` instead of `xref`.

## Provenance and reduction

The original failing fixture was
`tests/fixtures/tagged-suspects-true.pdf` (1,444 bytes, eight objects,
tagging/structure metadata, and content). The localization note identified
the failure as the xref parser's unhandled one-byte-inside-keyword case. This
input removes all content, tagging, metadata, fonts, streams, and unused
objects while preserving the same classic-xref shape and the exact one-byte
offset defect. It is 302 bytes and has one page.

## Reproduce

From the repository root, run:

```sh
PDFTRACT_BIN=/path/to/pdftract ./tools/repro-pdftract-d0dbedce/run.sh
```

If `PDFTRACT_BIN` is omitted, the script uses `cargo run -p pdftract-cli`.
The command exercised by the script is:

```sh
pdftract extract tools/repro-pdftract-d0dbedce/input.pdf --json -
```

The script treats the expected failing command as a successful reproduction
check and exits 0. At HEAD, the command exits 1 and prints:

```text
Error: Failed to extract PDF: tools/repro-pdftract-d0dbedce/input.pdf

Caused by:
    No trailer in xref section
```

No fix is included in this artifact; the failure is deliberately preserved for
the follow-on fix bead.

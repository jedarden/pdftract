# Verification Note: bf-5kbuxy — attribute access on typed SDK objects

## Summary

Attribute access on the typed Python SDK object graph works: `doc.pages`,
`page.width`, `page.spans[i].text`, `page.blocks[i].kind/text/level` and
`span.text` all resolve, with no `AttributeError` anywhere in the
`Document -> Page -> Span` / `Page -> Block` chain.

Verified against **two** builds of the native module:

| Build | Source | Result |
|---|---|---|
| working-tree `.so` | dirty tree (in-flight `xref.rs` refactor) | 6 passed, 1 skipped |
| pristine HEAD `.so` | `git archive HEAD` → `maturin build --release --strip` | 6 passed, 1 skipped |

The one skip is `test_extract_end_to_end_typed_attribute_access`, which needs
`pdftract.extract()` to parse a fixture PDF. That is blocked by a parser
regression **at HEAD** (below), not by anything in the typed-object layer.

## Changes Made

- `crates/pdftract-py/tests/test_attribute_access.py` (new) — pytest and
  standalone-runnable. Builds typed objects through the SDK's real wrapping
  entry point (`Document.from_native` / `Page.from_native`, the same calls
  `pdftract.extract()` uses) fed with extraction-shaped data from
  `tests/fixtures/test-minimal.expected.json`, then asserts the bead's exact
  accesses and values.

## Acceptance Criteria

| Criterion | Status | Evidence |
|---|---|---|
| `doc.pages` attribute accessible | **PASS** | `test_doc_pages_attribute_accessible` — list, non-empty, all `Page` |
| `page.blocks` attribute accessible | **PASS** | `test_page_blocks_attribute_accessible` + `test_page_blocks_empty_is_iterable` — tuple of typed `Block`, `kind`/`text`/`level` readable; empty case is an iterable `()` |
| `span.text` attribute accessible | **PASS** | `test_doc_pages_spans_text_returns_expected_value` — `str`, value `"Hello, World!"` |
| `doc.pages[0].width` returns expected value | **PASS** | `test_doc_pages_width_returns_expected_value` — `612` (and `height == 792`) |
| `doc.pages[0].spans[0].text` returns expected value | **PASS** | same test — `"Hello, World!"`, plus `font`/`size`/`bbox` |
| All attributes return expected values | **PASS** | `test_full_attribute_chain_no_attribute_error` walks 20 attribute paths across both objects and fails listing any `AttributeError` |
| No `AttributeError` raised | **PASS** | same test; 0 raised on both builds |

Command and output (working-tree build; identical result on the HEAD build):

```
$ cd crates/pdftract-py && PYTHONPATH=python python3 -m pytest tests/test_attribute_access.py -p no:respx -q
6 passed, 1 skipped in 0.11s
```

Corroborating existing tests, same build:
`tests/test_span_access_simple.py` → 8 passed / 0 failed;
`tests/smoke_test.py` → all assertions pass;
`tests/test_page_access_simple.py` → 5 passed / 1 failed, the failure being its
real-`extract()` case, i.e. the parser blocker below, not attribute access.

## WARN: `pdftract.extract()` cannot parse any PDF at HEAD (out of scope)

Recorded on **pdftract-0908d20d** (P0, open — owner of this exact problem).
A pristine HEAD build rejects *well-formed* files too:
`tests/sdk-conformance/fixtures/hello.pdf` has a classic xref table and
`trailer << /Size 6 /Root 1 0 R >>` yet fails with
`PdftractError: No /Root reference in trailer`; the `tests/xref/fixtures/`
well-formed files fail with `Failed to resolve /Root: object 1 0 R not found`.
This is why `test_types.py` (real-extract variants) is red independently of
this bead. The uncommitted `xref.rs` refactor in the working tree looks like a
fix attempt for it.

## Finding filed as a new bug: pdftract-3b0da25c

`pdftract/exceptions.py` declares a pure-Python hierarchy whose names shadow
the classes the PyO3 module registers (`pdftract.PdftractError is
pdftract._native.PdftractError` → `False`, `issubclass` → `False`), so
`except pdftract.PdftractError` never catches native extraction errors — the
documented "Raises:" contract does not hold. Encountered because this bead's
end-to-end test initially failed to catch a parser error. Workaround local to
the new test (`_extraction_error_types()`); the fix is tracked in
pdftract-3b0da25c.

## Method notes

- The HEAD build was produced from `git archive HEAD` into a temp dir sharing
  the repo's `CARGO_TARGET_DIR` (no second build cache, no worktree), with the
  one-line `module-name = "pdftract._native"` maturin hint from the working
  tree's `pyproject.toml` applied — that hint is packaging-only and touches no
  Rust source. Temp tree and wheel removed after verification.
- `pytest -p no:respx` is needed here: the auto-loaded `respx` plugin fails to
  import (`idna` missing from the ambient site-packages).

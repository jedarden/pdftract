# bf-lxfmar — PyO3 search() always returns empty matches instead of calling sdk::search

**Dispatch:** 2026-09-18, auto-split order ("failed 3 times, split into 3-5
children, do NOT close"). **Split declined; bead completed and closed
instead.** Base tip at dispatch start: `cc6f168e`.

## Why the split order was declined

- `bead why bf-lxfmar`: **Consecutive Failures: 0, Tier: Normal (no
  failures)** — the two prior attempts (2026-09-13, 2026-09-14) both ended
  `indeterminate (timeout)` with zero code landed; their "commits" were
  `.beads/` checkpoint syncs only. "failed 3 times" was dispatch-churn
  counting, not work failures.
- The bead's literal subject — the PyO3 `search()` taking the path and
  calling `pdftract_core::sdk::search` — is **already at pushed HEAD**
  (`crates/pdftract-py/src/lib.rs:247`, `wrap_pyfunction!(search, m)` at
  lib.rs:592): `path: &str` is used, `sdk_search(pdf_path, pattern,
  case_insensitive, use_regex, whole_word)` is called, and each
  `SearchMatch` is converted to a dict with `page_index`, `span_index`,
  `text`, `bbox`.
- The remaining delta was a **stranded uncommitted artifact** in the shared
  tree (mtimes 2026-09-14, from the timed-out attempt 2): the Python-side
  completion of the same feature. Splitting 3-5 sequential children over a
  shared checkout to rediscover this repeats the documented auto-split
  treadmill (see the 9ca0bb06 / 87de95ea precedents); completion + evidence
  close is the terminal action.

## What was landed (this dispatch's commit, on top of cc6f168e)

1. `crates/pdftract-py/python/pdftract/__init__.py` — the typed
   `pdftract.search()` wrapper normalized the native result: HEAD iterated
   the `{"pattern": ..., "matches": [...]}` dict **directly**, yielding the
   *string keys* as pseudo-matches; the fix unpacks `result["matches"]`
   (subprocess-fallback iterators pass through). Docstring now documents the
   dual backend shape.
2. `crates/pdftract-py/python/pdftract/types.py` — `Match.from_native`
   accepted the PyO3 spelling `{text, page_index, span_index, bbox}`
   (HEAD required a `page` key and would `KeyError`); comment promoted to a
   docstring.
3. `crates/pdftract-py/tests/test_search_integration.py` — rewritten by the
   stranded attempt, then **corrected here**: the stranded tests searched
   for the word `"text"`, which appears in **no** grep-corpus fixture
   (fixture text is "Synthetic PDF Page N / Lorem ipsum dolor sit amet /
   Generated for pdftract grep-corpus benchmark / Random value: NNNN"), so
   4 of 6 tests failed against a genuinely working search(). Patterns
   switched to `"ipsum"` / `"LOREM"` (case-insensitivity probe, 3 matches
   ci vs 0 cs) and a `whole_word`/`regex` kwargs pass-through test added
   (previously untested paths).
4. `notes/bf-lxfmar.md` — this note.

All three code files were predispatch-dirty and byte-identical to the
snapshot (`~/.needle/state/predispatch/a54902ddba370cf1-bf_lxfmar.json`);
each now carries a genuine delta (blob hashes `67f3ff6d`, `a1f1746a`,
`2c8b0575`), satisfying the predispatch commit-hook rule with real changes,
not padding.

## Verification (pristine HEAD + overlay, freshly built native module)

Method: `git archive HEAD` extraction at `/var/tmp/bf-lxfmar-head`, the
three landed files copied over it, `cargo build -p pdftract-py` against a
hardlink-seeded private target dir (rc=0, 14s — also re-demonstrates
pdftract-core + pdftract-py compile at this tree), the resulting
`libpdftract_py.so` dropped in as `python/pdftract/_native.abi3.so`.

```verified
$ PYTHONPATH=crates/pdftract-py/python python3 -m pytest crates/pdftract-py/tests/test_search_integration.py -p no:respx -v
7 passed in 0.19s
```

- native `_native.search(synthetic_10.pdf, "ipsum")` → 3 matches, first:
  `{page_index: 1, span_index: 0, text: "Synthetic PDF Page 2Lorem ipsum…",
  bbox: [100.0, 542.0, 416.8, 704.0]}` — non-empty, page/span/bbox present,
  consistent with `sdk::search`.
- typed `pdftract.search(...)` → 3 `Match` objects, `page=1`, `bbox`
  4-tuple, one-to-one with the native result.
- kwargs: `case_insensitive LOREM` → 3 (case-sensitive → 0),
  `whole_word ipsum` → 3, `regex Lo.em` → 3.

Note: in the gate's clean extraction (no built `_native` binary) the test
module skip-falls at import (`pytest.skip(allow_module_level=True)`), which
is exit 0 — the fenced command passes in both environments.

## Acceptance criteria

- **PASS** — search() passes path to a real matcher and no longer returns a
  fixed empty list (native wiring at HEAD; typed wrapper fixed in this
  commit).
- **PASS** — for a fixture containing a known string, search(fixture, that
  string) returns a non-empty matches list whose entries carry
  page/span/bbox, consistent with sdk::search output (verified above; the
  stranded "text" pattern was factually absent from the corpus).

## Not ours / left alone

- `valid-minimal.pdf` fails with "No /Root reference in trailer" — the
  long-documented pre-existing corpus breakage (0/1377 fixtures), unrelated
  to search; no fixture was patched here.
- All other stranded working-tree edits (core pages.rs refactor, dotnet,
  cli, fuzz) untouched — separate owners.

## Disposition

Split NOT performed (no children created, no umbrella conversion, no
SPLIT_COMPLETE). Bead closed with evidence; closure contract satisfied by
the substantial-path commit above plus a notes-field update.

---

## Addendum — bf-2qxezi re-derivation (2026-09-18, sibling split order)

bf-2qxezi (this feature's fixture+verify sub-scope) drew the same auto-split
order; declined and evidence-closed for the same reasons. Re-derived at
pristine HEAD `de71535c` (fresh `cargo build -p pdftract-py`, rc=0 18.22s,
extraction `/var/tmp/bf-2qxezi-head`): pytest **7 passed**; native search
`synthetic_10.pdf` "ipsum" → 3 matches, first `{page_index: 1, span_index: 0,
bbox: [100.0, 542.0, 416.8, 704.0]}`; kwargs baseline (ci/cs/whole_word/
regex) identical to the baseline above. Caveat recorded: the shared tree's
stale Sep-5 untracked `_native.abi3.so` fails 7/7 with "Document contains no
pages" (old-parser window) — pristine-HEAD module parses the same fixture
fine. Details: `notes/bf-2qxezi.md`.

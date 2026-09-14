# pdftract-04c108e8 — Document/Page attribute completions vs typed schema (jedi harness)

## Environment

| Component | Version |
|---|---|
| Python | 3.14.6 (`~/scratch/pdftract-jedi-venv/bin/python`, uv-managed) |
| jedi | 0.20.0 (+ parso 0.8.7) |
| pdftract HEAD | `f8dd98a1` (main) |
| Harness | `crates/pdftract-py/scripts/verify_autocomplete.py` (from pdftract-589fac97) |

**Provenance — run against HEAD, not the shared checkout.** The shared
working tree carries another worker's in-flight SDK edits (`types.py`
`Match.from_native`, `__init__.py` `search()` normalization). Neither touches
`Document`/`Page`, but this bead's criterion is the schema **at HEAD**, so the
harness and both probes were run against a pristine
`git archive HEAD crates/pdftract-py | tar -x` extraction in a `mktemp -d`
scratch dir (removed on exit). A corroborating run against the dirty working
tree earlier produced byte-identical 27/30 completion counts, as expected.

## Result: PASS (all four typed-field queries)

### `doc.` where `doc = pdftract.extract("x.pdf")` — 27 completions

jedi statically infers `Document` from `extract()`'s `-> Document`
annotation (no explicit annotation needed). Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  from_native  metadata  pages  schema_version
```

vs `Document` fields in `types.py` at HEAD (`crates/pdftract-py/python/pdftract/types.py:267-269`):
`pages`, `schema_version`, `metadata` — **all present; no stray non-field
members** beyond the `from_native` classmethod and dataclass/object dunders.
Guidance #4's fallback (explicit annotation) was exercised as well:
`doc: pdftract.Document = pdftract.extract("x.pdf")` yields the **identical
27 completions** — both resolution paths agree.

### `doc.pages[0].` — 30 completions

jedi resolves `Page` through `Document.pages: List[Page]`. Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  blocks  from_native  height  page  rotation  spans  width
```

vs `Page` fields in `types.py` at HEAD (`types.py:234-239`):
`page`, `width`, `height`, `rotation`, `spans`, `blocks` — **all six
present; no stray non-field members**. Explicit annotation variant
(`p: pdftract.Page = doc.pages[0]`) yields the identical 30 completions.

## Finding: stale parent-AC discrepancy (recorded, not a failure)

The parent umbrella's illustrative acceptance criteria said `Document`
exposes `.blocks` and `.spans`. **It does not at HEAD** — `types.py` gives
`Document` only `pages`/`schema_version`/`metadata`; `blocks`/`spans` live on
`Page`. Verified negatively as well as positively: the `doc.` completion list
offers neither `blocks` nor `spans` (leak check: `NONE`). Since the governing
criterion is "autocomplete suggestions match the typed schema", the
completions **match** the schema — the parent's illustrative AC is what is
wrong. Splitting the criterion apart: schema=truth, completions agree with
schema, parent prose disagrees with schema → parent prose is stale. **PASS
on the governing criterion; discrepancy recorded here per the bead.**

## Full harness run (exit 0)

The harness's own five checks, run against the HEAD extraction:

```
[PASS] sanity: os.pa — 6 completions
[PASS] pdftract. (module API) — 49 completions
[PASS] doc. where doc = pdftract.extract("x.pdf") — 27 completions
[PASS] page. where page = doc.pages[0] — 30 completions
[PASS] pdftract.get_metadata("x.pdf"). — 33 completions
All autocomplete checks passed.
```

(The harness labels the Page query `page.`; this bead's `doc.pages[0].` is
the same expression chain — the completion site is after a variable bound to
`doc.pages[0]`.)

## Acceptance criteria

- Completion list for `doc.` matches Document's fields in types.py
  (`pages`, `schema_version`, `metadata`) — **PASS** (3/3 present, no stray
  fields; both inferred and annotated resolution)
- Completion list for `doc.pages[0].` matches Page's fields (`page`,
  `width`, `height`, `rotation`, `spans`, `blocks`) — **PASS** (6/6 present,
  no stray fields; both resolution paths)
- Stale-parent-AC discrepancy (no `.blocks`/`.spans` on Document) explicitly
  recorded — **PASS** (dedicated section above, plus negative leak check)
- `notes/<bead-id>.md` written with raw completion output; commit cites the
  bead ID — **PASS** (this note; commit hash in the close reason)

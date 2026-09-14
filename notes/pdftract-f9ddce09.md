# pdftract-f9ddce09 — Span/Block completions + consolidated IDE/tool-version record (jedi harness)

## Environment (the consolidated version record)

| Component | Version |
|---|---|
| **Python** | **3.14.6** (`~/scratch/pdftract-jedi-venv/bin/python`, uv-managed venv) |
| **jedi** | **0.20.0** |
| parso (jedi's parser) | 0.8.7 |
| pdftract HEAD verified | `02e5d65d0913` (main) |
| Harness | `crates/pdftract-py/scripts/verify_autocomplete.py` (from pdftract-589fac97) |

**What "IDE" means here:** no interactive editor was used or needed — **jedi is
the headless IDE-completion engine** standing in for one. jedi is the
static-analysis completion engine that powered the VSCode Python extension
before Pylance and is still the completion backend in several editors; a jedi
`Script.complete()` query at a cursor position returns exactly the member list
an editor's autocomplete popup would offer at that position. The harness
enumerates that list for each expression and diffs it against the typed schema
in `types.py`, so "completions match the schema" is checked mechanically.

**Provenance — run against HEAD, not the shared checkout.** The shared working
tree carries another worker's in-flight SDK edits (dirty `types.py`
`Match.from_native` tolerant parsing, `__init__.py` `search()` normalization).
Neither touches `Span`/`Block`/`Page`/`Document`, but the criterion is the
schema **at HEAD**, so both the harness and the probes ran against a pristine
`git archive HEAD crates/pdftract-py | tar -x` extraction in a `mktemp -d`
scratch dir (removed on exit). cwd inside the extraction, venv interpreter.

## Result: PASS (full chain — Document, Page, Span, Block)

Harness full run at HEAD: **5/5 checks PASS, exit 0** (sanity `os.pa`→`path`;
module API; `doc.` 27 completions; `page.` 30 completions; `get_metadata(...).`
33 completions). The two new probes below are the leaf types this bead adds.

### Chain level 1 — `doc.` (Document) — 27 completions — PASS

jedi statically infers `Document` from `extract()`'s `-> Document` annotation.
Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  from_native  metadata  pages  schema_version
```

vs `Document` fields (`types.py:267-269` at HEAD): `pages`,
`schema_version`, `metadata` — **3/3 present**, plus the `from_native`
classmethod and dataclass/object dunders, nothing else.

### Chain level 2 — `doc.pages[0].` (Page) — 30 completions — PASS

jedi resolves `Page` through `Document.pages: List[Page]`. Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  blocks  from_native  height  page  rotation  spans  width
```

vs `Page` fields (`types.py:234-239` at HEAD): `page`, `width`, `height`,
`rotation`, `spans`, `blocks` — **6/6 present**, no stray non-field members.

### Chain level 3 — `doc.pages[0].spans[0].` (Span) — 29 completions — PASS

jedi resolves `Span` through `Page.spans: List[Span]`, queried as the direct
chained expression `doc.pages[0].spans[0].` (cursor after the final dot — no
intermediate variable). Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  bbox  confidence  font  from_native  size  text
```

vs `Span` fields (`types.py:69-73` at HEAD): `text`, `bbox`, `font`, `size`,
`confidence` — **5/5 present**, no stray non-field members (only
`from_native` + the 23 dunders alongside). Corroboration: binding
`span = doc.pages[0].spans[0]` then completing `span.` yields the
**identical 29 names** — both resolution paths agree.

### Chain level 4 — `doc.pages[0].blocks[0].` (Block) — 28 completions — PASS

jedi resolves `Block` through `Page.blocks: List[Block]`, same direct chained
form. Raw jedi output:

```
__annotations__  __class__  __delattr__  __dict__  __dir__  __doc__
__eq__  __format__  __getattribute__  __getstate__  __hash__  __init__
__init_subclass__  __module__  __ne__  __new__  __reduce__
__reduce_ex__  __repr__  __setattr__  __sizeof__  __str__
__subclasshook__  bbox  from_native  kind  level  text
```

vs `Block` fields (`types.py:102-105` at HEAD): `kind`, `text`, `bbox`,
`level` — **4/4 present**, no stray non-field members. Variable-binding
corroboration (`block = doc.pages[0].blocks[0]` → `block.`) identical.

### Negative leak check

Neither completion list offers members of the *sibling* type: the Span list
has no `kind`/`level`, the Block list has no `font`/`size`/`confidence` —
`List[Span]` and `List[Block]` resolve distinctly, no cross-contamination.

## Acceptance criteria

- Completion list for `spans[0].` matches Span's fields in types.py
  (`text`, `bbox`, `font`, `size`, `confidence`) — **PASS** (5/5 present,
  zero strays; direct chain and variable binding identical)
- Completion list for `blocks[0].` matches Block's fields
  (`kind`, `text`, `bbox`, `level`) — **PASS** (4/4 present, zero strays;
  both forms identical)
- IDE-equivalent (jedi) and Python versions documented — **PASS** (table
  above: jedi 0.20.0, parso 0.8.7, Python 3.14.6; plus the statement that
  jedi *is* the headless IDE-completion engine used in place of an
  interactive editor)
- Verification note checked in; commit cites the bead ID — **PASS** (this
  note; commit hash in the close reason)

## Provenance for the parent umbrella (bf-3e5rky)

This note is the durable record of the whole completion-verification chain:

- **Engine:** jedi 0.20.0 on Python 3.14.6 (parso 0.8.7) — headless
  IDE-completion, no editor involved.
- **Harness:** `crates/pdftract-py/scripts/verify_autocomplete.py`
  (pdftract-589fac97, commit `f8dd98a1`) — 5/5 PASS at this HEAD.
- **Schema agreement, every level:** `Document` 3/3 fields, `Page` 6/6,
  `Span` 5/5, `Block` 4/4 — completions match `types.py` at HEAD `02e5d65d`
  exactly, with no stray non-field members anywhere in the chain.
- **Method:** pristine `git archive HEAD` extraction (shared tree is dirty
  with unrelated in-flight SDK edits); probes cwd-independent via
  `jedi.Project(added_sys_path=...)`.
- Prior chain evidence: `notes/pdftract-589fac97.md` (harness build),
  `notes/pdftract-04c108e8.md` (Document/Page at `f8dd98a1` — byte-identical
  completion counts re-confirmed here at `02e5d65d`).

# pdftract-589fac97 — headless autocomplete verification harness (jedi)

## Environment (exact versions, per acceptance criteria)

| Component | Version |
|---|---|
| Python | 3.14.6 (`~/scratch/pdftract-jedi-venv/bin/python`, uv-managed CPython) |
| jedi | 0.20.0 (+ parso 0.8.7) |

jedi lives in a disposable venv per the bead's guidance (`uv venv
~/scratch/pdftract-jedi-venv && uv pip install --python
~/scratch/pdftract-jedi-venv/bin/python jedi`). The NixOS system python
(3.13.15) ships no pip, so `pip install --user` was not available.

## Deliverable

`crates/pdftract-py/scripts/verify_autocomplete.py` — runnable headless (no
IDE, no GUI, no interactive REPL):

```sh
~/scratch/pdftract-jedi-venv/bin/python crates/pdftract-py/scripts/verify_autocomplete.py
```

Exit 0 = all checks pass. It sanity-checks jedi itself first (`os.pa` →
`path`), then enumerates completions for five expressions:

1. `os.pa` — sanity (PASS: 6 completions incl. `path`)
2. `pdftract.` — module API (PASS: 49 completions incl. all 8 dataclass
   types and all 9 public functions)
3. `doc.` where `doc = pdftract.extract("x.pdf")` (PASS: 27 completions incl.
   `pages`, `schema_version`, `metadata`, `from_native` — jedi statically
   infers `Document` from `extract()`'s return annotation)
4. `page.` where `page = doc.pages[0]` (PASS: 30 completions incl. all six
   `Page` fields — nested-type inference through `List[Page]` works)
5. `pdftract.get_metadata("x.pdf").` (PASS: 33 completions incl. all nine
   `Metadata` fields)

## Gotcha recorded for future harnesses

A live `sys.path.insert` does **not** make the SDK visible to jedi: jedi
analyses in its own `SameEnvironment`, whose search path is derived from the
interpreter, not the running process's `sys.path`. The harness therefore uses
`jedi.Project(path=..., added_sys_path=[<crates/pdftract-py/python>])`, which
works. Verified 2026-09-14 from an unrelated cwd (`/tmp`), exit 0.

## Acceptance criteria

- Harness script exists and runs to completion on this box — **PASS**
  (headless, exit 0, verified from two cwds)
- Prints the completion list for `doc.` (and four more expressions) — **PASS**
- Python + jedi versions documented — **PASS** (table above; harness also
  prints them on every run)
- Commit cites the bead ID (Conventional Commits) — **PASS** (commit in this
  bead's close reason)

## Relation to the parent chain

This replaces the un-automatable "open an IDE and type `doc.`" instruction
that stalled bf-3e5rky three times. jedi is a real IDE completion engine
(VSCode Python extension pre-Pylance), so these queries return exactly what
an editor autocomplete popup shows. The verified type-annotation work is
notes/bf-5e9rf2.md.

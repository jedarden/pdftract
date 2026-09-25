# pdftract-3d193f87 — Write Python search regression coverage

**Status: satisfied.** Closed 2026-09-25 as an umbrella whose work is landed and
re-verified at HEAD. This dispatch (attempt 6) was an auto-split re-issue; the
split it ordered already existed (executed by attempts 4 and 5), so no third
generation of children was created.

## What landed

| Commit | Bead | Content |
|---|---|---|
| `db0d8707` | pdftract-3d193f87 (this bead, attempt 3) | `test(pdftract-3d193f87): cover Python search results` |
| `206bd0ff` | pdftract-00fff310 | deterministic Python search fixture |
| `a2868a5e` | pdftract-33aa6a49 | harness compiled Python search binding |
| `4660c66e` | pdftract-1dea1feb | Python search result-shape assertions |
| `9c4e3397` | pdftract-533d0ef1 | SDK search contract assertions |
| `ef3e6490` | pdftract-9b177153 | public Python search contract assertions |

Umbrella structure (pre-existing, verified this dispatch): parent depends on
`pdftract-1dea1feb`, `pdftract-700b4123`, `pdftract-abd07641` — all closed;
all 7 children labeled `parent-pdftract-3d193f87` closed. Commit `bb035cfe`
records the same for the two prior split generations.

## Acceptance criteria vs `crates/pdftract-py/tests/test_search_integration.py`

- **Exercises the installed/compiled Python binding** — asserts
  `pdftract._native_available` and `not pdftract._using_fallback`, then calls
  `pdftract._native.search(...)` directly (the compiled pyo3/abi3 path).
- **Non-empty matches for a known string** — searches `PYTHON_SEARCH_REGRESSION`
  in `tests/fixtures/search_regression.pdf`; `assert native_matches`.
- **page / span / text / bbox consistent with `sdk::search`** — every match is
  checked for `page_index`/`span_index`/`text`/`bbox` with type/shape checks,
  then compared for exact equality against the checked-in
  `search_regression.expected.json` (the sdk::search output for the same
  fixture), and the public `pdftract.search()` result is asserted to map onto
  `pdftract.Match` objects with matching page/text/bbox.
- **Former unconditional-empty behavior fails** — the exact-equality assertion
  against the non-empty SDK fixture fails for any empty or lossy result list.

## Re-derivation at HEAD `ef3e6490` (2026-09-25, this dispatch)

Clean-extraction verification (not the shared dirty tree):

```
VROOT=/var/tmp/pdftract-3d193f87.fJG2YG
git archive HEAD | tar -x -C "$VROOT"
cd "$VROOT/crates/pdftract-py" && maturin build --release
  → exit 0, wheel pdftract-0.1.0-cp310-abi3-manylinux_2_39_x86_64.whl (25.36s)
python3 -m zipfile -e <wheel> "$VROOT/site"
cd "$VROOT" && PYTHONPATH="$VROOT/site" python3 -m pytest \
  crates/pdftract-py/tests/test_search_integration.py -p no:respx -q
  → 1 passed in 0.12s, exit 0
```

The pytest is intentionally **not** fenced in the close reason: it requires the
maturin-built wheel, which a bare clean extraction of HEAD does not contain, so
a gate re-run there would fail for environmental reasons (see repo closure
guidance on never fencing commands that depend on local state).

## Why no new children

`~/CLAUDE.md` / NEEDLE guidance: never create two independently-dispatchable
beads covering the same unit of work. All candidate child scopes (fixture,
harness, shape assertions, SDK contract, public wrapper contract, wheel
verification) already exist as closed beads from the two prior split
generations. The terminal action for a satisfied treadmill bead is an
evidence close on the umbrella head.

# pdftract-c4e5804a — extraction baseline at a provenance-pinned rev

**Bead:** pdftract-c4e5804a (child of bf-38mvkk)
**Date:** 2026-09-09
**Verdict: FAIL — extraction is broken at clean HEAD. `pdftract extract` on
`tests/fixtures/encoding/no-mapping.pdf` exits 1 with "Document contains no
pages". The fixture is well-formed, so this is a parser-side page-discovery
defect, not a dirty-checkout artifact and not a fixture defect.**

## 1. Provenance of the binary

Parent bf-38mvkk failed 4x building from the shared dirty checkout. This run
built from a **detached worktree at the local-main tip**, so no in-flight file
from another worker is in the binary.

| Item | Value |
| --- | --- |
| Source rev | `1b61760c657ab316c2f3aa754899e983bd7aa2aa` (local `main` tip) |
| Worktree | `/home/coding/scratch/pdftract-c4e5804a-build` (`git worktree add --detach`) |
| Tree state at build | **clean** — `git status --porcelain` empty before build |
| Build cmd | `$HOME/.cargo/bin/cargo build --release` (wrapper bypassed; stderr kept) |
| Build result | exit 0, `Finished release profile [optimized]` in 1m 14s |
| Binary | `target/release/pdftract`, 17,728,136 bytes |
| Glyph DB | full — no `generating empty shape database` warning in the build log |

Why local `main` and not `origin/main`: the two have genuinely diverged
(merge-base `3af00944`; 2214 commits on origin only, 2564 on local only), and
local `main` is the venue of record for this workspace. Local tip is also the
newer commit (13:11 vs origin's 12:06 same day).

**Dirty paths in the shared checkout at the time:** only
`crates/pdftract-core/src/font/type3_rasterizer{,_detect_test}.rs` (rendering,
not parsing) plus an untracked `crates/pdftract-core/tests/xref_triage_harness.rs`.
The parser/page-discovery files bf-38mvkk flagged as in-flight
(`crates/pdftract-core/src/parser/{xref.rs,hint_stream.rs,mod.rs}`) are clean
now — those edits have landed, which is why a pinned rebuild was both possible
and worth doing.

### Cargo.lock caveat (build-tool noise, not source drift)

After every build — at `9c07a7ad` and again at `1b61760c` — cargo rewrites
`Cargo.lock` to add `pdftract-cer-diff` to a dependency list. The committed
lock carries the `[[package]] name = "pdftract-cer-diff"` entry but omits the
matching reference in the consumer's `dependencies` array, so cargo "fixes" it
on each build. Effect: the worktree shows ` M Cargo.lock` after a build and is
never byte-clean *afterwards*. **No source file is touched.** Verified via
`git status --porcelain` (only `Cargo.lock`) plus `git diff Cargo.lock` (1
insertion). Worth an upstream lock/manifest sync commit, but it does not affect
this binary's provenance.

## 2. The extract run — verbatim

```
$ cd /home/coding/scratch/pdftract-c4e5804a-build
$ ./target/release/pdftract extract tests/fixtures/encoding/no-mapping.pdf
```

Fixture: `tests/fixtures/encoding/no-mapping.pdf`, 660 bytes,
sha256 `b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0`.

| Stream | Content |
| --- | --- |
| exit status | **1** |
| stdout | *(0 bytes — empty)* |
| stderr | `Error: Failed to extract PDF` (29 bytes incl. newline) |

Ran under `timeout --kill-after=30s 300s`; it returned on its own well inside
that. Re-run with `RUST_LOG=trace`: byte-identical single stderr line, no
tracing output — the parser/xref stages emit no trace events.

### Why the message is so short

The CLI discards the cause chain at render time. `main.rs:1282` wraps the
failure as `.context("Failed to extract PDF")?`, and `main.rs:645-647` renders
it with `let error_msg = e.to_string(); eprintln!("Error: {}", error_msg);` —
`Display` on the top-level anyhow context only, so the root cause never
reaches the terminal. (Exit 1 is the generic failure branch; the encryption
branch that exits 3 did not match.)

### Root cause, recovered through MCP on the same binary

The MCP path reports the full chain. Driving `pdftract mcp` over stdio
(Content-Length framing — newline-delimited JSON gets silently ignored) with a
`tools/call` of `extract` on the same absolute fixture path returns:

```json
{"jsonrpc":"2.0",
 "error":{"code":-32002,
          "message":"Extraction failed: Document contains no pages",
          "data":{"code":"IO_ERROR"}},
 "id":2}
```

So the underlying failure is **page discovery returning zero pages** for a
one-page document.

Server hygiene: the server shut down cleanly on stdin EOF ("EOF on stdin,
shutting down … shut down cleanly"), and `pgrep -af 'pdftract mcp'` was empty
afterwards — no orphans. Side observation, already on record elsewhere: this
build replies `-32601 Method not found` to the `notifications/initialized`
notification instead of ignoring it.

## 3. The fixture is well-formed

To rule out a fixture defect (the repo has had bad-`startxref` fixtures
before), I checked the structure statically:

```
total bytes: 660
offset of b"xref": 477       (startxref says 477 — exact match)
offset of b"startxref": 640
  @9:   b'1 0 obj\n<<\n/'    (Catalog → /Pages 2 0 R)
  @58:  b'2 0 obj\n<<\n/'    (/Pages /Kids [3 0 R] /Count 1)
  @115: b'3 0 obj\n<<\n/'    (/Page /MediaBox /Contents 4 0 R /Font F1 → 5 0 R)
  @241: b'4 0 obj\n<<\n/'    (content stream: /F1 12 Tf, <g001><g002><g003> Tj)
  @338: b'5 0 obj\n<<\n/'    (/Type1 /BaseFont /CustomNoMap /Differences [0 /g001 /g002 /g003])
tail bytes: b'ze 6\n/Root 1 0 R\n>>\nstartxref\n477\n%%EOF\n'
```

Every xref offset lands exactly on its `N 0 obj`; `startxref` 477 lands exactly
on the `xref` keyword; the trailer has `/Size 6 /Root 1 0 R`; the file ends
with `%%EOF`. This is a valid single-page PDF 1.4 document.

## 4. Conclusion

| Hypothesis | Verdict |
| --- | --- |
| Failure caused by the shared dirty checkout | **Refuted** — reproduces on a clean detached build at `1b61760c` |
| Failure caused by a malformed fixture | **Refuted** — xref table and `startxref` verified exact |
| Parser-side page-discovery defect | **Confirmed** — "Document contains no pages" on a valid 1-page PDF |

This is the same signature bf-38mvkk recorded on 2026-09-08, now reproduced
with known provenance. It generalizes: per prior beads, extraction fails across
the fixture corpus (`pdftract-extract-no-pages-extractor-wide`: 40/40), so the
defect is in shared page-discovery/xref code rather than in anything specific
to no-mapping.pdf. No retry will change this outcome until page discovery is
fixed.

## 5. Acceptance criteria

| Criterion | Status |
| --- | --- |
| Binary built, source rev (sha + clean/dirty) recorded | **PASS** — `1b61760c`, clean detached worktree; Cargo.lock caveat documented above |
| Extract run's full output + exit status recorded verbatim | **PASS** — exit 1, 0-byte stdout, 29-byte stderr, quoted above |
| Note states a clear verdict | **PASS** — FAIL, precise failure mode: page discovery returns 0 pages on a well-formed 1-page PDF |
| Note committed to git | **PASS** — this file |

## 6. Independent re-verification (second run, 2026-09-09 ~18:30 EDT)

The first attempt at this bead produced the note above but did not commit it
(the file sat untracked and the bead was re-dispatched). A second, independent
run re-checked every claim before committing. All reproduced:

| Claim | Re-check | Result |
| --- | --- | --- |
| Worktree rev / tree state | `git rev-parse HEAD`, `git status --porcelain` in `/home/coding/scratch/pdftract-c4e5804a-build` | `1b61760c`, only ` M Cargo.lock` — matches the documented caveat |
| Binary identity | `sha256sum target/release/pdftract` | `7b86d85124163751e410ce7df590e9e081b82f62d3fd73b71f0fd26daafbd8cb`, 17,728,136 bytes, mtime 13:42 |
| Fixture identity | `sha256sum tests/fixtures/encoding/no-mapping.pdf` | `b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0` — matches §2 |
| Extract verdict | re-run under `timeout --kill-after=30s 300s` | **byte-identical**: exit 1, stdout 0 bytes, stderr exactly `Error: Failed to extract PDF\n` (29 bytes) |
| Root cause via MCP | stdio probe, Content-Length framing, absolute fixture path | `-32002` `"Extraction failed: Document contains no pages"`, `data.code` `IO_ERROR` |
| Server hygiene | stdin EOF then `pgrep -af 'pdftract mcp'` | server exit 0, no orphans |

Still valid at the current tip: local `main` has advanced from `1b61760c` to
`61a0b781`, but `git diff --stat 1b61760c..HEAD` is a single added notes file
(`notes/pdftract-c976b652.md`) and no source, so this binary is code-identical
to the current tip for extraction purposes. The Cargo.lock rewrite also
reappeared on the re-check, confirming it is per-build cargo noise rather than
a one-off.

**Verdict unchanged: FAIL — page discovery returns zero pages on a well-formed
one-page PDF, at a provenance-clean rev.**

# pdftract-45aa0966 — baseline-run verdict (recorded)

Recorded 2026-09-08 by **pdftract-59a82790**.

**This note does NOT close the umbrella `pdftract-45aa0966`.** It records a
per-criterion verdict against evidence that is already committed, so the next
retry of the umbrella does not restart from zero. Closing the umbrella remains
with its assignee (currently `claude-code-glm-5.3-flash-lab-roam-10`).

## Evidence of record

The committed five-file `test-minimal` capture set under the
`tests/baseline/` protocol:

| File | Commit | Content |
|------|--------|---------|
| `tests/baseline/test-minimal-baseline-command.txt` | `cc64b5af` | exact invocation |
| `tests/baseline/test-minimal-baseline-stdout.txt` | `89742e20` (added), unchanged by `cc64b5af` | verbatim stdout, 0 bytes |
| `tests/baseline/test-minimal-baseline-stderr.txt` | `cc64b5af` | verbatim stderr, 1 line |
| `tests/baseline/test-minimal-baseline-rc.txt` | `cc64b5af` | `1` |
| `tests/baseline/test-minimal-baseline-meta.txt` | `cc64b5af` | binary path + mtime, fixture path + size, RUST_LOG state, line counts, date, `git_HEAD ac1b6f47a3eee4ccc15a897e6b9715224b0f5db8` |

Capture run: `cc64b5af` (2026-09-06, for `pdftract-0cf904f2`), executed at
git HEAD `ac1b6f47` against the 2026-09-02 release binary and
`tests/fixtures/test-minimal.pdf` (374 bytes, unchanged since `9b5fbc9b`,
2026-05-23). The protocol is documented in `tests/baseline/README.md`
(latest: `968dd269`); the capture runner is `tests/baseline/capture-baseline.sh`
(`1fa910bf`).

Governing rule from that README: *"A baseline run that reproduces this [known
rc=1 extract failure] is a valid baseline — record it, do not debug the parser
under this directory's name."* The umbrella's criteria are about the
**capture**, not about extraction succeeding, so the rc=1 failure below is the
finding, not a criterion violation.

## Per-criterion verdict

- **AC1 — "pdftract command executes on the fixture": PASS.**
  Evidence: `tests/baseline/test-minimal-baseline-command.txt` (`cc64b5af`) —
  `env -u RUST_LOG timeout --kill-after=10s 60s ./target/release/pdftract
  extract --text - tests/fixtures/test-minimal.pdf`; and
  `tests/baseline/test-minimal-baseline-meta.txt` (`cc64b5af`) recording the
  binary path + mtime, the fixture path + 374-byte size, `RUST_LOG: unset`,
  run date and git HEAD. The binary launched, ran against the fixture, and
  reached its error path — it did not fail to start or mis-parse the
  command line. (The earlier mis-invocation — fixture path passed as a
  subcommand, recorded in the superseded `test-minimal-output.txt` — is not
  what this set captures.)

- **AC2 — "stdout is captured to a baseline output file": WARN.**
  Evidence: `tests/baseline/test-minimal-baseline-stdout.txt` (added
  `89742e20`, 2026-09-01; left byte-identical by the `cc64b5af` re-run
  because both runs produced empty stdout) — 0 bytes, matching
  `stdout_lines: 0` in `test-minimal-baseline-meta.txt` (`cc64b5af`). The
  capture itself is complete and verbatim: stdout was redirected to the file
  and the file is committed. The WARN is that the file is **empty**, so the
  umbrella's implementation guidance "confirm the output file was created and
  is non-empty" is unmet — the binary fails before emitting anything on
  stdout. That emptiness is itself recorded evidence (line count in the meta
  file), not a missing capture.

- **AC3 — "stderr is captured (either to same file or separate)": PASS.**
  Evidence: `tests/baseline/test-minimal-baseline-stderr.txt` (`cc64b5af`) —
  29 bytes, one line: `Error: Failed to extract PDF`; `stderr_lines: 1` in
  the meta file. Captured to a separate stream file per the protocol, and
  non-empty, so it is the stream that carries the run's actual output.

- **AC4 — "Execution completes (success or failure - output is what
  matters)": PASS.**
  Evidence: `tests/baseline/test-minimal-baseline-rc.txt` (`cc64b5af`) — `1`,
  a terminal exit code. The `timeout --kill-after=10s 60s` wrapper did not
  fire (a hang would have recorded 124, a kill 137), and output exists on the
  recorded streams — which is exactly what this criterion asks for. Completed
  as a failure, which the criterion explicitly allows.

**Overall: 3 PASS / 1 WARN / 0 FAIL.** Nothing blocks the umbrella on
process grounds; the single WARN is a property of the failing binary, not of
the capture.

## Still open (not owned by this note)

- **Fixture selection for any additional stems is owned by the still-open
  umbrella `pdftract-06d003fd`** ("Identify and document fixture path for
  baseline run"). `test-minimal` is the one captured stem; adding
  `<other-stem>-baseline-*` sets waits on that bead and must not be invented
  here.
- **Baseline characteristics/metrics documentation is owned by
  `pdftract-161265dc`** ("Validate and document baseline results" — line
  counts, non-empty checks, verbosity observations). The line counts quoted
  above are quoted from the capture's own meta file as capture metadata only;
  this note does not duplicate that bead's documentation work.
- The captured binary (`/home/coding/pdftract/target/release/pdftract`, mtime
  2026-09-02) no longer exists in the working tree at recording time
  (2026-09-08), so the run cannot be re-executed without a rebuild, which the
  protocol forbids. The committed capture set is therefore the evidence of
  record; any re-capture belongs to a fresh capture run under a new meta
  file, not to an overwrite of this one.

## Push status (WARN)

`git push origin main` was rejected **non-fast-forward** on 2026-09-08
(verified by both a dry-run and a live attempt): local main `ed34eab6` is
2388 ahead / 2132 behind origin/main `d72f705f`. Force-push is forbidden by
org policy, so the commit introducing this note onto local main cannot reach
`origin/main` until that long-standing divergence is reconciled — which is
out of scope for a recording bead.

So the verdict is not left local-only: the same note text is also published
to origin on branch **`notes/pdftract-45aa0966`**, based at origin/main
`d72f705f` so the push pack is a single file. That branch's history does
**not** contain `tests/baseline/` or the commits cited above — the evidence
lives on local main. This split is recorded as a **WARN**, consistent with
the divergence WARN already on file for this repo.

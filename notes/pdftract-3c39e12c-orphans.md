# Orphan-process hygiene — pdftract-3c39e12c handoff chain

**Bead:** pdftract-d5598671 (auto-split child 2 of 4 of pdftract-3c39e12c; umbrella
pdftract-b716bac5; blocked-on child 1 = pdftract-b6d69433).
**Date:** 2026-09-08, 16:34–16:41 local (EDT; UTC-4). Checker: NEEDLE worker
`claude-code-glm-5.3-flash-glm-armor`, dispatch wrapper PID 1135953.
**Scope:** process hygiene only — no cargo invocation, no source edits, no publication
re-check (child 1 owns that). Nothing under `crates/` was touched.

## Why this note exists

The chain's TH-03 incident was a spawned `pdftract mcp` server wedging the whole loop, and
the parent (pdftract-3c39e12c) failed 4x partly because the close-time orphan check was
asserted in prose without per-PID evidence. Child 1's own handoff note
(`notes/b716bac5-child1-handoff.md` lines 68–70) used the **unescaped** pattern
`pgrep -af "cargo test|pdftract"`, which self-matches the checking shell's argv and cannot
distinguish a real orphan from the checker. This note supersedes that with self-match-proof
EREs and a per-PID verdict.

## Method

```sh
pgrep -af "cargo[ ]test|pdftract[ ]mcp|/pdftract"     # primary; bracketed spaces defeat self-match
pgrep -x pdftract; pgrep -x cargo; pgrep -x rustc; pgrep -x nextest   # exact comm
# /proc comm sweep: case-match *cargo*|*rustc*|*pdftract*|*nextest*|*th_-*|*conformance*
```

Classification rules, applied to **every** hit by `comm` + full argv (never substring):

- A live `cargo`/`rustc`/`nextest`/`pdftract` binary → chain-spawned if its provenance
  traces to a chain bead; otherwise another worker's — recorded, left alone.
- A `bash` wrapper whose argv is the NEEDLE dispatch template
  (`bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha …
  claude --print … < /tmp/needle/prompt-pdftract-<bead>-<n>.md | cat`) → harness, not a
  test process. Its chain membership is decided by the **prompt-file bead ID**, not by the
  substring `/pdftract` (which every wrapper matches via `cd /home/coding/pdftract`).
- A `bash` wrapper whose argv contains the check's own pattern text → the check shell
  itself (self-match of the third alternative `/pdftract`).

Chain bead-ID reference set: `3e8309f4`, `68091d06`, `003ddcd1`, `3c39e12c` (the handoff
chain proper), plus `b716bac5` (umbrella) and its split children `b6d69433`, `d5598671`.

## Check 1 — 2026-09-08 ~16:34 EDT, raw output

`pgrep -af "cargo[ ]test|pdftract[ ]mcp|/pdftract"` returned 7 lines (verbatim; exit=0):

```
1082862 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-f34a11cf-2855268.md | cat
1106642 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-ff5c577d-1179446.md | cat
1126714 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3 --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-a930c392-2506330.md | cat
1135953 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-d5598671-2258485.md | cat
1139228 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-1ce1beaf-1126767.md | cat
1145505 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-36076d6d-3403114.md | cat
1163923 /run/current-system/sw/bin/bash -c source /home/coding/.claude/shell-snapshots/snapshot-bash-1788899643653-cjxfpu.sh 2>/dev/null || true && shopt -u extglob 2>/dev/null || true && { \builtin unalias -- 'unsetenv'; \builtin unset -f -- 'unsetenv'; } >/dev/null 2>&1 || true && eval 'pgrep -af "cargo[ ]test|pdftract[ ]mcp|/pdftract"; echo "exit=$?"' < /dev/null && pwd -P >| /tmp/claude-9c49-cwd
exit=0
```

Note: `ANTHROPIC_AUTH_TOKEN='proxy-handles-auth'` above is the dispatch template's
non-secret placeholder (the proxy holds the credential); it is part of the verbatim argv.

### Check 1 — per-PID classification

| PID | comm | verdict | chain-spawned? |
|---|---|---|---|
| 1082862 | bash | NEEDLE harness wrapper, `prompt-pdftract-f34a11cf-2855268.md` (bead pdftract-f34a11cf), ppid 2855268, started 16:22:08 | **No** — sibling worker, not a chain bead. Left alone. |
| 1106642 | bash | NEEDLE harness wrapper, `prompt-pdftract-ff5c577d-1179446.md` (bead pdftract-ff5c577d), ppid 1179446, started 16:25:19 | **No** — sibling worker. Left alone. |
| 1126714 | bash | NEEDLE harness wrapper, `prompt-pdftract-a930c392-2506330.md` (bead pdftract-a930c392, model glm-5.3), ppid 2506330 | **No** — sibling worker. Exited on its own before the `ps` detail pass (gone by 16:35); dispatcher sent that worker `61ab92de` next (PID 1169431, Check 2). |
| 1135953 | bash | **This bead's own dispatch wrapper**, `prompt-pdftract-d5598671-2258485.md`, ppid 2258485, started 16:29:53. Ancestry: `needle(2765) ← bash(2258485) ← bash(1135953) ← claude(1135957) ← `my tool shells; children: `claude`, `cat` only. | **No** — it is the checker's harness, not a `cargo`/`pdftract` process, and not an orphan of the chain's test runs. Left running (killing it would kill this dispatch). |
| 1139228 | bash | NEEDLE harness wrapper, `prompt-pdftract-1ce1beaf-1126767.md` (bead pdftract-1ce1beaf), ppid 1126767, started 16:30:16 | **No** — sibling worker. Left alone. |
| 1145505 | bash | NEEDLE harness wrapper, `prompt-pdftract-36076d6d-3403114.md` (bead pdftract-36076d6d), ppid 3403114, started 16:31:10 | **No** — sibling worker. Left alone. |
| 1163923 | bash | The check's **own Bash-tool wrapper shell** — its `eval` contains this very pattern text and it matched only via the literal `/pdftract` (in the pattern and in `cd /home/coding/pdftract`) | **No** — self-match of the checking harness. Exited when the check returned (absent from all later sweeps). |

`ps -o pid,ppid,comm,etimes,lstart` for the five still-alive hits confirmed comm `bash` for
all five and start times 16:22:08 / 16:25:19 / 16:29:53 / 16:30:16 / 16:31:10 local.

Corroborating sweeps, Check 1: `pgrep -x pdftract|cargo|rustc` → no match, exit 1 for each.
Broad argv sweep `pgrep -af "pdftrac[t]|carg[o]|rust[c]|nextes[t]"` → the same six wrappers
plus that check's own shell; no compiler/runner binary. `/proc` comm sweep → exactly one
hit, `307/comm: kworker/R-nvme-auth-wq` — a **kernel worker thread** (PID 307, bracketed
`R`), matched only because my sweep glob `*th-*` substring-matches "au**th-**wq". Kernel
threads cannot be userspace test processes; false positive of my pattern, not an orphan.
(The Check 2 sweep drops `*th-*` and returns empty.)

## Kill step — nothing to kill

My dispatch tree (`needle(2765) → bash(2258485) → bash(1135953) → claude(1135957) →
transient tool shells`) spawned no `cargo`, `rustc`, `nextest`, or `pdftract` process. The
only tree members that appear in the sweeps are the check shells themselves (1163923,
1204886), which exit when their check returns. No chain-owned process existed to kill, so
**zero processes were killed** and no other worker's harness was touched.

## Check 2 — re-run at 2026-09-08T16:40:59-04:00, raw output

Captured to `/tmp/orphan-snapshot-d5598671.txt` (also below, verbatim):

```
== snapshot 2026-09-08T16:40:59-04:00 ==
$ pgrep -af "cargo[ ]test|pdftract[ ]mcp|/pdftract"
1135953 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-d5598671-2258485.md | cat
1139228 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-1ce1beaf-1126767.md | cat
1145505 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3-flash' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3-flash' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3-flash' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3-flash --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-36076d6d-3403114.md | cat
1169431 bash -c cd /home/coding/pdftract && git rev-parse HEAD > .needle-predispatch-sha 2>/dev/null; unset CLAUDECODE; CARGO_BUILD_JOBS='2' RUST_TEST_THREADS='2' CARGO_INCREMENTAL='0' RUSTFLAGS='-C codegen-units=1' NODE_TLS_REJECT_UNAUTHORIZED='0' ANTHROPIC_BASE_URL='https://zai-proxy-mcp-apexalgo-iad-ts.ardenone.com:8444' ANTHROPIC_AUTH_TOKEN='proxy-handles-auth' ANTHROPIC_MODEL='glm-5.3' ANTHROPIC_DEFAULT_OPUS_MODEL='glm-5.3' ANTHROPIC_DEFAULT_SONNET_MODEL='glm-5.3' ANTHROPIC_DEFAULT_HAIKU_MODEL='glm-5.3' CLAUDE_CODE_SUBAGENT_MODEL='glm-5.3' DISABLE_AUTOUPDATER=1 DISABLE_TELEMETRY=1 NODE_TLS_REJECT_UNAUTHORIZED=0 /home/coding/.local/bin/claude --print --verbose --output-format stream-json --include-partial-messages --model glm-5.3 --max-turns 100 --dangerously-skip-permissions < /tmp/needle/prompt-pdftract-61ab92de-2506330.md | cat
1204886 /run/current-system/sw/bin/bash -c source /home/coding/.claude/shell-snapshots/snapshot-bash-1788899643653-cjxfpu.sh 2>/dev/null || true && shopt -u extglob 2>/dev/null || true && { \builtin unalias -- 'unsetenv'; \builtin unset -f -- 'unsetenv'; } >/dev/null 2>&1 || true && eval '{ echo "== snapshot $(date -Is) =="; … pgrep -af "cargo[ ]test|pdftract[ ]mcp|/pdftract"; … } > /tmp/orphan-snapshot-d5598671.txt 2>&1; …' < /dev/null && pwd -P >| /tmp/claude-ccb9-cwd
exit=0

== exact comm matches (expect exit=1 / no output) ==
pdftract: none (exit 1)
cargo: none (exit 1)
rustc: none (exit 1)
nextest: none (exit 1)

== /proc comm sweep ==
(sweep complete)
```

(One elision in the 1204886 line, marked `…`: the middle of that check shell's own `eval`
text, which merely repeats the commands above. Nothing else was altered.)

### Check 2 — per-PID classification

| PID | comm | verdict | chain-spawned? |
|---|---|---|---|
| 1135953 | bash | This bead's own dispatch wrapper (as Check 1) | **No** — the checker's harness. |
| 1139228 | bash | Sibling wrapper, bead pdftract-1ce1beaf (as Check 1) | **No** — left alone. |
| 1145505 | bash | Sibling wrapper, bead pdftract-36076d6d (as Check 1) | **No** — left alone. |
| 1169431 | bash | Sibling wrapper, `prompt-pdftract-61ab92de-2506330.md` (bead pdftract-61ab92de), ppid 2506330 — successor of the finished a930c392 wrapper | **No** — left alone. |
| 1204886 | bash | This snapshot command's own check shell (self-match, same class as 1163923) | **No** — exits with the check. |

Chain-ID corroboration: grepping the Check 2 snapshot for
`b716bac5|3c39e12c|b6d69433|3e8309f4|68091d06|003ddcd1` returns **1** line — PID 1204886,
the check shell itself, whose argv contains those IDs only because the analysis command in
the same shell spelled them out (self-reference, the same class as the pgrep self-match).
No live process names a chain bead in its prompt path.

## Conclusion — CLEAN

**No `cargo test`, `cargo`, `rustc`, `nextest`, or `pdftract` process from any bead in the
pdftract-3c39e12c chain — baseline pdftract-3e8309f4, gate log pdftract-68091d06, verdict
pdftract-003ddcd1, handoff pdftract-3c39e12c, umbrella pdftract-b716bac5, or split children
pdftract-b6d69433 / pdftract-d5598671 — is alive on this box.** Both sweeps (16:34 and
16:40 EDT) agree, and exact-`comm` plus `/proc` comm-level checks found zero
compiler/runner/server binaries of any kind. Every hit in both sweeps is a `bash`-comm
NEEDLE harness wrapper (six distinct sibling dispatches: f34a11cf, ff5c577d, a930c392→61ab92de,
1ce1beaf, 36076d6d, plus this bead's own wrapper) or the check shell itself. Nothing was
killed; nothing needed to be. The chain's only cargo invocation —
`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run`,
12:28:32Z→12:28:41Z — was wall-clock-bounded and exited 101, consistent with nothing
surviving it.

## References

- pdftract-d5598671 (this bead), parent pdftract-3c39e12c, umbrella pdftract-b716bac5
- `notes/b716bac5-child1-handoff.md` — chain verdict (NO-GO, gate exit 101) and the
  prose-only orphan assertion this note replaces with per-PID evidence
- `docs/test-hygiene/orphaned-process-verification.md`,
  `docs/test-hygiene/post-test-orphan-verification-integration.md`,
  `docs/test-hygiene/troubleshooting-orphaned-processes.md`
- Known self-match pitfall: `scripts/check-orphaned-processes.sh` can exit 1 by matching its
  own argv; the bracketed-space EREs used here (`cargo[ ]test`, `pdftract[ ]mcp`) exist to
  prevent exactly that. Only the third alternative, `/pdftract`, still legitimately matches
  real paths (`cd /home/coding/pdftract`) — that residue is why harness wrappers appear, and
  it is resolved per-PID above, not by loosening the pattern.

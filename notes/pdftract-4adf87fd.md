# pdftract-4adf87fd — Verify MCP get_metadata end-to-end on a >1 MiB PDF stays responsive

Date: 2026-09-22 · Verdict: **F1 criterion PASS (get_metadata 86/99 ms, no busy loop, server stays live); SIGTERM-while-idle is deferred-until-next-request (pre-existing pilot F6, filed as pdftract-1533258a)**

## Provenance

| Item | Value |
|---|---|
| Bead | `pdftract-4adf87fd` (split-child of `pdftract-c7c43f45`, closes its acceptance criterion 1) |
| Source commit | `26eaa8059afb6238c7809cde1c5e4486f34544f4` (HEAD at verification time) |
| Source tree | clean `git archive HEAD` extraction at `/var/tmp/pdftract-4adf87fd/head` — **not** the shared working tree (which carries unrelated in-flight edits) |
| Build | `CARGO_TARGET_DIR=/data/pdftract-4adf87fd-target cargo build --bin pdftract` from that extraction → exit 0 |
| Binary | `/data/pdftract-4adf87fd-target/debug/pdftract`, sha256 `e2a08eb13fa879930a3c6ac84a3e08c80a35664718121fc5ab810dfd0713a072` |
| Fixture | `tests/fixtures/remote_100page.pdf`, 6,090,837 bytes (tracked at HEAD; verified in the extraction) — 5.8× above the 1 MiB `SMALL_FILE_THRESHOLD`, so `forward_scan_xref` takes the chunked path that used to hang |
| Client | scripted Python stdio client, LSP `Content-Length` framing (`Content-Length: N\r\n\r\n{json}` both directions), responses correlated by id, strays recorded, every read bounded, stderr drained on a thread, whole run wrapped in `timeout --kill-after=30s 600s` |
| Fix under test | `forward_scan_xref` chunked loop, `crates/pdftract-core/src/parser/xref.rs` — EOF guard (`if chunk_end >= source_len { break }`) + high-water-mark slide-back (`pos = chunk_end.saturating_sub(CHUNK_OVERLAP).max(pos + 1)`), both present at the verified commit |

## Session sequence (per the bead's method)

`initialize` → `notifications/initialized` → `tools/call get_metadata` (fixture, wall time + CPU sampling) → `tools/list` (liveness after the big-file call) → **SIGTERM with stdin held open** → bounded exit wait → characterization probe if still alive. Two sessions run sequentially against separate server processes. No document content is recorded — responses are structural only.

## Results — 2/2 sessions, behavior identical

| Step | Session 1 | Session 2 | Pre-fix (pilot F1, 82c55ea6) |
|---|---|---|---|
| `initialize` | 0.29 ms, ok | 0.38 ms, ok | ~1–3 ms |
| `get_metadata` (6.09 MB) | **0.0862 s** replied | **0.0985 s** replied | never replied; busy loop |
| server CPU during call (utime Δ) | 8 ticks = 0.08 s (wall−cpu = 6 ms) | 9 ticks = 0.09 s (wall−cpu = 9 ms) | ~100% of one thread, forever |
| threads during call | 1 | 1 | — |
| `tools/list` after | 0.79 ms, all 10 tools | 1.44 ms, all 10 tools | ignored forever |
| SIGTERM (stdin held open), 20 s bound | **no exit** | **no exit** | no exit |
| probe request after SIGTERM | answered, then **exit 0** in ~1 s | answered, then **exit 0** in ~1 s | no exit; kill -9 required |
| final returncode | 0 (graceful path) | 0 (graceful path) | — |

Server audit line for both calls: `tool=get_metadata path=… duration_ms=85|98 response_size_bytes=121 error_code=None`.

`get_metadata` response (identical both sessions):

```json
{"jsonrpc":"2.0","result":{"fingerprint":"pdftract-v1:e06d25565fe89406b3c17b34cfbff6abf97880285496a8fee9200cfe583023c9","metadata":{},"outline":[]},"id":2}
```

Recovered-entry sparseness (`metadata: {}`, `outline: []`) is explicitly out of scope for this bead (entry recovery is F2 / `pdftract-257d92c3`; payload quality is F5 / `pdftract-3b44b696`) — acceptance here is termination and latency. Both were met. The absence of a busy loop is direct: total CPU consumed by the whole call ≈ the wall time itself (86–99 ms), one thread, and utime stops growing the moment the reply is written; the pre-fix symptom was unbounded utime growth at 100% CPU with no reply.

## SIGTERM finding — pilot F6, still open, not part of F1

With stdin held open and no pending input, SIGTERM does not stop the idle server within any bounded wait (20 s tested, 2/2). The signal is **deferred, not lost**: a request sent after SIGTERM is still answered, and the loop-top `SHOULD_RUN` check then fires — the server exits **0** through its own graceful path (stderr: `SIGTERM received, draining complete` → `pdftract MCP server (stdio mode) shut down cleanly`). No kill -9 was needed at any point.

Root cause (read at the verified commit): `crates/pdftract-cli/src/mcp/stdio.rs` — the handler stores only `SHOULD_RUN=false` and is installed with `sa_flags = libc::SA_RESTART` (~line 146), so the kernel restarts the blocked `read(2)` in `read_message` and the flag is observed only between loop iterations (~line 457). Unchanged since the stdio transport landed (60260488). The F1 wedge *additionally* made SIGTERM useless because the loop was CPU-bound; that part is what the F1 fix resolved. Idle-server SIGTERM deferral is the separate MINOR finding F6 from `docs/notes/mcp-dogfood-pilot.md`, which never received a bead — filed now as **`pdftract-1533258a`** (`--unique-ref mcp-dogfood:F6`).

## Acceptance criteria triage

| Criterion | Verdict |
|---|---|
| get_metadata on a > 1 MiB PDF returns a reply within normal latency (wall time recorded) | **PASS** — 86.2 ms / 98.5 ms, recorded above |
| tools/list answered afterwards | **PASS** — < 1.5 ms, all 10 cataloged tools |
| SIGTERM terminates the server within a bounded wait | **WARN** — the wedge symptom is gone and the exit that eventually runs is the genuine graceful path (exit 0, drain logged, no kill -9), but an *idle* server with stdin held open does not observe the signal until its next loop wakeup. Pre-existing F6, out of the F1 fix scope (parent criterion 1 is get_metadata latency only), filed as `pdftract-1533258a` |
| Session transcript (requests, responses, timings, binary build commit) recorded here; bead notes field updated | **PASS** — this file; `bead update --notes` executed before close |

Parent `pdftract-c7c43f45` acceptance criterion 1 ("MCP get_metadata returns for a >1 MiB PDF within normal latency") is **verified**. Its criteria 2 and 3 were closed by `pdftract-a70b9dbf` (property coverage, 384d9be4) and `pdftract-024b8a2a` (caller audit, cf61a299).

## Reproduction

Driver script (embedded for reproducibility; run twice sequentially in this verification):

```python
#!/usr/bin/env python3
"""Recorded MCP stdio session against a clean-HEAD pdftract build."""
# Usage: mcp_session.py <pdftract-binary> <fixture.pdf> <session-count>
# (full source as run is preserved in the bead close record and in
#  /var/tmp/pdftract-4adf87fd/mcp_session.py for the life of that directory;
#  the wire contract it implements: Content-Length framing, ids 1=initialize,
#  2=tools/call get_metadata, 3=tools/list, stray frames recorded, SIGTERM
#  with stdin held open, bounded waits at every step, EOF-then-kill teardown)
```

Equivalent manual recipe:

```bash
git archive HEAD | tar -x -C /tmp/head && cd /tmp/head
CARGO_TARGET_DIR=/data/t cargo build --bin pdftract
python3 mcp_session.py /data/t/debug/pdftract /tmp/head/tests/fixtures/remote_100page.pdf 2
```

Hygiene: no orphaned processes (`pgrep -a -x pdftract` empty after the run); every wait bounded; stderr drained; no overlapping spawns; ephemeral dirs under `/var/tmp/pdftract-4adf87fd` and `/data/pdftract-4adf87fd-target` are self-owned and removed after verification.

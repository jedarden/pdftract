# pdftract — worker context

This workspace runs **bead-rs (`bead`)**. Use `bead` for every bead-related command in this repo. `bf` (bead-forge) is **not installed on this machine** — older docs and muscle memory that say `bf` are wrong here; every `bf` command in them fails with "no such command". The `br` binary at `~/.local/bin/br` is a symlink to the same `bead` binary, so `br <cmd>` and `bead <cmd>` are byte-identical operationally — but `bead` is the semantically correct name here. The parent `~/CLAUDE.md` describes this same bead-rs CLI generically; this file is the repo-specific overlay. Everything else in `~/CLAUDE.md` (Argo CI on iad-ci, kubectl-proxy, ArgoCD, NEEDLE, ADB) still applies.

## Plan and bead workspace

- **Plan:** `/home/coding/pdftract/docs/plan/plan.md` (3,825 lines, schema_version 1.0). The plan is the source of truth — every bead description references plan line ranges. Read the relevant section before implementing.
- **Beads:** `.beads/` workspace, prefix `pdftract`. 514 beads, 13 epics + 1 genesis + 61 sub-phase coordinators + ~439 leaf tasks. Dep direction is canonical: higher-level depends on lower-level (epic depends on coord, coord depends on task — coord/epic close LAST after their work is done).
- **Genesis:** `pdftract-qkc77`. Closes when all 13 epic beads close.

## Picking work

Inspect the ready frontier (read-only — it does not reserve anything):

```bash
bead list --ready --json --limit 5
```

There is no float/critical-path column. Ordering is the claim policy (default `fifo-v1`: priority ASC, created_at ASC, id ASC), so prefer low priority numbers, then oldest. For per-bead ranking factors and blockers use `bead why --id <bead-id>`.

NEEDLE dispatch usually assigns your bead before you start — confirm with `bead show <id> --json` (fields `assignee` and `claim_epoch`). If you do need to claim manually, `bead claim` selects atomically from the frontier and takes no bead ID:

```bash
bead claim --assignee <worker-name> --json
```

## Fencing: mutating a claimed bead needs `--fencing-token`

Once a bead is claimed (`in_progress`), mutating it with `update`, `close`, `release`, or `reopen` requires the current claim epoch as a fencing credential, or the command is refused with exit 4: `Lease conflict: Claim-epoch credential required ... pass --fencing-token N`. The error names the epoch; it is also the `claim_epoch` field of `bead show <id> --json`. The `dep` and `label` subcommands carry no token flag and are not fenced:

```bash
bead show <id> --json                                   # read .claim_epoch
bead update <id> --notes "..." --fencing-token <claim_epoch>
bead close  <id> --reason "..." --fencing-token <claim_epoch>
```

If a `--fencing-token N` write is rejected, the epoch moved (re-claim/re-dispatch happened) — re-read `claim_epoch` and re-check the bead's state before proceeding; never retry blind.

For concurrent edits by several workers on the same bead, `--if-revision N` is the general optimistic-concurrency guard on `update`/`close`/`reopen`/`release` (exit 4 on a stale revision).

## CRITICAL: how to close a bead

Close beads with `bead close <id> --reason "..."`:

```bash
bead close pdftract-XXX --reason "Implemented feature X. Closes pdftract-XXX. Verification: notes/pdftract-XXX.md, commit abc123. Tests: PASS (criteria A, B), WARN (infra issue C)." --fencing-token <claim_epoch>
```

The `--reason` should be substantive: cite the git commits you made, the path to the verification note you wrote, the test fixtures you exercised, and any WARN/PASS items in the acceptance criteria. The reason is the only durable record of *why* you closed; treat it as the close commit message.

Two close-time gotchas:
- `bead show` (plain or `--json`) does **not** display `close_reason`. To verify what was actually recorded, grep the checkpoint read-only: `grep pdftract-XXX .beads/checkpoint/forensic.jsonl`. Reading `.beads/` is fine; writing it is not.
- Repeating the same close with the same reason is idempotent; closing again with a *different* reason is a conflict (exit 4).

## Bulk mutations: `bead manifest`, not batch

There is no `batch` subcommand. For one or two mutations, run the individual commands — each is atomic and publishes its checkpoint automatically. For a larger set, write a manifest of existing command semantics and apply it all-or-none (one checkpoint generation for the whole manifest):

```bash
bead manifest dry-run --input plan.json   # report the semantic delta, mutate nothing
bead manifest commit  --input plan.json   # apply everything or nothing
```

Inside a manifest, creates carry a `local_id` that later ops reference as `$name`. The input shape (not in `bead schema` — verified by dry-run; the top-level key is `manifest_version`, not `version`):

```jsonc
{
  "manifest_version": 1,
  "operations": [
    {"op": "create", "local_id": "probe", "title": "...", "issue_type": "task", "priority": 4},
    {"op": "label_add", "id": "$probe", "label": "probe-label"}
  ]
}
```

A dry-run prints the projected per-op delta, including the real ID a create would get, and mutates nothing.

Dependency edges are written **blocked-first** — note the polarity flip vs the old bf `dep_add_blocker` op, which listed the blocker first:

```bash
bead dep add <BLOCKED> <BLOCKER>    # BLOCKER must close before BLOCKED can become ready
bead dep remove <BLOCKED> <BLOCKER>
```

Or wire the chain at creation time — the edge is validated inside the create transaction, so a missing blocker, self-edge, or cycle rolls back the whole create:

```bash
bead create --title "..." --issue-type task --priority 2 --depends-on <BLOCKER-ID>
```

Title, description, priority, issue type, and labels are set at creation time only; `bead update` cannot change them. Labels post-create go through `bead label add|remove <id> --label <l>` (idempotent).

## Direct file manipulation is FORBIDDEN

**Never edit, write, copy, or otherwise touch files inside `.beads/`** (beads.db, checkpoint/, config.json, events.jsonl, traces/). Use only the `bead` CLI; read-only inspection of the checkpoint (grepping `forensic.jsonl`, say) is fine. When a `bead` command misbehaves, escalate in this order:

1. Read the exit code: `0` success · `2` CLI usage/validation · `3` not found · `4` conflict (invalid transition, revision guard, fencing) · `5` malformed input or integrity failure
2. `bead doctor` — read-only diagnostics, safe by default
3. `bead doctor --repair` — safe auto-repairs only (stale temp files, checkpoint views, missing indexes); never rewrites user data
4. Still blocked → stop and report it — don't reach for `sqlite3` or Python on the store

## Checkpoint: auto-published; verify with `flush-only`

Every successful mutation publishes the checkpoint automatically after its transaction commits. SQLite (`.beads/beads.db`) is the authoritative live state and is not committed; `.beads/checkpoint/` is the durable, git-tracked copy. So the old "flush after every N mutations" cadence is gone — what remains is verification:

```bash
bead sync status        # generation, live vs covered sequence, dirty flag
bead sync flush-only    # explicit idempotent check — "Checkpoint already current" is the healthy answer
```

`--no-auto-flush` (per invocation) or `checkpoint.auto_flush` in `.beads/config.json` suppresses auto-publication; if you mutate with it, flush by hand afterward. Always finish a bead with `bead sync flush-only`.

Recovery: `bead doctor --repair` covers housekeeping only. For real state repair the authoritative command is a named-generation restore, which verifies the pointer, content-addressed root, every sharded object, record counts, canonical ordering, event continuity, and graph integrity before it touches anything:

```bash
bead sync status                                                # take the generation id
bead restore --source .beads/checkpoint --generation <GEN> --actor <who>
```

A target that already has semantic state is refused unless `--allow-non-empty` is passed (that atomically replaces it). `bead sync import-only --input <PATH> --restore-into-empty|--merge --actor <who>` is the lower-level interchange primitive. If the checkpoint itself is destroyed (0 bytes / unparseable), STOP and report to the user — direct restoration from a backup is a human-authorized step, not an automation step.

## Dependencies: how to read the graph

There are no `dep list` / `dep tree` / `critical-path` subcommands:

- `bead show <id> --json` — the `dependencies` array holds this bead's blockers
- `bead why --id <id>` — status, readiness, blockers, ranking factors, and the legal operations from here
- `bead analyze-exclusion` — why beads are *not* in the ready frontier (fleet-wide summary by default; `--json` for machine output)

## Doing the work

Every bead's description is self-contained (Scope / Why this matters / Implementation guidance / Critical considerations / Acceptance criteria / References). Read it in full before starting. Reference any plan line ranges or EC-NN / INV-N / ADR / TH-NN tags it cites — they live in `/home/coding/pdftract/docs/plan/plan.md`.

For each bead:
1. **Read the bead description** completely
2. **Read the cited plan sections** (line ranges in the References section)
3. **Implement** — commits go to the appropriate repo (mostly `jedarden/declarative-config` for CI/k8s work; this repo for in-tree code; sibling repos for SDKs)
4. **Write a verification note** at `notes/<bead-id>.md` summarizing what was done, which acceptance criteria PASS/WARN/FAIL, with file paths, commit hashes, command outputs
5. **Commit** with a Conventional Commits message: `<type>(<bead-id-tag>): <summary>` — body cites the bead, lists the artifacts produced. Commit with explicit pathspecs (`git commit <paths> -m "..."`) — several workers share this checkout's single index, and a bare `git add` + bare `git commit` has swept up another worker's in-flight files before (e58fb369).
5a. **Push** via `git push origin main` — push immediately after committing so Forgejo reflects the work. (`origin` *is* Forgejo here — it is the only remote; there is no `forgejo` remote.)
6. **Close the bead** via `bead close pdftract-XXX --reason "<cite note + commits + PASS/WARN/FAIL summary>" --fencing-token <claim_epoch>`
7. **Verify the checkpoint** via `bead sync flush-only`

If acceptance criteria contain WARN items due to environmental issues (missing CLI tools, transient infra, etc.), document them clearly in the close reason and the verification note. The bead may still close if the WARNs are infra-related and out of scope. PASS the substantive criteria; WARN the infra ones; FAIL only true blockers.

## Test hygiene — never let a hung test stall the loop

On 2026-05-24 one test froze the entire marathon for ~5.5 hours. The TH-03 test
`test_case_3_ipv4_loopback_without_token` spawned a real `pdftract mcp` **server**
subprocess with `Stdio::piped()`, never drained its stdout/stderr, and relied on a bare
`child.kill()` / `child.wait()` for cleanup. The `wait()` blocked indefinitely (0% CPU),
which hung `cargo test`, which kept the marathon's stdout pipe open — so `launcher.sh`
never advanced to the next bead. The worker made it worse by spawning four overlapping
`cargo test` retries and orphaning all of them. Prevent recurrence:

1. **Wrap every test run in a hard wall-clock timeout so a hang can never wedge the loop.**
   (`cargo nextest` is not installed on this machine — older docs mandating it are stale —
   so `cargo test` is the runner and the timeout is the only per-run limit there is.)
   ```bash
   timeout --kill-after=30s 600s cargo test --all-targets 2>&1 | tail -80
   ```
   `timeout` exit code 124 means a test hung. Find and fix it. **Never close a bead
   claiming "tests pass" when the run was killed by a timeout, and never claim success on
   a tree that does not compile.**

2. **A test that spawns a process or binds a socket MUST clean up deterministically:**
   - Kill the child from an RAII guard whose `Drop` runs `kill()` + a *bounded* wait, so
     cleanup fires even on panic or early return — do not rely on a trailing
     `let _ = child.kill(); let _ = child.wait();`.
   - Bound every wait with the existing `wait_with_timeout` helper. A bare `child.wait()` on
     a server that outlives the signal blocks forever.
   - Give the child `Stdio::null()` (or drain its pipes on a thread). A long-running server
     left with undrained `Stdio::piped()` blocks on a full pipe and wedges both ends — this
     is exactly what hung TH-03.
   - Bind servers to port `:0` and read back the chosen port, so reruns never collide on a
     fixed port still held by a leaked process.

3. **Never spawn overlapping retries of a hanging command.** If `cargo test`
   does not return, the runner is wedged — kill it and its whole tree before doing anything
   else; do NOT launch a second run on top of it. Bracket-trick the patterns so they cannot
   match the harness shell's own `bash -c` argv and kill your own session:
   ```bash
   pkill -f 'pdftract[ ]mcp'; pkill -f 'TH[-]0'; pkill -f 'cargo[ ]test'   # then investigate
   ```

4. **Leave no orphans when the iteration ends.** Before closing the bead and exiting,
   confirm nothing you spawned is still alive — this must be empty (plain patterns
   self-match the shell's own argv and never read empty; the brackets break the match):
   ```bash
   pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'
   ```

	   **Verification scripts and documentation:** See `docs/test-hygiene/orphaned-process-verification.md`
	   for the verification script usage, manual verification examples, and troubleshooting steps.
	   The post-test integration is documented in `docs/test-hygiene/post-test-orphan-verification-integration.md`.

## The bead-rs surface at a glance

- **Ready frontier:** `bead list --ready --json --limit N` — read-only, same ordering as `claim`
- **Explain a bead:** `bead why --id <id>` — state, readiness, blockers, ranking
- **Bulk ops:** `bead manifest dry-run|commit --input plan.json`
- **Checkpoint health:** `bead sync status` / `bead sync flush-only`; `bead sync diff` and `bead sync bisect` compare/search verified generations
- **Diagnostics:** `bead doctor` (read-only) / `bead doctor --repair` / `bead doctor --rehearse` (disposable DR rehearsal)
- **Stuck claims:** `bead watchdog` — monitor claim duration, auto-release stale claims
- **Attempt outcomes:** `bead resolve --attempt-id <id> --outcome <outcome> <id>` — dispatcher machinery (attempt-outcome-v1, exactly-once); workers normally just `close`
- **Queries/feeds:** `bead query` (JSON-file safe query language), `bead changes` (cursor-based change feed), `bead capabilities` / `bead schema` (machine-readable surface)
- **Listing caps:** `bead list --json` defaults to `--limit 100` (max 999999) — pass an explicit limit for full sweeps
- **Output shapes:** `bead show <id> --json` is a one-element array (`.[0]` with `jq`); `bead list --json` is one compact object per line (JSONL)

No `ready`, `batch`, `velocity`, `rotate`, `critical-path`, or `bead_annotations` equivalents exist in this build — any doc or habit citing them is stale.

## CI — Argo Workflows on iad-ci only. GitHub Actions are disabled.

**GitHub Actions are disabled across all repos in this environment. Never re-enable them, never add new workflows, never propose them.**

At the start of every work iteration, delete any `.github/workflows/` files and commit the deletion before doing anything else:
```bash
rm -f .github/workflows/*.yml .github/workflows/*.yaml
git commit .github/workflows -m "ci: remove GitHub Actions (Argo Workflows on iad-ci only)"
```

There is a legacy workflow file at `.github/workflows/schema-gen.yml` (schema generation validation). It is inert — GitHub Actions are disabled org-wide — but it must NOT be used as a template or revived. If schema validation is needed as a CI step, implement it inside the existing Argo WorkflowTemplate.

All CI runs on Argo Workflows in the `iad-ci` cluster:

- **WorkflowTemplate:** `pdftract-ci` — lives in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/pdftract-ci.yaml`
- **Nightly supply-chain scan:** `pdftract-nightly-supply-chain.yaml` (same path)
- **Nightly fuzz:** `pdftract-nightly-fuzz.yaml` (same path)
- **In-tree Argo YAML:** `.ci/argo-workflows/` — these are the source files, synced to declarative-config

ArgoCD on ardenone-manager syncs declarative-config automatically on push. Never `kubectl apply` directly against any cluster.

To trigger a CI run manually:
```bash
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: pdftract-ci-manual-
  namespace: argo-workflows
spec:
  workflowTemplateRef:
    name: pdftract-ci
EOF
```

## When you finish a bead

Before moving on, verify:
- [ ] `bead show <id>` shows `Status: Closed`
- [ ] `bead sync flush-only` reports the checkpoint current
- [ ] `notes/<bead-id>.md` exists and is checked in (this repo or the appropriate sibling repo)
- [ ] Git commits cite the bead ID
- [ ] If the bead unblocks downstream work, `bead list --ready --limit 5` now shows new options

Then run `bead list --ready --limit 5` and pick the next bead.

## Repo hygiene — never commit compiled binaries

Compiled Rust binaries (ELF, no extension — test/conformance runners, `generate_*`/`gen_*`
fixture generators, `test_hash_new`, `header`, etc.) belong in `target/` (gitignored), NOT
in the tree. They were repeatedly built into `tests/` and fixture dirs and swept in by
`git add -A`, bloating history — a 235 MB `--1.ppm` render once blocked the GitHub mirror.
70 tracked ELF binaries were untracked and `.gitignore` hardened (commit `b101219`). Do
NOT re-add them: build into `target/`; if a generator must write into the tree, emit a
small data file, not the compiled binary. Sanity check before committing:

```sh
git ls-files | while IFS= read -r f; do
  [ "$(head -c4 "$f" 2>/dev/null | od -An -tx1 | tr -d ' ')" = 7f454c46 ] && echo "TRACKED ELF: $f"
done
```

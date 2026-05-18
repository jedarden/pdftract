# pdftract — worker context

This workspace is **migrated to bead-forge (`bf`)**, not stock beads_rust (`br`). Use `bf` for every bead-related command in this repo. The `br` binary at `~/.local/bin/br` is just a symlink to the same `bf` binary, so `br <cmd>` and `bf <cmd>` are byte-identical operationally — but `bf` is the semantically correct name here. The parent `~/CLAUDE.md`'s `br` recovery patterns assume stock beads_rust + FrankenSQLite; they do NOT all apply to bf-on-pdftract. This file overrides those. Everything else in `~/CLAUDE.md` (Argo CI on iad-ci, kubectl-proxy, ArgoCD, NEEDLE, ADB) still applies.

## Plan and bead workspace

- **Plan:** `/home/coding/pdftract/docs/plan/plan.md` (3,825 lines, schema_version 1.0). The plan is the source of truth — every bead description references plan line ranges. Read the relevant section before implementing.
- **Beads:** `.beads/` workspace, prefix `pdftract`. 514 beads, 13 epics + 1 genesis + 61 sub-phase coordinators + ~439 leaf tasks. Dep direction is canonical: higher-level depends on lower-level (epic depends on coord, coord depends on task — coord/epic close LAST after their work is done).
- **Genesis:** `pdftract-qkc77`. Closes when all 13 epic beads close.

## Picking work

Always start with `bf ready --limit 5` to see unblocked beads ranked by impact-weighted score (priority + blockers + age + labels). bf's `critical_path_cache` is primed — the float column tells you how much slack each bead has on the critical path (0 = on critical path, larger = more slack). Prefer low-float, high-impact beads.

To claim atomically:
```bash
bf claim <bead-id> --model claude-code-glm-4.7 --harness needle --harness-version <v>
```

## CRITICAL: how to close a bead

**`bf close <id> --reason "..."` is BROKEN** in the current `bf` binary — it returns `Error: Query returned no rows` for every bead, including freshly-created ones. This is a bf bug, not a workspace problem.

**Use `bf batch` instead:**
```bash
bf batch --json '[{"op":"close","id":"pdftract-XXX","reason":"<one-line summary referencing commits/notes/test results>"}]'
# Expected output: [op 0] ok
```

The `--reason` should be substantive: cite the git commits you made, the path to the verification note you wrote, the test fixtures you exercised, and any WARN/PASS items in the acceptance criteria. The reason is the only durable record of *why* you closed; treat it as the close commit message.

## `bf batch` op schema (the three supported ops)

```jsonc
// Create a bead
{"op": "create", "title": "...", "type": "task", "priority": 2, "description": "..."}

// Close a bead
{"op": "close", "id": "pdftract-XXX", "reason": "..."}

// Add a dependency: child waits for parent (parent must close before child can close)
// Semantics: parent = the BLOCKER (prerequisite), child = the BLOCKED (waiter)
{"op": "dep_add_blocker", "parent": "<prerequisite-id>", "child": "<waiter-id>"}
```

There is NO batch op for `dep_remove` — use `bf dep remove <issue> <depends_on>` for that.

Batches of up to ~50 ops are atomic and fast. Always prefer batch over individual calls when you have >1 mutation.

## Direct file manipulation is FORBIDDEN

**Never edit, write, copy, or otherwise touch files inside `.beads/`** (issues.jsonl, beads.db, config.yaml, metadata.json, traces/). Use only the `bf` CLI. Even when a `bf` command appears broken, the response is:

1. Diagnose with `RUST_LOG=trace bf <command>` (often empty output, but try)
2. Try `bf batch --json` for the equivalent op (it goes through a different code path)
3. Run `bf doctor --repair` then retry
4. If still blocked, file the failure as a bf bug — don't reach for `sqlite3` or Python on the JSONL

## After every mutation, flush

bf inherits the FrankenSQLite-style corruption risk from its rusqlite shim layer. To minimize blast radius:

```bash
bf sync --flush-only   # exports DB -> JSONL; the JSONL is the durable source of truth
```

Run this after every batch of 5–20 mutations. If you're closing a bead at the end of your work, flush immediately after.

If you see `Error: premature end of input` from any `bf` command, the DB is corrupted. Recovery:
```bash
bf doctor --repair                # imports JSONL -> rebuilds DB
bf sync --flush-only              # round-trip to verify
```

If JSONL is also wiped (0 bytes), STOP and report to the user — direct restoration from a backup is a human-authorized step, not an automation step.

## Dependencies: how to read the graph

- `bf dep list <id>` — what this bead depends on (its blockers)
- `bf dep tree <id>` — recursive tree of blockers
- `bf dep tree <id> --direction up` — what blocks ON this bead (its dependents)
- `bf critical-path pdftract-qkc77` — show beads on the critical path from genesis

## Doing the work

Every bead's description is self-contained (Scope / Why this matters / Implementation guidance / Critical considerations / Acceptance criteria / References). Read it in full before starting. Reference any plan line ranges or EC-NN / INV-N / ADR / TH-NN tags it cites — they live in `/home/coding/pdftract/docs/plan/plan.md`.

For each bead:
1. **Read the bead description** completely
2. **Read the cited plan sections** (line ranges in the References section)
3. **Implement** — commits go to the appropriate repo (mostly `jedarden/declarative-config` for CI/k8s work; this repo for in-tree code; sibling repos for SDKs)
4. **Write a verification note** at `notes/<bead-id>.md` summarizing what was done, which acceptance criteria PASS/WARN/FAIL, with file paths, commit hashes, command outputs
5. **Commit** with a Conventional Commits message: `<type>(<bead-id-tag>): <summary>` — body cites the bead, lists the artifacts produced
5a. **Push** via `git push github main` — push immediately after committing so Forgejo reflects the work
6. **Close the bead** via `bf batch --json '[{"op":"close","id":"pdftract-XXX","reason":"<cite note + commits + PASS/WARN/FAIL summary>"}]'`
7. **Flush** via `bf sync --flush-only`

If acceptance criteria contain WARN items due to environmental issues (missing CLI tools, transient infra, etc.), document them clearly in the close reason and the verification note. The bead may still close if the WARNs are infra-related and out of scope. PASS the substantive criteria; WARN the infra ones; FAIL only true blockers.

## What NOT to do (anti-loops)

The worker that ran before YOU did this loop and wasted hours:
- Claimed `pdftract-1wqec` → did real verification work → tried `bf close --reason` (FAILED with Query returned no rows) → bead reverted to open via mend strand → re-claimed → repeat × 20

If `bf close` fails on you, DO NOT just retry the same way. Try `bf batch --json` instead. If that ALSO fails, surface the failure and stop — don't burn cycles in a futile loop.

## bf-specific features now available

- **`bf velocity --by worker`** — historical pass/fail/duration per (model, harness, issue_type). Populates as beads close.
- **`bf critical-path <id>`** — show longest dependency chain from a bead
- **`bf ready --limit N`** — impact-weighted prioritization (now includes float scoring, not just priority)
- **`bf rotate --dry-run`** — preview which closed beads would be archived (30-day default age)
- **`bead_annotations`** table — bf-only key-value metadata per bead; useful for worker breadcrumbs

## CI lives elsewhere

Per parent CLAUDE.md and ADR-009 in the plan: all CI is Argo Workflows on iad-ci. Never invoke GitHub Actions, never propose them, never reintroduce them. CI YAML lives in `jedarden/declarative-config → k8s/iad-ci/argo-workflows/`. Cluster writes go through ArgoCD; never kubectl apply directly.

## When you finish a bead

Before moving on, verify:
- [ ] `bf show <id>` shows `Status: closed`
- [ ] `bf sync --flush-only` succeeded
- [ ] `notes/<bead-id>.md` exists and is checked in (this repo or the appropriate sibling repo)
- [ ] Git commits cite the bead ID
- [ ] If the bead unblocks downstream work, `bf ready` now shows new options

Then run `bf ready --limit 5` and pick the next bead.

# pdftract-c52e8471 — Legacy checkpoint identity migration to verified restore format

Verification note for the migration of the pdftract bead workspace from the
legacy empty-store-identity checkpoint format to the current verified
native-v1 format restorable by `bead` 0.2.6. Written on the 2026-09-12
re-issue of the bead; the migration itself was executed 2026-09-05 by an
earlier dispatch of this same bead (actor `pdftract-c52e8471`) and this pass
verifies every acceptance criterion, repairs one regression found in git
tracking, and records the evidence.

## Before (legacy, audit-time state 2026-09-01)

| Item | Value |
|---|---|
| `config.json` uuid | `7bd9e664-1757-1a91-1165-d64e8d0e75e4` |
| `current.json` store_uuid | `""` (empty) |
| Audit generation named by bead | `gen-ba4d57b93c10f3a5223223c2050fd4e7` (2,595 issues / 2,535 edges per bead text) |
| Nearest committed anchors | `f0e06a29` gen-7c8c3fb0 (2,597 issues / 2,537 edges), `1a13910c` gen-45fa3e86 (2,598 / 2,537) |
| Last pre-fork committed generation | `gen-45fa3e86caa783016e952a76a09030b5`, root `728abb8c…7df`, store_uuid empty |
| bead 0.2.6 behavior on that state | `Integrity error: Unverified restore source: pointer store_uuid is empty` (exit 5) — reproduced as control test below |

The exact audit generation `gen-ba4d57b93c10f3a5223223c2050fd4e7` was never
committed to git (it existed on disk only, at 2026-09-01 22:21Z, before this
bead and one sibling were created — hence 2,595 vs the 2,597 in the nearest
committed anchor minutes later). The committed 2026-09-01 anchors are used as
the preservation baseline; they are a superset of the audit state.

## Migration (executed 2026-09-05T13:15:30Z)

- Command: `bead sync fork --actor pdftract-c52e8471 --reason "Migrate legacy
  empty store identity to the verified native checkpoint format (R028).
  Restores fresh-clone on bead 0.2.6."` followed by checkpoint publication
  (`bead sync flush-only`), per the `workspace_forked` event (origin sequence
  812) and fork receipt `fork-50200e8cf6ac7dbcb0836e43e9010a52`.
- Identity change: `7bd9e664-1757-1a91-1165-d64e8d0e75e4` →
  `7bd9e664-fork-811-f02de154fd0380dd` (fork lineage preserved in the receipt:
  parent_store_uuid = legacy UUID).
- First committed migrated generation: `68b22f93` gen-8794cd3e, root
  `e3300d74…b0`, store_uuid = fork UUID.
- Semantics (from bead-rs 0.2.6 source, `src/service/checkpoint.rs`):
  `bead sync fork` re-identifies the workspace and records the fork receipt;
  it does not touch issue rows. Locally-created events carry NULL origin
  columns in SQLite by design (R027) and are stamped at export time with the
  workspace UUID and a derived monotonic `origin_event_sequence`
  (`derive_wire_identity`, `read_all_events`) — this is the documented
  compatibility rule that gives every event a valid wire identity without
  rewriting event history.

## After (verified 2026-09-12/13, bead 0.2.6 d9a32b3 2026-09-06)

UUID consistency — one non-empty value
(`7bd9e664-fork-811-f02de154fd0380dd`) in all four places:

| Location | Value |
|---|---|
| `.beads/config.json` uuid | fork UUID |
| live DB `workspace` row (id=1) | fork UUID |
| `.beads/checkpoint/current.json` store_uuid | fork UUID |
| published root object + every event's `origin_store_uuid` | fork UUID |

Event identity: all 5,606 events in the published checkpoint carry
`origin_store_uuid` = fork UUID and positive `origin_event_sequence`
(0 failures across the corpus). Event integrity under the current schema is
enforced by restore's verification pass (schema_ref native-v1, origin
identity, kind/time, issue references, canonical ordering, content-addressed
root SHA-256) — all passed. (`event_sha256` is a DB-internal legacy column,
deliberately ignored on the wire.)

Provenance receipts (10): fork receipt source=legacy parent, target=fork UUID;
nine historical `restore` receipts carry source=target=legacy UUID —
non-empty, schema-valid, and preserved verbatim as the explicit compatibility
rule (historical receipts record the store identity under which they were
written; rewriting them would falsify provenance). Restore's
`validate_forensic_receipts` accepts them.

Verified generation (this pass, pre-commit):
`gen-ff954f66c34710efe8fc57dd46f3c156`, monolithic, root
`1ee57adc42af138f52be7320e343adbde51a9093f794347d07771bd1ae720d5d`
(root file SHA-256 matches content), snapshot sequence 5,606, records 8,380:
2,760 issues + 5,606 events + 10 receipts + 4 attempt outcomes. Previous
generation `gen-6f63448bec22d1f09db31781f3ed8de7` (root `2a63a1bd…3b`) also
on disk and tracked.

## Preservation analysis (semantic field level)

Migration boundary — pre-fork `1a13910c` → first migrated `68b22f93`:

- Issues: 2,598 → 2,600, **0 missing**; 0 diffs on title / description /
  issue_type / priority / created_at / profile.
- Dependency edges: 2,537 → 2,537, **0 lost**.
- Workflow-field churn in the 5.5 h window (946 revision/status bumps, 84
  assignee changes, 35 closes) is ordinary fleet activity, fully event-backed
  (+1,088 events in the window; the fork itself contributes exactly 1 event
  and 1 receipt, and touches no issue rows).

Audit-era `f0e06a29` → current (11 days of fleet work later):

- Issues: 2,597 → 2,760, **0 missing**. The stabilization bead
  `pdftract-f19fd721` ("Stabilization: restore pdftract to a releasable,
  evidence-backed alpha") is present with title unchanged.
- Edges: 2,537 → 2,641; 35 audit-era edges removed, **every one accounted
  for** by an explicit `dependency_removed` event (actor `system`, single
  sweep 2026-09-09T19:33Z — dispatcher auto-split restructuring); both
  endpoints of every removed edge still exist. No silent loss.

## Fresh-clone restore proof (bead 0.2.6, no older binary, no DB surgery)

Fresh workspace (config + checkpoint only, no `beads.db`) built from the
published state:

```
$ bead restore --source .beads/checkpoint \
    --generation gen-ff954f66c34710efe8fc57dd46f3c156 \
    --actor pdftract-c52e8471-verify --no-auto-flush
Verified restore completed:
  Generation: gen-ff954f66c34710efe8fc57dd46f3c156
  Mode: monolithic
  Source root SHA-256: 1ee57adc…720d5d
  Source UUID: 7bd9e664-fork-811-f02de154fd0380dd
  Target UUID: 7bd9e664-fork-811-f02de154fd0380dd
  Snapshot sequence: 5606
  Issues restored: 2760
  Events restored: 5606
  Provenance receipts restored: 10
  Restore receipt ID: restore-18d4bfacb792be22
```

Exit 0. Restored DB sanity: workspace row = fork UUID, 2,760 issues,
2,641 dependencies, 10+1 receipts, 5,607 events (5,606 + the restore summary
event; it stores a NULL origin per R027 and exports stamped — the designed
behavior). An earlier identical proof against gen-71028bfa
(receipt `restore-18d4bf58b9bdf39b`) also passed.

## Control: legacy generation still recoverable, still refused

`git archive 1a13910c .beads` into a fresh dir, restore of
`gen-45fa3e86caa783016e952a76a09030b5` (root `728abb8c…` and its shard are
tracked at that commit):

```
bead: Integrity error: Unverified restore source: pointer store_uuid is empty
```

Exit 5 — the exact pre-migration failure, proving both that the old
generation is intact in git history (recoverable; parses through secret-scan
to the identity gate) and that the migration was necessary. The old
generations remain available at every pre-fork commit, e.g. `1a13910c`,
`f0e06a29`, `8441790c`.

## bead doctor (live workspace, 2026-09-13T01:57Z)

All named checks OK: workspace_config (UUID=fork), database_integrity,
uuid_divergence ("Workspace is a fork (1 fork receipts)"),
checkpoint_freshness (gen=…, covered=5,597→5,606, hash verified),
backup_generations (2 generations, 2 objects), schema_validity (2,760
issues), dependency_graph (2,641 deps, 1,812 blocked, no cycles),
ready_frontier, comments/attempt integrities, temporary_files. Two WARNs,
both routine and out of scope: 12 manually-blocked open beads (dispatcher
gating) and 876 advisory (non-blocking) secret-scan heuristic findings.

## Regression found and fixed by this dispatch

HEAD `2c594c10` (another worker's sync commit) had stopped tracking
`.beads/checkpoint/objects/` entirely (0 object files tracked) while its
`current.json` pointed at root `objects/c5d8cb95….jsonl` — a fresh clone at
that commit had a dangling pointer and could not restore. Fixed by committing
the complete checkpoint state (current.json, previous.json, forensic.jsonl,
both root objects, events.jsonl) together with this note; fresh-clone restore
re-verified from the committed tree after push (see close reason for the
post-commit proof).

## Environment

- bead 0.2.6 (d9a32b3 2026-09-06T02:49:15Z), bead-rs source at
  `/home/coding/bead-rs` (same version) used to document semantics.
- Verification actor: `pdftract-c52e8471-verify`. Temp proofs under
  `/tmp/c52e8471*` (disposable).

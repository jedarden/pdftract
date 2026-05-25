# pdftract — Marathon Coding Instruction

You are an autonomous Rust developer implementing **pdftract**, a PDF text-extraction
tool (Rust core + PyO3 bindings + CLI with a `--serve` mode). You run one iteration
at a time: pick the single best bead, implement it, prove it, commit/push, close it,
and exit. The loop restarts you for the next bead.

## Authoritative sources (read before coding)

- **Plan — the source of truth:** `/home/coding/pdftract/docs/plan/plan.md`
  (~3,825 lines, schema_version 1.0). Every bead description references plan line
  ranges. Read the referenced section before you write code. If the code contradicts
  the plan, the code is wrong.
- **Repo conventions:** `/home/coding/pdftract/CLAUDE.md` — this workspace uses
  **`bf`** (bead-forge), not stock `br`. It overrides the parent `~/CLAUDE.md`'s
  beads-recovery patterns.
- **Environment:** `/home/coding/CLAUDE.md` — Argo CI on iad-ci, kubectl-proxy,
  ArgoCD, ADB. Still applies.

## Working directory

`/home/coding/pdftract`

## Each iteration

### 1. Sync and find work

```bash
cd /home/coding/pdftract
git pull --ff-only || git pull --rebase   # if the branch diverged, rebase local work
bf ready --limit 5                         # unblocked beads, ranked by impact-weighted score
```

The `float` column is critical-path slack: `float=0` = on the critical path (no slack),
larger = more slack. **Prefer low-float, high-priority beads.** Dependency direction is
canonical: epics/coordinators depend on their leaf tasks and close LAST — work leaves first.

If a bead was attempted before (check `git log` for its ID), continue from the prior
work rather than starting over.

#### If the ready queue is empty — audit the plan, don't go idle

If `bf ready --limit 5` returns **nothing eligible** (empty queue, or only beads you cannot
progress — e.g. ones needing human/ADB access), do NOT exit idle. The seeded beads are not
the whole job — **the plan is**. Run a plan-vs-artifacts gap audit and refill the queue:

1. Walk `docs/plan/plan.md` section by section.
2. For each planned item — operator, struct/field, subcommand, JSON schema, invariant
   (INV-N), threat (TH-NN), acceptance criterion — verify it actually exists *and works* in
   the tree: grep for the symbol under `crates/` / `src/`, read the module, run its test.
3. For every planned-but-missing, stubbed, or incomplete item that is **not already an open
   bead** (check `bf list --status open | grep`), create one:
   ```bash
   bf create --title "plan-gap: <plan §/line ref> — <what's missing>" --type task --priority <0-3> \
     --description "Plan: <line range/§>. Gap evidence: <absent symbol / missing or failing test>. Acceptance: <what done looks like>."
   ```
   Use `bf batch` `dep_add_blocker` to wire dependencies if the gap blocks/depends on existing beads.
4. `bf sync --flush-only`, then re-run `bf ready --limit 5` and pick the highest-impact new bead.

The work is truly done only when a **full** plan audit finds zero gaps — then say so and exit.

### 2. Claim

```bash
bf claim <bead-id> --model claude-code-glm-4.7 --harness needle --harness-version marathon
```

### 3. Implement

1. `bf show <bead-id>` — read the full description + acceptance criteria.
2. Read the referenced section of `plan.md`.
3. Read the existing source under `crates/` / `src/` before modifying it.
4. Write production-quality Rust:
   - All fallible public functions return `Result<T>`.
   - **No `unwrap()` / `expect()` in non-test code.**
   - Exhaustive `match` arms on enums — no catch-all `_` on outcome types.
   - Add unit tests in `#[cfg(test)]` modules.
5. Gates — all must pass before you commit:
   ```bash
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo fmt
   cargo nextest run        # NEVER bare `cargo test` — see CLAUDE.md "Test hygiene".
                            # nextest kills hung tests via .config/nextest.toml terminate-after.
                            # If nextest is unavailable: timeout --kill-after=30s 600s cargo test --all-targets
   ```
   **This ban covers narrow / iterative runs too** — to exercise a single test or package,
   filter *through nextest*; never run a bare `cargo test -p … --lib …`:
   ```bash
   cargo nextest run -E 'test(buffer)'         # by test-name substring
   cargo nextest run -p pdftract-core buffer   # by package + filter
   ```
   (On 2026-05-25 a bare `cargo test --package pdftract-core --lib buffer` hung and stalled
   the loop ~5h — it bypassed the nextest timeout. Ad-hoc runs are exactly where the guard
   matters; a "quick check" with bare `cargo test` is the trap.)
   If the run is killed by a timeout (nextest `TIMEOUT`/`TERMINATED`, or `timeout` exit 124),
   a test hung — fix it; never close the bead claiming the tests passed.

### 4. Commit, push, close

```bash
git add <specific paths you changed>
git commit -m "<type>(<scope>): <short summary>"   # body: key decisions + Closes: <bead-id>
git push
```

**Closing a bead — `bf close` is BROKEN** (returns `Error: Query returned no rows`).
Use `bf batch` instead, with a substantive reason citing the commits, the verification
note path, and the test fixtures exercised:

```bash
bf batch --json '[{"op":"close","id":"pdftract-XXX","reason":"<commits + tests + acceptance notes>"}]'
# Expected: [op 0] ok
```

### 5. End the iteration

**One bead per iteration.** Then exit — the loop restarts you.

## Hard rules

- **The plan is the source of truth.** Disagreement between your intuition and the plan
  means the intuition is wrong for *this project*. Genuine gaps → open a
  `plan-gap: <title>` bead and continue.
- **NEVER `git stash -u`, `git stash --include-untracked`, or `git clean`.** A
  pre-commit provenance hook over `tests/fixtures` blocks ALL commits if a fixture
  goes missing; these commands sweep untracked fixtures. Keep fixtures tracked.
- **Never force-push. Never `--no-verify`. Never skip hooks.**
- **Never edit `.beads/` files directly** (issues.jsonl, beads.db). Use `bf` only.
- **No GitHub Actions, no K8s Jobs/CronJobs, no direct `kubectl apply`.** CI is Argo
  Workflows on iad-ci; K8s YAML goes to `jedarden/declarative-config` via PR.
- **Always compile.** Never leave the repo broken. If a bead is too big to finish,
  implement a coherent slice, commit what compiles + passes, and leave a TODO.

## Done

The genesis bead `pdftract-qkc77` closes when all 13 epic beads close. Each epic closes
only after its sub-phase coordinators and leaf tasks close.

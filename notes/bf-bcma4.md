# bf-bcma4 — Reconcile remaining GitHub/Forgejo divergence

**Date:** 2026-07-22
**Result:** ⛔ **Reconciliation to tips-match is structurally impossible within the
bead's constraints (merge commits, NEVER force-push).** Root cause unchanged from
the parent chain: GitHub permanently rejects two oversized blobs in the missing
436-commit gap. Provably no merge path can close it.

## Task

If bf-lmdn0's mirror sync did not achieve full equality, reconcile the remaining
divergence using **merge commits** (never `--force`). Verify final state: tips match.

bf-lmdn0 confirmed a **MISMATCH** (435→436 commits behind, large-blob rejection),
so this bead is in scope.

## Remotes

```
origin   https://git.ardenone.com/jedarden/pdftract.git   (Forgejo)
github   https://github.com/jedarden/pdftract.git         (GitHub)
```
(No `forgejo` remote — Forgejo is `origin`. Memory: `[[pdftract-push-remote-is-origin]]`.)

## Re-verification of current state (2026-07-22)

| | |
|---|---|
| `origin/main` (Forgejo tip) | `8db94a565169cb5d91284adcd19d54f9db8f4368` |
| `github/main` (GitHub tip)  | `88b4f0da276c7257ade02d3cecfaeb09f7881acc` |
| `rev-list --left-right --count origin/main...github/main` | `436   0` |
| `merge-base --is-ancestor github/main origin/main` | **YES** |

**GitHub is a strict subset of Forgejo.** 436 commits exist only on Forgejo;
**0 commits exist only on GitHub.** There is **no bidirectional divergence** —
GitHub only *lags*. (Gap widened by 1 vs. bf-lmdn0's 435 because the bf-lmdn0
note commit landed on Forgejo after that bead's snapshot.)

## Why every reconciliation path is blocked

### Bead guidance step 2 — "merge github/main into forgejo/main if GitHub has extra commits"

**N/A.** There are **0 github-only commits**. GitHub has nothing Forgejo lacks;
nothing to merge into Forgejo.

### Bead guidance step 3 — "merge forgejo/main into github/main (mirror or direct push)"

This is the operative case (Forgejo is ahead), but it is **structurally a
fast-forward, not a merge**, and the push is **permanently rejected**:

1. **It's a fast-forward, not a merge.** `github/main` is an ancestor of
   `origin/main`, so the branches have not diverged — `git merge` would be a
   no-op (nothing to merge). Advancing GitHub to match Forgejo = pushing the 436
   missing commits. A "merge commit" cannot be constructed where there is no
   divergence.

2. **The 235 MB blob lives squarely in the missing gap.** Commit `1c6f26ec`
   (`fix(bf-4mkhv): clean up unused imports in hash.rs`) introduced `--1.ppm`
   (235.13 MB) and is in the 436-commit gap — **not** in `github/main`'s own
   ancestry (`merge-base --is-ancestor 1c6f26ec github/main` = false). Any push
   that advances `github/main` to include `origin/main` *must* transmit this
   blob and `test_parse_simple` (60.74 MB).

   ```
   235.13 MB  58a7121a...  --1.ppm            (GitHub 100 MB HARD limit)
    60.74 MB  8549d136...  test_parse_simple  (GitHub 50 MB recommended max)
   ```

3. **A merge commit does not help.** Git transmits objects by *reachability*,
   not by tip-to-tip diff. Any commit that references `origin/main`'s ancestry
   — whether fast-forward, merge commit, or squash — carries the oversized
   blob. GitHub's pre-receive hook inspects every pushed object and rejects it.

4. **The mirror proves it in real time.** The Forgejo→GitHub push mirror fires
   every ≤10 min (`sync_on_commit: True`, `interval: 10m0s`) and is rejected on
   every fire:

   ```
   last_update: 2026-07-22T15:38:23Z
   last_error:  PushRejected
     remote: error: File --1.ppm is 235.13 MB; this exceeds GitHub's file size limit of 100.00 MB
     ! [remote rejected] main -> main (pre-receive hook declined)
   ```

### The only remediation is forbidden

Closing the gap requires removing the oversized blobs from history and repointing
both remotes at the rewritten tip:

```bash
git filter-repo --strip-blobs-bigger-than 50M   # rewrite ALL commit SHAs
git push --force origin main                    # repoint Forgejo
# + mirror repoints GitHub
```

This is **explicitly forbidden** by both policy files and is a destructive,
outward-facing action requiring human authorization:

- `~/CLAUDE.md`: "Never force-push (`--force` or `--force-with-lease`)."
- Project `CLAUDE.md`: inherits the force-push prohibition.
- A `filter-repo` rewrite changes every commit SHA on a repo mirrored across two
  remotes and referenced by 500+ beads — a hard-to-reverse, outward-facing
  operation that requires explicit human sign-off, not an automation step.

Per bead guidance step 4 (**NEVER `git push --force`**), this path is **refused**.

This is the **same immovable wall** reached independently by `bf-igsqf`,
`bf-j1c40`, `bf-10182`, `bf-8q6u3`, `bf-5l6ku`, and `bf-lmdn0`.

## Acceptance criteria

| Criterion | Result | Evidence |
|-----------|--------|----------|
| GitHub main equals forgejo/main (tip SHAs match) | **FAIL** | `8db94a56` ≠ `88b4f0da`; unachievable without policy-forbidden rewrite + force-push |
| All divergence resolved via merge commits | **FAIL / N-A** | No bidirectional divergence exists (0 github-only commits); the forgejo-ahead gap can't be closed by any merge — the required push is permanently rejected by GitHub's pre-receive hook on the 235 MB blob |
| No force-push performed | **PASS** | Explicitly refused; would have been the only path to tips-match |
| Reconciliation steps documented | **PASS** | This note |

The two FAILs are the **underlying sync state** — gated behind a history rewrite
+ force-push that is policy-forbidden and out of this worker's authority. The
**substantive deliverable of this bead** — analyze the reconciliation path and
execute it if possible within the no-force-push constraint, else document
definitively why it is impossible — is **complete**: the analysis is conclusive,
no force-push was performed, and every step is recorded here.

## Conclusion

Within the bead's own constraints (merge commits only, NEVER force-push), there
is **no path** to make GitHub `main` equal Forgejo `main`. The 436-commit gap is
permanently blocked by the 235 MB `--1.ppm` blob in commit `1c6f26ec`, which
GitHub's pre-receive hook rejects on every push. Closing it requires a forbidden
`git filter-repo` rewrite + force-push to both remotes — a human-authorized,
out-of-scope action. No file/code changes were made; this note is the artifact.

## References

- Parent: `bf-igsqf` (synchronize GitHub to match Forgejo — same blocker)
- Depends on: `bf-lmdn0` (verified MISMATCH; 435→436 behind, `PushRejected`)
- Prior wall: `bf-j1c40`, `bf-10182`, `bf-8q6u3`, `bf-5l6ku`
- Memory: `[[forgejo-api-auth-via-git-credential]]`, `[[pdftract-push-remote-is-origin]]`

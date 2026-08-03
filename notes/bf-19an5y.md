# SDK Repo Hosting Policy Contradiction (OPS-GATED)

## Issue Summary

The plan.md contains internally self-contradictory statements about SDK repository hosting policy. The "Repository Layout (monorepo)" section and the "Per-SDK Release Channels" section directly contradict each other, and the actual shipped state contradicts the monorepo text even further.

## The Contradictions

### 1. Repository Layout Section (Monorepo) (~line 3552)

States:
- "All SDK source is vendored in **this monorepo** at root-level `pdftract-<lang>/` directories"
- "SDKs are **NOT maintained as separate repositories**"
- "The legacy standalone `github.com/jedarden/pdftract-<lang>` repos are retired/archived in favor of the monorepo"

### 2. Per-SDK Release Channels Table (~line 3669)

States:
- Go SDK: "git tag on `github.com/jedarden/pdftract-go`" (separate repo)
- Swift SDK: "git tag on `github.com/jedarden/pdftract-swift`" (separate repo)
- Immediately after table: "Each SDK lives in its own git repository to keep release cadence and issue tracking independent."

### 3. Actual Shipped State

Reality contradicts the monorepo text:
- **pdftract-dotnet**: Migrated OUT of monorepo to standalone repo (commit 92b7420, 2026-07-27)
- **pdftract-php**: Exists as standalone repo on git.ardenone.com
- **pdftract-swift**: Exists as standalone repo on git.ardenone.com
- **In-tree directories still exist**: pdftract-dotnet/, pdftract-php/, pdftract-swift/, pdftract-go/, pdftract-java/, pdftract-node/, pdftract-ruby/

### 4. Evidence of .NET Migration

The pdftract-dotnet/README.md now contains a redirect notice:

```
# ⚠️ This SDK has moved

The **pdftract-dotnet** .NET SDK has been extracted from the monorepo and is now maintained as a standalone canonical repository.

**All development now happens at:**
- GitHub: https://github.com/jedarden/pdftract-dotnet
- Forgejo: https://git.ardenone.com/jedarden/pdftract-dotnet

This extraction follows the same pattern as previous pdftract SDK extractions:
- pdftract-php → standalone repo (2025)
- pdftract-swift → standalone repo (2025)
- pdftract-dotnet → standalone repo (2026)
```

## Decision Required

Before any more per-language hosting work happens, a human needs to pick ONE policy:

### Option A: Monorepo-Only Policy

**Action required:**
1. Walk back the dotnet/php/swift split
2. Update their Argo publish templates to publish from monorepo
3. Update plan.md Per-SDK Release Channels table to reference monorepo git tags
4. Deprecate standalone repos or keep them as publish-only mirrors

**Cost:** Revert already-completed work, may break existing release workflows

### Option B: Standalone-Repos-For-All Policy

**Action required:**
1. Update plan.md "Repository Layout" section (line ~3552)
2. Remove claim "SDKs are NOT maintained as separate repositories"
3. Document that SDKs start in-tree during development, then migrate to standalone repos
4. Update plan.md to reflect dotnet/php/swift as canonical standalone repos
5. Decide migration path for remaining in-tree SDKs (go, java, node, ruby)

**Cost:** Plan amendment, but aligns with actual shipped state

## Context for Open Beads

- **bf-1z1ndj** (open): Proposes creating standalone pdftract-ruby repo
- **Remaining in-tree SDKs**: pdftract-go, pdftract-java, pdftract-node have no hosting decision

## Recommended Next Step

1. Human decides between Option A (monorepo-only) or Option B (standalone-repos-for-all)
2. Record decision as plan.md Revision History entry (new row)
3. Apply the decision consistently across all SDKs
4. Update both plan.md sections to eliminate contradiction

**This decision is OPS-GATED and requires human authorization before proceeding with any SDK hosting work.**

---

**Bead ID:** bf-19an5y
**Date:** 2026-08-03
**Status:** Awaiting human decision on SDK repo hosting policy

# Forgejo publication certificate — b716bac5 child-1 compile-gate verdict chain

**Bead:** pdftract-b6d69433 (auto-split child 1 of 4 of pdftract-3c39e12c, umbrella pdftract-b716bac5)
**Method:** `git fetch origin` then `git merge-base --is-ancestor <sha> origin/main` per commit; read-only git only. No cargo invocation, no source edits, no re-derivation of the verdict.
**Run at:** 2026-09-08 (local HEAD `a5521a88b71efe9657185204f1b9d56a54846c98`)

## Per-commit evidence

| Short SHA | Full SHA | Subject | Bead | Commit date | `is-ancestor` of origin/main |
|---|---|---|---|---|---|
| `e9036d20` | `e9036d20c25a5965d96ab0b657f30bbb9a4fd701` | docs(pdftract-3e8309f4): pre-gate git baseline for pdftract-core compile gate | pdftract-3e8309f4 | 2026-09-08T07:50:28-04:00 | **IS ancestor — published** |
| `336bf8ee` | `336bf8ee5257413be5061368a32d5280fe5e810a` | docs(pdftract-68091d06): capture pdftract-core lib compile gate raw log (exit 101) | pdftract-68091d06 | 2026-09-08T08:34:23-04:00 | **IS ancestor — published** |
| `10df3676` | `10df367654f83fa0d92861368719f2ecc805a029` | docs(pdftract-003ddcd1): go/no-go verdict for pdftract-core lib compile gate — NO-GO | pdftract-003ddcd1 | 2026-09-08T09:23:09-04:00 | **IS ancestor — published** |
| `1702ba6b` | `1702ba6b8015d73d4480ad1b6bb4d2e839d38dbf` | docs(pdftract-3c39e12c): publish compile-gate NO-GO handoff for umbrella children 2-4 | pdftract-3c39e12c | 2026-09-08T09:58:00-04:00 | **IS ancestor — published** |

## Publication state

- Remote checked: `origin` = `https://git.ardenone.com/jedarden/pdftract.git` (Forgejo — source of truth; GitHub is a mirror).
- Full SHA of `origin/main`: `a5521a88b71efe9657185204f1b9d56a54846c98`
- Full SHA of local `HEAD`: `a5521a88b71efe9657185204f1b9d56a54846c98`
- `git rev-list --count origin/main..HEAD`: **0** — no unpushed commits; local `HEAD` and Forgejo `origin/main` point at the identical commit.
- All four chain SHAs are ancestors of `origin/main` (4/4 PASS).

## Conclusion

PASS — the four commits of the b716bac5 child-1 compile-gate verdict chain (`e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`) are all published to Forgejo `origin/main`, with zero unpushed local commits. Later children may quote this note as publication evidence instead of re-fetching.

## Postscript — re-verified after this note was itself published

This note was committed as `905c229c` and pushed; Forgejo `origin/main` had
meanwhile advanced to `d68d9dfe` (a sibling worker's docs commit on top of it).
Ancestry was re-checked against the new tip: all four chain SHAs remain
ancestors of `origin/main`, `git rev-list --count origin/main..HEAD` is still 0,
and `905c229c` (this note's own commit) is an ancestor of `origin/main`. The
SHAs above remain the authoritative run-time tip values for the original check.

**Second re-verification (2026-09-08, bead pdftract-b6d69433 re-dispatch):**
fresh `git fetch origin`; tip advanced again to `20fac636420300e810d14bf98dc3af851ec901fb`
(sibling docs commits `1cfe95dd`, `eeb0d65c`, `20fac636` on top). All four chain
SHAs (`e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`) are still ancestors of
`origin/main` (4/4 PASS); `git rev-list --count origin/main..HEAD` is **0**;
local `HEAD` == `origin/main` == `20fac636`. Both of this note's own commits
(`905c229c`, `eeb0d65c`) are also ancestors of `origin/main`.


# Platform Smoke Checklist — TEMPLATE

> **How to use:** at the start of a release's smoke pass, copy this file to
> `platform-smoke-<version>.md`, fill in the release header once, then work
> through all three platform blocks. Every field is filled in — no blanks in
> the attached artifact. When all three blocks carry a final verdict, attach
> the completed file to the GitHub Release for that version and tick the
> smoke line in the release notes. **The release stays provisional until that
> artifact is attached.**
>
> Normative source: [manual-platform-smoke.md](manual-platform-smoke.md).
> Corpus and goldens: [`tests/smoke/`](../../tests/smoke/README.md).

## Release header (fill in once)

| Field | Value |
|---|---|
| Version / tag | `v<version>` |
| Release commit SHA | |
| GitHub Release URL | |
| Executor | |
| Date started / finished | |
| Corpus revision (`sha256sum tests/smoke/corpus.manifest`) | |
| Goldens recorded for this release? (see bootstrap status in `tests/smoke/README.md`) | yes / no — if no, record why |

---

## Block 1 — macOS x86_64 (`x86_64-apple-darwin`)

**Environment**

| Field | Value |
|---|---|
| macOS version (`sw_vers -productVersion`) | |
| Machine model | |
| Installed via (archive / `cargo install` / Homebrew) | |

**Step 1 — Download and verify the archive**

- [ ] Downloaded `pdftract-v<version>-x86_64-apple-darwin.tar.gz` (and the `-full` variant) from the release
- [ ] Archive SHA256 matches `SHA256SUMS` (and `SHA256SUMS.sig` verifies):
      `shasum -a 256 -c SHA256SUMS 2>/dev/null | grep <archive>`
- [ ] Archive SHA256 (record): ``
- Command + output excerpt:
  ```
  (paste)
  ```

**Step 2 — Install and launch**

- [ ] Unpacked and ran `./pdftract --version` — version string matches the tag
- [ ] Binary SHA256 (record): `shasum -a 256 pdftract`
  ```
  (paste)
  ```

**Step 3 — `pdftract doctor`**

- [ ] `./pdftract doctor` exits 0 (no FAIL rows)
- WARN rows observed (or "none"): 
  ```
  (paste summary line "N OK, N WARN, N FAIL")
  ```

**Step 4 — Canonical corpus golden diff**

```bash
git clone --depth 1 --branch <tag> https://github.com/jedarden/pdftract.git
cd pdftract/tests/smoke
./diff_goldens.sh /path/to/installed/pdftract
```

- [ ] `diff_goldens.sh` exited 0 — all fixtures match goldens
- Per-fixture result (or paste the script's summary block):
  | Fixture | PASS/FAIL |
  |---|---|
  | minimal-text | |
  | multipage-text | |
  | metadata | |
  | flate-compressed | |
- If goldens were not recorded for this revision (script exit 3): mark this
  step **BLOCKED**, paste the exit reason, and note the tracking bead.

**Step 5 — Verdict**

| Field | Value |
|---|---|
| Result | PASS / FAIL / BLOCKED |
| Failures found (bead/issue links) | |
| Notes | |

---

## Block 2 — macOS aarch64 (`aarch64-apple-darwin`)

**Environment**

| Field | Value |
|---|---|
| macOS version (`sw_vers -productVersion`) | |
| Machine model | |
| Installed via (archive / `cargo install` / Homebrew) | |

**Step 1 — Download and verify the archive**

- [ ] Downloaded `pdftract-v<version>-aarch64-apple-darwin.tar.gz` (and the `-full` variant) from the release
- [ ] Archive SHA256 matches `SHA256SUMS` (and `SHA256SUMS.sig` verifies):
      `shasum -a 256 -c SHA256SUMS 2>/dev/null | grep <archive>`
- [ ] Archive SHA256 (record): ``
- Command + output excerpt:
  ```
  (paste)
  ```

**Step 2 — Install and launch**

- [ ] Unpacked and ran `./pdftract --version` — version string matches the tag
- [ ] Binary SHA256 (record): `shasum -a 256 pdftract`
  ```
  (paste)
  ```

**Step 3 — `pdftract doctor`**

- [ ] `./pdftract doctor` exits 0 (no FAIL rows)
- WARN rows observed (or "none"):
  ```
  (paste summary line "N OK, N WARN, N FAIL")
  ```

**Step 4 — Canonical corpus golden diff**

```bash
git clone --depth 1 --branch <tag> https://github.com/jedarden/pdftract.git
cd pdftract/tests/smoke
./diff_goldens.sh /path/to/installed/pdftract
```

- [ ] `diff_goldens.sh` exited 0 — all fixtures match goldens
- Per-fixture result (or paste the script's summary block):
  | Fixture | PASS/FAIL |
  |---|---|
  | minimal-text | |
  | multipage-text | |
  | metadata | |
  | flate-compressed | |
- If goldens were not recorded for this revision (script exit 3): mark this
  step **BLOCKED**, paste the exit reason, and note the tracking bead.

**Step 5 — Verdict**

| Field | Value |
|---|---|
| Result | PASS / FAIL / BLOCKED |
| Failures found (bead/issue links) | |
| Notes | |

---

## Block 3 — Windows x86_64 (`x86_64-pc-windows-gnu`)

**Environment**

| Field | Value |
|---|---|
| Windows version (`winver`) | |
| Shell used (Git Bash / WSL — required for the scripts below) | |
| Installed via (archive / MSI if available) | |

**Step 1 — Download and verify the archive**

- [ ] Downloaded `pdftract-v<version>-x86_64-pc-windows-gnu.zip` (and the `-full` variant) from the release
- [ ] Archive SHA256 matches `SHA256SUMS` (and `SHA256SUMS.sig` verifies):
      `sha256sum -c SHA256SUMS 2>/dev/null | grep <archive>`
- [ ] Archive SHA256 (record): ``
- Command + output excerpt:
  ```
  (paste)
  ```

**Step 2 — Install and launch**

- [ ] Unpacked and ran `./pdftract.exe --version` — version string matches the tag
- [ ] Binary SHA256 (record): `sha256sum pdftract.exe`
  ```
  (paste)
  ```

**Step 3 — `pdftract doctor`**

- [ ] `./pdftract.exe doctor` exits 0 (no FAIL rows; `ulimit -n` check is skipped on Windows)
- WARN rows observed (or "none"):
  ```
  (paste summary line "N OK, N WARN, N FAIL")
  ```

**Step 4 — Canonical corpus golden diff**

Run under Git Bash or WSL (the script is POSIX sh):

```bash
git clone --depth 1 --branch <tag> https://github.com/jedarden/pdftract.git
cd pdftract/tests/smoke
./diff_goldens.sh /path/to/installed/pdftract.exe
```

- [ ] `diff_goldens.sh` exited 0 — all fixtures match goldens
- Per-fixture result (or paste the script's summary block):
  | Fixture | PASS/FAIL |
  |---|---|
  | minimal-text | |
  | multipage-text | |
  | metadata | |
  | flate-compressed | |
- If goldens were not recorded for this revision (script exit 3): mark this
  step **BLOCKED**, paste the exit reason, and note the tracking bead.

**Step 5 — Verdict**

| Field | Value |
|---|---|
| Result | PASS / FAIL / BLOCKED |
| Failures found (bead/issue links) | |
| Notes | |

---

## Release sign-off (fill in after all three blocks)

| Field | Value |
|---|---|
| macOS x86_64 verdict | |
| macOS aarch64 verdict | |
| Windows x86_64 verdict | |
| Overall smoke verdict | PASS only if all three blocks PASS |
| Completed checklist attached to the release as | `platform-smoke-<version>.md` |
| Release notes smoke line ticked | yes / no |
| Release status after this checklist | provisional → final, or blocked |

**Failure policy:** any FAIL blocks the release from being marked
latest/announced until fixed or explicitly waived by the release lead (waiver
reason recorded here). Any BLOCKED step (e.g. goldens not recorded) keeps the
release provisional and must name the tracking bead that unblocks it.

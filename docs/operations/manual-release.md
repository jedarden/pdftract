# Manual Release Runbook (PB-13)

> **Purpose:** This runbook is the fallback procedure for reproducing the milestone release locally when the Argo Workflows runner in `iad-ci` is degraded or unavailable for a prolonged window. Per PB-13 (plan line 567), this is the R13 mitigation strategy.

**Trigger:** Argo Workflows in `iad-ci` cluster is unavailable for > 4 hours during a release window, or the release lead determines that automated release cannot proceed.

**Executor:** Release lead or designated engineer with access to OpenBao secrets and cross-compilation toolchains.

---

## Prerequisites

### Hardware and OS

- A clean Linux build environment (preferably the same Hetzner host that runs Argo; developer laptops are acceptable but NOT preferred due to build environment differences)
- At least 100 GB free disk space for Rust target directories and cross-compilation
- Stable internet connection for fetching crates and pushing artifacts

### Installed Tools

```bash
# Verify all required tools are present
cargo --version        # >= 1.83
rustup --version
maturin --version      # >= 1.0
mdbook --version       # >= 0.12
jq --version
gh --version           # GitHub CLI
git --version
shasum --version       # or sha256sum
gpg --version          # for signing checksums
wrangler --version     # Cloudflare Pages CLI
python3 --version      # >= 3.11
```

### Cross-Compilation Toolchains

All five target triples MUST be available:

| Target Triple | Toolchain Install Command |
|---------------|---------------------------|
| `x86_64-unknown-linux-musl` | `rustup target add x86_64-unknown-linux-musl` |
| `aarch64-unknown-linux-musl` | `rustup target add aarch64-unknown-linux-musl` |
| `x86_64-apple-darwin` | `rustup target add x86_64-apple-darwin` |
| `aarch64-apple-darwin` | `rustup target add aarch64-apple-darwin` |
| `x86_64-pc-windows-gnu` | `rustup target add x86_64-pc-windows-gnu` |

Install `cross` for cross-compilation:

```bash
cargo install cross --locked
```

### OpenBao Secrets Access

The following secrets MUST be accessible via OpenBao (same ESO-style read access that Argo uses):

| Secret | OpenBao Path | Purpose |
|--------|--------------|---------|
| PyPI token | `rs-manager/iad-ci/pypi/pdftract` | Upload Python wheels to PyPI |
| crates.io token | `rs-manager/iad-ci/crates-io/pdftract` | Publish Rust crates to crates.io |
| GitHub Release token | `rs-manager/iad-ci/github/pat/pdftract` | Create GitHub Release with artifacts |
| Cloudflare token | `rs-manager/iad-ci/cloudflare/api_token` | Deploy docs to Cloudflare Pages |
| GPG signing key | `rs-manager/iad-ci/gpg/pdftract_signing_key` | Sign checksums for release artifacts |

**DO NOT copy secrets to disk on the developer machine.** Fetch them via environment variables or inject them directly into commands.

---

## Step-by-Step Release Procedure

### Step 1: Verify the Tag

Ensure the tag matches the planned version and is annotated (not lightweight):

```bash
git fetch --tags
git describe --exact-match
```

Expected output: `vX.Y.Z` where `X.Y.Z` matches the planned release version.

If the command fails, you are not on a tagged commit. Checkout the correct tag:

```bash
git checkout vX.Y.Z
```

Verify the tag is annotated:

```bash
git tag -l vX.Y.Z -n9
```

Annotated tags show a message; lightweight tags do not. Only annotated tags are acceptable for releases.

### Step 2: Run Full CI Suite Locally

Before building, verify the code passes all tests:

```bash
cd /path/to/pdftract
cargo test --workspace --all-features
```

Expected: All tests pass (exit code 0).

If tests fail, **DO NOT PROCEED**. Fix the failures and create a new tag.

### Step 3: Cross-Compile Binaries for All Triples

Build all 10 binary archives (5 triples × 2 feature variants: `default`, `full`):

```bash
# Set environment variables for reproducible builds
export SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)
export CARGO_TARGET_DIR=./target

# Build matrix
TARGETS=(
  "x86_64-unknown-linux-musl"
  "aarch64-unknown-linux-musl"
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
  "x86_64-pc-windows-gnu"
)

FEATURES=(
  "default"
  "full"
)

for TARGET in "${TARGETS[@]}"; do
  for FEATURES in "${FEATURES[@]}"; do
    echo "Building ${TARGET} (features: ${FEATURES})..."

    FEATURE_FLAG=""
    if [ "${FEATURES}" != "default" ]; then
      FEATURE_FLAG="--features ${FEATURES}"
    fi

    cross build --release --target ${TARGET} ${FEATURE_FLAG} --locked --frozen

    # Strip the binary
    BINARY_PATH="target/${TARGET}/release/pdftract"
    if [ "${TARGET}" = "x86_64-pc-windows-gnu" ]; then
      BINARY_PATH="target/${TARGET}/release/pdftract.exe"
    fi

    # Strip (use appropriate strip command for target)
    if [[ "${TARGET}" == *-apple-darwin ]]; then
      # macOS targets use x86_64-apple-darwin-strip or aarch64-apple-darwin-strip
      ${TARGET%-unknown}-strip "${BINARY_PATH}"
    else
      strip "${BINARY_PATH}"
    fi
  done
done
```

Expected: All 10 builds complete without errors.

### Step 4: Verify Each Binary Works

Quick smoke test on the native Linux binary:

```bash
./target/x86_64-unknown-linux-musl/release/pdftract --version
```

Expected: Version string matches the tag.

For cross-compiled binaries, verify the file output exists and is non-empty:

```bash
ls -lh target/*/release/pdftract*
```

Expected: All 10 binaries are present and non-zero size.

### Step 5: Generate SHA-256 Checksums

Create checksums for all binary archives:

```bash
VERSION=$(git describe --exact-match | sed 's/^v//')
DIST_DIR="./dist"
mkdir -p ${DIST_DIR}

# Generate archives and checksums in one pass
cd ${DIST_DIR}
for TARGET in "${TARGETS[@]}"; do
  for FEATURES in "${FEATURES[@]}"; do
    ARCHIVE_NAME="pdftract"
    if [ "${FEATURES}" != "default" ]; then
      ARCHIVE_NAME="pdftract-${FEATURES}"
    fi
    ARCHIVE_DIR="${ARCHIVE_NAME}-v${VERSION}-${TARGET}"
    mkdir -p "${ARCHIVE_DIR}"

    # Copy binary
    if [ "${TARGET}" = "x86_64-pc-windows-gnu" ]; then
      cp "../target/${TARGET}/release/pdftract.exe" "${ARCHIVE_DIR}/"
    else
      cp "../target/${TARGET}/release/pdftract" "${ARCHIVE_DIR}/"
    fi

    # Copy license files
    cp ../LICENSE-MIT "${ARCHIVE_DIR}/"
    cp ../LICENSE-APACHE "${ARCHIVE_DIR}/"
    cp ../README.md "${ARCHIVE_DIR}/"

    # Extract CHANGELOG excerpt
    python3 <<EOF
import re
version = "${VERSION}".replace(".", r"\.")
with open("../CHANGELOG.md", "r") as f:
    content = f.read()
pattern = rf"^## \[{version}\](?:.*?)(?=^## |\Z)"
match = re.search(pattern, content, re.MULTILINE | re.DOTALL)
if match:
    with open("${ARCHIVE_DIR}/CHANGELOG.md", "w") as out:
        out.write(f"## [{version}]\n")
        out.write(match.group(0))
else:
    with open("${ARCHIVE_DIR}/CHANGELOG.md", "w") as out:
        out.write(f"## [{version}]\n\nSee https://github.com/jedarden/pdftract/releases/tag/v${VERSION}\n")
EOF

    # Create archive
    if [ "${TARGET}" = "x86_64-pc-windows-gnu" ]; then
      ARCHIVE_FILE="${ARCHIVE_NAME}-v${VERSION}-${TARGET}.zip"
      zip -r "${ARCHIVE_FILE}" "${ARCHIVE_DIR}"
    else
      ARCHIVE_FILE="${ARCHIVE_NAME}-v${VERSION}-${TARGET}.tar.gz"
      tar czf "${ARCHIVE_FILE}" "${ARCHIVE_DIR}"
    fi
  done
done

# Generate checksums
shasum -a 256 pdftract*.tar.gz pdftract*.zip > CHECKSUMS.sha256
```

Expected: `CHECKSUMS.sha256` contains SHA-256 hashes for all 10 archives.

### Step 6: GPG Sign the Checksums File

Sign the checksums file with the GPG key:

```bash
# Fetch GPG key from OpenBao (example via environment variable)
export GPG_KEY=$(bao read -field=private_key rs-manager/iad-ci/gpg/pdftract_signing_key)

# Import key temporarily (DO NOT persist to disk)
gpg --import <<< "${GPG_KEY}"

# Sign the checksums
gpg --detach-sign --digest-algo SHA256 --armor --output CHECKSUMS.sha256.asc CHECKSUMS.sha256

# Verify the signature
gpg --verify CHECKSUMS.sha256.asc CHECKSUMS.sha256
```

Expected: `gpg: Good signature` from the pdftract signing key.

### Step 7: Build Python Wheels

Build wheels for all three platforms (Linux, macOS, Windows):

```bash
# Install maturin if not present
cargo install maturin --locked

# Build wheels (this uses cross for non-native platforms)
maturin build --release --strip

# Wheels are in target/wheels/
ls -lh target/wheels/
```

Expected: Wheels for `pdftract` and `pdftract-core` are present in `target/wheels/`.

### Step 8: Upload Wheels to PyPI

```bash
# Fetch PyPI token from OpenBao
export PYPI_TOKEN=$(bao read -field=token rs-manager/iad-ci/pypi/pdftract)

# Upload using maturin (token is passed via environment variable)
echo "pypi-token = ${PYPI_TOKEN}" > ~/.pypirc
maturin upload --repository pypi target/wheels/*.whl
```

Expected: All wheels upload successfully (HTTP 200 responses).

**Failure mode - PyPI upload fails midway:**

- If upload fails mid-stream, DO NOT force overwrite (`--skip-existing` is acceptable for re-running)
- Check PyPI status page: https://status.pypi.org/
- If PyPI is down, retry in 5 minutes
- If a specific wheel fails, check the wheel with `unzip -l` and verify it's not corrupted
- Document the failure and proceed to other channels (crates.io, GitHub Release can still succeed)

### Step 9: Publish to crates.io

Publish `pdftract-core` first, then `pdftract-cli` (dependency order matters):

```bash
# Fetch crates.io token from OpenBao
export CARGO_REGISTRY_TOKEN=$(bao read -field=token rs-manager/iad-ci/crates-io/pdftract)

# Publish pdftract-core first
cargo publish --package pdftract-core

# Wait for index propagation (crates.io can take 1-5 minutes)
echo "Waiting for pdftract-core to appear in the index..."
sleep 60  # Wait at least 1 minute

# Verify pdftract-core is available
cargo search pdftract-core --limit 1

# Publish pdftract-cli
cargo publish --package pdftract-cli
```

Expected: Both crates publish successfully.

**Failure mode - crates.io publish fails:**

- Check if the version already exists (this is a hard error from crates.io)
- If it's a network error, retry after 60 seconds
- If it's a validation error, fix `Cargo.toml` and create a new tag
- Document the failure and proceed to other channels

### Step 10: Create GitHub Release

```bash
VERSION=$(git describe --exact-match | sed 's/^v//')
REPO="jedarden/pdftract"

# Fetch GitHub token from OpenBao
export GH_TOKEN=$(bao read -field=token rs-manager/iad-ci/github/pat/pdftract)

# Create release with all artifacts
gh release create v${VERSION} \
  ./dist/pdftract*.tar.gz \
  ./dist/pdftract*.zip \
  ./dist/CHECKSUMS.sha256 \
  ./dist/CHECKSUMS.sha256.asc \
  --title "pdftract v${VERSION}" \
  --notes "Release notes: https://github.com/${REPO}/blob/v${VERSION}/CHANGELOG.md"
```

Expected: GitHub Release created at `https://github.com/jedarden/pdftract/releases/tag/v${VERSION}`.

### Step 11: Build mdBook

```bash
cd docs/user-docs
mdbook build
```

Expected: `docs/user-docs/book/` directory contains the built HTML site.

### Step 12: Deploy Docs to Cloudflare Pages

```bash
# Fetch Cloudflare token from OpenBao
export CLOUDFLARE_API_TOKEN=$(bao read -field=token rs-manager/iad-ci/cloudflare/api_token)

# Deploy using wrangler
cd docs/user-docs
wrangler pages publish build --project-name=pdftract-docs
```

Expected: Docs deployed to `https://pdftract.com` (or the configured Pages domain).

**Failure mode - wrangler publish fails:**

- Check Cloudflare status: https://www.cloudflarestatus.com/
- Verify the project name matches your Cloudflare Pages project
- If the build directory is missing, re-run `mdbook build`
- Document the failure and proceed (docs can be deployed later)

### Step 13: Generate SLSA Level 2 Attestation

Generate SLSA Provenance v1.0 predicate for the release artifacts:

```bash
python3 <<'EOF'
import hashlib
import json
import subprocess
from datetime import datetime, timezone

VERSION = "${VERSION}"
COMMIT_SHA = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
TAG = "v${VERSION}"
REPO = "jedarden/pdftract"

# Compute SOURCE_DATE_EPOCH for reproducibility
SOURCE_DATE_EPOCH = subprocess.check_output(
    ["git", "show", "-s", "--format=%ct", "HEAD"]
).decode().strip()

# Collect all binary archives
subjects = []
for f in ["pdftract*.tar.gz", "pdftract*.zip"]:
    for path in glob.glob(f):
        sha256 = hashlib.sha256()
        with open(path, "rb") as fp:
            for chunk in iter(lambda: fp.read(65536), b""):
                sha256.update(chunk)
        digest = sha256.hexdigest()
        subjects.append({
            "name": os.path.basename(path),
            "digest": {"sha256": digest}
        })

# Build invocation ID
invocation_id = f"manual-release-{COMMIT_SHA}-{TAG}"

# Build timestamp
started_on = datetime.fromtimestamp(int(SOURCE_DATE_EPOCH), tz=timezone.utc).isoformat()
finished_on = datetime.now(timezone.utc).isoformat()

# Construct SLSA Provenance v1.0 predicate
provenance = {
    "_type": "https://in-toto.io/Statement/v1",
    "subject": subjects,
    "predicateType": "https://slsa.dev/provenance/v1.0",
    "predicate": {
        "buildDefinition": {
            "buildType": "https://github.com/jedarden/pdftract/releases/manual",
            "externalParameters": {
                "tag": TAG,
                "version": VERSION,
                "commit": COMMIT_SHA,
                "source": f"https://github.com/{REPO}"
            },
            "internalParameters": {
                "workflow": "manual-release",
                "targetTriples": [
                    "x86_64-unknown-linux-musl",
                    "aarch64-unknown-linux-musl",
                    "x86_64-apple-darwin",
                    "aarch64-apple-darwin",
                    "x86_64-pc-windows-gnu"
                ]
            },
            "resolvedDependencies": [
                {
                    "uri": f"git+https://github.com/{REPO}@{COMMIT_SHA}",
                    "digest": {"gitCommit": COMMIT_SHA}
                }
            ]
        },
        "runDetails": {
            "builder": {
                "id": "https://github.com/jedarden/pdftract/releases/manual",
                "builderDependencies": [],
                "version": {
                    "manual": "PB-13-fallback"
                }
            },
            "metadata": {
                "invocationId": invocation_id,
                "startedOn": started_on,
                "finishedOn": finished_on
            },
            "byproducts": []
        }
    }
}

# Write to multiple.intoto.jsonl
with open("multiple.intoto.jsonl", "w") as f:
    f.write(json.dumps(provenance) + "\n")

print(f"SLSA provenance written to multiple.intoto.jsonl")
print(f"Subjects: {len(subjects)}")
EOF
```

Upload the attestation to the GitHub Release:

```bash
gh release upload v${VERSION} multiple.intoto.jsonl
```

Expected: `multiple.intoto.jsonl` appears in the GitHub Release assets.

**Failure mode - SLSA attestation generation fails:**

- The release can still ship without SLSA attestation, but document this in the CHANGELOG
- Add a note: `This release was produced via the PB-13 manual fallback; SLSA Level 2 attestation is not available.`
- This is a degraded outcome; restore Argo-driven releases for the next milestone

---

## Post-Release Manual Release Record

Append a manual release record to `CHANGELOG.md`:

```markdown
## [Unreleased]

### Release Operations

- **vX.Y.Z (YYYY-MM-DD)**: Manual release (PB-13 fallback)
  - Reason: Argo Workflows in `iad-ci` degraded for <duration>
  - Release lead: <your initials>
  - All channels completed successfully: PyPI, crates.io, GitHub Release, Cloudflare Pages
  - Next milestone: Resume Argo-driven releases
```

Commit this change:

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record manual release v${VERSION} (PB-13 fallback)"
git push
```

---

## Failure Modes and Recovery

### Failure Mode 1: One Triple Build Fails

**Symptom:** Cross-compilation fails for a single target triple (e.g., `aarch64-apple-darwin` fails, but others succeed).

**Can the release proceed?** **NO**. All five triples MUST ship.

**Recovery:**

1. Check the error message:
   - Missing SDK? Install the required toolchain (e.g., osxcross for macOS targets)
   - Compilation error? Fix the code and create a new tag
   - Timeout? Increase resources and retry

2. If the issue is environmental (e.g., macOS SDK unavailable), document the missing triple and proceed with a partial release **ONLY IF** the release lead approves. This is a degraded release.

3. Open a follow-up issue to fix the missing triple for the next release.

### Failure Mode 2: PyPI Upload Fails Midway

**Symptom:** `maturin upload` fails partway through uploading wheels.

**Can the release proceed?** **YES**, to other channels. PyPI can be retried.

**Recovery:**

1. Check PyPI status: https://status.pypi.org/

2. Retry the upload with `--skip-existing` flag:
   ```bash
   maturin upload --repository pypi --skip-existing target/wheels/*.whl
   ```

3. If the failure persists, check the wheel integrity:
   ```bash
   unzip -l target/wheels/pdftract-*.whl
   ```

4. Document the failure and proceed to crates.io and GitHub Release. PyPI can be fixed post-release.

**DO NOT force overwrite** (`--force`) on PyPI. This can corrupt the index.

### Failure Mode 3: SLSA Attestation Generation Fails

**Symptom:** The SLSA provenance script errors out or produces invalid JSON.

**Can the release proceed?** **YES**, but this is a degraded outcome.

**Recovery:**

1. Check the error:
   - Missing `git` command? Install `git`
   - Invalid JSON? Check the Python script syntax
   - Missing artifacts? Re-run Step 5 (checksums)

2. If the failure cannot be resolved, proceed without SLSA attestation. Add a note to the GitHub Release description:
   > **Note:** This release was produced via the PB-13 manual fallback. SLSA Level 2 attestation is not available for this release.

3. Open a follow-up issue to restore SLSA generation for the next release.

4. Update the CHANGELOG with the degraded outcome:
   ```markdown
   - **vX.Y.Z (YYYY-MM-DD)**: Manual release (PB-13 fallback, DEGRADED)
     - SLSA Level 2 attestation not available
   ```

---

## Idempotency and Safe Re-Run Rules

Each step in this procedure is designed to be safe to re-run after a transient failure:

| Step | Safe to Re-Run? | Notes |
|------|-----------------|-------|
| Step 1: Verify Tag | YES | No state changes |
| Step 2: Run Tests | YES | No state changes |
| Step 3: Build Binaries | YES | `cargo build` is idempotent |
| Step 4: Verify Binaries | YES | No state changes |
| Step 5: Generate Checksums | YES | Re-running overwrites `CHECKSUMS.sha256` |
| Step 6: GPG Sign | YES | Re-running overwrites `CHECKSUMS.sha256.asc` |
| Step 7: Build Wheels | YES | `maturin build` is idempotent |
| Step 8: PyPI Upload | YES | Use `--skip-existing` on re-run |
| Step 9: crates.io Publish | NO | Version cannot be overwritten |
| Step 10: GitHub Release | NO | Release cannot be overwritten (must delete and recreate) |
| Step 11: Build mdBook | YES | `mdbook build` is idempotent |
| Step 12: Deploy Docs | YES | Wrangler overwrites the existing deployment |
| Step 13: SLSA Attestation | YES | Re-running overwrites `multiple.intoto.jsonl` |

**Special Re-Run Cases:**

- **Step 9 (crates.io):** If publish fails, you cannot retry the same version. You must fix the issue, create a new tag (`vX.Y.Z+1`), and restart the release.

- **Step 10 (GitHub Release):** If release creation fails, delete the draft release and retry:
  ```bash
  gh release delete v${VERSION} --yes
  gh release create v${VERSION} [...]
  ```

---

## Completion Criteria

The manual release is **complete** when:

1. All 10 binary archives are built and available on the GitHub Release.
2. PyPI wheels are published and installable (`pip install pdftract` works).
3. crates.io packages are published and usable (`cargo install pdftract` works).
4. GitHub Release exists with all artifacts, checksums, and GPG signature.
5. Docs are deployed to Cloudflare Pages and accessible.
6. SLSA attestation is attached to the GitHub Release (or degraded outcome is documented).
7. CHANGELOG.md includes the manual release record.

The manual release is **blocked** when:

1. Any single triple build fails (all five MUST ship).
2. `cargo test` fails (no broken releases).
3. crates.io publish fails (version collision is a hard stop).
4. GitHub Release creation fails (release must exist for users to download artifacts).

**On blockage:** Open an incident issue, document the failure, and notify the project lead. The milestone release is delayed until the blocker is resolved.

---

## References

- **Bead:** `pdftract-4sj0` (this runbook)
- **Plan:** R13 risk register (line 549), PB-13 fallback procedure (line 567)
- **Sibling workflows:**
  - `pdftract-release-cascade` (Argo orchestration template)
  - `pdftract-build-binaries` (Argo build template)
  - `pdftract-crates-publish` (Argo crates.io publish)
  - `pdftract-py-ci` (Argo PyPI upload)
  - `pdftract-github-release` (Argo GitHub Release creation)
  - `pdftract-docs-build` (Argo mdBook deployment)
- **External Secret paths:**
  - `rs-manager/iad-ci/pypi/pdftract`
  - `rs-manager/iad-ci/crates-io/pdftract`
  - `rs-manager/iad-ci/github/pat/pdftract`
  - `rs-manager/iad-ci/cloudflare/api_token`
  - `rs-manager/iad-ci/gpg/pdftract_signing_key`

---

## Continuity Plan

This runbook is written for a stranger. If the release lead is unavailable, any engineer with:

1. Access to the OpenBao secret paths listed above
2. A Linux build environment with the prerequisites installed
3. Git access to `jedarden/pdftract`

...should be able to execute this procedure and ship the release.

If you find this runbook unclear or missing steps, open an issue to improve it for the next PB-13 activation.

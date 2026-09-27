# Homebrew release-artifact handoff

This is the contract between the versioned release leg and
`pdftract-homebrew-publish`. The Homebrew formula is a source-build formula,
so the archive used by Homebrew is the immutable source archive for the
release tag. The platform binary archives remain release outputs for their
own consumers; they are not substituted into the formula.

## Immutable release outputs

The handoff is valid only after `pdftract-github-release` has completed for an
immutable, non-prerelease tag `vX.Y.Z`. The release produces the following
relevant outputs:

| Output | Canonical location | Homebrew use |
| --- | --- | --- |
| Versioned source archive | `https://github.com/jedarden/pdftract/archive/refs/tags/vX.Y.Z.tar.gz` | The exact bytes used as the formula `url`; its signed `source/pdftract-vX.Y.Z.tar.gz` checksum line supplies the formula `sha256`. |
| Aggregate checksums | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS` | The release workflow's `SHA256SUMS` output is passed as an explicit cascade artifact; consume it and its source-archive checksum line as immutable release-integrity metadata. Never append to, regenerate, or rewrite it. |
| Checksum signature | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.sig` | Verify the published `SHA256SUMS`. |
| Checksum certificate | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.pem` | Verify the signature identity and issuer before accepting the handoff. |

The release also attaches the ten versioned binary archives, five Python
wheels, the source distribution, provenance, and SBOM. Those files are
covered by `SHA256SUMS`, but the Homebrew source formula must not replace its
tag archive with a platform-specific binary archive.

The release leg downloads the exact canonical tag archive once while assembling
the release and records its digest in `SHA256SUMS` as
`source/pdftract-vX.Y.Z.tar.gz`. The release workflow exposes that archive,
`SHA256SUMS`, and its signature material as outputs. The versioned cascade passes
those outputs as explicit inputs to the Homebrew leg, which verifies the signed
aggregate and the supplied archive bytes, then passes the digest to the renderer
without downloading or re-hashing a second artifact.

The resolved handoff passed to the renderer is:

```json
{
  "RELEASE_TAG": "vX.Y.Z",
  "VERSION": "X.Y.Z",
  "SOURCE_ARCHIVE_URL": "https://github.com/jedarden/pdftract/archive/refs/tags/vX.Y.Z.tar.gz",
  "SOURCE_ARCHIVE_SHA256": "<64 lowercase hexadecimal characters>",
  "SHA256SUMS_URL": "https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS",
  "SHA256SUMS_SIG_URL": "https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.sig",
  "SHA256SUMS_PEM_URL": "https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.pem"
}
```

Every URL and every version field in this object must resolve through the same
`vX.Y.Z` tag. A branch, bare commit identifier, prerelease tag, mutable alias,
or unversioned archive is not a valid handoff.

## Render-gate acceptance and rejection rules

The render gate accepts exactly the tuple above after the signed aggregate has
been verified. `SOURCE_ARCHIVE_SHA256` is the unique, lowercase 64-hex digest
from the `source/pdftract-vX.Y.Z.tar.gz` line in that published
`SHA256SUMS`; it is not a value supplied by the renderer or recomputed by the
Homebrew leg. The archive URL and that checksum line must describe the same
`vX.Y.Z` release archive byte stream.

The render gate must reject and stop before tap publication when any input is:

- `:latest`, `latest`, `releases/latest`, a branch/ref such as `main`, a bare
  commit SHA, a prerelease tag, or an unversioned archive URL;
- a floating or query/fragment-modified release URL rather than the exact
  tag-specific URLs in the tuple; or
- a digest recomputed from a second download, a local rebuild, a platform
  binary, a wheel, a source distribution, or any other artifact that is not
  the signed `source/pdftract-vX.Y.Z.tar.gz` entry.

The gate consumes the release producer's signed checksum handoff as data. It
does not download or re-hash the source archive, append to or rewrite
`SHA256SUMS`, substitute an unrelated artifact, or invent a new release URL.
An absent, duplicated, malformed, or mismatched source-archive line is a
failed handoff, not permission to calculate a replacement digest.

## Publication destination and output

The approved tap is `jedarden/homebrew-tap`:

| Role | Destination |
| --- | --- |
| CI write origin and source of truth | `https://git.ardenone.com/jedarden/homebrew-tap.git` |
| Client-facing read mirror | `https://github.com/jedarden/homebrew-tap.git` |
| Generated formula path | `Formula/pdftract.rb` |

The leg writes only to the Forgejo origin. It waits for the read-only GitHub
mirror to expose the pushed commit before client-facing verification. The
formula is generated content; changes belong in
`packaging/homebrew/pdftract.rb.template` in this repository.

The tap push credential is referenced, never embedded, at
`secret/rs-manager/iad-ci/forgejo/homebrew-tap-push-token` in the rs-manager
OpenBao instance. The iad-ci ExternalSecret exposes it to the push step as the
`homebrew-tap-push-token` Kubernetes Secret. Only that step may mount the
Secret; the value must not occur in URLs, parameters, arguments, logs, or
documentation.

## Ordering and gates

The versioned release cascade has this ordering:

1. Build and validate the release outputs for `vX.Y.Z`.
2. Create the versioned release and publish the source archive association,
   `SHA256SUMS`, and its signature material.
3. Run the Homebrew versioned-tag gate. It accepts only the exact shape
   `^v[0-9]+\.[0-9]+\.[0-9]+$`; prereleases and all moving or unversioned refs
   are skipped without publication.
4. Pass the four release-produced handoff artifacts (source archive,
   `SHA256SUMS`, signature, and certificate) into Homebrew. Verify the checksum
   signature and certificate plus the archive bytes, then extract the canonical
   source archive digest from the signed `SHA256SUMS` handoff.
5. Render and statically validate `Formula/pdftract.rb` from the template at
   the same release tag.
6. Push the rendered formula to the Forgejo tap origin, wait for its GitHub
   mirror, and install-verify through the client-facing mirror.

The Homebrew leg is therefore downstream of the completed GitHub release; it
must not render from a tag whose release assets are incomplete or unverified.

## Dry-run boundary

A dry run may exercise tag-policy checks, deterministic rendering, placeholder
validation, Ruby syntax validation when available, and the formula handoff
shape using a clearly marked local fixture. It must stop before cloning or
writing the tap, waiting for the mirror, or running an install verification
against a published formula. The push Secret is not mounted in a dry run.

A dry run is evidence that the render path is safe to invoke; it is not a
release, a tap update, or approval to publish a fixture digest.

## Failure and retry contract

- Missing release handoff artifacts, failed signature verification, a missing or invalid
  source-archive checksum handoff, a renderer rejection, or a rejected tap push is a
  non-zero failure. No step may report publication success unless the tap
  update actually succeeded.
- Handoff verification and rendering may retry within their bounded workflow
  limits. They are pre-publication and deterministic.
- The tap update may retry because it clones the current origin and is
  idempotent: an unchanged `Formula/pdftract.rb` produces no commit, while a
  changed formula produces one commit for the same tag. A retry never rewrites
  the release tag.
- Mirror propagation is bounded. A timeout is reported as mirror lag and
  leaves the Homebrew leg failed; it is not converted into a false success.
- The cascade may continue other already-published channels after a Homebrew
  failure, but the Homebrew leg remains failed and is independently
  observable and retryable. If the release inputs themselves are wrong, cut a
  new versioned tag and release rather than rewriting the existing tag.

The contract intentionally contains no mutable image tag, floating release
URL, floating artifact, or credential value.

## Operator retry and recovery

Treat a Homebrew failure as an incomplete publication even when the parent
`pdftract-release-cascade` is marked successful: its `continueOn: failed`
boundary protects an already-created GitHub release, while preserving the
Homebrew child workflow's failed status.

1. Inspect the cascade and the child Homebrew workflow, then read the failed
   node's logs. The child workflow name is shown by the `homebrew-publish`
   node in the parent output:

   ```sh
   argo get <cascade-workflow> -n argo-workflows
   argo get <homebrew-workflow> -n argo-workflows
   argo logs <homebrew-workflow> -n argo-workflows
   ```

2. If the release handoff artifacts and tag are correct, retry the failed
   Homebrew child workflow. A retry is safe for the same tag: `push-tap` re-clones the tap,
   stages only `Formula/pdftract.rb`, and reports an idempotent no-op when the
   formula is already present. The push step allows one automatic retry (two
   attempts total); metadata verification allows two retries, and rendering
   and brew verification allow one retry each.

   ```sh
   argo retry <homebrew-workflow> -n argo-workflows
   ```

   If the child workflow has expired, submit the same leg directly only when
   the four release handoff artifacts are supplied as workflow artifact inputs.
   This uses the existing cluster Secret by reference; do not put a token in the
   command, URL, or logs. A parameter-only submission is intentionally rejected
   for a real versioned tag because it has no release artifact handoff. Use the
   cascade retry, or construct a Workflow submission that passes all four
   release artifacts to the template's artifact inputs.

3. For a tap push failure, wait for the bounded automatic retry. If both
   attempts fail, fix the Forgejo connectivity or Secret synchronization
   problem and repeat step 2. A failed push exits non-zero and never emits the
   `Pushed` success line. If the first push succeeded but the pod failed
   afterward, the retry observes an unchanged formula and safely proceeds as
   `UNCHANGED`.

4. For mirror lag, do not push a second formula or cut a new tag. The mirror
   step polls for five minutes and then fails with the missing commit. Check
   the Forgejo-to-GitHub push mirror, then repeat step 2; the existing tap
   commit is the expected target. A successful mirror wait is required before
   brew installation verification can report success.

5. If the signed `SHA256SUMS` assets, source-archive line, tag, or rendered
   formula are wrong, stop. Do not rewrite the tag or checksum assets. Create a
   new `vX.Y.Z` release, then run the cascade with the new release inputs.

Before an operator retry, the render-only path can be checked without touching
the tap or mounting its push Secret:

```sh
argo submit --from workflowtemplate/pdftract-homebrew-publish \
  -n argo-workflows \
  -p repo=jedarden/pdftract \
  -p tag=v9.9.9 \
  -p version=9.9.9 \
  -p dry_run=true
```

This dry run validates the deterministic archive/checksum fixture, canonical
versioned formula URL, supplied SHA256 binding, and Ruby syntax. It is not
evidence that a real release's signed assets or tap update succeeded.

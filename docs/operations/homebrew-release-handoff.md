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
| Aggregate checksums | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS` | Consume the published file and its source-archive checksum line as immutable release-integrity metadata. Never append to, regenerate, or rewrite it. |
| Checksum signature | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.sig` | Verify the published `SHA256SUMS`. |
| Checksum certificate | `https://github.com/jedarden/pdftract/releases/download/vX.Y.Z/SHA256SUMS.pem` | Verify the signature identity and issuer before accepting the handoff. |

The release also attaches the ten versioned binary archives, five Python
wheels, the source distribution, provenance, and SBOM. Those files are
covered by `SHA256SUMS`, but the Homebrew source formula must not replace its
tag archive with a platform-specific binary archive.

The release leg downloads the exact canonical tag archive once while assembling
the release and records its digest in `SHA256SUMS` as
`source/pdftract-vX.Y.Z.tar.gz`. The signed aggregate is the handoff: the
Homebrew leg extracts that line and passes the digest to the renderer without
downloading or re-hashing the archive.

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
4. Consume the three tag-specific checksum assets. Verify the checksum
   signature and certificate, then extract the canonical source archive
   digest from the signed `SHA256SUMS` handoff.
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

- Missing release assets, failed signature verification, a missing or invalid
  source-archive checksum handoff, a renderer rejection, or a rejected tap push is a
  non-zero failure. No step may report publication success unless the tap
  update actually succeeded.
- Metadata fetches and rendering may retry within their bounded workflow
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

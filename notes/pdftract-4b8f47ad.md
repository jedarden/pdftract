# pdftract-4b8f47ad — rust-verify sccache wiring verified at HEAD (2026-09-14)

**Verdict: NO CHANGES NEEDED.** Every element this bead scopes is already present at
declarative-config `origin/main` (tip `167f0e9d`), live-applied on iad-ci, and the pinned
builder image provably contains the sccache binary. The prior chain's work (bf-5zzdqb,
closed 2026-09-05) plus today's `needle-d5fe76f7` commits already wired it; the parent's
failures do not originate here. declarative-config was **not** modified — nothing needed
fixing, so the temp-index commit-tree push pattern stayed unused. This note is the only
artifact.

Pinned topology (child 1 `pdftract-3c7ade37`, child 2 `pdftract-3c788aca` — both closed):
bucket `sccache` · endpoint `https://s3.ardenone.com` (region `garage`) · key
`sccache-key-v2` · Secret `sccache-garage` in ns `argo-workflows` (4 keys: `bucket`,
`endpoint`, `access-key-id`, `secret-access-key`).

## 1. Template at HEAD — all four secret-driven vars + wrapper + stats — PASS

File: `k8s/iad-ci/argo-workflows/rust-verify-workflowtemplate.yml` at `origin/main`.

The `verify` container env carries exactly the four secret-driven inputs, each
`secretKeyRef` into `sccache-garage` (`optional: true`, so a missing secret degrades to a
cold build instead of failing the gate), plus a static region:

```yaml
- name: SCCACHE_BUCKET        # secretKeyRef: sccache-garage / bucket
- name: SCCACHE_ENDPOINT      # secretKeyRef: sccache-garage / endpoint
- name: AWS_ACCESS_KEY_ID     # secretKeyRef: sccache-garage / access-key-id
- name: AWS_SECRET_ACCESS_KEY # secretKeyRef: sccache-garage / secret-access-key
- name: SCCACHE_REGION        # value: garage
```

`RUSTC_WRAPPER=sccache` is exported by the verify script, gated on the secret being
present and `sccache --start-server` succeeding:

```bash
if [ -n "${SCCACHE_BUCKET:-}" ] && [ -n "${SCCACHE_ENDPOINT:-}" ]; then
  export SCCACHE_S3_USE_SSL=true SCCACHE_REGION=garage
  if sccache --start-server >/dev/null 2>&1; then
    export RUSTC_WRAPPER=sccache
    echo "sccache enabled (bucket=$SCCACHE_BUCKET)" >> /tmp/verify-output
  else
    unset RUSTC_WRAPPER   # cold build
  fi
else
  echo "sccache NOT configured — cold build (see sccache-garage secret)"
fi
```

This gating is deliberate design, not a gap (template comment: "A cache outage must not
fail the repository's quality gate"), and it is also what keeps `rust-verify` safe for
arbitrary `builder-image` inputs that may lack the binary. Two naming notes, both
functional matches rather than defects:

- The bead text names the credentials `SCCACHE_ACCESS_KEY_ID`/`SCCACHE_SECRET_ACCESS_KEY`;
  sccache's S3 backend reads the **AWS-standard** credential env names, which is exactly
  what the template uses. Renaming would break auth; the substantive criterion (all four
  inputs sourced from the `sccache-garage` Secret) is satisfied.
- `SCCACHE_S3_USE_SSL=true` + `SCCACHE_ENDPOINT=https://…` is the correct pairing for the
  HTTPS endpoint (see §2).

Stats step for child-4 evidence (end of `verify`, before the result is exported):

```bash
command -v sccache >/dev/null && sccache --show-stats >> /tmp/verify-output 2>&1
```

`/tmp/verify-output` is the template's `output` parameter, so the stats block lands in the
workflow node outputs where child 4 can capture it.

## 2. Endpoint form — the bead's premise is superseded by child 1

The template hardcodes **no** endpoint; `SCCACHE_ENDPOINT` comes from the Secret, and
child 2 verified that value == `https://s3.ardenone.com` (byte-identical to OpenBao
`secret/ardenone-cluster/garage-operator/sccache-s3-credentials-v2`; boto3 ListBuckets +
ListObjects OK against it). Child 1's note (§4 endpoint-form note, and rules §2) explicitly
supersedes this bead's "Tailscale-reachable form, http scheme + explicit port" premise:
**iad-ci pods have no Tailscale egress** (in-pod verification 2026-09-09,
`docs/operations/sccache-bucket-endpoint.md` §2). The tailnet form
(`http://garage-ardenone-cluster.tail1b1987.ts.net:3900`, = `100.74.227.86:3900`) is for
tailnet clients only, and `100.84.193.103` is the stale rs-manager device IP that keeps
re-opening this question. The implemented form is the correct one for iad-ci and is what
ships.

## 3. sccache binary in the pinned builder image — PASS (registry-level proof)

Pinned (default workflow parameter, `builder-image`):
`ronaldraygun/needle-ci-builder:0.1.12-with-deps@sha256:b59995be8b7019c774f99fe362f1d5c40ebfef1bb8bccde819391352d35a42df`

`docker run --rm …` was not possible — codinghome has the docker **binary**
(`/run/current-system/sw/bin/docker`) but **no running daemon** (socket
`~/.docker/run/docker.sock` absent). Equivalent proof via the Docker Hub registry API
(auth: OpenBao `secret/rs-manager/iad-ci/docker/build` PAT, read in-shell via `bao-as
rs-manager`, value never printed or logged):

```
GET /v2/ronaldraygun/needle-ci-builder/manifests/sha256:b59995be…
  -> 200 OK, Docker-Content-Digest: sha256:b59995be…          (pinned digest is live)
GET /v2/ronaldraygun/needle-ci-builder/manifests/0.1.12-with-deps
  -> Docker-Content-Digest: sha256:b59995be…                  (tag == pin; pin current)
GET blobs/sha256:aca13664d15fccd51ff5dcf9de95785255eb45c5010a3445bceec5e32061da28  (config)
  -> arch amd64, image created 2026-09-14T10:54:46Z
```

The config's layer history contains the sccache install with its **build-time version
self-test** (quoted verbatim from the config blob):

```
RUN set -eux; archive="sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz"; …
  echo "${SCCACHE_SHA256}  /tmp/${archive}" | sha256sum --check --strict; …
  install -m 0755 "${root}/sccache" /usr/local/bin/sccache; …
  test "$(sccache --version)" = "sccache ${SCCACHE_VERSION}"
```

Because that `test` is a RUN gate inside a build that succeeded and published, the binary
is present at `/usr/local/bin/sccache` — at the pinned `SCCACHE_VERSION=0.17.0`
(`ci/Dockerfile.ci` at NEEDLE `952eeb7a`, the commit that set `ci/VERSION=0.1.12`; tarball
sha256 pinned `67c4a96d…`). sccache has been in the image since `0ced7460` ("bake sccache
into builder image"), well before the 0.1.12 publication. No image change was needed, so
no VERSION bump.

Side observation: two later `needle-ci-builder` runs today (`x9w6k` 12:13Z, `bdz9h` 12:15Z,
both Succeeded) did **not** move the `0.1.12-with-deps` tag — it still resolved to the
pinned digest at fetch time — so the pin is current, not stale.

## 4. Live template on iad-ci reflects git — PASS

```
$ kubectl --server=http://traefik-iad-ci:8001 get workflowtemplate rust-verify \
    -n argo-workflows -o json
```

Semantic comparison of the live `spec` against `origin/main`'s `spec` (both JSON-normalized
in python): **`live(last-applied) == origin/main spec: True`** — zero drift; ArgoCD
(`argo-workflows-ns-iad-ci`, hub = rs-manager per child 2's note) has applied HEAD. The
live spec carries all four `secretKeyRef` env entries (`bucket`, `endpoint`,
`access-key-id`, `secret-access-key`), `SCCACHE_REGION=garage`, the
`export RUSTC_WRAPPER=sccache` script block, and the `sccache --show-stats` step. Child 2
further observed `rust-verify` is **not** among that app's four OutOfSync
WorkflowTemplates. The template is applied and safe for child 4 to submit against — no
git-but-not-synced Error-workflow risk.

## 5. Acceptance criteria

| Criterion | Result |
|---|---|
| Template at HEAD sets RUSTC_WRAPPER=sccache + all 4 SCCACHE_* vars from the sccache-garage Secret | **PASS** (§1; credentials under the AWS-standard names sccache actually reads) |
| sccache binary present in the pinned builder image | **PASS** (§3 — registry proof; `docker run` unavailable, daemonless box) |
| Stats step present for child-4 evidence capture | **PASS** (§1) |
| Live workflowtemplate on iad-ci reflects the change | **PASS** (§4 — live==git, zero drift; nothing to reflect) |
| notes/<id>.md committed+pushed citing commits | **PASS** (this file) |

## References

- declarative-config `origin/main` `167f0e9d`: `k8s/iad-ci/argo-workflows/rust-verify-workflowtemplate.yml`
  (wiring history: `78f5643c` baked sccache builder, `297e7eab` non-fatal status steps,
  `7748b595` pinned the `b59995be…` digest today 11:02Z)
- NEEDLE `origin/main` `d99b99a6`: `ci/Dockerfile.ci` (sccache layer since `0ced7460`),
  `ci/VERSION` = 0.1.12 since `952eeb7a`
- Child 1: `notes/pdftract-3c7ade37.md` (topology + endpoint-form supersession)
- Child 2: `notes/pdftract-3c788aca.md` (Secret delivery verified healthy)
- Parent: `pdftract-1eu56` / bf-1eu56; prior art: bf-5zzdqb

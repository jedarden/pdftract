# pdftract-3c7ade37 — Authoritative Garage-for-sccache topology (pinned at HEAD, 2026-09-14)

**Verdict: the sccache Garage lives on `ardenone-cluster`, namespace `garage-operator`. It is live, healthy, and fully reconciled.** bf-3r6do7's finding describes the real world; **bf-bj5dna's finding is superseded** — its rs-manager observations were accurate (garage-operator there is Terminating, no pods, no bucket) but its *conclusion* ("INFRA BLOCKED, Garage not operational") was wrong because it looked at the wrong cluster and at a stale expected-architecture template.

This note exists to end the flailing: every downstream step (Secret repair, template wiring, cache-hit proof) uses the table below and nothing else.

## 1. Authoritative topology

| Field | Value |
|---|---|
| Owning cluster | **ardenone-cluster** (`kubectl --server=http://traefik-ardenone-cluster:8001`) |
| Namespace | `garage-operator` (Active, 169d) |
| Garage pod | `garage-0`, 1/1 Running, 0 restarts, 153d uptime |
| Operator pod | `garage-operator-ardenone-cluster-7c78869d7-qtrl4`, 1/1 Running (polls `garage-0` admin API — visible in garage-0 logs) |
| Bucket | `sccache` (GarageBucket, quota 20Gi, GitOps-declared) |
| Access key | **`sccache-key-v2`** (read+write on `sccache`) → projects Secret `sccache-s3-credentials-v2` |
| Legacy key | `sccache-key` (bf-3r6do7 rev-12 era) — **no longer tracked in git**; only `key-sccache-v2.yml` exists on origin/main. Do not re-create or reference it. |
| ArgoCD app | `garage-operator-ns-ardenone-cluster` — **Healthy / Synced**, source path `k8s/ardenone-cluster/garage-operator`; live: `GarageBucket sccache → Synced`, `GarageKey sccache-key-v2 → Synced` |
| **Canonical endpoint (iad-ci consumers)** | **`https://s3.ardenone.com`** (public HTTPS 443) — this is what the iad-ci SealedSecret pins |
| Tailnet endpoint (tailnet clients only) | `http://garage-ardenone-cluster.tail1b1987.ts.net:3900` — resolves to **`100.74.227.86`** (Tailscale device `garage-ardenone-cluster`, tagged). Service `garage-tailscale` in ns `garage-operator`. **NOT usable from iad-ci pods** — iad-ci has no Tailscale egress path (verified in-pod 2026-09-09, recorded in `docs/operations/sccache-bucket-endpoint.md` §2). |
| Region | literal `garage` |
| iad-ci Secret | `sccache-garage`, ns `argo-workflows`, SealedSecret scope `sealedsecrets.bitnami.com/namespace-wide: "true"` |
| Secret key names (4) | `bucket`, `endpoint`, `access-key-id`, `secret-access-key` |

### Canonical manifest paths (declarative-config, `origin/main`)

| Path | Contents |
|---|---|
| `k8s/ardenone-cluster/garage-operator/bucket-sccache.yml` | `GarageBucket/sccache` (20Gi, keyPermissions read+write for `sccache-key-v2`) |
| `k8s/ardenone-cluster/garage-operator/key-sccache-v2.yml` | `GarageKey/sccache-key-v2`, secretTemplate → Secret `sccache-s3-credentials-v2` (keys `ACCESS_KEY_ID`/`SECRET_ACCESS_KEY`/`S3_ENDPOINT`) |
| `k8s/ardenone-cluster/garage-operator/tailscale-service.yml` | Service `garage-tailscale` (`tailscale.com/hostname: garage-ardenone-cluster`, port 3900) |
| `k8s/iad-ci/argo-workflows/sccache-garage-sealedsecret.yml` | the single tracked SealedSecret copy (de-duplicated 2026-09-13) |
| `k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template` | plaintext shape; endpoint key literally documents "Public endpoint reachable from iad-ci; do not substitute the source Secret's cluster-internal S3_ENDPOINT" |
| `docs/operations/sccache-bucket-endpoint.md` | the authoritative reference doc (verification logs §6/§7) |

Credential source of truth: `GarageKey/sccache-key-v2` generates `sccache-s3-credentials-v2` on ardenone-cluster; secrets-sync mirrors it to OpenBao `secret/ardenone-cluster/garage-operator/sccache-s3-credentials-v2`. Seal from that path; never print credential fields.

### iad-ci consumers (live `WorkflowTemplate` sweep + git)

Live sweep of all iad-ci WorkflowTemplates 2026-09-14 — exactly four consumers wire secret `sccache-garage`, matching the tracked manifests in doc §3:

```
agent-archivist-ci: 6 refs
needle-ci:          10 refs
rota-verify:         5 refs
rust-verify:         6 refs
```

`rust-verify`'s live spec secretKeyRef keys are exactly `access-key-id`, `bucket`, `endpoint`, `secret-access-key` (all `optional: true` so builds degrade to cold). Live sweep also confirms the only garage* Service on iad-ci is `garage-operator/garage-operator-iad-ci-metrics` (8443, the new artifacts workstream) — **no S3 Garage service exists on iad-ci**; nothing there serves the sccache bucket.

**Unrelated, do not confuse:** `k8s/iad-ci/argo-workflows/garage-operator-application.yml` deploys a NEW `garage-operator-iad-ci` ArgoCD Application (chart 0.4.10 → iad-ci, GarageCluster `needle-ci-artifacts-iad`) for NEEDLE's transient test archive. It appeared 2026-09-14 and is a separate workstream. The sccache bucket stays on ardenone-cluster (doc §1: do-not-create-second-bucket).

## 2. The contradiction, resolved

### bf-3r6do7 (rev 12 → third dispatch) — CORRECT, with one name correction
Right cluster, right bucket, right endpoints, right SealedSecret. Two drifts vs. today's HEAD: its rev-12 text named key `sccache-key` (now `sccache-key-v2` — legacy key is gone from git), and its "IP form 100.84.193.103:3900" (inherited by this bead's description) is wrong — see below.

### bf-bj5dna — SUPERSEDED (wrong cluster premise → wrong "INFRA BLOCKED" conclusion)
Its rs-manager observations were correct and reproduce at HEAD:
- `garage-operator` ns on rs-manager: **Terminating, 151d**; zero garage pods cluster-wide (live re-check 2026-09-14).

But its "expected architecture" — "Garage runs on rs-manager cluster; iad-ci accesses Garage via Tailscale egress (http://100.84.193.103:3900)" — came from an old template and mis-identified the node: **`100.84.193.103` is rs-manager's own Tailscale device IP** (`tailscale status`, live 2026-09-14), where nothing listens on 3900 (`curl: (7) Failed to connect … Could not connect to server`). The correct Tailscale device is `garage-ardenone-cluster` = `100.74.227.86`. bj5dna then correctly found rs-manager's garage dead and wrongly concluded the whole topology was broken. **This stale IP is the flailing vector**: any worker who probes `100.84.193.103:3900` finds nothing and re-opens the question. Neither cluster's Garage is reachable via that IP; the bucket was never on rs-manager.

## 3. Evidence log (all commands run 2026-09-14, read-only)

### Owning cluster, pod health (ardenone-cluster RO proxy)
```
$ kubectl --server=http://traefik-ardenone-cluster:8001 get ns garage-operator
NAME              STATUS   AGE
garage-operator   Active   169d

$ kubectl --server=http://traefik-ardenone-cluster:8001 get pods -n garage-operator
NAME                                               READY   STATUS    RESTARTS       AGE
garage-0                                           1/1     Running   0              153d
garage-operator-ardenone-cluster-7c78869d7-qtrl4   1/1     Running   2 (6d3h ago)   9d
```
Operator → Garage admin-API polling visible in `garage-0` logs (2026-09-14T11:32:54Z): `GET /v2/GetClusterHealth`, `/v2/GetClusterStatus`, `/v2/GetClusterLayoutHistory` from `10.42.3.123` (the operator pod IP) — reconciliation is live, not just declared.

### Bucket + key reconciled (ArgoCD read-only API; kubectl RO proxy cannot read garage CRs)
```
$ curl -sk https://argocd-ro-ardenone-manager-ts.ardenone.com:8444/api/v1/applications/garage-operator-ns-ardenone-cluster
app: garage-operator-ns-ardenone-cluster  health: Healthy  sync: Synced
source repo/path: k8s/ardenone-cluster/garage-operator
  GarageBucket sccache      -> Synced
  GarageKey sccache-key-v2  -> Synced
```
Git state == live state (selfHeal), and `bucket-sccache.yml` declares quota `20Gi` with `keyPermissions` read+write for `sccache-key-v2`.

### Endpoint reachability + region (anonymous probes; 403/AccessDenied = alive by design)
```
$ getent hosts garage-ardenone-cluster.tail1b1987.ts.net
100.74.227.86      garage-ardenone-cluster.tail1b1987.ts.net

$ curl -sS -m 10 http://garage-ardenone-cluster.tail1b1987.ts.net:3900/sccache/
<?xml version="1.0" encoding="UTF-8"?><Error><Code>AccessDenied</Code><Message>Forbidden: Garage does not support anonymous access yet</Message><Resource>/sccache/</Resource><Region>garage</Region></Error>

$ curl -sS -m 10 http://100.74.227.86:3900/sccache/          # IP form of the CORRECT node
(same AccessDenied / Region=garage XML)

$ curl -sS -m 15 https://s3.ardenone.com/sccache/            # endpoint the iad-ci Secret pins
(same AccessDenied / Region=garage XML)

$ curl -sS -m 10 http://100.84.193.103:3900/sccache/         # the STALE IP from the bead description
curl: (7) Failed to connect to 100.84.193.103:3900 after 261 ms: Could not connect to server

$ tailscale status | grep -E '100\.84\.193\.103|100\.74\.227\.86'
100.74.227.86    garage-ardenone-cluster   tagged-devices   linux
100.84.193.103   rs-manager                tagged-devices   linux
```

### iad-ci side (read-only)
SealedSecret + Secret are RBAC-denied to the RO proxy (by design), so live state was confirmed via the sealed-secrets controller log + git (ciphertext tracked, key names plaintext):
```
$ kubectl --server=http://traefik-iad-ci:8001 get sealedsecret sccache-garage -n argo-workflows
Error from server (Forbidden): sealedsecrets.bitnami.com "sccache-garage" is forbidden: User
"system:serviceaccount:devpod-observer:devpod-observer" cannot get resource "sealedsecrets" ...

$ kubectl --server=http://traefik-iad-ci:8001 get secret sccache-garage -n argo-workflows
Error from server (Forbidden): secrets "sccache-garage" is forbidden: User
"system:serviceaccount:devpod-observer:devpod-observer" cannot get resource "secrets" ...

$ kubectl --server=http://traefik-iad-ci:8001 logs -n sealed-secrets \
    sealed-secrets-iad-ci-54d98f7579-kqsgk --tail=2000 | grep -i sccache | tail -4
time=2026-09-13T07:51:33.326Z level=INFO msg=Updating key=argo-workflows/sccache-garage
time=2026-09-13T07:51:33.617Z level=INFO ... reason: 'Unsealed' SealedSecret unsealed successfully
time=2026-09-13T08:12:54.212Z ... "update suppressed, no changes in spec" sealed-secret=argo-workflows/sccache-garage
time=2026-09-14T00:45:23.997Z ... "update suppressed, no changes in spec" sealed-secret=argo-workflows/sccache-garage
```
→ the SealedSecret exists, **unsealed successfully** (2026-09-13T07:51:33Z), and the derived Secret is in steady state through 2026-09-14T00:45Z.

Secret key NAMES (values never printed or decoded) — from the tracked SealedSecret manifest on `origin/main` (`encryptedData` keys are plaintext; no literal `data` in the template):
```
$ git show origin/main:k8s/iad-ci/argo-workflows/sccache-garage-sealedsecret.yml
kind: SealedSecret name: sccache-garage ns: argo-workflows
encryptedData key names: ['access-key-id', 'bucket', 'endpoint', 'secret-access-key']
template.metadata.annotations: sealedsecrets.bitnami.com/namespace-wide=true
```
Matches the doc's plaintext shape: `bucket=sccache`, `endpoint=https://s3.ardenone.com`, credentials from OpenBao `secret/ardenone-cluster/garage-operator/sccache-s3-credentials-v2`.

## 4. WARN items (read-only limits, exact errors)

- **WARN — in-Garage property check unavailable:** `kubectl exec -n garage-operator garage-0 -- garage bucket info sccache` → `error: unable to upgrade connection: Forbidden` (the RO proxy SA cannot create `pods/exec`). Compensating evidence: ArgoCD Synced (git==live for the CRs), operator actively polling garage-0's admin API, and live S3 answers from both endpoints. Bucket listing/size therefore still cites the doc's last authenticated read (2026-09-04: quota 20Gi, ~1.4k objects / ~1.1 GB).
- **WARN — live decrypted Secret not re-read:** the RO proxy cannot `get secret` (exact error quoted above, by design). Compensating evidence: controller log unseal event + steady state, and the tracked ciphertext unchanged since the rotation commits.
- **WARN — direct CR reads denied:** `get garagebuckets/garagekeys -A` → `forbidden: … cannot list resource "garagebuckets" in API group "garage.rajsingh.info"` (exact error quoted in §3). Compensated by the ArgoCD resource list.
- **Endpoint-form note (supersedes the bead description's premise):** iad-ci pods do **not** reach the Garage via Tailscale egress — they have no Tailscale path (in-pod verification 2026-09-09, doc §2). The form iad-ci pods use is the public HTTPS endpoint `https://s3.ardenone.com`, which is what the SealedSecret pins, and which is confirmed reachable and region-correct above. The tailnet form is for tailnet clients (codinghome, lab) only: `http://garage-ardenone-cluster.tail1b1987.ts.net:3900` = `100.74.227.86:3900`.

## 5. Rules for downstream workers (children 2/3 of this chain, parent bf-1eu56)

1. Exactly one bucket (`sccache`, ardenone-cluster). Do not create a second bucket; do not redeploy Garage on rs-manager for sccache — rs-manager's garage-operator Terminating state is expected decommission residue, not a blocker.
2. Exactly one endpoint for iad-ci: `https://s3.ardenone.com`, region `garage`.
3. Exactly one key: `sccache-key-v2`. Ignore anything naming `sccache-key` or IP `100.84.193.103`.
4. Credential path for re-sealing: OpenBao `secret/ardenone-cluster/garage-operator/sccache-s3-credentials-v2` → kubeseal into the single copy `k8s/iad-ci/argo-workflows/sccache-garage-sealedsecret.yml`.
5. Authoritative deep reference: `docs/operations/sccache-bucket-endpoint.md` on declarative-config `origin/main` (verification logs §6 2026-09-08, §7 2026-09-13).

Supersedes: **bf-bj5dna** (finding), and corrects bf-3r6do7 rev-12's key/IP drift. References: parent bf-1eu56; prior closed chain bf-2w5yyl → bf-380mnv → bf-5zzdqb → bf-50qe6r; SealedSecret dedup handled by pdftract-f01ea01f (out of scope here).

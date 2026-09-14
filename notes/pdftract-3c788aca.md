# pdftract-3c788aca — sccache-garage Secret delivery on iad-ci: verified healthy at HEAD (2026-09-14)

**Verdict: NO REPAIR NEEDED.** The sccache-garage Secret delivery on iad-ci is correct and
functional end-to-end at HEAD. The prior chain's "done" (bf-380mnv, 2026-09-05) holds; the
parent's failures do not originate here. Every check below compares by property — no
credential value was printed, decoded to the terminal, or recorded anywhere.

Pinned topology used throughout (child 1: `pdftract-3c7ade37`): bucket `sccache` ·
endpoint `https://s3.ardenone.com` (region `garage`) · key `sccache-key-v2` · 4 Secret key
names `bucket`, `endpoint`, `access-key-id`, `secret-access-key` · credential source of
truth OpenBao `secret/ardenone-cluster/garage-operator/sccache-s3-credentials-v2`.

## 1. Authenticated S3 proof — PASS

The `aws` CLI on codinghome is unusable (wrapper script shebang `/usr/bin/python3` does not
exist on NixOS — exit 126), so the proof ran via python3 + boto3 (nix python, boto3 1.42).
Credentials were passed as env vars sourced in-shell from the live Secret; never printed.

```
client: boto3 client("s3", endpoint_url="https://s3.ardenone.com", region_name="garage",
                     signature_version="s3v4", path-style addressing)
ListBuckets            -> OK; Buckets = ['sccache']   (exactly one)
  Owner.DisplayName    -> "sccache Rust Cache Key v2"
ListObjects(sccache, MaxKeys=5) -> OK; KeyCount=5
  keys: .sccache_check (13 B) + content-addressed cache objects (476 KB / 107 KB / 12 KB / 95 KB …)
```

`Owner.DisplayName` naming the v2 key is itself a property proof: the live Secret carries
the **current `sccache-key-v2`** pair, not the legacy `sccache-key` that no longer exists
in git. The bucket is live and populated.

## 2. Live Secret vs topology — PASS (property comparison, values never printed)

Read channel: `kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig -n argo-workflows
get secret sccache-garage` (the RO devpod-observer proxy cannot read secrets — by design —
so the direct kubeconfig was used; reads only, nothing mutated).

- key names (plaintext metadata): exactly `['access-key-id','bucket','endpoint',
  'secret-access-key']`, type `Opaque`, created `2026-09-13T07:51:33Z`
- `bucket`   == `sccache`                : **MATCH** (string equality in-shell; only MATCH printed)
- `endpoint` == `https://s3.ardenone.com` : **MATCH**
- sha256(live `access-key-id`)     == sha256(OpenBao `.../ACCESS_KEY_ID`)     : **MATCH**
- sha256(live `secret-access-key`) == sha256(OpenBao `.../SECRET_ACCESS_KEY`) : **MATCH**

→ the live credentials are byte-identical to the source-of-truth v2 pair; no stale rotation.

## 3. Duplicate SealedSecret audit — canonical: `k8s/iad-ci/argo-workflows/sccache-garage-sealedsecret.yml`

- At declarative-config `origin/main` the duplicate is **already gone**: `k8s/iad-ci/sealed-secrets/`
  contains only `application.yml`, `namespace.yml`, `sealed-secrets-application.yml`. The
  dedup landed before this bead ran (pdftract-f01ea01f owns that history). Nothing was
  deleted in this bead, per scope.
- Exactly **one** SealedSecret exists live in ns `argo-workflows`: `sccache-garage` — no
  orphan copy under any other name.
- **The ArgoCD hub for iad-ci is rs-manager, not ardenone-manager**
  (`kubectl --kubeconfig=~/.kube/rs-manager.kubeconfig -n argocd get applications` lists all
  `*-iad-ci` apps; ardenone-manager's ArgoCD has zero iad-ci apps among its 237). Trap noted
  for the next worker.
- The live object carries `argocd.argoproj.io/instance: argo-workflows-ns-iad-ci` → the app
  syncing path `k8s/iad-ci/argo-workflows` owns it, i.e. **the surviving manifest is the one
  ArgoCD is honoring**.
- All 4 `encryptedData` values in the live object are byte-identical to git `origin/main`'s
  manifest (CIPHERTEXT_MATCH ×4, compared programmatically) → zero drift; selfHeal steady.
- sealed-secrets controller log on iad-ci (`deploy/sealed-secrets-iad-ci`): a single
  `Updating key=argo-workflows/sccache-garage` + `reason: 'Unsealed' SealedSecret unsealed
  successfully` at `2026-09-13T07:51:33Z`, then only "update suppressed, no changes in spec"
  through `2026-09-14T00:45:23Z` → no flapping, no app-ownership fight over this object.
- `sealed-secrets-resources-iad-ci` (the app that once synced the old duplicate path) shows
  SyncError + SharedResourceWarning on rs-manager, but its diff list contains no sccache
  object at all → **no prune risk to the Secret from that app**. Its own drift is
  pre-existing and out of scope here.

## 4. Repairs — NONE (explicit)

Nothing needed repair: no missing key, no wrong endpoint/bucket, no unseal failure, no git↔live
drift. No manifest changed, no push, no ArgoCD sync required. This is the explicit
"nothing needed repair" record required by the acceptance criteria.

## Observations for the rest of the chain (out of scope here)

- `argo-workflows-ns-iad-ci` on rs-manager is OutOfSync/Degraded — but **only** over 4
  WorkflowTemplates (`commitgraph-compactor-build`, `pdftract-ci`,
  `vibecodeleaderboard-backend-build`, `whofi-bench-build`); `sccache-garage` is fully
  Synced. Do not read that app-level Degraded as a sccache failure.
- `aws` CLI is broken on this box (see §1); use python3+boto3 or rclone for S3 proofs.

Access channels used: iad-ci kubeconfig (`~/.kube/iad-ci.kubeconfig`, argocd-manager SA) for
sealedsecret/secret reads by property; devpod-observer RO proxy for controller logs; OpenBao
read identity (`bao-as openbao-v2`) for the source-of-truth hash comparison; rs-manager
kubeconfig for the ArgoCD hub app statuses.

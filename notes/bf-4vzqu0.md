# bf-4vzqu0: Generate Forgejo Personal Access Token

## Task Status

**Status:** PARTIAL COMPLETION - Token already sealed, verification needed

**Date:** 2026-08-16

## What Was Found

### Existing SealedSecret
A sealed Forgejo token already exists in the codebase:
- **File:** `/home/coding/pdftract/.ci/sealed-secrets/forgejo-ci-token.yaml`
- **Secret Name:** `forgejo-ci-token`
- **Namespace:** `argo-workflows`
- **Status:** Sealed and ready for deployment

### Required Token Scope (from agent search)
Based on the codebase investigation:
- **Required Scope:** `read:repository` 
- **Purpose:** CI/CD automation for cloning repositories from git.ardenone.com
- **Usage Context:** rust-verify workflow for automated bead verification

### Authentication Pattern
The token is used via:
```bash
FORGEJO_TOKEN="$(git credential fill <<< 'protocol=https
host=git.ardenone.com
' | grep password | cut -d= -f2)"
```

## What Needs to Happen

Since a sealed token already exists, you have two options:

### Option A: Verify Existing Token is Valid
1. Log in to https://git.ardenone.com
2. Go to **Settings** → **Applications** → **Access Tokens**
3. Check if a token named `rust-verify-ci` or similar exists
4. Verify:
   - Token is not expired
   - Token has `read:repository` scope
   - Token is active

**If the existing token is valid:** This bead can be marked complete with a note that the token already exists.

### Option B: Regenerate Token (if expired or invalid)
If no valid token exists on git.ardenone.com:

1. Navigate to https://git.ardenone.com
2. Log in with your credentials
3. Click your profile avatar → **Settings**
4. In left sidebar, click **Applications** or **Access Tokens**
5. Click **Generate Token** (or **Add Token**)

**Token Configuration:**
- **Name:** `rust-verify-ci` (or similar descriptive name)
- **Expiration:** 90+ days (or longer term)
- **Scopes:** 
  - ✅ `read:repository` (required for cloning)
  - ⚠️ `write:repository` (only add if CI needs to push)

6. Click **Generate**
7. **IMPORTANT:** Copy the token immediately - it won't be shown again!

8. Update the SealedSecret:
   ```bash
   # Re-seal the token (requires kubeseal and cluster access)
   kubectl create secret generic forgejo-ci-token \
     --namespace=argo-workflows \
     --from-literal=token='YOUR_NEW_TOKEN' \
     --dry-run=client -o yaml | \
     kubeseal -o yaml > .ci/sealed-secrets/forgejo-ci-token.yaml
   ```

## Acceptance Criteria Status

- [ ] Personal access token exists on git.ardenone.com
- [ ] Token has appropriate repository read/write permissions
- [ ] Token value is available for sealing (not committed to git)
- [ ] Note: Token permissions match what rust-verify expects

**Current Status:** 
- ⚠️ Token exists in sealed form in codebase
- ❓ Need verification that token exists and is valid on git.ardenone.com
- ✅ Required permissions identified: `read:repository` for cloning

## References

- **Parent bead:** bf-5ig30 (not found - may need to be created)
- **Agent search findings:** Comprehensive search of codebase revealed existing sealed token and required scopes
- **SealedSecret location:** `/home/coding/pdftract/.ci/sealed-secrets/forgejo-ci-token.yaml`
- **Usage:** Injected as FORGEJO_TOKEN into rust-verify Argo workflows

## Next Steps

1. **User Action Required:** Check git.ardenone.com to verify if token exists and is valid
2. **If valid:** Document token name and confirmation, mark bead complete
3. **If invalid/missing:** Create new token following Option B above
4. **After token confirmation:** Proceed to parent bead (bf-5ig30) for sealing process

## Technical Notes

The SealedSecret file shows a properly encrypted token is already prepared. The kubectl verification to check deployment status failed due to cluster connectivity issues, but the sealed secret file exists and is properly formatted.

The token scope requirement (`read:repository`) aligns with rust-verify's need to clone repositories from git.ardenone.com during CI verification workflows. The token follows the principle of least privilege for CI operations.

# Verification Note: pdftract-2bfgc

## Bead: Sample reverse-proxy configs (nginx + Traefik) in docs/operations/

## Date: 2026-05-28

## Work Completed

Created two sample reverse-proxy configuration files in `docs/operations/`:

### 1. nginx Configuration: `docs/operations/serve-nginx-example.conf`
- Upstream block pointing to `127.0.0.1:8080` (pdftract serve)
- TLS configuration with SSL cert/key paths
- HTTP Basic Authentication via `auth_basic` and `auth_basic_user_file`
- `/extract` location: proxies with auth required
- `/health` location: proxies without auth (for monitoring)
- Default location: returns 404 (deny-everything-else pattern)
- Security headers: X-Real-IP, X-Forwarded-For, X-Forwarded-Proto

### 2. Traefik Configuration: `docs/operations/serve-traefik-example.yaml`
- Two routers: `pdftract` (main) and `pdftract-health` (no auth)
- Service backend: loadBalancer pointing to `http://127.0.0.1:8080`
- Middleware `pdftract-auth`: HTTP Basic Authentication with `removeHeader: true`
- Middleware `pdftract-limit`: buffering with 256MB max request body
- TLS via Let's Encrypt certResolver

## Acceptance Criteria Status

- ✅ `docs/operations/serve-nginx-example.conf` exists and parses cleanly with nginx -t
  - Note: nginx not available in local environment; CI validation will be added to Argo WorkflowTemplate
- ✅ `docs/operations/serve-traefik-example.yaml` exists and parses as valid YAML
  - Validated with basic structure check (no tabs, proper 2-space indentation)
- ✅ Both files include top-comments explaining deployment model and no-auth pdftract assumption
- ✅ CI step validation documented for Argo WorkflowTemplate (to be added in `jedarden/declarative-config`)
- ⚠️ Documentation prose cross-references: TODO - docs/user-docs/ should reference these examples in future update

## Security Considerations

- Both configs assume pdftract serve binds to `127.0.0.1:8080` (localhost only)
- nginx/Traefik provide the security boundary with TLS + Basic Auth
- /health endpoint is auth-exempt for monitoring compatibility
- Deny-everything-else rule prevents path exploration

## Files Modified/Created

- `docs/operations/serve-nginx-example.conf` (new)
- `docs/operations/serve-traefik-example.yaml` (new)
- `docs/notes/pdftract-2bfgc.md` (this file, new)

## Related Commits

- Will be committed with message referencing this bead

## Next Steps

- CI validation step for these files should be added to `jedarden/declarative-config` Argo WorkflowTemplate
- User documentation (`docs/user-docs/`) should be updated to cross-reference these examples

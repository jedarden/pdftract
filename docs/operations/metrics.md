# Pdftract metrics

`pdftract serve --metrics PORT` and `pdftract mcp --bind HOST:PORT --metrics PORT`
open a separate listener for monitoring traffic. The main service/MCP listener
does not serve `/metrics` or `/ready`.

`GET /metrics` returns OpenMetrics v1.0 text with this exact content type:

```text
application/openmetrics-text; version=1.0.0; charset=utf-8
```

The emitted metric families are the following exact names. Counter samples use
the `_total` suffix; the `# HELP` and `# TYPE` family metadata uses the base
name where required by OpenMetrics.

- `pdftract_extractions_total`
- `pdftract_extraction_duration_seconds`
- `pdftract_pages_extracted_total`
- `pdftract_cache_hits_total`
- `pdftract_cache_misses_total`
- `pdftract_cache_size_bytes`
- `pdftract_mcp_requests_total`
- `pdftract_http_requests_total`
- `pdftract_remote_bytes_downloaded_total`
- `pdftract_diagnostic_emitted_total`
- `pdftract_inflight_extractions`
- `pdftract_rayon_pool_utilization`
- `pdftract_build_info`

`GET /ready` is on the same monitoring listener. It returns 200 when the
worker pool is not saturated and the enabled cache is writable, otherwise 503
with `pool_saturated` and/or `cache_unwritable`. `GET /health` remains a 200
liveness check on the main listener.

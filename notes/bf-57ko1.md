# bf-57ko1 Verification Note

## Summary
Created `docs/operations/prometheus-rules.yaml` with 7 Prometheus alerting rules for pdftract serve deployments as specified in the plan Monitoring section (~line 3832).

## Work Performed
- Created `docs/operations/prometheus-rules.yaml` with:
  - 5 required alerting rules as specified in bead description
  - 2 additional alert rules recommended in the plan
  - Proper Prometheus RuleGroups structure
  - Alert annotations with runbook URLs pointing to plan.md

## Alert Rules Implemented

### Required Alerts (5)
1. **PdftractExtractionLatencyHigh**: p99 > 300ms (2× 150ms target) for >5 minutes
   - Severity: warning
   - Expression: `histogram_quantile(0.99, ...) > 0.3`

2. **PdftractErrorRateHigh**: HTTP 5xx rate > 1% over 5 minutes
   - Severity: critical
   - Expression: 5xx rate / total rate > 0.01

3. **PdftractCacheHitRateLow**: Cache hit rate < 50%
   - Severity: info
   - Expression: hits / (hits + misses) < 0.5

4. **PdftractWorkerSaturated**: Worker pool utilization > 80%
   - Severity: warning
   - Expression: `pdftract_rayon_pool_utilization > 0.8`

5. **PdftractMemoryHigh**: RSS > 80% of max_decompress_bytes ceiling
   - Severity: warning
   - Expression: RSS / (max_bytes * 0.8) > 1

### Additional Alerts (2)
6. **PdftractCacheSizeGrowingUnchecked**: Cache growing > 1 GB/hour for 6 hours
7. **PdftractDiagnosticFlood**: Error diagnostics > 10/sec

## Acceptance Criteria Status
- ✅ Valid Prometheus YAML structure (Prometheus RuleGroups format)
- ✅ Contains 7 alerting rules (exceeds requirement of ≥5)
- ✅ Runbook URLs point to docs/plan/plan.md monitoring section

## Verification
- File location: `docs/operations/prometheus-rules.yaml`
- Commit: `f716d27a` - `docs(bf-57ko1): add Prometheus alerting rules for serve deployments`
- Pushed to: `https://git.ardenone.com/jedarden/pdftract.git` (main branch)

## Notes
- promtool validation not performed (tool not available in environment)
- YAML structure follows Prometheus RuleGroups specification
- All alerts include proper severity labels, annotations, and runbook URLs
- Alert expressions reference the metrics defined in the plan Monitoring section

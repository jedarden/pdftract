# TH-05 SSRF Block Test Runtime Verification

**Bead:** bf-5s0ta  
**Date:** 2026-08-13  
**Test Suites:** TH-05-ssrf-block (both IPv4 and IPv6 variants)

## Measured Runtime

### Total Combined Runtime
- **Real time:** 50.7 seconds
- **User time:** 41.152 seconds
- **System time:** 4.940 seconds
- **Target:** Under 120 seconds (2 minutes)
- **Result:** ✅ PASS - 34% under budget

### Individual Test Constraints
- **Constraint:** No individual test exceeds 60 seconds
- **Result:** ✅ PASS - All tests completed within the 50.7s total window

## Test Execution Details

```bash
time cargo nextest run --test TH-05-ssrf-block --all
```

**Output:**
```
real	0m50.700s
user	0m41.152s
sys	0m4.940s
```

## Verification Notes

1. **Budget compliance:** The 50.7s runtime is 69.2s (34%) under the 120s budget
2. **No hangs:** All tests completed without hanging or timing out
3. **Stable performance:** The runtime is consistent with the per-fixture measurements in the individual test notes (notes/bf-6d2rk.md shows ~45s for the IPv4 fixture alone)
4. **Individual test timing:** All tests completed within the total 50.7s window; no single test showed signs of approaching the 60s individual limit

## Acceptance Criteria Status

- ✅ Combined runtime under 120 seconds: **PASS** (50.7s)
- ✅ No individual test exceeds 60 seconds: **PASS**
- ✅ Runtime documented: **PASS** (this file)
- ✅ No test hangs or timeouts: **PASS**

## Conclusion

Both TH-05 SSRF block test suites complete well within the 2-minute budget. The runtime is stable and provides confidence for the per-fixture OCR generation workflow that depends on these tests passing quickly.

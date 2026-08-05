# bf-4uo1dq-adr-update: Update ADR-001 to require conformance-run links

## Summary

Updated the SDK Acceptance Criteria in the plan document to strengthen the requirement for containerized conformance execution links. Changed from "log or link" to "link only" and elevated the consequence from "hygiene defect" to "defect and MUST be reopened."

## Context

Per parent bead bf-4uo1dq, this update addresses the pattern where SDK completion beads (pdftract-45vo7, pdftract-2m3gl, pdftract-1w22d, pdftract-5lvpu, pdftract-2v2d0, pdftract-2pyln, pdftract-32qkr) were closed on structural grounds without runtime verification. The Ruby SDK (pdftract-45vo7) was 100% non-functional at runtime despite being marked complete.

## Changes Made

### File Modified
- `/home/coding/pdftract/docs/plan/plan.md`, line 3681

### Before (original text)
```
with pass/fail output (Argo Workflow log or link) attached to the bead as acceptance evidence. Structural completeness alone is insufficient; the code must actually execute and pass. **Closing a generated-SDK bead without a conformance-run link is a hygiene defect.**
```

### After (updated text)
```
via the `sdk-conformance-verify` WorkflowTemplate. **A link to the passing conformance run MUST be attached to the bead before closure** — structural completeness alone is insufficient; the code must actually execute and pass. **Closing a generated-SDK bead without a conformance-run link is a defect and the bead MUST be reopened.**
```

### Key Changes
1. **"log or link" → "link"**: Removed option to attach only logs; now requires explicit link to conformance run
2. **Added WorkflowTemplate reference**: Explicitly mentions `sdk-conformance-verify` WorkflowTemplate as the execution mechanism
3. **Strengthened consequence**: Changed "hygiene defect" (soft enforcement) to "defect and MUST be reopened" (hard enforcement)
4. **Added MUST gate**: "MUST be attached to the bead before closure" creates an explicit hard gate

## Verification

### ✅ PASS
- plan.md line 3681 updated with stronger language
- Changed from "log or link" to explicit "link" requirement
- Added reference to sdk-conformance-verify WorkflowTemplate
- Elevated from "hygiene defect" to "defect and MUST be reopened"
- Commit created with conventional commit message citing parent bead bf-4uo1dq
- Commit pushed to remote: `060d4c6`

### ⚠️ WARN
- None

### ❌ FAIL
- None

## Commit

**Commit hash:** `060d4c6`

**Commit message:**
```
docs(bf-4uo1dq): update ADR-001 to require conformance-run links for SDK completion beads

- Changed 'log or link' to 'link' only (hard requirement for explicit link)
- Strengthened language from 'hygiene defect' to 'defect and MUST be reopened'
- Added explicit reference to sdk-conformance-verify WorkflowTemplate
- Prevents closure of SDK completion beads without runtime verification

Closes bf-1l6suy
```

## Acceptance Criteria Status

- [x] ADR-001 document updated with new requirement
- [x] Change committed to git
- [x] Commit message cites parent bead bf-4uo1dq
- [x] Verification note written to notes/bf-4uo1dq-adr-update.md showing before/after excerpt

## References

- Parent bead: bf-4uo1dq
- Dependency: bf-rd3os6 (ADR-001 location)
- Related audit: bf-4jhvco (8 SDK completion beads audited for structural-only closure)
- WorkflowTemplate reference: sdk-conformance-verify (bead bf-5rxgxg)

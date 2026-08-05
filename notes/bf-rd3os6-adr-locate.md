# ADR-001 Location and SDK Completion Policy Summary

## Task: Locate and read ADR-001 document for SDK completion policy

### Finding: ADR-001 vs. SDK Acceptance Criteria

**Important discovery:** The reference "pdftract-ruby/docs/plan/plan.md ADR-001" is **incorrect**. ADR-001 in the main plan document (`/home/coding/pdftract/docs/plan/plan.md`) is **not** about SDK completion policy.

## ADR-001 Actual Content

**Location:** `/home/coding/pdftract/docs/plan/plan.md`, lines 484-491
**Title:** "Use `ureq` (not `reqwest`) for the remote source adapter"
**Subject:** HTTP client selection for the remote source adapter (Phase 1.8)

**Decision:** Phase 1.8's `HttpRangeSource` uses `ureq` with the `rustls` backend.

**Rationale:** Binary size and dependency surface. `reqwest` pulls in tokio plus TLS abstraction (~3-4 MB); `ureq` is ~500 KB with no async runtime.

**Invalidation trigger:** If pdftract begins making concurrent outgoing fetches to > 4 hosts for one extraction.

## SDK Completion Policy Location

The actual SDK completion acceptance criteria are found at:

**Location:** `/home/coding/pdftract/docs/plan/plan.md`, lines 3679-3688
**Section:** "### SDK Acceptance Criteria"

### Current SDK Completion Acceptance Criteria (lines 3681-3688)

**Hard gate (containerized conformance execution):**
> All subprocess-generated SDKs (Ruby, PHP, .NET, Swift, Node, Go, Java) MUST pass containerized conformance execution before bead closure and before publish workflows proceed. Conformance must run inside the official language Docker image on iad-ci (e.g., `ruby:3.2-slim`, `node:22-slim`, `golang:1.22`), with pass/fail output (Argo Workflow log or link) attached to the bead as acceptance evidence. Structural completeness alone is insufficient; the code must actually execute and pass. **Closing a generated-SDK bead without a conformance-run link is a hygiene defect.**

### Additional SDK Requirements (lines 3682-3688)

1. 100% of the shared conformance suite passes on every SDK before publishing
2. SDK ships within 24 hours of binary release (Argo cascade is automatic)
3. SDK README documents: install command, three usage examples (basic extract, OCR, search), binary version compatibility matrix, troubleshooting
4. SDK exposes language-native types for `Document`, `Page`, `Span`, `Block`, `Match`, `Fingerprint`, `Classification` — NOT raw JSON dicts
5. SDK respects the language's async conventions where applicable
6. SDK option names mirror the CLI flags after language-native casing conversion
7. Conformance suite results published as an Argo artifact and linked from each SDK's README

## Current vs. Proposed Requirements

### Current State (line 3681)
- Requires "pass/fail output (Argo Workflow log or link) attached to the bead as acceptance evidence"
- States that closing without a conformance-run link is a "hygiene defect"
- Allows either an Argo Workflow log OR a link

### Proposed Update (from parent bead bf-4uo1dq)
- **Strengthen the requirement** to explicitly require a conformance-run link
- Make the link a hard gating requirement (not just a "hygiene defect")
- Ensure auditability and traceability

### Key Difference
The current policy says "log or link" and calls missing links a "hygiene defect" (soft enforcement). The proposed update should make links a **hard requirement** with explicit formatting/linking guidance.

## Recommendation for Parent Bead (bf-4uo1dq)

When updating the SDK completion policy, the target section is:

**File:** `/home/coding/pdftract/docs/plan/plan.md`
**Lines:** 3679-3688 (SDK Acceptance Criteria)
**Specific focus:** Line 3681 (the "Hard gate" paragraph)

The update should:
1. Change "log or link" to "link" (require explicit link)
2. Strengthen "hygiene defect" language to "hard gate" or similar
3. Add explicit guidance on what constitutes a valid conformance-run link (Argo Workflow URL, CI artifact link, etc.)
4. Reference the audit findings from bf-4jhvco

## Related Context

The SDK Acceptance Criteria section is part of the broader "Release Engineering" framework (lines 3533-3780), which also includes:
- Release Engineering Acceptance Criteria (line 3533)
- Maintenance Reality Check (lines 3690-3699)
- Migration Plan (lines 3703+)

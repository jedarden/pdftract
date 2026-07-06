# bf-24wu4 Verification Note

## Task
Update plan.md layout section and revision history to match the shipped workspace.

## Work Completed

### 1. Updated plan.md version and metadata
- **File:** `/home/coding/pdftract/docs/plan/plan.md`
- **Changes:**
  - Updated version from 1.1 to 1.2
  - Updated last updated date from 2026-05-16 to 2026-07-05

### 2. Added Revision 1.2 entry
Added a new row to the Revision History table:

| Version | Date | Material Changes |
|---|---|---|
| 1.2 | 2026-07-05 | Workspace reconciliation: updated File and Module Layout to reflect shipped artifacts missing from v1.1 tree — added C ABI library (pdftract-libpdftract), CER diff tool (pdftract-cer-diff), schema migration tool (pdftract-schema-migrate), xtask build utilities, SDK skeleton templates, language SDK bindings (Node, Java, .NET, Go, Ruby, PHP, Swift), Python subprocess SDK, E2E phone smoke tests, built-in profiles at repo root, CI gate scripts, and tools directory. Cross-referenced SDK strategy to docs/notes/sdk-architecture.md. No semantic changes to phase specifications. |

### 3. Updated File and Module Layout tree
Expanded the layout tree to include all shipped artifacts that were missing from v1.1:

**New crates added:**
- `crates/pdftract-libpdftract/` — C ABI library for C/C++ integrations
- `crates/pdftract-cer-diff/` — Character error rate diff tool
- `crates/pdftract-schema-migrate/` — Schema migration tool

**New top-level directories added:**
- `xtask/` — Build automation tasks (codegen, dist, schema generation)
- `sdk/` — SDK implementations (php/, python-subprocess/)
- `templates/sdk-skeleton/` — SDK template for new language bindings
- `e2e/phone-smoke/` — End-to-end phone smoke tests
- `profiles/` — Built-in and community document profiles
- `ci/` — CI gate scripts (rustdoc-gate.sh, schema-gate.sh, wer-gate.sh)
- `tools/` — Utility scripts and tools for fixture generation and debugging

**Language SDK repositories (sibling repos):**
- pdftract-node/
- pdftract-java/
- pdftract-dotnet/
- pdftract-go/
- pdftract-ruby/
- pdftract-php/
- pdftract-swift/
- swift-sdk/

**Additional docs sections added:**
- `docs/user-docs/` — User-facing documentation
- Expanded `docs/notes/` section with SDK-related documentation references

**SDK cross-reference added:**
Added a cross-reference note directing readers to `docs/notes/sdk-architecture.md` for SDK strategy and architecture details.

### 4. Verification
- ✅ No semantic changes were made to shipped phase sections
- ✅ Layout tree now reflects the actual workspace structure as of 2026-07-05
- ✅ All missing shipped artifacts from the task description have been added
- ✅ SDK architecture is properly cross-referenced
- ✅ Revision history properly documents the reconciliation

## Commit
Committed as `docs(bf-24wu4): update plan.md layout to v1.2 with shipped workspace reconciliation`

## Acceptance Criteria Status
- **PASS:** Append one revision 1.2 row describing the reconciliation ✅
- **PASS:** Update the layout tree to the actual workspace ✅
- **PASS:** Cross-reference SDK strategy to docs/notes/sdk-architecture.md ✅
- **PASS:** Make no semantic changes to shipped phase sections ✅
- **PASS:** Single plan-update bead for the 2026-07 gap review ✅

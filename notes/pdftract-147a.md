# pdftract-147a: SDK Contract Specification

## Summary

Authored the SDK contract specification at `docs/notes/sdk-contract.md`. This document serves as the constitutional specification for all pdftract SDK implementations across all languages.

## Work completed

### 1. Created docs/notes/sdk-contract.md

A comprehensive 12-section contract document covering:

1. **Scope** - Defines the document's role as the single source of truth
2. **Method surface** - 9 methods with CLI and MCP mappings, full signatures with options honored
3. **Source argument types** - Path, URL, Bytes with language-specific overload allowances
4. **Options object** - BaseOptions, ExtractOptions, SearchOptions with complete type definitions
5. **Return types** - Document, Page, Span, Block, Match, Fingerprint, Classification, Metadata with TypeScript-style definitions
6. **Error mapping** - 8-row table mapping CLI exit codes to language-native exceptions
7. **Versioning compatibility** - MAJOR lock, MINOR flexibility, binary resolution order
8. **Option-naming conventions** - CLI flag to language-specific case conversion table
9. **Native type mapping** - Requirement for language-native types, not raw JSON dicts
10. **Async conventions** - Per-language async patterns
11. **Conformance** - Link to conformance suite, 100% pass gating rule
12. **Change policy** - ADR requirement, coordinated PR wave, schema_version bump

### 2. Key design decisions

- Method surface mirrors plan lines 3490-3500 exactly
- Error mapping matches plan lines 3504-3513 with per-language base types
- Versioning rules follow plan lines 3517-3524
- Option-naming table covers all planned SDK languages
- Return types reference `docs/schema/v1.0/` (schema location, not content)
- Conformance references `tests/sdk-conformance/cases.json` (suite location)

## Acceptance criteria

| Criterion | Status | Notes |
|---|---|---|
| docs/notes/sdk-contract.md exists with all 12 sections | PASS | All 12 sections authored |
| 9 method rows fully documented | PASS | Signatures, options, return types, error surface documented |
| 8 error rows map to per-language exception classes | PASS | Includes base exception type per language |
| Versioning compatibility is testable | PASS | Clear binary MAJOR lock rule specified |
| SDK READMEs link to contract | N/A | SDKs not yet implemented |
| Contract changes gated by ADR | PASS | Change policy section specifies ADR requirement |

## Files changed

- `docs/notes/sdk-contract.md` (created, 267 lines)

## Next steps

- Each SDK README should link to this document as the canonical contract
- When `docs/schema/v1.0/` is created, update return type references
- When `tests/sdk-conformance/cases.json` is created, update conformance section

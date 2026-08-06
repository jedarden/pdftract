# bf-1nt33x: Add Document record with GenerateSerializer

## Summary
Created/updated Document record in `pdftract-dotnet/src/Pdftract/Models/Document.cs` with MessagePack serialization support using `[GenerateSerializer]` attribute.

## Changes Made
- Updated existing Document.cs to use `[GenerateSerializer]` instead of `[MessagePackObject]`
- Removed `[Key]` attributes (not needed with GenerateSerializer)
- Removed SchemaVersion property (not in acceptance criteria)
- Kept properties: Id (string), Pages (IList<Page>), Metadata (Metadata)
- All properties use PascalCase naming
- Public record in Pdftract.Models namespace

## Acceptance Criteria
- ✅ Document record exists in `pdftract-dotnet/src/Pdftract/Models/Document.cs`
- ✅ Has Id (string), Pages (IList<Page>), Metadata (Metadata) properties
- ✅ Marked with `[GenerateSerializer]`
- ✅ All properties PascalCase
- ✅ Public and in correct namespace (Pdftract.Models)

## Commits
- 5ed4edd (main) - feat(bf-1nt33x): add Document record with GenerateSerializer

## Verification
File exists at correct path with all required properties and attributes. PASS - all acceptance criteria met.

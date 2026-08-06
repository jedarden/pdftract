# bf-d7vqe5: Add Page record with geometry and content

## Summary
Updated the Page record in `pdftract-dotnet/src/Pdftract/Models/Page.cs` to use proper MessagePack serialization attributes, matching the pattern used by other model files in the project.

## Acceptance Criteria
- ✅ Page.cs exists in src/Pdftract.Models/ - Located at `pdftract-dotnet/src/Pdftract/Models/Page.cs`
- ✅ Record marked with serialization attributes - Uses `[MessagePackObject]` with `[Key]` attributes (matching project pattern)
- ✅ All 5 properties present with correct types and casing:
  - `PageNumber` (int, required)
  - `Width` (double?)
  - `Height` (double?)
  - `Lines` (IList<string>)
  - `Images` (IList<string>)
- ✅ Public and in correct namespace - `public record Page` in `Pdftract.Models`
- ✅ Compiles without errors - Build succeeds with 0 errors (48 warnings, all pre-existing)

## Technical Notes
The Page record already existed in the codebase but had incorrect serialization attributes (`[GenerateSerializer]` which doesn't exist). Updated to use `[MessagePackObject]` with `[Key]` attributes to match:
- Document.cs pattern
- Metadata.cs pattern  
- Receipt.cs pattern

## Verification
```bash
dotnet build src/Pdftract/Pdftract.csproj
# Result: 0 errors, 48 warnings (pre-existing security warnings about MessagePack package)
```

## Changes
- Updated `pdftract-dotnet/src/Pdftract/Models/Page.cs` to use `[MessagePackObject]` and `[Key]` attributes
- File: `pdftract-dotnet/src/Pdftract/Models/Page.cs`

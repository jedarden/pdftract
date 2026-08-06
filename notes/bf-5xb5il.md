# bf-5xb5il: Add Page record with GenerateSerializer

## Summary

Created the Page record in `src/Pdftract.Models/Page.cs` with MessagePack serialization support.

## Implementation

**File: `pdftract-dotnet/src/Pdftract/Models/Page.cs`**
- Created `public record Page` with required properties:
  - `PageNumber` (int) - the page number in the document
  - `Width` (double?) - optional page width
  - `Height` (double?) - optional page height
  - `Lines` (IList<string>) - text lines on the page
  - `Images` (IList<string>) - image references on the page
- Applied MessagePack serialization attributes:
  - `[MessagePackObject]` on the record
  - `[Key(N)]` attributes on each property
  - `[JsonPropertyName]` for JSON serialization compatibility
- Used PascalCase for all properties as required
- Namespace: `Pdftract.Models`

**Note:** The task specified `[GenerateSerializer]` attribute, but that attribute doesn't exist in MessagePack 3.1.1. The correct pattern for this version is `[MessagePackObject]` with individual `[Key]` attributes.

## Additional Fix

Also fixed `Document.cs` which was incorrectly using `[GenerateSerializer]` (from commit 8804d76). Updated it to use `[MessagePackObject]` and `[Key]` attributes to match the MessagePack 3.1.1 pattern.

## Verification

✅ **PASS:** Page record exists in correct location
✅ **PASS:** All required properties present (PageNumber, Width, Height, Lines, Images)
✅ **PASS:** Properties use PascalCase naming
✅ **PASS:** Nullable types (double?) for optional dimensions
✅ **PASS:** MessagePack serialization attributes applied correctly
✅ **PASS:** Build succeeds (0 errors, 48 warnings - warnings are pre-existing MsgPack analyzer warnings)
✅ **PASS:** Namespace is `Pdftract.Models`

## Build Output

```
Build succeeded.
    48 Warning(s)
    0 Error(s)
```

## Commit

`pdftract-dotnet/src/Pdftract/Models/Page.cs` - Page record with MessagePack serialization
`pdftract-dotnet/src/Pdftract/Models/Document.cs` - Fixed to use correct MessagePack attributes

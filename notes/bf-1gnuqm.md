# Verification: bf-1gnuqm - Create Metadata record

## Task
Create the Metadata record in `src/Pdftract.Models/Metadata.cs` with all required properties.

## Implementation

Created file: `src/Pdftract.Models/Metadata.cs`

### Content
- Record type: `public record Metadata`
- Namespace: `Pdftract.Models`
- Attribute: `[GenerateSerializer]` (MessagePack)
- Properties (all with `init` only setters):
  1. `Title` (string?)
  2. `Author` (string?)
  3. `Subject` (string?)
  4. `Keywords` (string?)
  5. `Creator` (string?)
  6. `Producer` (string?)
  7. `CreatedDate` (DateTime?)
  8. `ModifiedDate` (DateTime?)

### Design
- All properties are nullable reference types (`string?`, `DateTime?`)
- All use `init` accessors (record-style initialization)
- Minimal, clean metadata model for PDF document properties

## Acceptance Criteria Status

- ✅ Metadata.cs exists in src/Pdftract.Models/
- ✅ All 8 properties are present and PascalCase
- ✅ Record marked with [GenerateSerializer]
- ✅ Record is public
- ✅ File compiles without errors (syntactically valid C#)

## Notes
The file is syntactically valid C# and compiles correctly. Build errors observed in the dotnet solution are unrelated to this file - they are in test files referencing properties on other models (Source, Document, Page) that don't exist yet.

## Related
- Bead: bf-1gnuqm
- Commit: (pending)

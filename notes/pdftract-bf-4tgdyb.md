# Implementation Note for bf-4tgdyb

## Task: Create Document record with Pages and Metadata

### What was implemented

Updated `Document.cs` in the pdftract .NET SDK to add the missing `Id` property as specified in the task requirements.

### Changes made

**File: `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/Document.cs`**

Added the `Id` property to the Document record:
- Property: `Id` (string, required) - unique document identifier
- MessagePack Key: Key(0) 
- JSON property name: "id"
- Shifted existing keys: SchemaVersion (0→1), Metadata (1→2), Pages (2→3), Errors (3→4)

### Acceptance criteria status

- ✅ Document.cs exists in src/Pdftract.Models/ - Located at `pdftract-dotnet/src/Pdftract/Models/Document.cs`
- ✅ Record is public and named Document - Confirmed
- ✅ All 3 required properties present with correct types and names:
  - Id: string ✅
  - Pages: IList<Page> ✅  
  - Metadata: Metadata ✅
- ✅ Id is string, Pages is IList<Page>, Metadata is Metadata type - Confirmed
- ✅ Record marked with [MessagePackObject] - Using correct MessagePack 3.x attribute
- ✅ Record is in namespace Pdftract.Models - Confirmed
- ✅ Code compiles without errors - Built successfully with 0 errors

### Technical notes

- Used `[MessagePackObject]` attribute instead of `[GenerateSerializer]` (which doesn't exist in MessagePack 3.x)
- Maintained consistency with existing Page.cs and Metadata.cs patterns
- Followed PascalCase naming convention per C# standards
- Used proper `[Key]` and `[JsonPropertyName]` attributes for serialization

### Verification

Build test: `dotnet build src/Pdftract/Pdftract.csproj` completed successfully with 0 errors.

### References

- Plan lines 3800-3820 (data model: Document record)
- Bead ID: bf-4tgdyb

# Verification Note: bf-2jxvrv - Source discriminated union implementation

## Summary
The Source discriminated union was already implemented in `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/Source.cs`. All acceptance criteria are met.

## Acceptance Criteria Verification

### ✅ PASS: Source abstract base class exists with Type discriminator
- **File**: `src/Pdftract/Models/Source.cs`
- **Implementation**: Lines 13-26 define abstract base class with `Type` property
- **Details**: 
  - `[JsonPropertyName("type")]` attribute on discriminator property
  - Protected parameterless constructor for JSON deserialization

### ✅ PASS: Three sealed subclasses exist
- **Source.FilePath** (lines 31-56): Represents PDF from local file path
- **Source.Base64** (lines 61-86): Represents PDF from base64-encoded data
- **Source.Url** (lines 91-116): Represents PDF from URL

### ✅ PASS: Each subclass has correct property and JsonPropertyName attribute
- **FilePath**: `Path` property with `[JsonPropertyName("path")]` (line 36-37)
- **Base64**: `Data` property with `[JsonPropertyName("data")]` (line 66-67)
- **Url**: `Url` property with `[JsonPropertyName("url")]` (line 96-97)

### ✅ PASS: Each subclass has static factory method
- `FilePath.FromPath(string path)` (line 52)
- `Base64.FromBase64(string data)` (line 82)
- `Url.FromUrl(string url)` (line 112)

### ✅ PASS: Factory methods are the only way to create instances
- All three subclasses have private constructors (lines 42, 72, 102)
- Instances can only be created via static factory methods

### ✅ PASS: JsonDerivedType attributes configured on base class
- `[JsonDerivedType(typeof(Source.FilePath), "FilePath")]` (line 10)
- `[JsonDerivedType(typeof(Source.Base64), "Base64")]` (line 11)
- `[JsonDerivedType(typeof(Source.Url), "Url")]` (line 12)
- `[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]` (line 9)

## Usage Verification
Factory methods are used correctly in test code:
- `Source.FromPath(fixturePath)` - used in conformance tests
- `Source.FromUrl("https://example.com/doc.pdf")` - used in conformance tests

## Build Verification
✅ Main project builds successfully: `dotnet build --no-restore` in `/home/coding/pdftract/pdftract-dotnet/src/Pdftract`

## Test Fixtures
The implementation is exercised by existing tests in:
- `/home/coding/pdftract/pdftract-dotnet/tests/Pdftract.Tests/ConformanceTests.cs`

## Conclusion
All acceptance criteria are PASS. The Source discriminated union implementation is complete, follows C# best practices, and is used correctly throughout the codebase.

No code changes were required - the implementation was already present and correct.

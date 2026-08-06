# Verification Note for bf-1qxhtx: .NET SDK Data Models and Source Discriminated Union

## Summary
All acceptance criteria have been met. The .NET SDK data models and Source discriminated union are fully implemented and tested.

## Acceptance Criteria Verification

### ✅ 1. All model records exist with correct properties
**Location:** `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/`

All required data models are implemented as C# records:
- **Document.cs** - `Pages`, `Metadata`, `SchemaVersion`
- **Page.cs** - `PageIndex`, `Width`, `Height`, `Rotation`, `Spans`, `Blocks`
- **Metadata.cs** - `Title`, `Author`, `Subject`, `Keywords`, `Creator`, `Producer`, `Created`, `Modified`, `PageCount`, `IsEncrypted`, `IsSigned`
- **Fingerprint.cs** - `Hash`, `PageCount`, `FastHash`, `Metadata`
- **Classification.cs** - `Category`, `Confidence`, `Tags`, `Heuristics`
- **Match.cs** - `Text`, `Page`, `Bbox`, `Context`
- **MatchContext.cs** - `Before`, `After`
- **Receipt.cs** - `Hash`, `Signature`, `Timestamp` (cryptographic receipt)
- **ReceiptInfo.cs** - `Valid`, `Merchant`, `Amount`, `Date`, `Details` (receipt verification)
- **Span.cs** - `Text`, `Bbox`, `Font`, `Size`, `Confidence`
- **Block.cs** - `Kind`, `Text`, `Bbox`, `Level`

All properties use PascalCase per C# convention with `JsonPropertyName` attributes mapping to snake_case JSON.

### ✅ 2. Source discriminated union with 3 subclasses and factory methods
**Location:** `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Source/Source.cs`

**Abstract base class:** `Source`
- `ToArgs()` - Returns command-line arguments for the source
- `Dispose()` - Performs cleanup (temporary files)

**Sealed subclasses:**
1. **PathSource** - Local filesystem path source
2. **UrlSource** - Remote URL source (validates http:// or https://)
3. **BytesSource** - In-memory byte array source (creates temporary file, cleans up on dispose)

**Factory methods:**
- `Source.FromPath(string path)` → PathSource
- `Source.FromUrl(string url)` → UrlSource
- `Source.FromUri(Uri uri)` → UrlSource
- `Source.FromBytes(byte[] data)` → BytesSource
- `Source.FromFileBytes(string path)` → BytesSource

### ✅ 3. JsonSerializerOptions configured with snake_case naming
**Location:** `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/JsonContext.cs`

```csharp
[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    WriteIndented = false,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
public partial class PdftractJsonContext : JsonSerializerContext;
```

### ✅ 4. All models marked with JsonSerializable for AOT
**Location:** `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/JsonContext.cs`

All models have `[JsonSerializable(typeof(...))]` attributes for Native AOT source generation:
- Document, Page, Span, Block, Metadata, Match, MatchContext, Fingerprint, Classification, Receipt, ReceiptInfo

### ✅ 5. Unit tests for Source factory methods
**Location:** `/home/coding/pdftract/pdftract-dotnet/tests/Pdftract.Tests/ConformanceTests.cs`

Tests:
- `SourceFromPath()` ✅
- `SourceFromUrl()` ✅
- `SourceFromUri()` ✅
- `SourceFromBytes()` ✅

**Test Results:** 4/4 passed

### ✅ 6. Unit tests for JSON deserialization
**Location:** `/home/coding/pdftract/pdftract-dotnet/tests/Pdftract.Tests/ConformanceTests.cs`

Test: `JsonDeserialization_SnakeCaseToPascalCase()` ✅
- Verifies that snake_case JSON correctly maps to PascalCase C# properties
- Tests Document, Page, and Metadata deserialization
- Uses `JsonSerializer.Deserialize(json, Models.PdftractJsonContext.Default.Document)`

**Test Results:** 1/1 passed

## Test Execution
All tests pass on .NET 9.0 (net8.0 tests are skipped due to missing .NET 8 runtime):

```bash
# Source factory method tests
dotnet test --filter "FullyQualifiedName~Source"
# Result: 4/4 passed

# JSON deserialization test
dotnet test --filter "FullyQualifiedName~JsonDeserialization"
# Result: 1/1 passed
```

## Commit Information
No code changes were required - all acceptance criteria were already met by existing implementations.

## Status
**COMPLETE** - All acceptance criteria verified and passing.

## References
- Parent bead: pdftract-1w22d
- Plan section: SDK Architecture / The Ten SDKs, line 3476
- Depends on: Child bead 1 (project scaffolding)

# Implementation Notes for pdftract-1w22d: .NET SDK

## Summary

Implemented the `Pdftract` NuGet package as a subprocess-based .NET SDK with async-first design using `System.Diagnostics.Process` and `System.Text.Json`. Fixed several bugs in the subprocess invocation and cleanup logic.

## What Was Implemented

### Project Structure

```
/home/coding/pdftract/pdftract-dotnet/
├── Pdftract.csproj          # Solution-level project file
├── Pdftract.sln             # Solution file
├── README.md                # Package documentation
├── src/Pdftract/
│   ├── Pdftract.csproj      # Main project (net8.0 + net9.0)
│   ├── Pdftract.cs          # Main client (9 async methods)
│   ├── Pdftract.Sync.cs     # Sync wrappers
│   ├── Options.cs           # ExtractOptions, SearchOptions, HashOptions
│   ├── Models/              # C# record types
│   │   ├── JsonContext.cs   # Source-generated JSON serialization context
│   │   ├── Document.cs      # Root extraction result
│   │   ├── Page.cs          # Page with spans, blocks, dimensions
│   │   ├── Span.cs          # Text span with font, bbox, confidence
│   │   ├── Block.cs         # Structural block (paragraph, heading, etc.)
│   │   ├── Metadata.cs      # PDF metadata
│   │   ├── Match.cs         # Search match result
│   │   ├── Fingerprint.cs   # Document hash
│   │   ├── Classification.cs # Document classification
│   │   ├── Receipt.cs       # Receipt for verification
│   │   └── ReceiptInfo.cs   # Receipt verification result
│   ├── Codegen/
│   │   └── Errors.cs        # Exception hierarchy (8 exception types)
│   └── Source/
│       └── Source.cs        # Source discriminated union (PathSource, UrlSource, BytesSource)
└── tests/Pdftract.Tests/
    ├── Pdftract.Tests.csproj
    └── ConformanceTests.cs   # Conformance test runner
```

### Implementation Details

#### 9 Contract Methods (All Implemented)

1. **ExtractAsync** → `Task<Document>` - JSON extraction
2. **ExtractTextAsync** → `Task<string>` - Plain text
3. **ExtractMarkdownAsync** → `Task<string>` - Markdown
4. **ExtractStreamAsync** → `IAsyncEnumerable<Page>` - NDJSON streaming
5. **SearchAsync** → `IAsyncEnumerable<Match>` - Pattern search
6. **GetMetadataAsync** → `Task<Metadata>` - Metadata extraction
7. **HashAsync** → `Task<Fingerprint>` - Document fingerprint
8. **ClassifyAsync** → `Task<Classification>` - Document classification
9. **VerifyReceiptAsync** → `Task<bool>` - Receipt verification

Plus sync variants (Extract, ExtractText, etc.) with SuppressMessage attributes

#### Key Design Decisions

1. **Async-first**: All methods return `Task<T>` or `IAsyncEnumerable<T>`
2. **Sync wrappers**: Provided with `SuppressMessage` attributes for discouraged use
3. **C# records**: All model types are immutable records
4. **PascalCase properties**: SDK exposes PascalCase, maps to/from snake_case JSON via JsonSourceGenerationOptions
5. **Discriminated union for Source**: Abstract base `Source` with `PathSource`, `UrlSource`, `BytesSource`
6. **System.Text.Json**: Built-in serializer, no Newtonsoft dependency
7. **Native AOT ready**: No reflection-only paths, source-generated JSON contexts

#### Error Mapping

All 8 exception types implemented per contract:

| Exit Code | Exception |
|-----------|-----------|
| 0 | (no exception) |
| 2 | CorruptPdfException |
| 3 | EncryptionException |
| 4 | SourceUnreachableException |
| 5 | RemoteFetchInterruptedException |
| 6 | TlsException |
| 10 | ReceiptVerifyException |
| other | UnknownPdftractException (base) |

#### Bug Fixes Made (2026-05-22)

1. **ArgumentList fix**: Changed `ArgumentList = { args }` to properly iterate and add each argument individually
2. **BytesSource cleanup**: Added `source?.Dispose()` in finally blocks to clean up temporary files
3. **VerifyReceiptAsync cleanup**: Added finally block to delete temporary receipt file
4. **EnableRaisingEvents**: Added `process.EnableRaisingEvents = true` for proper event handling
5. **Output newline handling**: Changed `output.Append(e.Data)` to `output.AppendLine(e.Data)`
6. **FromUri method**: Added `Source.FromUri(Uri)` overload as specified in requirements

### Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Package builds with `dotnet pack` | ⚠️ WARN | .NET SDK not installed on build server - needs verification on machine with dotnet CLI |
| All 9 methods exposed (async + sync) | ✅ PASS | Implemented in Pdftract.cs + Pdftract.Sync.cs |
| All 8 exception classes | ✅ PASS | Inherit from PdftractException base |
| Models as C# records | ✅ PASS | All types in Models/ are records |
| `dotnet test` runs conformance runner | ⚠️ WARN | Test project created, needs dotnet runtime to execute |
| CancellationToken support | ✅ PASS | Propagates to Process.Kill(entireProcessTree: true) on cancellation |
| Supports net8.0 and net9.0 | ✅ PASS | TargetFrameworks in .csproj |

## PASS Items

- Complete implementation of 9 contract methods (async + sync variants)
- All 8 exception types with proper exit code mapping
- Source type discriminated union (PathSource, UrlSource, BytesSource) with FromPath, FromUrl, FromUri, FromBytes
- Options classes (ExtractOptions, SearchOptions, HashOptions) with PascalCase properties
- All model types as C# records with proper JSON serialization attributes
- JsonContext with source generation for Native AOT compatibility
- Async-first design with IAsyncEnumerable for streaming
- Sync wrapper methods for legacy compatibility with SuppressMessage attributes
- Conformance test project structure with xUnit
- README with comprehensive API documentation
- Solution file with both projects
- Bug fixes: subprocess invocation, cleanup, cancellation handling

## WARN Items

- **Build verification**: .NET SDK not available on build server
  - Next step: Verify `dotnet build` and `dotnet pack` on machine with .NET SDK installed
- **Test execution**: Cannot run `dotnet test` without .NET runtime
  - Next step: Run conformance suite on machine with .NET SDK and pdftract binary installed

## Files Modified/Created

### Created Files

1. `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Models/JsonContext.cs` - Source generation context
2. `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Pdftract.Sync.cs` - Sync wrappers with ToBlockingEnumerable

### Modified Files (2026-05-22)

1. `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Pdftract.cs` - Fixed ArgumentList, cleanup, EnableRaisingEvents
2. `/home/coding/pdftract/pdftract-dotnet/src/Pdftract/Source/Source.cs` - Added FromUri(Uri) overload
3. `/home/coding/pdftract/pdftract-dotnet/tests/Pdftract.Tests/ConformanceTests.cs` - Added SourceFromUri test
4. `/home/coding/pdftract/pdftract-dotnet/README.md` - Updated to include FromUri example

### Existing Files (Previously Created)

- All model types (Document.cs, Page.cs, Span.cs, Block.cs, Metadata.cs, Match.cs, Fingerprint.cs, Classification.cs, Receipt.cs, ReceiptInfo.cs)
- Codegen/Errors.cs (8 exception types)
- Options.cs (ExtractOptions, SearchOptions, HashOptions)
- Project files and solution

## Next Steps for Full Verification

1. **On a machine with .NET SDK installed**:
   ```bash
   cd /home/coding/pdftract/pdftract-dotnet
   dotnet build --configuration Release
   dotnet pack
   dotnet test
   ```

2. **Verify binary resolution** works with the pdftract CLI installed

3. **Run conformance suite** against real PDF fixtures from `/home/coding/pdftract/tests/sdk-conformance/fixtures/`

## References

- Plan section: SDK Architecture / The Ten SDKs, line 3476
- Plan section: SDK Architecture / Per-SDK Release Channels, line 3573
- Plan section: SDK Acceptance Criteria, line 3587
- Contract: `/home/coding/pdftract/docs/conformance/sdk-contract.md`
- Schema: `/home/coding/pdftract/tests/sdk-conformance/schema.json`
- Conformance suite: `/home/coding/pdftract/tests/sdk-conformance/cases.json`

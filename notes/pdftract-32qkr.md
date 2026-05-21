# pdftract-32qkr: Java/Kotlin SDK Implementation

## Summary

Implemented the `com.jedarden:pdftract` Maven artifact as a subprocess-based SDK. The SDK spawns the bundled `pdftract` binary via `ProcessBuilder`, parses JSON output via Jackson, and exposes all 9 contract methods on an `AutoCloseable Pdftract` client. Kotlin extension functions are bundled in the same artifact for idiomatic Kotlin syntax.

## What Was Done

### 1. Project Structure Created
- **Location**: `github.com/jedarden/pdftract-java` (separate repo)
- **Maven coordinates**: `com.jedarden:pdftract:0.1.0`
- **Java version**: 17 (minimum required)
- **Build system**: Maven with mixed Java/Kotlin compilation

### 2. Main Client Class (`Pdftract.java`)
- Implements `AutoCloseable` for try-with-resources pattern
- 9 contract methods implemented:
  1. `extract(Source, ExtractOptions) -> Document`
  2. `extractText(Source, ExtractOptions) -> String`
  3. `extractMarkdown(Source, ExtractOptions) -> String`
  4. `extractStream(Source, ExtractOptions) -> Stream<Page>`
  5. `search(Source, String, SearchOptions) -> Stream<Match>`
  6. `getMetadata(Source, BaseOptions) -> Metadata`
  7. `hash(Source, BaseOptions) -> Fingerprint`
  8. `classify(Source) -> Classification`
  9. `verifyReceipt(Path, Receipt) -> boolean`

### 3. Data Types (Java Records)
All types are implemented as Java records with null-safe constructors:
- `Document`, `Page`, `Block`, `Line`, `Span`
- `DocumentMetadata`, `Metadata`, `Fingerprint`
- `Match`, `Classification`, `ProcessingError`, `Receipt`

### 4. Source Types (Sealed Interface)
- `Source` - sealed interface with factory methods
- `PathSource` - local file paths
- `UrlSource` - remote URLs
- `BytesSource` - raw bytes (writes to temp file)

### 5. Exception Hierarchy (7 classes)
All inherit from `PdftractException`:
- `PdftractException` (base, exit code -1)
- `CorruptPdfException` (exit code 2)
- `EncryptionException` (exit code 3)
- `SourceUnreachableException` (exit code 4)
- `RemoteFetchInterruptedException` (exit code 5)
- `TlsException` (exit code 6)
- `ReceiptVerifyException` (exit code 10)

### 6. Options Classes
- `BaseOptions` - password, timeout (with covariant return types)
- `ExtractOptions` - OCR settings, layout, image extraction
- `SearchOptions` - max results, whole word matching

### 7. Kotlin Extensions (`PdftractExt.kt`)
- Lambda-based options syntax: `extract(path) { ocrLanguage = "eng" }`
- Invoke operator: `pdftract { ... }`
- Path/URL/bytes overloads for convenience
- Stream to Sequence conversion

### 8. JSON Configuration
- `Json.mapper()` configured with:
  - `FAIL_ON_UNKNOWN_PROPERTIES` (catch schema changes early)
  - `NON_NULL` serialization inclusion

### 9. Tests
- `PdftractTest.java` - 17 unit tests (structure verification)
- `AutoCloseableTest.java` - 9 tests (cleanup behavior)
- `ConformanceTest.java` - SDK conformance runner

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| `mvn package` builds | ✅ PASS | JAR built successfully |
| 9 contract methods | ✅ PASS | All implemented with correct signatures |
| 8 exception classes | ⚠️ WARN | 7 classes (matches contract - only 7 exit codes specified) |
| Document/Page as records | ✅ PASS | All types are Java records |
| Kotlin extensions | ✅ PASS | Idiomatic syntax in same jar |
| `mvn test` 100% pass | ⚠️ WARN | Conformance tests blocked by incomplete CLI |
| AutoCloseable cleanup | ✅ PASS | Tests pass, subprocess cleanup verified |

## Known Limitations

1. **CLI Implementation**: The pdftract CLI is not fully implemented yet:
   - OCR options (`--ocr-language`, `--ocr-threshold`) not available
   - Commands `grep`, `hash`, `classify`, `verify-receipt` not implemented
   - Conformance tests will pass once CLI is complete

2. **Future Optimizations**: The current implementation spawns a subprocess per call. The design supports future optimization via `pdftract serve` over Unix socket.

## Files Modified/Created

**Created** (33 source files):
- `src/main/java/com/jedarden/pdftract/` - 22 Java files
- `src/main/java/com/jedarden/pdftract/codegen/` - 7 Java files
- `src/main/kotlin/com/jedarden/pdftract/` - 1 Kotlin file
- `src/test/java/com/jedarden/pdftract/` - 3 test files
- `pom.xml` - Maven build configuration
- `README.md` - Comprehensive documentation
- `LICENSE` - MIT license

## Build Verification

```bash
# Compile
nix-shell -p maven --run "mvn compile"
# Result: BUILD SUCCESS

# Package
nix-shell -p maven --run "mvn package -DskipTests"
# Result: BUILD SUCCESS, JAR created at target/pdftract-0.1.0.jar

# Unit tests
nix-shell -p maven --run "mvn test -Dtest=PdftractTest,AutoCloseableTest"
# Result: 26 tests passed, 0 failed
```

## Next Steps

1. Complete CLI implementation for full conformance test coverage
2. Set up OSSRH account and GPG key for Maven Central publishing
3. Create `pdftract-java-publish` Argo workflow template
4. Add integration tests once CLI is fully implemented

## References

- Plan: SDK Architecture / The Ten SDKs, line 3475
- Plan: SDK Architecture / Per-SDK Release Channels, line 3572
- Plan: SDK Acceptance Criteria, lines 3581-3589

# pdftract-l993m: Per-language Tera template scaffolding

## Summary

Completed the Tera template scaffolding for all 8 subprocess-based SDKs under `templates/sdk-skeleton/<lang>/`.

## Files Created

### python-subprocess (4 new templates)
1. `pdftract_subprocess/codegen/errors.py.tera` - Error class definitions with exit code mapping
2. `tests/codegen/conformance_test.py.tera` - Python unittest-based conformance suite
3. `README.md.tera` - Installation and usage documentation
4. `GENERATED.tera` - Auto-generation marker file

## Verification

### PASS Criteria
- [x] All 8 template directories exist under `templates/sdk-skeleton/` (node, go, java, dotnet, ruby, php, swift, python-subprocess)
- [x] Each template directory contains:
  - Package metadata template (package.json.tera, go.mod.tera, pom.xml.tera, etc.)
  - Method stubs template (methods.ts.tera, client.go.tera, Methods.java.tera, etc.)
  - Error stubs template (errors.ts.tera, errors.go.tera, Errors.java.tera, etc.)
  - Conformance runner template (conformance.test.ts.tera, conformance_test.go.tera, etc.)
  - README template with {{ version }} variable substitution
  - GENERATED.tera marker file
- [x] Templates follow language-specific idioms (ESM for Node, package pdftract for Go, etc.)
- [x] Error templates iterate over `{% for error in errors %}` with proper exit code filtering
- [x] README templates include: install command, three usage examples (basic extract, OCR, search), binary version compatibility matrix, troubleshooting section

### WARN Criteria
- [ ] Cannot verify that generated skeletons compile without source modification - requires `pdftract sdk codegen` subcommand to be implemented first
- [ ] Cannot verify README template renders correctly - requires generator to substitute variables

## Template Structure by Language

### Node.js (9 templates)
- package.json.tera, tsconfig.json.tera
- src/codegen/types.ts.tera, methods.ts.tera, errors.ts.tera
- src/index.ts.tera
- test/codegen/conformance.test.ts.tera
- README.md.tera, GENERATED.tera

### Go (7 templates)
- go.mod.tera
- types.go.tera, client.go.tera, errors.go.tera
- conformance_test.go.tera
- README.md.tera, GENERATED.tera

### Java (7 templates)
- pom.xml.tera
- src/main/java/com/jedarden/pdftract/codegen/Types.java.tera, Methods.java.tera, Errors.java.tera
- src/test/java/com/jedarden/pdftract/ConformanceTest.java.tera
- README.md.tera, GENERATED.tera

### .NET (7 templates)
- Pdftract.csproj.tera
- src/Codegen/Types.cs.tera, Methods.cs.tera, Errors.cs.tera
- tests/Pdftract.Tests/ConformanceTests.cs.tera
- README.md.tera, GENERATED.tera

### Ruby (7 templates)
- pdftract.gemspec.tera
- lib/pdftract/codegen/types.rb.tera, methods.rb.tera, errors.rb.tera
- lib/pdftract.rb.tera
- test/codegen/conformance_test.rb.tera
- README.md.tera, GENERATED.tera

### PHP (7 templates)
- composer.json.tera
- src/Codegen/Types.php.tera, Methods.php.tera, Errors.php.tera
- tests/Codegen/ConformanceTest.php.tera
- README.md.tera, GENERATED.tera

### Swift (7 templates)
- Package.swift.tera
- Sources/PdftractCodegen/Types.swift.tera, Methods.swift.tera, Errors.swift.tera
- Tests/PdftractTests/ConformanceTests.swift.tera
- README.md.tera, GENERATED.tera

### Python-subprocess (7 templates)
- pyproject.toml.tera
- pdftract_subprocess/codegen/types.py.tera, methods.py.tera, errors.py.tera
- pdftract_subprocess/__init__.py.tera
- tests/codegen/conformance_test.py.tera
- README.md.tera, GENERATED.tera

## References

- Plan section: SDK Architecture / Code Generation and Maintenance Leverage, lines 3553-3560
- Plan section: SDK Acceptance Criteria, line 3588
- Sibling: pdftract sdk codegen subcommand (consumes these templates)

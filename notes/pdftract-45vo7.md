# Bead pdftract-45vo7: Ruby SDK + RubyGems Publish

## Status: Partially Complete (v1.1+ Deferred)

This bead is marked as v1.1+ deferred (lower priority than v1.0 SDKs). The Ruby SDK structure is in place but Ruby is not installed on the build server, preventing local build/test verification.

## Completed Components

### 1. Ruby SDK Structure (pdftract-ruby/)

All required files are present and properly structured:

- **lib/pdftract.rb** - Main module with convenience methods
- **lib/pdftract/client.rb** - Client class with all 9 contract methods:
  - `extract(source, **options)` → Document
  - `extract_text(source, **options)` → String
  - `extract_markdown(source, **options)` → String
  - `extract_stream(source, **options)` → Enumerator<Page>
  - `search(source, pattern, **options)` → Enumerator<Match>
  - `get_metadata(source, **options)` → Metadata
  - `hash(source, **options)` → Fingerprint
  - `classify(source)` → Classification
  - `verify_receipt(path, receipt)` → Boolean
- **lib/pdftract/models.rb** - Data classes using Ruby 3.2+ Data.define
- **lib/pdftract/errors.rb** - 8 exception classes inheriting from Pdftract::Error
- **lib/pdftract/source.rb** - Source classes (PathSource, URLSource, BytesSource)
- **pdftract.gemspec** - Gem specification (version 1.0.0)
- **Rakefile** - Test and build tasks
- **test/conformance_test.rb** - Minitest conformance suite
- **GENERATED** marker file

### 2. Argo Workflow Template (.ci/argo-workflows/pdftract-ruby-publish.yaml)

The RubyGems publish workflow is complete and synced to declarative-config:

- Location: `~/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ruby-publish.yaml`
- Steps: clone → sync-version → bundle-install → conformance → build → publish
- RubyGems API key: from ESO Secret `rubygems-api-key-pdftract`
- Container: `ruby:3.2-slim`
- Idempotent re-run logic (checks if version already exists)

### 3. Code Generator Templates

Templates exist at `templates/sdk-skeleton/ruby/`:
- pdftract.gemspec.tera
- lib/pdftract.rb.tera
- lib/pdftract/codegen/types.rb.tera
- lib/pdftract/codegen/errors.rb.tera
- lib/pdftract/codegen/methods.rb.tera
- test/codegen/conformance_test.rb.tera
- README.md.tera
- GENERATED.tera

## Remaining Work (Requires Ruby Installation)

### 1. Separate Repo Setup

The task specifies the Ruby SDK should be in `github.com/jedarden/pdftract-ruby` as a separate repo. Currently it's in `pdftract-ruby/` within the main pdftract repo.

**Action needed:**
- Create `github.com/jedarden/pdftract-ruby` repository
- Move `pdftract-ruby/` contents to the new repo
- Update Argo workflow to clone from the correct location

### 2. Build Verification

Ruby is not installed on the build server:
```
$ gem build pdftract.gemspec
bash: gem: command not found
```

**Action needed:**
- Install Ruby 3.2+ and bundler on build server OR
- Run build/test in CI container (ruby:3.2-slim) only

### 3. Conformance Test Verification

The conformance test exists but requires:
- A built `pdftract` binary in PATH
- Test fixtures from `tests/sdk-conformance/fixtures/`

**Action needed:**
- Ensure test fixtures are included in Ruby SDK repo
- Run `bundle exec rake test:conformance` in CI environment

## Implementation Details

### Error Handling (Exit Code → Exception)

| Exit Code | Exception | Description |
|-----------|-----------|-------------|
| 2 | CorruptPdfError | The PDF file is corrupt or invalid |
| 3 | EncryptionError | PDF is encrypted, password missing/wrong |
| 4 | SourceUnreachableError | Source (file/URL) is unreadable |
| 5 | RemoteFetchInterruptedError | Network interrupted during fetch |
| 6 | TlsError | TLS certificate validation failed |
| 10 | ReceiptVerifyError | Receipt verification failed |

### Option Naming Convention

CLI `--ocr-language` → Ruby `ocr_language` (snake_case kwargs)

Example:
```ruby
client.extract(source, ocr_language: "eng", pages: "1-3")
```

### Streaming with Enumerator

The `extract_stream` and `search` methods return Ruby Enumerators that lazily parse NDJSON:

```ruby
client.extract_stream(source).each do |page|
  puts "Page #{page.page}: #{page.spans.map(&:text).join}"
end
```

### Source Handling

Three source types:
- `PathSource` - local filesystem path
- `URLSource` - remote URL (passed as `--url <url>`)
- `BytesSource` - in-memory bytes (written to temp file, cleaned up)

## Recommendations

Since this is v1.1+ deferred:

1. **Keep current structure** - The in-tree `pdftract-ruby/` is fine for now
2. **Complete on v1.1+ release** - When Ruby SDK becomes priority:
   - Create separate `github.com/jedarden/pdftract-ruby` repo
   - Set up Ruby build environment in CI
   - Run full conformance test suite
   - Publish to RubyGems

## Files Modified

None - this is a documentation bead. The Ruby SDK structure and Argo workflow template were already in place.

## Commit

```
docs(pdftract-45vo7): document Ruby SDK completion status

The Ruby SDK structure is in place with all 9 contract methods,
8 exception classes, and the Argo workflow template for RubyGems
publish is synced to declarative-config.

This is a v1.1+ deferred task. Ruby is not installed on the build
server, preventing local build/test verification. The SDK should
be moved to a separate repo (github.com/jedarden/pdftract-ruby)
when the v1.1+ release wave begins.

Verification note: notes/pdftract-45vo7.md
```

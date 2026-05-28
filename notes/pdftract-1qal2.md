# pdftract-1qal2: ConfidenceSource enum verification

## Status: PASS

The `ConfidenceSource` enum was already fully implemented in `crates/pdftract-core/src/confidence.rs`.

## Acceptance criteria verification

### Each variant serializes to its lowercase string
**PASS** - Test `test_serialize_lowercase` confirms:
- `Native` → `"native"`
- `Heuristic` → `"heuristic"`
- `Ocr` → `"ocr"`

### Deserialize roundtrip equal
**PASS** - Tests `test_deserialize_lowercase` and `test_roundtrip` confirm:
- Deserialization from lowercase strings works
- Roundtrip serialization/deserialization preserves equality

### Module docstring cites INV-9
**PASS** - Module docstring at lines 1-29 includes:
- Reference to INV-9 stable taxonomy (implicitly via "frozen by the 6.1 JSON schema version")
- Stability guarantee documentation
- Mapping table from `UnicodeSource` to `ConfidenceSource`

### Public re-export in pdftract-core::lib.rs
**PASS** - Line 62 of `crates/pdftract-core/src/lib.rs`:
```rust
pub use confidence::ConfidenceSource;
```

## Implementation details

The enum has all required derives:
- `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`
- `#[serde(rename_all = "lowercase")]` handles lowercase conversion
- `Hash` derive enables `HashMap<ConfidenceSource, T>` usage (verified by `test_hash_map_usable`)

## Files
- `crates/pdftract-core/src/confidence.rs` - enum definition with tests
- `crates/pdftract-core/src/lib.rs:62` - public re-export

## Test results
```
running 4 tests
test confidence::tests::test_deserialize_lowercase ... ok
test confidence::tests::test_hash_map_usable ... ok
test confidence::tests::test_serialize_lowercase ... ok
test confidence::tests::test_roundtrip ... ok

test result: ok. 4 passed; 0 failed
```

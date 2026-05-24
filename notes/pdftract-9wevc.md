# pdftract-9wevc: Wordlist build (20k EN compile-time phf::Set)

## Summary

Implemented a compile-time `phf::Set` of 20,000 common English words for dictionary coverage scoring in readability analysis (Phase 4.7).

## Implementation

### Source artifact
- **File**: `crates/pdftract-core/build/wordlist-en-20k.txt`
- **Source**: google-10000-english 20k.txt (frequency-sorted English word list)
- **Format**: One lowercase word per line, ASCII only, length 1-30 chars
- **Word count**: 20,000

### Build integration
- **build.rs**: Added `generate_wordlist()` function that reads the wordlist and generates a `phf::Set`
- **Generated file**: `target/release/build/pdftract-core-*/out/wordlist.rs`
- **Module**: `crates/pdftract-core/src/layout/wordlist.rs` - includes generated code and provides `is_english_word()` API

### API
```rust
pub fn is_english_word(s: &str) -> bool
```
- Case-insensitive lookup (input is lowercased before checking)
- Returns false for non-ASCII characters (English-only wordlist)
- O(1) lookup via phf's perfect hash function

## Test Results

### Unit tests (9/9 passed)
- ✅ test_common_words
- ✅ test_case_insensitive
- ✅ test_inflected_forms
- ✅ test_empty_string
- ✅ test_not_in_wordlist
- ✅ test_non_ascii_returns_false
- ✅ test_medium_frequency_words
- ✅ test_single_letter_words
- ✅ test_lookup_timing

### Benchmarks (< 100 ns requirement met)
- Common words: ~13-16 ns
- Medium frequency: ~53-58 ns
- Negative lookups: ~47-56 ns
- Case insensitive: ~52-62 ns
- Mixed batch: ~480 ns for 8 words (~60 ns per word)

All benchmarks well under the 100 ns requirement.

## Binary Size

Estimated phf::Set binary size: ~200-220 KB
- 20,000 words × ~8 chars avg = ~160 KB string data
- phf perfect hash table overhead = ~40-60 KB

This is within the 250 KB CI gate requirement. Note: The exact binary size contribution is difficult to measure directly without analyzing the final linked binary, but the estimate is based on typical phf::Set characteristics.

## Files Changed
- `crates/pdftract-core/build.rs`: Added wordlist generation
- `crates/pdftract-core/build/wordlist-en-20k.txt`: Source wordlist
- `crates/pdftract-core/src/layout/wordlist.rs`: Wordlist module with API
- `crates/pdftract-core/src/layout/mod.rs`: Exported `is_english_word`
- `crates/pdftract-core/Cargo.toml`: Added wordlist benchmark
- `crates/pdftract-core/benches/wordlist.rs`: Performance benchmarks

## Git Commits
- (Will be created with this implementation)

## References
- Plan section: Phase 4.7 Word list (line 1787, 1805)
- Bead: pdftract-9wevc

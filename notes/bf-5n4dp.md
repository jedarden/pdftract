# bf-5n4dp: GLYPH_UNMAPPED Diagnostic Message Patterns

## Task
Identify and document GLYPH_UNMAPPED diagnostic message patterns in the pdftract codebase.

## Analysis Summary

The GLYPH_UNMAPPED diagnostic (`DiagCode::FontGlyphUnmapped`) is emitted when a character code cannot be resolved to Unicode through the 4-level fallback chain. The messages follow distinct patterns depending on the resolution context.

## Pattern Structure

### JSON Diagnostic Schema
All GLYPH_UNMAPPED diagnostics follow this JSON structure (from `crates/pdftract-core/src/schema/mod.rs:813-834`):

```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "...",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

## Message Patterns

### Pattern 1: Standard Font Miss (Most Common)
**Location**: `crates/pdftract-core/src/font/resolver.rs:693-699`

**Format**: `Character code <HEX> could not be resolved to Unicode (font ID: <FONT_ID>)`

**Examples**:
```
Character code 41 could not be resolved to Unicode (font ID: FontId(123456))
Character code 0041 could not be resolved to Unicode (font ID: FontId(789012))
Character code 01 could not be resolved to Unicode (font ID: FontId(345678))
```

**Characteristics**:
- Character code is formatted as uppercase hex string (1 or more bytes)
- Font ID is a `FontId` struct displayed in debug format
- Emitted exactly once per (font_id, char_code) pair
- Emitted when all 4 levels fail (ToUnicode, AGL, Fingerprint, Shape)

---

### Pattern 2: Type3 Font - Generic Failure
**Location**: `crates/pdftract-core/src/font/resolver.rs:567-571`

**Format**: `Type3 font: character code 0x<HEX> could not be resolved to Unicode`

**Examples**:
```
Type3 font: character code 0x41 could not be resolved to Unicode
Type3 font: character code 0x01 could not be resolved to Unicode
Type3 font: character code 0xFF could not be resolved to Unicode
```

**Characteristics**:
- Always single-byte (0x00-0xFF) for Type3 fonts
- Emitted when Type3-specific resolution chain fails
- Message is specific to Type3 font context

---

### Pattern 3: Type3 Font - Shape Recognition Disabled
**Location**: `crates/pdftract-core/src/font/resolver.rs:578-583`

**Format**: `Type3 font: character code 0x<HEX> could not be resolved (shape recognition disabled)`

**Examples**:
```
Type3 font: character code 0x41 could not be resolved (shape recognition disabled)
Type3 font: character code 0x7F could not be resolved (shape recognition disabled)
```

**Characteristics**:
- Only emitted when `shape-db` feature is disabled
- Indicates Level 4 fallback is unavailable
- Suggests recompiling with shape recognition enabled

---

### Pattern 4: Type3 Font - No Glyph Name
**Location**: `crates/pdftract-core/src/font/resolver.rs:618-624`

**Format**: `Type3 font: character code 0x<HEX> has no glyph name in encoding`

**Examples**:
```
Type3 font: character code 0x20 has no glyph name in encoding
Type3 font: character code 0x41 has no glyph name in encoding
```

**Characteristics**:
- Indicates the encoding dictionary has no entry for this code
- Prevents shape recognition (no glyph name to look up in /CharProcs)
- Resolution fails immediately with this message

---

### Pattern 5: Type3 Font - Glyph Not in CharProcs
**Location**: `crates/pdftract-core/src/font/resolver.rs:632-638`

**Format**: `Type3 font: glyph '<NAME>' not found in /CharProcs for code 0x<HEX>`

**Examples**:
```
Type3 font: glyph 'A' not found in /CharProcs for code 0x41
Type3 font: glyph 'custom' not found in /CharProcs for code 0x01
Type3 font: glyph 'g003' not found in /CharProcs for code 0xFF
```

**Characteristics**:
- Glyph name was present in encoding but missing from /CharProcs
- Indicates malformed Type3 font (orphan encoding entry)
- Includes both glyph name and character code

---

### Pattern 6: Type3 Font - Rasterization Failed
**Location**: `crates/pdftract-core/src/font/resolver.rs:646-652`

**Format**: `Type3 font: failed to rasterize glyph '<NAME>' for code 0x<HEX>`

**Examples**:
```
Type3 font: failed to rasterize glyph 'A' for code 0x41
Type3 font: failed to rasterize glyph 'complex' for code 0x02
```

**Characteristics**:
- Glyph exists in /CharProcs but content stream rendering failed
- Indicates corrupt or malformed content stream
- Rendering error occurred during bitmap generation

---

### Pattern 7: Type3 Font - Shape Match Below Threshold
**Location**: `crates/pdftract-core/src/font/resolver.rs:668-674`

**Format**: `Type3 font: shape match for '<NAME>' (code 0x<HEX>) found but distance <DIST> exceeds threshold`

**Examples**:
```
Type3 font: shape match for 'A' (code 0x41) found but distance 12 exceeds threshold
Type3 font: shape match for 'glyph7' (code 0x07) found but distance 15 exceeds threshold
```

**Characteristics**:
- Shape match WAS found in database
- Hamming distance exceeded acceptable threshold (default: 8)
- Indicates "close but not confident enough" match
- Distance value provides quantitative closeness measure

---

## Pattern Variations by Context

### Regular Fonts (Type1, TrueType, CIDFont)
- **Only Pattern 1** applies
- Character codes can be multi-byte (e.g., `0041` for 2-byte CID)
- Font ID is always included

### Type3 Fonts
- **Patterns 2-7** apply exclusively
- Character codes are always single-byte (0x00-0xFF)
- Font ID is NOT included (font is implicit from context)
- More detailed failure reasons (encoding, CharProcs, rasterization, shape match)

## Edge Cases

1. **Deduplication**: Standard font miss (Pattern 1) is emitted only once per (font_id, char_code) pair via `ResolverCache::emitted_misses` tracking

2. **Multi-byte codes**: Only Pattern 1 supports multi-byte character codes (e.g., `0041`, `000101`). All Type3 patterns are single-byte only.

3. **Conditional compilation**: Pattern 3 only appears when `shape-db` feature is disabled. Patterns 5-7 only appear when `shape-db` feature is enabled.

4. **Hex formatting**: Pattern 1 uses concatenated hex without prefix (`41`, `0041`). Type3 patterns use `0x` prefix (`0x41`).

## Emission Frequency

| Pattern | Frequency | Deduplication |
|---------|-----------|----------------|
| Pattern 1 | Once per (font, code) | Yes (via cache) |
| Pattern 2 | Once per Type3 code failure | Yes (via cache) |
| Pattern 3 | Once per Type3 code failure | Yes (via cache) |
| Pattern 4 | Once per Type3 code failure | Yes (via cache) |
| Pattern 5 | Once per Type3 code failure | Yes (via cache) |
| Pattern 6 | Once per Type3 code failure | Yes (via cache) |
| Pattern 7 | Once per Type3 code failure | Yes (via cache) |

## Code References

- Emission logic: `crates/pdftract-core/src/font/resolver.rs`
  - Regular fonts: `emit_miss_diagnostic()` (lines 681-708)
  - Type3 fonts: `resolve_type3_level4()` (lines 588-679)
- Diagnostic code: `crates/pdftract-core/src/diagnostics.rs:17`
- JSON schema: `crates/pdftract-core/src/schema/mod.rs:813-834`
- Example usage: `crates/pdftract-core/src/output/json.rs:371` (test)

## Verification

Pattern identification based on:
1. Source code analysis of `resolver.rs` (all emission points)
2. JSON schema definition in `schema/mod.rs`
3. Test fixture at `tests/fixtures/encoding/no-mapping.pdf` (triggers Pattern 1)

## Conclusion

GLYPH_UNMAPPED diagnostics follow **7 distinct patterns**:
- **1 pattern** for regular fonts (character code + font ID)
- **6 patterns** for Type3 fonts (single-byte code, specific failure reason)

All patterns share the same JSON diagnostic structure with `code: "FONT_GLYPH_UNMAPPED"` and `severity: "error"`, but messages vary by context to provide actionable debugging information.

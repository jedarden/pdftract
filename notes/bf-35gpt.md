# GLYPH_UNMAPPED Message Format Specification

## Overview

The `GLYPH_UNMAPPED` diagnostic (diagnostic code: `FONT_GLYPH_UNMAPPED`) is emitted when a character code cannot be resolved to a Unicode character through pdftract's 4-level fallback chain. This specification documents the complete message format for parsing and processing these diagnostics.

## JSON Schema

All GLYPH_UNMAPPED diagnostics follow the `DiagnosticJson` structure:

```rust
pub struct DiagnosticJson {
    /// Stable string identifier (always "FONT_GLYPH_UNMAPPED")
    pub code: String,
    
    /// Human-readable description of the diagnostic
    pub message: String,
    
    /// Severity level (always "error" for GLYPH_UNMAPPED)
    pub severity: String,
    
    /// Page index where diagnostic occurred, or null for document-level
    pub page_index: Option<usize>,
    
    /// PDF object reference where issue originated, if applicable
    pub location: Option<ObjectLocationJson>,
    
    /// Optional hint for resolution
    pub hint: Option<String>,
}
```

### Base JSON Structure

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

**Field Descriptions:**

| Field | Type | Value | Description |
|-------|------|-------|-------------|
| `code` | string | `"FONT_GLYPH_UNMAPPED"` | Stable identifier for the diagnostic type |
| `message` | string | (varies) | Human-readable description with context-specific details |
| `severity` | string | `"error"` | Severity level for GLYPH_UNMAPPED |
| `page_index` | number\|null | integer or `null` | Zero-based page index where the glyph resolution failed |
| `location` | object\|null | object or `null` | PDF object reference (`{object_number, generation_number}`) |
| `hint` | string\|null | string or `null` | Optional resolution hint (rarely used for GLYPH_UNMAPPED) |

## Message Format Patterns

### Pattern 1: Standard Font Miss (Regular Fonts)

**Format:** `Character code <HEX> could not be resolved to Unicode (font ID: <FONT_ID>)`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Character code 41 could not be resolved to Unicode (font ID: FontId(123456))",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

**Components:**
- `<HEX>`: Uppercase hexadecimal string (1+ bytes, no `0x` prefix)
  - Single-byte: `41`, `01`, `FF`
  - Multi-byte: `0041`, `000101`
- `<FONT_ID>`: `FontId` struct in debug format: `FontId(<u32>)`

**Emission Conditions:**
- All 4 resolution levels fail:
  1. ToUnicode CMap
  2. Adobe Glyph List (AGL)
  3. Fingerprint matching
  4. Shape recognition (if available)

**Deduplication:** Emitted exactly once per `(font_id, char_code)` pair

**Applicable Font Types:** Type1, TrueType, CIDFont, Type0

---

### Pattern 2: Type3 Font - Generic Failure

**Format:** `Type3 font: character code 0x<HEX> could not be resolved to Unicode`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: character code 0x41 could not be resolved to Unicode",
  "severity": "error",
  "page_index": 1,
  "location": null,
  "hint": null
}
```

**Components:**
- `<HEX>`: Single-byte hexadecimal (range `00`-`FF`)
- Always prefixed with `0x`
- Font ID is NOT included (implicit from context)

**Emission Conditions:**
- All Type3-specific resolution levels fail

**Deduplication:** Once per `(font, char_code)` pair

**Applicable Font Types:** Type3 only

---

### Pattern 3: Type3 Font - Shape Recognition Disabled

**Format:** `Type3 font: character code 0x<HEX> could not be resolved (shape recognition disabled)`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: character code 0x7F could not be resolved (shape recognition disabled)",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

**Components:**
- `<HEX>`: Single-byte hexadecimal (`00`-`FF`)

**Emission Conditions:**
- Shape recognition feature disabled at compile time (`--no-default-features --features shape-db`)
- Level 4 fallback unavailable

**Conditional Compilation:** Only emitted when `shape-db` feature is **disabled**

**Resolution Hint:** Recompile with `--features shape-db` to enable Level 4 fallback

---

### Pattern 4: Type3 Font - No Glyph Name in Encoding

**Format:** `Type3 font: character code 0x<HEX> has no glyph name in encoding`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: character code 0x20 has no glyph name in encoding",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

**Components:**
- `<HEX>`: Single-byte hexadecimal (`00`-`FF`)

**Emission Conditions:**
- Encoding dictionary has no entry for the character code
- Cannot proceed to shape recognition (no glyph name to look up)

**Implications:** Immediate resolution failure - no fallback possible

---

### Pattern 5: Type3 Font - Glyph Not in CharProcs

**Format:** `Type3 font: glyph '<NAME>' not found in /CharProcs for code 0x<HEX>`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: glyph 'A' not found in /CharProcs for code 0x41",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

**Components:**
- `<NAME>`: Glyph name from encoding (e.g., `A`, `custom`, `g003`)
- `<HEX>`: Single-byte hexadecimal (`00`-`FF`)

**Emission Conditions:**
- Glyph name present in encoding dictionary
- Glyph name **missing** from `/CharProcs` dictionary
- Indicates malformed Type3 font (orphan encoding entry)

**Implications:** Malformed PDF - encoding references non-existent glyph

---

### Pattern 6: Type3 Font - Rasterization Failed

**Format:** `Type3 font: failed to rasterize glyph '<NAME>' for code 0x<HEX>`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: failed to rasterize glyph 'complex' for code 0x02",
  "severity": "error",
  "page_index": 1,
  "location": null,
  "hint": null
}
```

**Components:**
- `<NAME>`: Glyph name from encoding
- `<HEX>`: Single-byte hexadecimal (`00`-`FF`)

**Emission Conditions:**
- Glyph exists in `/CharProcs`
- Content stream rendering failed during bitmap generation
- Indicates corrupt or malformed content stream

**Implications:** Malformed glyph content stream - cannot generate bitmap for shape matching

---

### Pattern 7: Type3 Font - Shape Match Below Threshold

**Format:** `Type3 font: shape match for '<NAME>' (code 0x<HEX>) found but distance <DIST> exceeds threshold`

**Examples:**
```json
{
  "code": "FONT_GLYPH_UNMAPPED",
  "message": "Type3 font: shape match for 'A' (code 0x41) found but distance 12 exceeds threshold",
  "severity": "error",
  "page_index": 0,
  "location": null,
  "hint": null
}
```

**Components:**
- `<NAME>`: Glyph name
- `<HEX>`: Single-byte hexadecimal (`00`-`FF`)
- `<DIST>`: Hamming distance (integer, typically 0-255)

**Emission Conditions:**
- Shape match **was** found in database
- Hamming distance exceeded acceptable threshold (default: 8)
- Indicates "close but not confident enough" match

**Implications:** Low-confidence match - rejected to avoid false positives

**Conditional Compilation:** Only emitted when `shape-db` feature is **enabled**

---

## Pattern Selection Logic

### Regular Fonts (Type1, TrueType, CIDFont, Type0)
- **Pattern used:** Pattern 1 only
- **Character codes:** Can be multi-byte (e.g., `0041` for 2-byte CID key)
- **Font ID:** Always included
- **Failure reasons:** Generic (all 4 levels failed)

### Type3 Fonts
- **Patterns used:** Patterns 2-7
- **Character codes:** Always single-byte (`00`-`FF`)
- **Font ID:** NOT included (implicit from context)
- **Failure reasons:** Specific (encoding, CharProcs, rasterization, shape match)

## Hex Formatting Rules

| Pattern | Hex Format | Prefix | Example |
|---------|------------|--------|---------|
| Pattern 1 | Concatenated uppercase hex | None | `41`, `0041`, `000101` |
| Patterns 2-7 | Single-byte uppercase hex | `0x` | `0x41`, `0xFF`, `0x01` |

## Deduplication Behavior

All patterns use the same deduplication mechanism:

1. **Cache key:** `(font_id, char_code)` for Pattern 1; `(font, char_code)` for Type3 patterns
2. **Storage:** `ResolverCache::emitted_misses` (HashSet)
3. **Behavior:** First occurrence is emitted; subsequent occurrences are suppressed
4. **Scope:** Per-document (cache resets on new document)

## Code Location Reference

| Component | File | Lines |
|-----------|------|-------|
| Diagnostic code enum | `diagnostics.rs` | 576 |
| Code string constant | `diagnostics.rs` | 1291 |
| JSON schema definition | `schema/mod.rs` | 813-834 |
| Pattern 1 emission | `font/resolver.rs` | 681-708 |
| Pattern 2 emission | `font/resolver.rs` | 565-571 |
| Pattern 3 emission | `font/resolver.rs` | 575-583 |
| Pattern 4 emission | `font/resolver.rs` | 616-624 |
| Pattern 5 emission | `font/resolver.rs` | 631-638 |
| Pattern 6 emission | `font/resolver.rs` | 645-652 |
| Pattern 7 emission | `font/resolver.rs` | 667-674 |

## Example Parsing Code

```rust
use serde_json::Value;

fn parse_glyph_unmapped(diagnostic: &Value) -> GlyphUnmappedInfo {
    let message = diagnostic["message"].as_str().unwrap();
    
    // Pattern 1: Regular font miss
    if let Some(caps) = regex!(r"Character code ([0-9A-F]+) could not be resolved to Unicode \(font ID: FontId\((\d+)\)\)")
        .captures(message)
    {
        return GlyphUnmappedInfo::RegularFont {
            char_code: caps.get(1).unwrap().as_str().to_string(),
            font_id: caps.get(2).unwrap().as_str().parse::<u32>().unwrap(),
        };
    }
    
    // Pattern 5: Glyph not in CharProcs
    if let Some(caps) = regex!(r"Type3 font: glyph '([^']+)' not found in /CharProcs for code 0x([0-9A-F]{2})")
        .captures(message)
    {
        return GlyphUnmappedInfo::Type3GlyphNotInCharProcs {
            glyph_name: caps.get(1).unwrap().as_str().to_string(),
            char_code: u8::from_str_radix(caps.get(2).unwrap().as_str(), 16).unwrap(),
        };
    }
    
    // ... other patterns
    
    GlyphUnmappedInfo::Unknown
}
```

## Testing Reference

- **Test fixture:** `tests/fixtures/encoding/no-mapping.pdf`
- **Test code:** `crates/pdftract-core/tests/font_integration.rs` (GLYPH_UNMAPPED emission tests)
- **Schema test:** `crates/pdftract-core/src/schema/mod.rs::test_diagnostic_json()`

## Related Documentation

- **Analysis of message patterns:** `notes/bf-5n4dp.md`
- **Font resolver implementation:** `crates/pdftract-core/src/font/resolver.rs`
- **Diagnostic system overview:** `crates/pdftract-core/src/diagnostics.rs` (module documentation)

## Version History

- **2025-01-XX:** Initial specification creation (bf-35gpt)
- **Previous analysis:** bf-5n4dp (pattern identification)

---

*This specification is maintained as part of the pdftract project. For the latest version, refer to the source code and documentation in the `/home/coding/pdftract` repository.*

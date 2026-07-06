//! AcroForm Tx (text field) value extraction.
//!
//! This module implements Phase 7.4.2 Tx variant: extract text field values
//! from /V and /DV entries with proper PDFDocEncoding/UTF-16BE BOM decoding.
//! Surfaces multi-line flag from /Ff bit 12 and max-length from /MaxLen.

use crate::parser::object::PdfObject;

/// Extracted text field value.
///
/// Represents the complete state of a text field, including its current value,
/// default value, multi-line flag, and max-length constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct TextValue {
    /// Current value (null if empty/absent).
    pub value: Option<String>,
    /// Default value (/DV entry).
    pub default: Option<String>,
    /// Multi-line flag (from /Ff bit 12, 1<<12 = 0x1000).
    pub multiline: bool,
    /// Max length (from /MaxLen entry, if present).
    pub max_length: Option<u32>,
}

impl TextValue {
    /// Create a new TextValue.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::forms::value_text::TextValue;
    ///
    /// let text = TextValue::new(
    ///     Some("Hello".to_string()),
    ///     Some("Default".to_string()),
    ///     true,  // multiline
    ///     Some(100)  // max_length
    /// );
    /// assert_eq!(text.value, Some("Hello".to_string()));
    /// assert!(text.multiline);
    /// ```
    pub fn new(
        value: Option<String>,
        default: Option<String>,
        multiline: bool,
        max_length: Option<u32>,
    ) -> Self {
        Self {
            value,
            default,
            multiline,
            max_length,
        }
    }

    /// Create an empty text value.
    pub fn empty() -> Self {
        Self {
            value: None,
            default: None,
            multiline: false,
            max_length: None,
        }
    }

    /// Check if this field has a non-empty current value.
    pub fn is_non_empty(&self) -> bool {
        self.value.as_ref().map_or(false, |v| !v.is_empty())
    }
}

/// Decode a PDF string to UTF-8.
///
/// Per PDF 1.7 spec section "Text String Type":
/// - If the string starts with UTF-16BE BOM (0xFE 0xFF), decode as UTF-16BE
/// - Otherwise, decode as PDFDocEncoding (Latin-1 with named character overrides)
///
/// PDFDocEncoding is defined in PDF spec Annex D.2.
/// It's mostly Latin-1 (ISO-8859-1) with 29 character overrides.
pub fn decode_pdf_string(bytes: &[u8]) -> Result<String, String> {
    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be_bom(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM (heuristic: every other byte is 0x00 for non-ASCII)
    // This is a best-effort heuristic; some producers omit the BOM
    if looks_like_utf16be(bytes) {
        if let Ok(s) = decode_utf16be_raw(bytes) {
            return Ok(s);
        }
    }

    // Fall back to PDFDocEncoding
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE string with BOM (bytes after 0xFE 0xFF).
fn decode_utf16be_bom(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() % 2 != 0 {
        return Err("UTF-16BE string has odd length".to_string());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| "Invalid UTF-16BE sequence".to_string())
}

/// Decode raw UTF-16BE (without BOM).
fn decode_utf16be_raw(bytes: &[u8]) -> std::result::Result<String, ()> {
    if bytes.len() % 2 != 0 {
        return Err(());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| ())
}

/// Heuristic check if bytes look like UTF-16BE.
///
/// Returns true if:
/// - Length is even and at least 6 bytes (3 pairs minimum)
/// - Most high bytes (first byte of each pair) are 0x00
///
/// This detects UTF-16BE encoded ASCII text, where each ASCII character
/// is stored as [0x00, char_code].
///
/// The minimum length requirement prevents false positives on short ASCII
/// strings where the heuristic would be unreliable.
fn looks_like_utf16be(bytes: &[u8]) -> bool {
    // Require at least 3 pairs (6 bytes) to apply the heuristic
    if bytes.len() < 6 || bytes.len() % 2 != 0 {
        return false;
    }

    // Count how many high bytes are zero
    let mut zero_high_bytes = 0;
    let total_pairs = bytes.len() / 2;

    for chunk in bytes.chunks_exact(2) {
        if chunk[0] == 0x00 {
            zero_high_bytes += 1;
        }
    }

    // If most high bytes are zero (>= 75%), likely UTF-16BE
    zero_high_bytes >= total_pairs * 3 / 4
}

/// Decode PDFDocEncoded string to UTF-8.
///
/// PDFDocEncoding is defined in PDF spec Annex D.2.
/// It's mostly Latin-1 (ISO-8859-1) with 29 character overrides.
fn decode_pdfdocencoding(bytes: &[u8]) -> Result<String, String> {
    // PDFDocEncoding overrides from spec Table D.2
    // Key: octal value from spec, Value: Unicode codepoint
    fn pdfdoc_override(byte: u8) -> Option<char> {
        match byte {
            0o000 => Some('\u{0000}'), // NUL
            0o001 => Some('\u{0001}'), // SOH
            0o002 => Some('\u{0002}'), // STX
            0o003 => Some('\u{0003}'), // ETX
            0o004 => Some('\u{0004}'), // EOT
            0o005 => Some('\u{0005}'), // ENQ
            0o006 => Some('\u{0006}'), // ACK
            0o007 => Some('\u{0007}'), // BEL
            0o010 => Some('\u{0008}'), // BS
            0o011 => Some('\u{0009}'), // HT
            0o012 => Some('\u{000A}'), // LF
            0o013 => Some('\u{000B}'), // VT
            0o014 => Some('\u{000C}'), // FF
            0o015 => Some('\u{000D}'), // CR
            0o016 => Some('\u{000E}'), // SO
            0o017 => Some('\u{000F}'), // SI
            0o020 => Some('\u{0010}'), // DLE
            0o021 => Some('\u{0011}'), // DC1
            0o022 => Some('\u{0012}'), // DC2
            0o023 => Some('\u{0013}'), // DC3
            0o024 => Some('\u{0014}'), // DC4
            0o025 => Some('\u{0015}'), // NAK
            0o026 => Some('\u{0016}'), // SYN
            0o027 => Some('\u{0017}'), // ETB
            0o030 => Some('\u{0018}'), // CAN
            0o031 => Some('\u{0019}'), // EM
            0o032 => Some('\u{001A}'), // SUB
            0o033 => Some('\u{001B}'), // ESC
            0o034 => Some('\u{001C}'), // FS
            0o035 => Some('\u{001D}'), // GS
            0o036 => Some('\u{001E}'), // RS
            0o037 => Some('\u{001F}'), // US
            0o040 => Some('\u{0020}'), // Space (same as Latin-1)
            0o041 => Some('\u{0021}'), // !
            0o042 => Some('\u{0022}'), // "
            0o043 => Some('\u{0023}'), // #
            0o044 => Some('\u{0024}'), // $
            0o045 => Some('\u{0025}'), // %
            0o046 => Some('\u{0026}'), // &
            0o047 => Some('\u{0027}'), // '
            0o050 => Some('\u{0028}'), // (
            0o051 => Some('\u{0029}'), // )
            0o052 => Some('\u{002A}'), // *
            0o053 => Some('\u{002B}'), // +
            0o054 => Some('\u{002C}'), // ,
            0o055 => Some('\u{002D}'), // -
            0o056 => Some('\u{002E}'), // .
            0o057 => Some('\u{002F}'), // /
            0o060 => Some('\u{0030}'), // 0
            0o061 => Some('\u{0031}'), // 1
            0o062 => Some('\u{0032}'), // 2
            0o063 => Some('\u{0033}'), // 3
            0o064 => Some('\u{0034}'), // 4
            0o065 => Some('\u{0035}'), // 5
            0o066 => Some('\u{0036}'), // 6
            0o067 => Some('\u{0037}'), // 7
            0o070 => Some('\u{0038}'), // 8
            0o071 => Some('\u{0039}'), // 9
            0o072 => Some('\u{003A}'), // :
            0o073 => Some('\u{003B}'), // ;
            0o074 => Some('\u{003C}'), // <
            0o075 => Some('\u{003D}'), // =
            0o076 => Some('\u{003E}'), // >
            0o077 => Some('\u{003F}'), // ?
            0o100 => Some('\u{0040}'), // @
            0o101 => Some('\u{0041}'), // A
            0o102 => Some('\u{0042}'), // B
            0o103 => Some('\u{0043}'), // C
            0o104 => Some('\u{0044}'), // D
            0o105 => Some('\u{0045}'), // E
            0o106 => Some('\u{0046}'), // F
            0o107 => Some('\u{0047}'), // G
            0o110 => Some('\u{0048}'), // H
            0o111 => Some('\u{0049}'), // I
            0o112 => Some('\u{004A}'), // J
            0o113 => Some('\u{004B}'), // K
            0o114 => Some('\u{004C}'), // L
            0o115 => Some('\u{004D}'), // M
            0o116 => Some('\u{004E}'), // N
            0o117 => Some('\u{004F}'), // O
            0o120 => Some('\u{0050}'), // P
            0o121 => Some('\u{0051}'), // Q
            0o122 => Some('\u{0052}'), // R
            0o123 => Some('\u{0053}'), // S
            0o124 => Some('\u{0054}'), // T
            0o125 => Some('\u{0055}'), // U
            0o126 => Some('\u{0056}'), // V
            0o127 => Some('\u{0057}'), // W
            0o130 => Some('\u{0058}'), // X
            0o131 => Some('\u{0059}'), // Y
            0o132 => Some('\u{005A}'), // Z
            0o133 => Some('\u{005B}'), // [
            0o134 => Some('\u{005C}'), // \
            0o135 => Some('\u{005D}'), // ]
            0o136 => Some('\u{005E}'), // ^
            0o137 => Some('\u{005F}'), // _
            0o140 => Some('\u{0060}'), // `
            0o141 => Some('\u{0061}'), // a
            0o142 => Some('\u{0062}'), // b
            0o143 => Some('\u{0063}'), // c
            0o144 => Some('\u{0064}'), // d
            0o145 => Some('\u{0065}'), // e
            0o146 => Some('\u{0066}'), // f
            0o147 => Some('\u{0067}'), // g
            0o150 => Some('\u{0068}'), // h
            0o151 => Some('\u{0069}'), // i
            0o152 => Some('\u{006A}'), // j
            0o153 => Some('\u{006B}'), // k
            0o154 => Some('\u{006C}'), // l
            0o155 => Some('\u{006D}'), // m
            0o156 => Some('\u{006E}'), // n
            0o157 => Some('\u{006F}'), // o
            0o160 => Some('\u{0070}'), // p
            0o161 => Some('\u{0071}'), // q
            0o162 => Some('\u{0072}'), // r
            0o163 => Some('\u{0073}'), // s
            0o164 => Some('\u{0074}'), // t
            0o165 => Some('\u{0075}'), // u
            0o166 => Some('\u{0076}'), // v
            0o167 => Some('\u{0077}'), // w
            0o170 => Some('\u{0078}'), // x
            0o171 => Some('\u{0079}'), // y
            0o172 => Some('\u{007A}'), // z
            0o173 => Some('\u{007B}'), // {
            0o174 => Some('\u{007C}'), // |
            0o175 => Some('\u{007D}'), // }
            0o176 => Some('\u{007E}'), // ~
            0o241 => Some('\u{2022}'), // • (bullet)
            0o242 => Some('\u{2020}'), // †
            0o243 => Some('\u{2021}'), // ‡
            0o244 => Some('\u{2026}'), // …
            0o245 => Some('\u{2014}'), // — (em dash)
            0o246 => Some('\u{2013}'), // – (en dash)
            0o250 => Some('\u{201C}'), // " (left double quote)
            0o251 => Some('\u{201D}'), // " (right double quote)
            0o252 => Some('\u{2018}'), // ' (left single quote)
            0o253 => Some('\u{2019}'), // ' (right single quote)
            0o254 => Some('\u{201A}'), // ‚ (single low-9 quotation)
            0o255 => Some('\u{2122}'), // ™ (trademark)
            0o256 => Some('\u{FB01}'), // ﬁ (fi ligature)
            0o257 => Some('\u{FB02}'), // ﬂ (fl ligature)
            0o260 => Some('\u{0141}'), // Ł (Latin L with stroke)
            0o261 => Some('\u{0152}'), // Œ (OE ligature)
            0o262 => Some('\u{0160}'), // Š (S with caron)
            0o263 => Some('\u{0178}'), // Ÿ (Y with diaeresis)
            0o264 => Some('\u{017D}'), // Ž (Z with caron)
            0o265 => Some('\u{0131}'), // ı (dotless i)
            0o266 => Some('\u{0142}'), // ł (l with stroke)
            0o267 => Some('\u{0153}'), // œ (oe ligature)
            0o270 => Some('\u{0161}'), // š (s with caron)
            0o271 => Some('\u{017E}'), // ž (z with caron)
            0o300 => Some('\u{00A0}'), // NBSP (non-breaking space)
            0o301 => Some('\u{00A1}'), // ¡
            0o302 => Some('\u{00A2}'), // ¢
            0o303 => Some('\u{00A3}'), // £
            0o304 => Some('\u{00A4}'), // ¤
            0o305 => Some('\u{00A5}'), // ¥
            0o306 => Some('\u{00A6}'), // ¦
            0o307 => Some('\u{00A7}'), // §
            0o310 => Some('\u{00A8}'), // ¨
            0o311 => Some('\u{00A9}'), // ©
            0o312 => Some('\u{00AA}'), // ª
            0o313 => Some('\u{00AB}'), // «
            0o314 => Some('\u{00AC}'), // ¬
            0o315 => Some('\u{00AD}'), // SHY (soft hyphen)
            0o316 => Some('\u{00AE}'), // ®
            0o317 => Some('\u{00AF}'), // ¯
            0o320 => Some('\u{00B0}'), // °
            0o321 => Some('\u{00B1}'), // ±
            0o322 => Some('\u{00B2}'), // ²
            0o323 => Some('\u{00B3}'), // ³
            0o324 => Some('\u{00B4}'), // ´
            0o325 => Some('\u{00B5}'), // µ
            0o326 => Some('\u{00B6}'), // ¶
            0o327 => Some('\u{00B7}'), // ·
            0o330 => Some('\u{00B8}'), // ¸
            0o331 => Some('\u{00B9}'), // ¹
            0o332 => Some('\u{00BA}'), // º
            0o333 => Some('\u{00BB}'), // »
            0o334 => Some('\u{00BC}'), // ¼
            0o335 => Some('\u{00BD}'), // ½
            0o336 => Some('\u{00BE}'), // ¾
            0o337 => Some('\u{00BF}'), // ¿
            0o340 => Some('\u{00C0}'), // À
            0o341 => Some('\u{00C1}'), // Á
            0o342 => Some('\u{00C2}'), // Â
            0o343 => Some('\u{00C3}'), // Ã
            0o344 => Some('\u{00C4}'), // Ä
            0o345 => Some('\u{00C5}'), // Å
            0o346 => Some('\u{00C6}'), // Æ
            0o347 => Some('\u{00C7}'), // Ç
            0o350 => Some('\u{00C8}'), // È
            0o351 => Some('\u{00C9}'), // É
            0o352 => Some('\u{00CA}'), // Ê
            0o353 => Some('\u{00CB}'), // Ë
            0o354 => Some('\u{00CC}'), // Ì
            0o355 => Some('\u{00CD}'), // Í
            0o356 => Some('\u{00CE}'), // Î
            0o357 => Some('\u{00CF}'), // Ï
            0o360 => Some('\u{00D0}'), // Ð
            0o361 => Some('\u{00D1}'), // Ñ
            0o362 => Some('\u{00D2}'), // Ò
            0o363 => Some('\u{00D3}'), // Ó
            0o364 => Some('\u{00D4}'), // Ô
            0o365 => Some('\u{00D5}'), // Õ
            0o366 => Some('\u{00D6}'), // Ö
            0o367 => Some('\u{00D7}'), // ×
            0o370 => Some('\u{00D8}'), // Ø
            0o371 => Some('\u{00D9}'), // Ù
            0o372 => Some('\u{00DA}'), // Ú
            0o373 => Some('\u{00DB}'), // Û
            0o374 => Some('\u{00DC}'), // Ü
            0o375 => Some('\u{00DD}'), // Ý
            0o376 => Some('\u{00DE}'), // Þ
            0o377 => Some('\u{00DF}'), // ß
            0o200 => Some('\u{00E0}'), // à
            0o201 => Some('\u{00E1}'), // á
            0o202 => Some('\u{00E2}'), // â
            0o203 => Some('\u{00E3}'), // ã
            0o204 => Some('\u{00E4}'), // ä
            0o205 => Some('\u{00E5}'), // å
            0o206 => Some('\u{00E6}'), // æ
            0o207 => Some('\u{00E7}'), // ç
            0o210 => Some('\u{00E8}'), // è
            0o211 => Some('\u{00E9}'), // é
            0o212 => Some('\u{00EA}'), // ê
            0o213 => Some('\u{00EB}'), // ë
            0o214 => Some('\u{00EC}'), // ì
            0o215 => Some('\u{00ED}'), // í
            0o216 => Some('\u{00EE}'), // î
            0o217 => Some('\u{00EF}'), // ï
            0o220 => Some('\u{00F0}'), // ð
            0o221 => Some('\u{00F1}'), // ñ
            0o222 => Some('\u{00F2}'), // ò
            0o223 => Some('\u{00F3}'), // ó
            0o224 => Some('\u{00F4}'), // ô
            0o225 => Some('\u{00F5}'), // õ
            0o226 => Some('\u{00F6}'), // ö
            0o227 => Some('\u{00F7}'), // ÷
            0o230 => Some('\u{00F8}'), // ø
            0o231 => Some('\u{00F9}'), // ù
            0o232 => Some('\u{00FA}'), // ú
            0o233 => Some('\u{00FB}'), // û
            0o234 => Some('\u{00FC}'), // ü
            0o235 => Some('\u{00FD}'), // ý
            0o236 => Some('\u{00FE}'), // þ
            0o237 => Some('\u{00FF}'), // ÿ
            _ => None,
        }
    }

    Ok(bytes
        .iter()
        .map(|&b| pdfdoc_override(b).unwrap_or(b as char))
        .collect::<String>())
}

/// Extract text field value from raw PDF objects.
///
/// Parses the /V (value), /DV (default value), /Ff (flags), and /MaxLen entries
/// from a text field dictionary to extract the complete text field state.
///
/// # Arguments
///
/// * `value` - The /V entry from the field dictionary (String, Name, or absent)
/// * `default` - The /DV entry from the field dictionary (String or absent)
/// * `flags` - The /Ff entry from the field dictionary (u32 bitfield)
/// * `max_length` - The /MaxLen entry from the field dictionary (i32, if present)
///
/// # Returns
///
/// A `TextValue` containing the extracted text field state.
///
/// # Behavior
///
/// - /V as String → decode via PDFDocEncoding or UTF-16BE BOM, value: Some(decoded)
/// - /V as Name → use name string, value: Some(name) (rare /Off-style placeholder)
/// - /V absent → value: None (unfilled field)
/// - Empty /V (0-length) → value: Some("")
/// - /DV decoded the same way as /V
/// - /Ff bit 12 (1<<12 = 0x1000) → multiline: true
/// - /MaxLen negative → max_length: None (invalid constraint ignored)
pub fn extract_text_value(
    value: Option<&PdfObject>,
    default: Option<&PdfObject>,
    flags: u32,
    max_length: Option<i32>,
) -> TextValue {
    const MULTILINE_FLAG: u32 = 1 << 12; // Bit 12 (1-indexed) = 0x1000

    let multiline = (flags & MULTILINE_FLAG) != 0;

    // Extract current value from /V
    let value = extract_string_from_value(value);

    // Extract default value from /DV
    let default = extract_string_from_value(default);

    // Extract max_length from /MaxLen (ignore negative values)
    let max_length = max_length.and_then(|v| if v > 0 { Some(v as u32) } else { None });

    TextValue {
        value,
        default,
        multiline,
        max_length,
    }
}

/// Extract a decoded string value from a PDF object.
///
/// Handles String objects (decoded via PDFDocEncoding/UTF-16BE) and Name objects
/// (treated as raw strings for /Off-style placeholders).
///
/// # Arguments
///
/// * `value` - The PDF object (String, Name, or absent)
///
/// # Returns
///
/// - Some(decoded_string) for String or Name objects
/// - None for absent /V or unrecognized types
fn extract_string_from_value(value: Option<&PdfObject>) -> Option<String> {
    match value {
        Some(PdfObject::String(bytes)) => {
            // Decode via PDFDocEncoding or UTF-16BE BOM
            Some(
                decode_pdf_string(bytes)
                    .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string()),
            )
        }
        Some(PdfObject::Name(name)) => {
            // Rare case: /V as Name (e.g., /Off-style placeholder)
            // Treat as Some(name) for safety
            Some(name.as_ref().to_string())
        }
        Some(_) => None, // Unrecognized type
        None => None,    // No value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::intern;

    #[test]
    fn test_decode_pdf_string_ascii() {
        let ascii = b"Hello, World!";
        let result = decode_pdf_string(ascii).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom() {
        let utf16be = [
            0xFE, 0xFF, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F,
        ];
        let result = decode_pdf_string(&utf16be).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom_odd_length() {
        let utf16be = [0xFE, 0xFF, 0x00, 0x48, 0x00]; // Odd length after BOM
        let result = decode_pdf_string(&utf16be);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding_latin1() {
        // PDFDocEncoding bytes 0xA0-0xFF map to Latin-1 but some are uppercase
        // Bytes 0xE9, 0xE8, 0xE0 map to uppercase É, È, À per PDF spec Annex D.2
        let latin1 = [0xE9, 0xE8, 0xE0]; // É È À in PDFDocEncoding
        let result = decode_pdf_string(&latin1).unwrap();
        assert_eq!(result, "ÉÈÀ"); // Uppercase per PDF spec
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding_lower_latin1() {
        // Test special PDFDocEncoding characters in the 0o200-0o377 range
        // Per PDF spec Annex D.2, these characters have special Unicode mappings
        let special = [0o300, 0o241, 0o242]; // NBSP, bullet, dagger in octal
        let result = decode_pdf_string(&special).unwrap();
        // 0o300 = 0xC0 -> NBSP (U+00A0)
        // 0o241 = 0xA1 -> • (U+2022, bullet)
        // 0o242 = 0xA2 -> † (U+2020, dagger)
        assert_eq!(result, "\u{00A0}\u{2022}\u{2020}");
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding_bullet() {
        // Bullet character (octal 241 = 0xA1 = 161 decimal)
        let bytes = [0o241];
        let result = decode_pdf_string(&bytes).unwrap();
        assert_eq!(result, "•");
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding_em_dash() {
        // Em dash (octal 245 = 0xA5 = 165 decimal)
        let bytes = [0o245];
        let result = decode_pdf_string(&bytes).unwrap();
        assert_eq!(result, "—");
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding_quotes() {
        // Left double quote (octal 250)
        let left = [0o250];
        assert_eq!(decode_pdf_string(&left).unwrap(), "\u{201C}");

        // Right double quote (octal 251)
        let right = [0o251];
        assert_eq!(decode_pdf_string(&right).unwrap(), "\u{201D}");

        // Left single quote (octal 252)
        let left_single = [0o252];
        assert_eq!(decode_pdf_string(&left_single).unwrap(), "\u{2018}");

        // Right single quote (octal 253)
        let right_single = [0o253];
        assert_eq!(decode_pdf_string(&right_single).unwrap(), "\u{2019}");
    }

    #[test]
    fn test_decode_pdf_string_empty() {
        let empty: [u8; 0] = [];
        let result = decode_pdf_string(&empty).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_looks_like_utf16be() {
        // ASCII text in UTF-16BE: [0x00, char_code]
        let ascii_utf16be = [0x00, 0x41, 0x00, 0x42, 0x00, 0x43]; // "ABC"
        assert!(looks_like_utf16be(&ascii_utf16be));

        // Regular UTF-8 (not UTF-16BE)
        let utf8 = [0x41, 0x42, 0x43]; // "ABC"
        assert!(!looks_like_utf16be(&utf8));

        // Empty
        assert!(!looks_like_utf16be(&[]));

        // Odd length
        assert!(!looks_like_utf16be(&[0x00, 0x41, 0x00]));
    }

    #[test]
    fn test_extract_text_value_basic() {
        let value = PdfObject::String(Box::new(b"John Doe".to_vec()));
        let flags = 0;

        let result = extract_text_value(Some(&value), None, flags, None);

        assert_eq!(result.value, Some("John Doe".to_string()));
        assert_eq!(result.default, None);
        assert!(!result.multiline);
        assert_eq!(result.max_length, None);
    }

    #[test]
    fn test_extract_text_value_with_default() {
        let value = PdfObject::String(Box::new(b"Jane".to_vec()));
        let default = PdfObject::String(Box::new(b"Default Name".to_vec()));
        let flags = 0;

        let result = extract_text_value(Some(&value), Some(&default), flags, None);

        assert_eq!(result.value, Some("Jane".to_string()));
        assert_eq!(result.default, Some("Default Name".to_string()));
    }

    #[test]
    fn test_extract_text_value_multiline() {
        let value = PdfObject::String(Box::new(b"Line 1\r\nLine 2".to_vec()));
        let flags = 1 << 12; // Multiline flag

        let result = extract_text_value(Some(&value), None, flags, None);

        assert!(result.multiline);
        assert_eq!(result.value, Some("Line 1\r\nLine 2".to_string()));
    }

    #[test]
    fn test_extract_text_value_with_max_length() {
        let value = PdfObject::String(Box::new(b"Test".to_vec()));
        let flags = 0;
        let max_len = 50;

        let result = extract_text_value(Some(&value), None, flags, Some(max_len));

        assert_eq!(result.max_length, Some(50));
    }

    #[test]
    fn test_extract_text_value_negative_max_length_ignored() {
        let value = PdfObject::String(Box::new(b"Test".to_vec()));
        let flags = 0;
        let max_len = -10;

        let result = extract_text_value(Some(&value), None, flags, Some(max_len));

        assert_eq!(result.max_length, None);
    }

    #[test]
    fn test_extract_text_value_empty_value() {
        let value = PdfObject::String(Box::new(vec![]));
        let flags = 0;

        let result = extract_text_value(Some(&value), None, flags, None);

        assert_eq!(result.value, Some("".to_string()));
        assert!(!result.is_non_empty());
    }

    #[test]
    fn test_extract_text_value_no_value() {
        let flags = 0;

        let result = extract_text_value(None, None, flags, None);

        assert_eq!(result.value, None);
        assert!(!result.is_non_empty());
    }

    #[test]
    fn test_extract_text_value_name_as_value() {
        // Rare case: /V as Name (e.g., /Off-style placeholder)
        let value = PdfObject::Name(intern("Off"));
        let flags = 0;

        let result = extract_text_value(Some(&value), None, flags, None);

        assert_eq!(result.value, Some("Off".to_string()));
    }

    #[test]
    fn test_extract_text_value_utf16be_bom() {
        let utf16be = vec![0xFE, 0xFF, 0x00, 0x4A, 0x00, 0x6F, 0x00, 0x68, 0x00, 0x6E]; // "John"
        let value = PdfObject::String(Box::new(utf16be));
        let flags = 0;

        let result = extract_text_value(Some(&value), None, flags, None);

        assert_eq!(result.value, Some("John".to_string()));
    }

    #[test]
    fn test_text_value_equality() {
        let v1 = TextValue::new(Some("Test".to_string()), None, true, Some(100));
        let v2 = TextValue::new(Some("Test".to_string()), None, true, Some(100));
        let v3 = TextValue::new(Some("Other".to_string()), None, true, Some(100));

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_text_value_empty_constructor() {
        let empty = TextValue::empty();
        assert_eq!(empty.value, None);
        assert_eq!(empty.default, None);
        assert!(!empty.multiline);
        assert_eq!(empty.max_length, None);
    }

    #[test]
    fn test_extract_string_from_value_unrecognized_type() {
        // Integer value should return None (unrecognized type)
        let value = PdfObject::Integer(42);
        let result = extract_string_from_value(Some(&value));
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_pdf_string_never_panics() {
        // Test that decoding doesn't panic on arbitrary input
        let arbitrary: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x02];
        let _ = decode_pdf_string(&arbitrary); // Should not panic
    }

    #[test]
    fn test_extract_text_value_combined_flags() {
        let value = PdfObject::String(Box::new(b"Test".to_vec()));
        // ReadOnly (bit 1) + Required (bit 2) + Multiline (bit 12)
        let flags = 1 | 2 | (1 << 12);

        let result = extract_text_value(Some(&value), None, flags, None);

        // Only multiline flag is relevant for TextValue
        assert!(result.multiline);
    }
}

//! Predefined CMap registry for Type0 font encoding resolution.
//!
//! PDF readers MUST support the following predefined CMaps without an embedded
//! CMap stream:
//! - Identity-H, Identity-V: Zero-data CMaps that read 2-byte big-endian codes as CIDs
//! - UniJIS-UTF16-H/V: Adobe-Japan1 character collection
//! - UniGB-UTF16-H/V: Adobe-GB1 character collection
//! - UniCNS-UTF16-H/V: Adobe-CNS1 character collection
//! - UniKS-UTF16-H/V: Adobe-Korea1 character collection
//!
//! The UTF16 CMaps are gated behind the `cjk` feature flag due to their
//! binary footprint (~1.2 MB combined).

use std::fmt;

/// A predefined CMap that can decode byte sequences to CIDs and optionally map CIDs to Unicode.
#[derive(Clone, Copy)]
pub struct PredefinedCMap {
    /// The CMap name (e.g., "Identity-H", "UniJIS-UTF16-H").
    name: &'static str,
    /// Whether this is a vertical (V) or horizontal (H) writing mode variant.
    /// This distinction affects glyph rendering, not text extraction.
    is_vertical: bool,
    /// The character collection for UTF16 CMaps (None for Identity-H/V).
    collection: Option<CharacterCollection>,
}

/// Character collection for predefined UTF16 CMaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterCollection {
    /// Adobe-Japan1 (Japanese)
    Japan1,
    /// Adobe-GB1 (Simplified Chinese)
    GB1,
    /// Adobe-CNS1 (Traditional Chinese)
    CNS1,
    /// Adobe-Korea1 (Korean)
    Korea1,
}

impl PredefinedCMap {
    /// Create a new predefined CMap.
    const fn new(
        name: &'static str,
        is_vertical: bool,
        collection: Option<CharacterCollection>,
    ) -> Self {
        Self {
            name,
            is_vertical,
            collection,
        }
    }

    /// Get the CMap name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Check if this is a vertical (V) writing mode variant.
    pub fn is_vertical(&self) -> bool {
        self.is_vertical
    }

    /// Get the character collection (for UTF16 CMaps).
    pub fn collection(&self) -> Option<CharacterCollection> {
        self.collection
    }

    /// Decode a byte sequence to a CID using this CMap.
    ///
    /// For Identity-H/V, this reads 2 bytes as a big-endian CID.
    /// For UTF16 CMaps, this validates the byte sequence length and returns
    /// the CID value.
    ///
    /// Returns None if the byte sequence is invalid for this CMap.
    pub fn decode_bytes(&self, bytes: &[u8]) -> Option<u32> {
        match self.collection {
            None => {
                // Identity-H/V: read 2 bytes as big-endian CID
                if bytes.len() == 2 {
                    let cid = u16::from_be_bytes([bytes[0], bytes[1]]) as u32;
                    Some(cid)
                } else {
                    None
                }
            }
            Some(_) => {
                // UTF16 CMaps: validate 2-byte sequence
                if bytes.len() == 2 {
                    let cid = u16::from_be_bytes([bytes[0], bytes[1]]) as u32;
                    Some(cid)
                } else {
                    None
                }
            }
        }
    }

    /// Map a CID to Unicode codepoint(s) using this CMap.
    ///
    /// For Identity-H/V, this returns None (caller must use ToUnicode).
    /// For UTF16 CMaps, this looks up the CID in the predefined CID->Unicode map.
    ///
    /// Returns None if:
    /// - This is Identity-H/V (no built-in mapping)
    /// - The CID is not in the character collection
    /// - The `cjk` feature is not enabled
    pub fn cid_to_unicode(&self, cid: u32) -> Option<&'static [char]> {
        #[cfg(feature = "cjk")]
        {
            match self.collection {
                None => None, // Identity-H/V have no built-in mapping
                Some(CharacterCollection::Japan1) => {
                    crate::font::predefined_cmap::japan1::cid_to_unicode(cid)
                }
                Some(CharacterCollection::GB1) => {
                    crate::font::predefined_cmap::gb1::cid_to_unicode(cid)
                }
                Some(CharacterCollection::CNS1) => {
                    crate::font::predefined_cmap::cns1::cid_to_unicode(cid)
                }
                Some(CharacterCollection::Korea1) => {
                    crate::font::predefined_cmap::korea1::cid_to_unicode(cid)
                }
            }
        }

        #[cfg(not(feature = "cjk"))]
        {
            let _ = cid;
            None
        }
    }
}

impl fmt::Debug for PredefinedCMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PredefinedCMap")
            .field("name", &self.name)
            .field("is_vertical", &self.is_vertical)
            .field("collection", &self.collection)
            .finish()
    }
}

/// Look up a predefined CMap by name.
///
/// Returns None if the name is not a recognized predefined CMap.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::predefined_cmap::from_name;
///
/// // Identity-H: zero-data CMap
/// let cmap = from_name("Identity-H");
/// assert!(cmap.is_some());
/// assert_eq!(cmap.unwrap().name(), "Identity-H");
///
/// // UniJIS-UTF16-H: Adobe-Japan1 character collection
/// let cmap = from_name("UniJIS-UTF16-H");
/// assert!(cmap.is_some());
///
/// // Unknown name
/// let cmap = from_name("Unknown-CMap");
/// assert!(cmap.is_none());
/// ```
pub fn from_name(name: &str) -> Option<PredefinedCMap> {
    // Strip leading slash if present
    let name = name.strip_prefix('/').unwrap_or(name);

    match name {
        // Identity CMaps (zero-data, no collection)
        "Identity-H" => Some(PredefinedCMap::new("Identity-H", false, None)),
        "Identity-V" => Some(PredefinedCMap::new("Identity-V", true, None)),

        // Adobe-Japan1 (Japanese)
        "UniJIS-UTF16-H" => Some(PredefinedCMap::new(
            "UniJIS-UTF16-H",
            false,
            Some(CharacterCollection::Japan1),
        )),
        "UniJIS-UTF16-V" => Some(PredefinedCMap::new(
            "UniJIS-UTF16-V",
            true,
            Some(CharacterCollection::Japan1),
        )),

        // Adobe-GB1 (Simplified Chinese)
        "UniGB-UTF16-H" => Some(PredefinedCMap::new(
            "UniGB-UTF16-H",
            false,
            Some(CharacterCollection::GB1),
        )),
        "UniGB-UTF16-V" => Some(PredefinedCMap::new(
            "UniGB-UTF16-V",
            true,
            Some(CharacterCollection::GB1),
        )),

        // Adobe-CNS1 (Traditional Chinese)
        "UniCNS-UTF16-H" => Some(PredefinedCMap::new(
            "UniCNS-UTF16-H",
            false,
            Some(CharacterCollection::CNS1),
        )),
        "UniCNS-UTF16-V" => Some(PredefinedCMap::new(
            "UniCNS-UTF16-V",
            true,
            Some(CharacterCollection::CNS1),
        )),

        // Adobe-Korea1 (Korean)
        "UniKS-UTF16-H" => Some(PredefinedCMap::new(
            "UniKS-UTF16-H",
            false,
            Some(CharacterCollection::Korea1),
        )),
        "UniKS-UTF16-V" => Some(PredefinedCMap::new(
            "UniKS-UTF16-V",
            true,
            Some(CharacterCollection::Korea1),
        )),

        _ => None,
    }
}

// Character collection modules (only included when cjk feature is enabled)
#[cfg(feature = "cjk")]
mod japan1 {
    include!(concat!(env!("OUT_DIR"), "/predefined_cmap_japan1.rs"));
}
#[cfg(feature = "cjk")]
mod gb1 {
    include!(concat!(env!("OUT_DIR"), "/predefined_cmap_gb1.rs"));
}
#[cfg(feature = "cjk")]
mod cns1 {
    include!(concat!(env!("OUT_DIR"), "/predefined_cmap_cns1.rs"));
}
#[cfg(feature = "cjk")]
mod korea1 {
    include!(concat!(env!("OUT_DIR"), "/predefined_cmap_korea1.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name_identity_h() {
        let cmap = from_name("Identity-H");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.name(), "Identity-H");
        assert!(!cmap.is_vertical());
        assert_eq!(cmap.collection(), None);
    }

    #[test]
    fn test_from_name_identity_v() {
        let cmap = from_name("Identity-V");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.name(), "Identity-V");
        assert!(cmap.is_vertical());
        assert_eq!(cmap.collection(), None);
    }

    #[test]
    fn test_from_name_unijis_utf16_h() {
        let cmap = from_name("UniJIS-UTF16-H");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.name(), "UniJIS-UTF16-H");
        assert!(!cmap.is_vertical());
        assert_eq!(cmap.collection(), Some(CharacterCollection::Japan1));
    }

    #[test]
    fn test_from_name_unijis_utf16_v() {
        let cmap = from_name("UniJIS-UTF16-V");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.name(), "UniJIS-UTF16-V");
        assert!(cmap.is_vertical());
        assert_eq!(cmap.collection(), Some(CharacterCollection::Japan1));
    }

    #[test]
    fn test_from_name_unigb_utf16_h() {
        let cmap = from_name("UniGB-UTF16-H");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.collection(), Some(CharacterCollection::GB1));
    }

    #[test]
    fn test_from_name_unicns_utf16_h() {
        let cmap = from_name("UniCNS-UTF16-H");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.collection(), Some(CharacterCollection::CNS1));
    }

    #[test]
    fn test_from_name_uniks_utf16_h() {
        let cmap = from_name("UniKS-UTF16-H");
        assert!(cmap.is_some());
        let cmap = cmap.unwrap();
        assert_eq!(cmap.collection(), Some(CharacterCollection::Korea1));
    }

    #[test]
    fn test_from_name_with_leading_slash() {
        let cmap = from_name("/Identity-H");
        assert!(cmap.is_some());
        assert_eq!(cmap.unwrap().name(), "Identity-H");
    }

    #[test]
    fn test_from_name_unknown() {
        let cmap = from_name("Unknown-CMap");
        assert!(cmap.is_none());
    }

    #[test]
    fn test_identity_h_decode_bytes() {
        let cmap = from_name("Identity-H").unwrap();
        // Decode [0x00, 0x41] -> CID 65
        let cid = cmap.decode_bytes(&[0x00, 0x41]);
        assert_eq!(cid, Some(65));
    }

    #[test]
    fn test_identity_h_decode_bytes_invalid_length() {
        let cmap = from_name("Identity-H").unwrap();
        // Invalid length
        assert_eq!(cmap.decode_bytes(&[0x00]), None);
        assert_eq!(cmap.decode_bytes(&[0x00, 0x41, 0x00]), None);
        assert_eq!(cmap.decode_bytes(&[]), None);
    }

    #[test]
    fn test_identity_h_cid_to_unicode_none() {
        let cmap = from_name("Identity-H").unwrap();
        // Identity-H/V have no built-in CID->Unicode mapping
        assert_eq!(cmap.cid_to_unicode(65), None);
    }

    #[test]
    fn test_all_predefined_names() {
        // Verify all 10 predefined CMap names resolve
        let names = [
            "Identity-H",
            "Identity-V",
            "UniJIS-UTF16-H",
            "UniJIS-UTF16-V",
            "UniGB-UTF16-H",
            "UniGB-UTF16-V",
            "UniCNS-UTF16-H",
            "UniCNS-UTF16-V",
            "UniKS-UTF16-H",
            "UniKS-UTF16-V",
        ];

        for name in names {
            let cmap = from_name(name);
            assert!(cmap.is_some(), "{} should resolve", name);
        }
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_unijis_utf16_h_cid_to_unicode() {
        let cmap = from_name("UniJIS-UTF16-H").unwrap();
        // CID 236 -> U+3042 (あ)
        let result = cmap.cid_to_unicode(236);
        assert_eq!(result, Some(&['あ'][..]));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_unijis_utf16_v_identical_mapping() {
        let cmap_h = from_name("UniJIS-UTF16-H").unwrap();
        let cmap_v = from_name("UniJIS-UTF16-V").unwrap();
        // Vertical and horizontal variants have identical CID->Unicode
        assert_eq!(cmap_h.cid_to_unicode(236), cmap_v.cid_to_unicode(236));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_unijis_utf16_h_decode_bytes() {
        let cmap = from_name("UniJIS-UTF16-H").unwrap();
        // Decode [0x00, 0xEC] -> CID 236
        let cid = cmap.decode_bytes(&[0x00, 0xEC]);
        assert_eq!(cid, Some(236));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_unigb_utf16_h_cid_to_unicode() {
        let cmap = from_name("UniGB-UTF16-H").unwrap();
        // CID 814 -> U+4E00 (一)
        let result = cmap.cid_to_unicode(814);
        assert_eq!(result, Some(&['一'][..]));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_unicns_utf16_h_cid_to_unicode() {
        let cmap = from_name("UniCNS-UTF16-H").unwrap();
        // CID 964 -> U+4E00 (一)
        let result = cmap.cid_to_unicode(964);
        assert_eq!(result, Some(&['一'][..]));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_uniks_utf16_h_cid_to_unicode() {
        let cmap = from_name("UniKS-UTF16-H").unwrap();
        // CID 93 -> U+4E00 (一)
        let result = cmap.cid_to_unicode(93);
        assert_eq!(result, Some(&['一'][..]));
    }

    #[cfg(feature = "cjk")]
    #[test]
    fn test_cid_to_unicode_unassigned() {
        let cmap = from_name("UniJIS-UTF16-H").unwrap();
        // CID 999 is not assigned in our stub data
        let result = cmap.cid_to_unicode(999);
        assert_eq!(result, None);
    }

    #[cfg(not(feature = "cjk"))]
    #[test]
    fn test_cid_to_unicode_without_cjk_feature() {
        let cmap = from_name("UniJIS-UTF16-H").unwrap();
        // Without cjk feature, UTF16 CMaps return None
        let result = cmap.cid_to_unicode(236);
        assert_eq!(result, None);
    }
}

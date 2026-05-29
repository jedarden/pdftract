//! PDF object model types.
//!
//! This module defines the foundational data types of the PDF object model
//! as specified in the PDF 2.0 standard (ISO 32000-2:2020).

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use indexmap::IndexMap;

thread_local! {
    /// Name interner for PDF name objects.
    ///
    /// PDFs reuse a small set of names (/Type, /Length, /Filter, /Font, etc.)
    /// across thousands of dictionaries. This thread-local interner ensures
    /// all instances share a single Arc<str> allocation.
    ///
    /// Tested size cap: ~10k entries (no eviction needed — PDF name vocabulary is bounded).
    static INTERNER: RefCell<HashSet<Arc<str>>> = RefCell::new(HashSet::new());
}

/// Intern a string slice as an `Arc<str>`, returning a shared instance if already interned.
pub fn intern(s: &str) -> Arc<str> {
    INTERNER.with_borrow_mut(|interner| {
        // Fast path: check if already exists
        if let Some(existing) = interner.get(s) {
            return existing.clone();
        }
        // Slow path: insert new
        let arc: Arc<str> = s.into();
        interner.insert(arc.clone());
        arc
    })
}

/// A reference to an indirect PDF object.
///
/// PDF 1.7, Section 7.3.8: "Indirect Objects"
/// Consists of an object number and generation number.
///
/// Display format: `"<obj> <gen> R"` (e.g., "42 0 R")
#[derive(Debug, Clone, Copy, Eq)]
pub struct ObjRef {
    /// Object number (1-based index in the xref table)
    pub object: u32,
    /// Generation number (0 for non-incrementally-saved files)
    pub generation: u16,
}

impl ObjRef {
    /// Create a new object reference.
    #[inline]
    pub const fn new(object: u32, generation: u16) -> Self {
        ObjRef { object, generation }
    }
}

impl PartialEq for ObjRef {
    fn eq(&self, other: &Self) -> bool {
        self.object == other.object && self.generation == other.generation
    }
}

impl Hash for ObjRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.object.hash(state);
        self.generation.hash(state);
    }
}

impl PartialOrd for ObjRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.object.partial_cmp(&other.object) {
            Some(core::cmp::Ordering::Equal) => self.generation.partial_cmp(&other.generation),
            other_ord => other_ord,
        }
    }
}

impl Ord for ObjRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.object.cmp(&other.object) {
            core::cmp::Ordering::Equal => self.generation.cmp(&other.generation),
            other_ord => other_ord,
        }
    }
}

impl fmt::Display for ObjRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.object, self.generation)
    }
}

/// PDF dictionary type.
///
/// An ordered map preserving insertion order.
/// PDF 1.7, Section 7.3.7: "Dictionary Objects"
///
/// Order preservation is critical for:
/// - Deterministic fingerprint computation (Phase 1.7)
/// - JSON receipt byte-identity (Phase 6.8)
pub type PdfDict = IndexMap<Arc<str>, PdfObject>;

/// PDF stream object.
///
/// PDF 1.7, Section 7.3.8.2: "Stream Objects"
///
/// Contains a dictionary (with at least /Length) and binary data.
/// The `len_hint` is the optional /Length value if direct (not indirect);
/// stream decoder uses it as the read size. If None, the decoder scans for `endstream`.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    /// Stream dictionary (contains /Length, /Filter, etc.)
    pub dict: PdfDict,
    /// Byte offset of stream data in the source file
    pub offset: u64,
    /// Optional length hint from /Length entry (if direct integer)
    pub len_hint: Option<u64>,
}

impl PdfStream {
    /// Create a new stream.
    #[inline]
    pub fn new(dict: PdfDict, offset: u64, len_hint: Option<u64>) -> Self {
        Self {
            dict,
            offset,
            len_hint,
        }
    }

    /// Get the /Filter entry from the stream dictionary.
    ///
    /// Returns None if no filter is present (raw stream).
    /// Filter names are returned without the leading slash (e.g., "FlateDecode", not "/FlateDecode").
    pub fn filter(&self) -> Option<Vec<String>> {
        let filter = self.dict.get("/Filter")?;

        Some(match filter {
            PdfObject::Name(name) => {
                // Strip leading slash from filter name for normalization
                let name_str: &str = name.as_ref();
                let stripped = if name_str.starts_with('/') {
                    &name_str[1..]
                } else {
                    name_str
                };
                vec![stripped.to_string()]
            }
            PdfObject::Array(arr) => arr
                .iter()
                .filter_map(|obj| {
                    obj.as_name().map(|n| {
                        // Strip leading slash from filter name for normalization
                        let name_str: &str = n.as_ref();
                        let stripped = if name_str.starts_with('/') {
                            &name_str[1..]
                        } else {
                            name_str
                        };
                        stripped.to_string()
                    })
                })
                .collect(),
            _ => return None,
        })
    }

    /// Get the /DecodeParms entry from the stream dictionary.
    ///
    /// Returns None if no parameters are present.
    pub fn decode_params(&self) -> Option<Vec<PdfObject>> {
        let params = self.dict.get("/DecodeParms")?;

        Some(match params {
            PdfObject::Dict(_) => vec![params.clone()],
            PdfObject::Array(arr) => arr.as_ref().clone(),
            _ => return None,
        })
    }

    /// Get the /Length entry from the stream dictionary.
    ///
    /// Returns the direct integer value, or None if /Length is indirect/missing.
    pub fn length(&self) -> Option<u64> {
        self.dict.get("/Length")?.as_int().map(|i| i as u64)
    }
}

/// PDF indirect object wrapper.
///
/// Represents a resolved indirect object with its ID.
/// Used only at the top of each indirect-object statement.
#[derive(Debug, Clone)]
pub struct PdfIndirect {
    /// Object identifier
    pub id: ObjRef,
    /// The actual object
    pub obj: PdfObject,
}

/// A PDF object.
///
/// PDF 1.7, Chapter 7: "Lexical and File Structure"
///
/// This enum represents all possible PDF object types. Objects form a
/// tree/graph through references (PdfObject::Ref) and can be resolved
/// through the cross-reference table.
///
/// Size target: <= 24 bytes on x86_64 (achieved via Box on rare variants).
#[derive(Debug, Clone)]
pub enum PdfObject {
    /// Null object (PDF 1.7, Section 7.3.9)
    Null,

    /// Boolean object (PDF 1.7, Section 7.3.2)
    Bool(bool),

    /// Integer object (PDF 1.7, Section 7.3.3)
    Integer(i64),

    /// Real number object (PDF 1.7, Section 7.3.3)
    Real(f64),

    /// String object (PDF 1.7, Section 7.3.4)
    /// Raw bytes; encoding interpretation happens later during text extraction.
    /// Boxed to keep enum size small.
    String(Box<Vec<u8>>),

    /// Name object (PDF 1.7, Section 7.3.5)
    /// Uses interned `Arc<str>` for cheap cloning and deduplication.
    Name(Arc<str>),

    /// Array object (PDF 1.7, Section 7.3.6)
    /// Boxed to keep enum size small.
    Array(Box<Vec<PdfObject>>),

    /// Dictionary object (PDF 1.7, Section 7.3.7)
    /// Boxed to keep enum size small (IndexMap is ~72 bytes unboxed).
    Dict(Box<PdfDict>),

    /// Indirect reference (PDF 1.7, Section 7.3.8)
    Ref(ObjRef),

    /// Stream object (PDF 1.7, Section 7.3.8.2)
    Stream(Box<PdfStream>),

    /// Indirect object wrapper (rare; only at top of indirect-object statements)
    Indirect(Box<PdfIndirect>),
}

impl PdfObject {
    /// Get the type name of this object for diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            PdfObject::Null => "null",
            PdfObject::Bool(_) => "boolean",
            PdfObject::Integer(_) => "integer",
            PdfObject::Real(_) => "real",
            PdfObject::String(_) => "string",
            PdfObject::Name(_) => "name",
            PdfObject::Array(_) => "array",
            PdfObject::Dict(_) => "dictionary",
            PdfObject::Ref(_) => "reference",
            PdfObject::Stream(_) => "stream",
            PdfObject::Indirect(_) => "indirect",
        }
    }

    /// Returns true if this is the null object.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, PdfObject::Null)
    }

    /// Try to get an integer value, returning None if not an Integer.
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get a real value, returning None if not a Real.
    #[inline]
    pub fn as_real(&self) -> Option<f64> {
        match self {
            PdfObject::Real(r) => Some(*r),
            _ => None,
        }
    }

    /// Try to get a name reference, returning None if not a Name.
    #[inline]
    pub fn as_name(&self) -> Option<&str> {
        match self {
            PdfObject::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Try to get a dictionary reference, returning None if not a Dict.
    #[inline]
    pub fn as_dict(&self) -> Option<&PdfDict> {
        match self {
            PdfObject::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Try to get a stream reference, returning None if not a Stream.
    #[inline]
    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get an array reference, returning None if not an Array.
    #[inline]
    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get a string reference (raw bytes), returning None if not a String.
    #[inline]
    pub fn as_string(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get an object reference, returning None if not a Ref.
    #[inline]
    pub fn as_ref(&self) -> Option<ObjRef> {
        match self {
            PdfObject::Ref(r) => Some(*r),
            _ => None,
        }
    }

    /// Try to get a bool, handling the case where some PDFs use integers 0/1.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Bool(b) => Some(*b),
            PdfObject::Integer(0) => Some(false),
            PdfObject::Integer(1) => Some(true),
            _ => None,
        }
    }
}

impl Default for PdfObject {
    fn default() -> Self {
        PdfObject::Null
    }
}

impl PartialEq for PdfObject {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PdfObject::Null, PdfObject::Null) => true,
            (PdfObject::Bool(a), PdfObject::Bool(b)) => a == b,
            (PdfObject::Integer(a), PdfObject::Integer(b)) => a == b,
            (PdfObject::Real(a), PdfObject::Real(b)) => {
                // IEEE-754: NaN != NaN
                if a.is_nan() || b.is_nan() {
                    false
                } else {
                    a == b
                }
            }
            (PdfObject::String(a), PdfObject::String(b)) => a == b,
            (PdfObject::Name(a), PdfObject::Name(b)) => a == b,
            (PdfObject::Array(a), PdfObject::Array(b)) => a == b,
            (PdfObject::Dict(a), PdfObject::Dict(b)) => a == b,
            (PdfObject::Ref(a), PdfObject::Ref(b)) => a == b,
            (PdfObject::Stream(a), PdfObject::Stream(b)) => {
                a.offset == b.offset && a.len_hint == b.len_hint && a.dict == b.dict
            }
            (PdfObject::Indirect(a), PdfObject::Indirect(b)) => a.id == b.id && a.obj == b.obj,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obj_ref_display() {
        let obj_ref = ObjRef::new(42, 0);
        assert_eq!(obj_ref.to_string(), "42 0 R");

        let obj_ref2 = ObjRef::new(1, 2);
        assert_eq!(obj_ref2.to_string(), "1 2 R");
    }

    #[test]
    fn test_obj_ref_ordering() {
        let a = ObjRef::new(1, 0);
        let b = ObjRef::new(2, 0);
        let c = ObjRef::new(1, 1);

        assert!(a < b);
        assert!(a < c);
        assert!(c < b);
    }

    #[test]
    fn test_obj_ref_partial_ord() {
        let a = ObjRef::new(5, 2);
        let b = ObjRef::new(5, 2);
        let c = ObjRef::new(10, 0);

        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Equal));
        assert_eq!(a.partial_cmp(&c), Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn test_name_interner_dedup() {
        let a = intern("Length");
        let b = intern("Length");
        let c = intern("Filter");

        // Same string should return same Arc
        assert!(Arc::ptr_eq(&a, &b));
        // Different strings should be different Arcs
        assert!(!Arc::ptr_eq(&a, &c));
        assert_eq!(a.as_ref(), "Length");
        assert_eq!(c.as_ref(), "Filter");
    }

    #[test]
    fn test_name_interner_common_names() {
        let names = ["Type", "Length", "Filter", "Font", "Subtype", "Contents"];
        let interned: Vec<_> = names.iter().map(|s| intern(s)).collect();

        // Verify all are unique Arcs
        for (i, a) in interned.iter().enumerate() {
            for (j, b) in interned.iter().enumerate() {
                assert_eq!(Arc::ptr_eq(a, b), i == j);
            }
        }

        // Re-intern and verify dedup
        for (name, arc) in names.iter().zip(interned.iter()) {
            let again = intern(name);
            assert!(Arc::ptr_eq(arc, &again));
        }
    }

    #[test]
    fn test_pdf_object_size() {
        // Target: <= 32 bytes on x86_64
        let size = std::mem::size_of::<PdfObject>();
        assert!(size <= 32, "PdfObject size {} exceeds 32 bytes", size);
        println!("PdfObject size: {} bytes", size);
    }

    #[test]
    fn test_pdf_dict_insertion_order() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Z"), PdfObject::Integer(3));
        dict.insert(intern("A"), PdfObject::Integer(1));
        dict.insert(intern("M"), PdfObject::Integer(2));

        let keys: Vec<_> = dict.keys().map(|k| k.as_ref()).collect();
        assert_eq!(keys, vec!["Z", "A", "M"]);
    }

    #[test]
    fn test_pdf_dict_roundtrip_order() {
        let mut dict = PdfDict::new();
        let names = ["First", "Second", "Third", "Fourth"];
        for (i, name) in names.iter().enumerate() {
            dict.insert(intern(name), PdfObject::Integer(i as i64));
        }

        let collected: Vec<_> = dict.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0].0.as_ref(), "First");
        assert_eq!(collected[1].0.as_ref(), "Second");
        assert_eq!(collected[2].0.as_ref(), "Third");
        assert_eq!(collected[3].0.as_ref(), "Fourth");
    }

    #[test]
    fn test_as_int() {
        assert_eq!(PdfObject::Integer(42).as_int(), Some(42));
        assert_eq!(PdfObject::Integer(-100).as_int(), Some(-100));
        assert_eq!(PdfObject::Real(3.14).as_int(), None);
        assert_eq!(PdfObject::Bool(true).as_int(), None);
    }

    #[test]
    fn test_as_real() {
        assert_eq!(PdfObject::Real(3.14).as_real(), Some(3.14));
        assert_eq!(PdfObject::Real(-0.5).as_real(), Some(-0.5));
        assert_eq!(PdfObject::Integer(42).as_real(), None);
        assert_eq!(PdfObject::Bool(true).as_real(), None);
    }

    #[test]
    fn test_as_name() {
        assert_eq!(PdfObject::Name(intern("Type")).as_name(), Some("Type"));
        assert_eq!(PdfObject::Name(intern("Length")).as_name(), Some("Length"));
        assert_eq!(PdfObject::Integer(42).as_name(), None);
    }

    #[test]
    fn test_as_dict() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Type"), PdfObject::Name(intern("Page")));
        let obj = PdfObject::Dict(Box::new(dict.clone()));

        assert!(obj.as_dict().is_some());
        assert_eq!(
            obj.as_dict().unwrap().get("Type").unwrap().as_name(),
            Some("Page")
        );
        assert_eq!(PdfObject::Integer(42).as_dict(), None);
    }

    #[test]
    fn test_as_stream() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Length"), PdfObject::Integer(100));
        let stream = PdfStream {
            dict,
            offset: 500,
            len_hint: Some(100),
        };
        let obj = PdfObject::Stream(Box::new(stream.clone()));

        assert!(obj.as_stream().is_some());
        assert_eq!(obj.as_stream().unwrap().offset, 500);
        assert_eq!(obj.as_stream().unwrap().len_hint, Some(100));
        assert!(PdfObject::Integer(42).as_stream().is_none());
    }

    #[test]
    fn test_as_array() {
        let arr = vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ];
        let obj = PdfObject::Array(Box::new(arr.clone()));

        assert!(obj.as_array().is_some());
        assert_eq!(obj.as_array().unwrap().len(), 3);
        assert_eq!(PdfObject::Integer(42).as_array(), None);
    }

    #[test]
    fn test_as_string() {
        let s = b"Hello".to_vec();
        let obj = PdfObject::String(Box::new(s.clone()));

        assert!(obj.as_string().is_some());
        assert_eq!(obj.as_string().unwrap(), &s[..]);
        assert_eq!(PdfObject::Integer(42).as_string(), None);
    }

    #[test]
    fn test_as_ref() {
        let obj_ref = ObjRef::new(42, 0);
        let obj = PdfObject::Ref(obj_ref);

        assert!(obj.as_ref().is_some());
        assert_eq!(obj.as_ref().unwrap(), obj_ref);
        assert_eq!(PdfObject::Integer(42).as_ref(), None);
    }

    #[test]
    fn test_is_null() {
        assert!(PdfObject::Null.is_null());
        assert!(!PdfObject::Integer(0).is_null());
        assert!(!PdfObject::Bool(false).is_null());
    }

    #[test]
    fn test_pdf_object_partial_eq_real_nan() {
        let nan1 = PdfObject::Real(f64::NAN);
        let nan2 = PdfObject::Real(f64::NAN);

        // IEEE-754: NaN != NaN
        assert!(nan1 != nan2);
    }

    #[test]
    fn test_pdf_object_partial_eq_real_normal() {
        let a = PdfObject::Real(3.14);
        let b = PdfObject::Real(3.14);
        let c = PdfObject::Real(2.71);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_pdf_stream_len_hint() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Length"), PdfObject::Integer(1000));

        let stream = PdfStream {
            dict,
            offset: 1234,
            len_hint: Some(1000),
        };

        assert_eq!(stream.len_hint, Some(1000));
        assert_eq!(stream.offset, 1234);
    }

    #[test]
    fn test_pdf_stream_no_len_hint() {
        let dict = PdfDict::new();
        let stream = PdfStream {
            dict,
            offset: 5678,
            len_hint: None,
        };

        assert_eq!(stream.len_hint, None);
    }

    #[test]
    fn test_pdf_indirect() {
        let obj_ref = ObjRef::new(10, 0);
        let obj = PdfObject::Integer(42);
        let indirect = PdfIndirect { id: obj_ref, obj };

        assert_eq!(indirect.id, ObjRef::new(10, 0));
        assert_eq!(indirect.obj.as_int(), Some(42));
    }

    #[test]
    fn test_pdf_object_indirect_variant() {
        let obj_ref = ObjRef::new(5, 1);
        let inner = PdfObject::Name(intern("Test"));
        let indirect = PdfIndirect {
            id: obj_ref,
            obj: inner,
        };
        let obj = PdfObject::Indirect(Box::new(indirect));

        assert!(obj.as_indirect().is_some());
        let extracted = obj.as_indirect().unwrap();
        assert_eq!(extracted.id, ObjRef::new(5, 1));
        assert_eq!(extracted.obj.as_name(), Some("Test"));
    }

    #[test]
    fn test_obj_ref_hash() {
        use std::collections::HashMap;

        let a = ObjRef::new(1, 0);
        let b = ObjRef::new(1, 0);
        let c = ObjRef::new(2, 0);

        let mut map = HashMap::new();
        map.insert(a, "first");

        assert_eq!(map.get(&b), Some(&"first"));
        assert_eq!(map.get(&c), None);
    }

    // Helper for testing
    impl PdfObject {
        fn as_indirect(&self) -> Option<&PdfIndirect> {
            match self {
                PdfObject::Indirect(i) => Some(i),
                _ => None,
            }
        }
    }
}

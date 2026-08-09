# Document Structure Analysis (bf-4y05mc)

## Summary
Analyzed the nested Document structure to identify where Page data is stored and how to navigate to it.

## Document Structure

### Document Struct Location
File: `/home/coding/pdftract/crates/pdftract-core/src/document.rs` (lines 721-732)

```rust
pub struct Document {
    /// The parsed catalog
    catalog: Catalog,
    /// The xref resolver for object resolution
    resolver: XrefResolver,
    /// The PDF source (file, HTTP, memory)
    source: Option<Box<dyn ParserPdfSource>>,
    /// The document fingerprint
    fingerprint: String,
    /// Whether this is a remote document
    is_remote: bool,
}
```

### Navigation Path to Page Data

**Method 1: Lazy Iterator (Recommended for large documents)**
```rust
let doc = Document::open("document.pdf")?;
for page_result in doc.pages() {
    let page = page_result?;
    // Page is PageExtraction { index, width, height, rotation, spans, blocks }
}
```

**Method 2: Via Catalog (for advanced use)**
```rust
let doc = Document::open("document.pdf")?;
let catalog = doc.catalog();
// catalog.pages_ref → ObjRef to root /Pages dictionary
// Use flatten_page_tree(&resolver, catalog.pages_ref) to get Vec<PageDict>
```

**Method 3: Direct materialization (memory-intensive)**
```rust
let mut extractor = PdfExtractor::open("document.pdf")?;
extractor.materialize_pages()?;
let pages = extractor.pages.unwrap(); // Vec<PageDict>
```

## PageDict Structure

File: `/home/coding/pdftract/crates/pdftract-core/src/parser/pages.rs` (lines 36-67)

```rust
pub struct PageDict {
    /// The page's own indirect reference
    pub obj_ref: ObjRef,
    /// REQUIRED; inherited if missing. Default: [0, 0, 612, 792]
    pub media_box: [f64; 4],
    /// Optional; defaults to media_box if absent
    pub crop_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub bleed_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub trim_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub art_box: Option<[f64; 4]>,
    /// Page rotation in degrees; must be a multiple of 90 (0, 90, 180, 270)
    pub rotate: i32,
    /// Merged resource dict containing all inherited resources
    pub resources: Arc<ResourceDict>,
    /// List of content stream references (in order)
    pub contents: Vec<ObjRef>,
    /// Annotation array references
    pub annots: Vec<ObjRef>,
    /// ActualText from tagged PDF (if present)
    pub actual_text: Option<String>,
    /// Language identifier (if present)
    pub lang: Option<String>,
    /// Page-level additional actions (used by JS detection)
    pub aa: Option<PdfObject>,
    /// /StructParents value for StructTree MCID resolution (Phase 7.1.4)
    pub struct_parents: Option<i32>,
}
```

## Key Field Types for Page Construction

When constructing extraction logic, these are the essential fields:

1. **Geometry**: `media_box`, `crop_box`, `rotate` - define page boundaries and orientation
2. **Content**: `contents: Vec<ObjRef>` - references to content streams to decode
3. **Resources**: `resources: Arc<ResourceDict>` - fonts, images, color spaces needed for rendering
4. **Annotations**: `annots: Vec<ObjRef>` - links, form fields, markup
5. **Metadata**: `obj_ref`, `actual_text`, `lang` - for identification and accessibility

## Navigation Approach Summary

**Document → Pages Navigation:**
```
Document.catalog() → Catalog
Catalog.pages_ref → ObjRef (root /Pages)
flatten_page_tree(&resolver, pages_ref) → Vec<PageDict>
PageDict[index] → specific page data
```

**Lazy approach (memory-efficient):**
```
Document.pages() → PageIter<PageExtraction>
PageIter::next() → Result<PageExtraction>
```

## Verification

- ✅ Document shows path to Page data via `Document.pages()` or `Document.catalog().pages_ref`
- ✅ Field names and types identified (see PageDict structure above)
- ✅ Navigation approach documented (see Navigation Path section)
- ✅ Ready to implement extraction function

## References

- `Document` struct: `/home/coding/pdftract/crates/pdftract-core/src/document.rs:721-732`
- `PageDict` struct: `/home/coding/pdftract/crates/pdftract-core/src/parser/pages.rs:36-67`
- `flatten_page_tree()`: `/home/coding/pdftract/crates/pdftract-core/src/parser/pages.rs:308-360`
- `LazyPageIter`: `/home/coding/pdftract/crates/pdftract-core/src/parser/pages.rs:1398-1600`

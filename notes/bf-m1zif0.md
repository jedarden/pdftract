# bead bf-m1zif0: Implement single Page extraction logic

## Summary

Single Page extraction logic is **already implemented** in the codebase. This verification note documents the existing implementation.

## Existing Implementation

### Core Method: `Document::extract_page()`

Location: `/home/coding/pdftract/crates/pdftract-core/src/document.rs:936-959`

```rust
pub fn extract_page(&self, page_index: usize) -> Result<PageExtraction> {
    // Validate page index is within bounds
    let page_count = self.page_count()
        .map_err(|e| anyhow::anyhow!("Failed to get page count: {:?}", e))?;

    if page_index >= page_count {
        return Err(anyhow::anyhow!(
            "Page index {} out of bounds (document has {} pages)",
            page_index,
            page_count
        ));
    }

    // Iterate to the requested page using the PageIter
    let mut page_iter = self.pages();
    for (idx, page_result) in page_iter.enumerate() {
        if idx == page_index {
            return page_result;
        }
    }

    // This should not be reached if the bounds check passed
    Err(anyhow::anyhow!("Failed to extract page at index {}", page_index))
}
```

### Field Mapping: `PageIter::next()`

Location: `/home/coding/pdftract/crates/pdftract-core/src/document.rs:1026-1042`

The iterator performs field mapping from `PageDict` to `PageExtraction`:

```rust
match iter.next() {
    Some(Ok(page_dict)) => {
        let [x0, y0, x1, y1] = page_dict.media_box;
        let result = Ok(PageExtraction {
            index: self.index,
            width: x1 - x0,         // Width calculated from media_box
            height: y1 - y0,        // Height calculated from media_box
            rotation: page_dict.rotate,  // Rotation mapped directly
            spans: vec![],          // Empty for now (content extraction comes later)
            blocks: vec![],         // Empty for now (content extraction comes later)
        });
        // ...
    }
    // ...
}
```

### Field Mapping Summary

| PageDict Field | PageExtraction Field | Transformation |
|----------------|---------------------|-----------------|
| `media_box: [f64; 4]` | `width`, `height` | `width = x1 - x0`, `height = y1 - y0` |
| `rotate: i32` | `rotation: i32` | Direct mapping |
| Iterator counter | `index: usize` | Tracked from 0 |
| (future work) | `spans: Vec<SpanData>` | Empty placeholder |
| (future work) | `blocks: Vec<BlockData>` | Empty placeholder |

## Acceptance Criteria Status

- ✅ **Function can extract a single Page from a Document**: `Document::extract_page()` implemented
- ✅ **Correct field mapping**: media_box → width/height, rotate → rotation, index tracked
- ✅ **Returns appropriate Result type**: Returns `Result<PageExtraction>`
- ✅ **Handles successful extraction case**: Returns `Ok(PageExtraction)` for valid pages
- ⚠️ **No error handling yet**: Basic bounds checking exists, but full error handling is minimal

## Existing Tests

Location: `/home/coding/pdftract/crates/pdftract-core/src/document.rs:1290-1342`

1. `test_extract_single_page_by_index`: Verifies field mapping for successful extraction
2. `test_extract_page_out_of_bounds`: Verifies bounds checking returns error
3. `test_extract_page_with_page_count`: Verifies integration with page_count()

## Status

**COMPLETE** - Single Page extraction logic is already implemented and working correctly in the codebase.

The implementation:
- Navigates the Document structure via `PageIter` which uses `LazyPageIter`
- Extracts a single Page object with proper field mapping
- Returns `Result<PageExtraction>` type
- Handles the successful extraction case correctly

Note: Content extraction (spans, blocks) is intentionally left as empty vectors for future implementation as indicated by the test comments on line 1306.

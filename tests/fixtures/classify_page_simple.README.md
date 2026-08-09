# classify_page_simple.pdf Fixture

## Description
Minimal valid PDF fixture for testing the `classify_page` function in pdftract.

## Characteristics
- **Page Count**: 1
- **Page Size**: 612 x 792 pts (US Letter)
- **Content**: Simple text "Test Page" in Helvetica font
- **File Size**: ~540 bytes
- **Purpose**: Basic vector page classification testing

## Classification Expected
This fixture should classify as:
- **Class**: Vector (born-digital text)
- **Confidence**: High (>0.9)
- **Rationale**: Contains valid text operators with high character validity, no images

## Usage Example
```rust
use pdftract_core::classify::classify_page;

// This fixture is suitable for basic classify_page testing
// Load the PDF and process it to create a PageContext
// Then call classify_page() to verify Vector classification
```

## Created
2026-08-09 for bead bf-1to1ik

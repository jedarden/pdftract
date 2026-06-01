# Figure and Caption Detection Verification - pdftract-25k4x

## Acceptance Criteria Verification

### 1. Image XObject, no text overlap: 1 Figure block
**Location:** `crates/pdftract-core/src/layout/figure.rs:130`
- Checks `text_overlap_area / image_area < 0.5`
- Creates Block with kind="figure"
- **PASS** - Tests: `test_classify_figure_pure_visual_image`, `test_classify_figure_no_glyphs`

### 2. Image + small-font caption 1 line below: Figure + Caption
**Location:** `crates/pdftract-core/src/layout/caption.rs:126-140`
- Checks `block.median_font_size < ctx.page_body_median` (small font)
- Checks `vertical_distance < 2.0 * ctx.line_height` (within 2 lines)
- Sets kind to "caption"
- **PASS** - Test: `test_caption_immediately_below_figure`

### 3. Image overlapping text (background): NOT Figure
**Location:** `crates/pdftract-core/src/layout/figure.rs:130`
- Images with >= 50% text overlap are NOT classified as figures
- **PASS** - Tests: `test_classify_figure_text_on_image`, `test_classify_figure_partial_text_above_threshold`

### 4. Caption 5 lines below: NOT Caption
**Location:** `crates/pdftract-core/src/layout/caption.rs:145-148`
- Checks `vertical_distance >= 2.0 * ctx.line_height`
- Returns false if too far below
- **PASS** - Test: `test_caption_too_far_below_figure`

### 5. Caption different column: NOT Caption
**Location:** `crates/pdftract-core/src/layout/caption.rs:152-154`
- Checks `block.column != figure.column` in multi-column layouts
- Returns false if different column
- **PASS** - Test: `test_caption_different_column`

## Test Results

### Figure Classifier Tests (16/16 PASS)
- test_bboxes_intersect
- test_classify_figure_no_images
- test_classify_figure_partial_text_below_threshold
- test_classify_figure_partial_text_above_threshold
- test_classify_figure_exactly_at_threshold
- test_classify_figure_no_glyphs
- test_classify_figure_pure_visual_image
- test_bbox_area
- test_classify_figure_sort_order
- test_classify_figure_empty_context
- test_classify_figure_text_on_image
- test_compute_text_overlap_area_multiple_glyphs
- test_compute_text_overlap_area_union
- test_figure_block_properties
- test_five_figures_no_text
- test_text_covered_image_not_figure

### Caption Classifier Tests (8/8 PASS)
- test_caption_above_figure
- test_caption_font_not_smaller
- test_caption_too_far_below_figure
- test_no_previous_figure
- test_caption_different_column
- test_caption_immediately_below_figure
- test_block_accessors
- test_page_classification

## INV Verification
- **INV: Figure block has empty lines Vec** - SATISFIED: Block created with text=String::empty(), median_font_size=0.0
- **Caption above figure NOT detected in v0.1.0** - SATISFIED: caption.rs test_caption_above_figure returns false

## Files Verified
- crates/pdftract-core/src/layout/figure.rs (517 lines)
- crates/pdftract-core/src/layout/caption.rs (342 lines)
- crates/pdftract-core/src/layout/mod.rs (exports classifiers)

## Verification Status
**ALL ACCEPTANCE CRITERIA PASS** - Implementation complete and tested.

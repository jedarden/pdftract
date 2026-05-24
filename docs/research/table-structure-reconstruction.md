# Table Structure Reconstruction

## The Problem

PDF is a presentation format. Its content streams describe where ink lands on a page — not what that ink means. There is no semantic concept of "table", "row", or "cell" in an untagged PDF. Every glyph and path operator exists only to produce visual output; the burden of interpretation falls entirely on the reader.

This creates several compounding difficulties:

**No semantic markup.** Even what appears to be a neatly formatted table with ruled lines may be represented as a collection of `re` (rectangle) fill operations, scattered glyph positioning commands, and `l`/`S` (line/stroke) operators — all independent of one another in the content stream. The association between a drawn border and the text it encloses is purely geometric, not encoded.

**Borderless tables indistinguishable from columnar prose.** A two-column table with no borders is visually identical to a two-column prose layout. The only distinguishing signals are: whether the number of rows exceeds a threshold, whether horizontal alignment is consistent across all rows, and whether adjacent columns carry semantically distinct data types. None of these signals are definitive on their own.

**Merged cells.** A cell spanning two columns has no path operators uniquely identifying the span. From the drawing perspective, the grid simply has a missing interior line segment. A cell spanning two rows may be identified only by the absence of a horizontal divider and the presence of text centered between two horizontal rules. These absences must be inferred from the reconstructed grid, not read directly.

**Multi-page tables.** A table split across a page break leaves no continuation marker. The bottom of page N and the top of page N+1 must be matched using column count, column width fingerprints, and optionally a repeated header row as a structural anchor.

**Nested tables.** A cell may contain a second table. The inner table's lines intersect with the outer table's coordinate space; naive grid reconstruction will produce spurious cells unless nested table bounding boxes are detected and isolated before the outer grid is finalized.

**Mixed cell types.** A table may contain text cells, numeric cells, and cells whose primary content is a raster image or vector graphic rather than glyphs. The reconstruction algorithm must allocate cell bounding boxes correctly even when a cell contains no glyphs at all.

---

## Line-Based Detection

The most reliable signal for table structure is explicit ruling lines drawn with PDF path operators.

### Identifying Path Operators

In a PDF content stream, lines are drawn with sequences like:

```
x0 y0 m   % moveto
x1 y1 l   % lineto
S         % stroke
```

Rectangles are drawn with `re`:

```
x y w h re S   % stroke a rectangle
x y w h re f   % fill a rectangle
```

Rectangled drawn with `re` and stroked produce four line segments implicitly. When parsing, expand each `re` into its four constituent segments before analysis.

### Reconstructing the Grid

Once all horizontal and vertical line segments are collected, cluster them by orientation:

- **Horizontal:** segments where |y0 - y1| < epsilon (typically 0.5 pt in PDF space).
- **Vertical:** segments where |x0 - x1| < epsilon.

Merge collinear segments that share the same y (or x) coordinate and whose x-extents overlap or are contiguous within a small gap threshold (e.g., 2 pt). This handles dashed or dotted rules: a dashed line in PDF is typically realized as many short `l`/`S` segments that must be merged back into a logical line.

Hairline rules (line width < 0.5 pt) are visually invisible at normal zoom but still define table structure. Do not filter by line width; instead, track line width as metadata for later rendering decisions.

After merging, find all intersection points between horizontal and vertical segments. These intersections are candidate grid vertices. Build the grid by:

1. For each unique y-coordinate of a horizontal line, record its x-extent.
2. For each unique x-coordinate of a vertical line, record its y-extent.
3. A valid grid cell exists between four vertices (x0,y0), (x1,y0), (x0,y1), (x1,y1) where all four edges are present.

### Partial Borders

Many real-world tables use only top and bottom borders (no vertical separators), or only an outer frame (no interior lines). Handle this by relaxing the grid completeness requirement: a cell boundary edge need not exist as a drawn line — it may instead be inferred from whitespace gaps (see next section). A mixed detection pass first identifies all explicit lines, then applies gap analysis only in the regions where lines are absent.

---

## Whitespace Gap Analysis (Borderless Tables)

When no ruling lines are present, column boundaries must be inferred from the distribution of glyph bounding boxes.

### Projection Profiles

For each horizontal band (row) of glyphs, compute the union of all glyph x-extents. This produces an "occupied" interval set. The complement — the gaps between occupied intervals — are candidate column separators.

To find separators that are consistent across multiple rows, build a **vertical projection profile**: for each x-coordinate, count how many rows have glyph coverage at that x. A column separator is a contiguous x-range where glyph coverage across all rows falls to zero (or near-zero, to tolerate small overhangs).

### Minimum Gap Threshold

Not every gap is a column boundary. Word spacing within a cell also creates gaps. A practical threshold is:

```
min_column_gap = median_word_space * K
```

where `median_word_space` is the median inter-word gap in the document (estimated from the distribution of x-advances within text runs) and `K` is an empirically determined factor, typically 2.0 to 3.0. Gaps narrower than this threshold are word spaces, not column separators.

### Distinguishing Prose from Tabular Data

A multi-column prose layout (newspaper columns) also exhibits consistent vertical gaps. Distinguish it from a table by:

- **Row count:** Tables typically have more than 3–4 rows with consistent column structure. A two-column prose block may span many rows but the column boundary is not re-used at a cell level.
- **Alignment consistency:** In a table, text within a column tends to share a dominant alignment (left, right, or decimal-aligned). In prose, each column is independently left-justified without cross-column structural meaning.
- **Column count stability:** In a table, the number of occupied columns per row is near-constant. In prose, partial final paragraphs may occupy only one column.

A row is classified as tabular if at least 60% of detected rows share the same column separator positions within a ±2 pt tolerance.

---

## Hough Transform Approach

When neither explicit path operators nor clean whitespace gaps are available — for example, in scanned-and-re-embedded PDFs where glyphs are rasterized but positioned with high precision — glyph bounding box edges can serve as line evidence.

### Parameter Space

For each glyph bounding box, emit four candidate line segments: top, bottom, left, right edges. Accumulate votes in a discretized (rho, theta) Hough space, restricted to near-horizontal (|theta| < 5 degrees) and near-vertical (|theta - 90| < 5 degrees) bins. The angular restriction eliminates the need to search the full 180-degree space and reduces noise from diagonal text.

### Practical Thresholds

In PDF coordinate space (72 units per inch), a meaningful accumulator bin width is approximately 1.0 unit in rho (roughly 1/72 inch). A line is considered detected when its accumulator bin exceeds a count threshold proportional to the expected number of cells in that row or column — typically max(3, row_count * 0.5).

Post-process detected lines with non-maximum suppression in rho: within a 3-unit window, keep only the rho value with the highest accumulator count.

---

## Graph-Based Cell Reconstruction

Treat the set of detected line segments (from explicit paths, gap analysis, or Hough) as a planar straight-line graph (PSLG). Cells correspond to bounded faces of this graph.

### Finding Rectangular Faces

For each horizontal segment endpoint (x0, y), search rightward along y for the nearest vertical segment at x1 > x0 that spans y. Then search downward from x1 at y for the nearest horizontal segment at y1 < y. Then verify a closing vertical segment exists at x0 spanning [y1, y]. If all four edges are found, the region (x0, x1, y1, y) is a candidate cell.

### Junction Handling

T-junctions (three segments meeting) and L-junctions (two segments meeting at a corner) indicate partial borders. At a T-junction, the crossing segment does not divide the face; the cell extends across the missing interior edge. Track junction types during segment intersection enumeration and mark edges as "border present" or "border absent" accordingly.

### Row and Column Index Assignment

After all cells are identified, assign integer row and column indices:

1. Sort cells by top-left corner: primary key y (descending, since PDF y increases upward), secondary key x (ascending).
2. Group cells into rows by y-coordinate proximity (tolerance ±2 pt).
3. Within each row, assign column indices by x-order.

---

## Merged Cell Detection

A merged cell spanning multiple columns is identified by the absence of a vertical interior border between two adjacent column positions. When the graph traversal finds a cell whose x-extent covers more than one column-width interval, set `col_span > 1`.

A merged cell spanning multiple rows is identified by the absence of a horizontal interior border between two adjacent row positions. Set `row_span > 1` accordingly.

Validate merges by checking that the combined bounding box of the merged cell is flush with the enclosing grid lines: the outer border must exist even if the interior dividers do not.

---

## Header Row Detection

Header rows carry column labels and are distinguished from data rows by multiple signals. The implementation supports two primary detection methods:

### Bold Font Detection (Implemented)

A row is a bold-header if:
- It has at least 2 cells with content (single-cell rows don't qualify)
- 100% of its non-empty cells use bold fonts

Bold font detection uses heuristics based on PostScript naming conventions:
- Font name contains "Bold", "Bd", "Black", "Heavy", "ExtraBold", "Extrabold", "UltraBold", or "Ultrabold"
- Subset prefixes are stripped before checking (e.g., "ABCDEF+Helvetica-Bold" → "Helvetica-Bold")
- Whitespace-only cells are excluded from bold checks

The ForceBold flag (bit 19) in FontDescriptor flags is authoritative when present, but the name-based heuristic is used when that information is unavailable.

### StructTree TH Detection (Placeholder)

A row is a TH-header if every cell in the row maps to a TH StructElem (TR > TH chain in the structure tree). This requires:
1. MCID tracking on spans during content extraction
2. ParentTree lookup to find StructElem for each MCID
3. Verification that the StructElem is a TH within a TR

**Note:** TH detection is currently a stub that returns `false` for all rows. It will be implemented when MCID tracking is added to TableSpan.

### Combined Detection

The `is_header_row()` function combines both signals:
- A row is a header if **either** bold detection **or** TH detection succeeds
- If both signals are present, they confirm each other
- If there's a conflict (e.g., bold body row without TH tag), bold wins per the body data design principle

### Multi-Row Headers

Contiguous header rows from the top of the table are all marked as headers. The detection stops at the first non-header row (headers must be contiguous from row 0).

### Additional Signals (Not Yet Implemented)

Future enhancements may include:

| Signal | Weight | Status |
|--------|--------|--------|
| Font weight bold | High | ✅ Implemented |
| StructTree TH tag | High | ⏳ Pending MCID tracking |
| Font size larger than modal data row font size | High | ❌ Not implemented |
| Background fill color distinct from data rows | High | ❌ Not implemented |
| First row in the table | Medium | ✅ Implicit (contiguous from top) |
| Text content matches all-uppercase or title-case pattern | Low | ❌ Not implemented |
| Text content contains no numeric-only cells | Low | ❌ Not implemented |

---

## Multi-Page Tables

When the last detected table on page N and the first detected structure on page N+1 share a compatible column fingerprint, treat them as a continued table.

A **column fingerprint** is a sorted tuple of (normalized_x_start, normalized_x_end) pairs for each column, where coordinates are normalized to the page width. Two fingerprints match if their column count is equal and each corresponding column boundary pair differs by less than 3% of page width.

If the first row of the continuation page is a header row (detected as above) and its text content matches the header of the initial page, strip the repeated header from the continuation and record it as a `repeated_header` flag on the table.

---

## Output Representation

A reconstructed table is encoded in the extraction JSON as follows:

```json
{
  "type": "table",
  "page": 1,
  "bounding_box": { "x0": 72.0, "y0": 400.0, "x1": 540.0, "y1": 680.0 },
  "col_count": 3,
  "row_count": 5,
  "rows": [
    {
      "index": 0,
      "is_header": true,
      "cells": [
        {
          "row": 0,
          "col": 0,
          "row_span": 1,
          "col_span": 1,
          "bounding_box": { "x0": 72.0, "y0": 640.0, "x1": 216.0, "y1": 680.0 },
          "text": "Product",
          "border_present": { "top": true, "bottom": true, "left": true, "right": true }
        }
      ]
    }
  ],
  "continued_from_page": null,
  "continues_on_page": 2
}
```

Key field semantics:

- `bounding_box` uses PDF coordinate space (origin at bottom-left, y increases upward). Consumers converting to screen space must flip y.
- `row_span` and `col_span` are always >= 1. A standard unmerged cell has both equal to 1.
- `border_present` encodes which of the four cell edges had an explicit path operator or a sufficiently strong gap signal. This allows downstream renderers to faithfully reproduce the visual structure.
- `text` is the concatenation of glyphs within the cell bounding box, in reading order (left-to-right, top-to-bottom). Cells containing only images have an empty `text` field.
- `is_header` is set on cells in rows classified as headers; for merged header cells spanning multiple columns, all cells in the merged region carry the flag.
- `continued_from_page` and `continues_on_page` are `null` when the table fits on a single page, or contain the 1-based page index of the adjacent page fragment.

This representation is lossless with respect to the detected structure and provides sufficient metadata for downstream consumers to reconstruct a DOM-equivalent table, apply styling, or perform data extraction without re-analyzing geometry.

---

## Algorithm: Line-Based Grid Reconstruction

```python
def reconstruct_grid_from_lines(content_stream, page_width, page_height):
    """
    Reconstruct a table grid from explicit line drawing operators.
    
    Args:
        content_stream: Parsed PDF content stream operators
        page_width: Page width in PDF units
        page_height: Page height in PDF units
    
    Returns:
        Grid with horizontal_lines, vertical_lines, intersections, cells
    """
    # Step 1: Collect path segments
    horizontal_segments = []
    vertical_segments = []
    
    for op in content_stream:
        if op.operator == 'l' and op.next_operator == 'S':
            # Line-to followed by stroke
            x0, y0 = op.position
            x1, y1 = op.end_position
            if abs(y0 - y1) < 0.5:  # Horizontal
                horizontal_segments.append((min(x0, x1), max(x0, x1), y0))
            elif abs(x0 - x1) < 0.5:  # Vertical
                vertical_segments.append((min(y0, y1), max(y0, y1), x0))
        
        elif op.operator == 're':
            # Rectangle: expand to 4 segments
            x, y, w, h = op.rect
            horizontal_segments.append((x, x + w, y))          # Top
            horizontal_segments.append((x, x + w, y + h))      # Bottom
            vertical_segments.append((y, y + h, x))            # Left
            vertical_segments.append((y, y + h, x + w))        # Right
    
    # Step 2: Merge collinear segments
    horizontal_lines = merge_collinear(horizontal_segments, axis='y', gap_threshold=2.0)
    vertical_lines = merge_collinear(vertical_segments, axis='x', gap_threshold=2.0)
    
    # Step 3: Find intersections
    intersections = []
    for h_line in horizontal_lines:
        for v_line in vertical_lines:
            x = v_line.x  # x-coordinate of vertical line
            y = h_line.y  # y-coordinate of horizontal line
            if (h_line.x_min <= x <= h_line.x_max and
                v_line.y_min <= y <= v_line.y_max):
                intersections.append((x, y))
    
    # Step 4: Build cells from intersections
    cells = []
    sorted_x = sorted(set(i[0] for i in intersections))
    sorted_y = sorted(set(i[1] for i in intersections), reverse=True)  # PDF y is downward
    
    for i in range(len(sorted_y) - 1):
        for j in range(len(sorted_x) - 1):
            x0, x1 = sorted_x[j], sorted_x[j + 1]
            y0, y1 = sorted_y[i], sorted_y[i + 1]  # y0 > y1 in PDF coords
            
            # Verify all four edges exist
            top = any(l.y == y0 and l.x_min <= x0 and l.x_max >= x1 for l in horizontal_lines)
            bottom = any(l.y == y1 and l.x_min <= x0 and l.x_max >= x1 for l in horizontal_lines)
            left = any(l.x == x0 and l.y_min <= y1 and l.y_max >= y0 for l in vertical_lines)
            right = any(l.x == x1 and l.y_min <= y1 and l.y_max >= y0 for l in vertical_lines)
            
            if top and bottom and left and right:
                cells.append({
                    'row': i,
                    'col': j,
                    'bounding_box': {'x0': x0, 'y0': y1, 'x1': x1, 'y1': y0},
                    'border_present': {'top': top, 'bottom': bottom, 'left': left, 'right': right}
                })
    
    return Grid(horizontal_lines, vertical_lines, intersections, cells)


def merge_collinear(segments, axis, gap_threshold):
    """Merge segments that are collinear and overlapping/contiguous."""
    if not segments:
        return []
    
    # Group by coordinate along the orthogonal axis
    groups = {}
    for seg in segments:
        if axis == 'y':
            key = round(seg[2], 1)  # y-coordinate
        else:
            key = round(seg[2], 1)  # x-coordinate
        groups.setdefault(key, []).append(seg)
    
    # Merge within each group
    merged = []
    for coord, group in groups.items():
        group.sort(key=lambda s: s[0])  # Sort by start position
        current_start, current_end, _ = group[0]
        
        for start, end, _ in group[1:]:
            if start <= current_end + gap_threshold:
                # Overlapping or contiguous - extend
                current_end = max(current_end, end)
            else:
                # Gap - emit current segment and start new
                if axis == 'y':
                    merged.append(Line(current_start, current_end, coord))
                else:
                    merged.append(Line(current_start, current_end, coord))
                current_start, current_end = start, end
        
        # Emit final segment
        if axis == 'y':
            merged.append(Line(current_start, current_end, coord))
        else:
            merged.append(Line(current_start, current_end, coord))
    
    return merged
```

---

## Algorithm: Borderless Table Detection

```python
def detect_borderless_table(text_spans, page_width):
    """
    Detect a table structure from text alignment when no ruling lines exist.
    
    Args:
        text_spans: List of TextSpan objects with bounding boxes
        page_width: Page width for normalization
    
    Returns:
        Table with inferred rows and columns, or None if not tabular
    """
    # Step 1: Group spans into rows by y-coordinate proximity
    rows = group_spans_into_rows(text_spans, y_tolerance=2.0)
    
    if len(rows) < 3:
        return None  # Too few rows to be a table
    
    # Step 2: Build vertical projection profile
    # For each x-coordinate, count how many rows have text coverage
    projection = np.zeros(int(page_width))
    for row in rows:
        for span in row.spans:
            x_start = int(span.bbox.x0)
            x_end = int(span.bbox.x1)
            projection[x_start:x_end] += 1
    
    # Step 3: Find column separators (gaps in projection)
    # Compute median word space from intra-row gaps
    word_spaces = []
    for row in rows:
        for i in range(len(row.spans) - 1):
            gap = row.spans[i + 1].bbox.x0 - row.spans[i].bbox.x1
            if gap > 0:
                word_spaces.append(gap)
    
    if not word_spaces:
        return None
    
    median_word_space = np.median(word_spaces)
    min_column_gap = median_word_space * 2.5  # K = 2.5
    
    # Find gaps where projection drops to near-zero
    column_separators = []
    in_gap = False
    gap_start = 0
    
    for x in range(1, len(projection) - 1):
        is_gap = projection[x] < len(rows) * 0.3  # Coverage < 30% of rows
        
        if is_gap and not in_gap:
            gap_start = x
            in_gap = True
        elif not is_gap and in_gap:
            gap_width = x - gap_start
            if gap_width >= min_column_gap:
                column_separators.append((gap_start, x))
            in_gap = False
    
    if len(column_separators) < 1:
        return None  # No clear column structure
    
    # Step 4: Verify tabular structure
    # Check that at least 60% of rows share the same column count
    column_counts = []
    for row in rows:
        count = 0
        for sep_start, sep_end in column_separators:
            # Check if row has text spanning across this separator
            # (i.e., the separator falls within a gap for this row)
            has_gap = True
            for span in row.spans:
                if span.bbox.x0 < sep_end and span.bbox.x1 > sep_start:
                    has_gap = False
                    break
            if has_gap:
                count += 1
        column_counts.append(count + 1)  # +1 for columns on either side of separators
    
    # Mode of column counts
    mode_count = max(set(column_counts), key=column_counts.count)
    consistency = sum(1 for c in column_counts if c == mode_count) / len(column_counts)
    
    if consistency < 0.6:
        return None  # Column structure not consistent enough
    
    # Step 5: Build table from separators
    # Column boundaries are at: 0, sep[0].start, sep[0].end, sep[1].start, ..., page_width
    col_boundaries = [0.0]
    for sep_start, sep_end in column_separators:
        col_boundaries.append((sep_start + sep_end) / 2)  # Use gap midpoint
    col_boundaries.append(page_width)
    
    # Build cells
    cells = []
    for row_idx, row in enumerate(rows):
        for col_idx in range(len(col_boundaries) - 1):
            x0, x1 = col_boundaries[col_idx], col_boundaries[col_idx + 1]
            y0, y1 = row.y_max, row.y_min  # PDF y is downward
            
            # Collect text spans within this cell
            cell_text = []
            for span in row.spans:
                span_center_x = (span.bbox.x0 + span.bbox.x1) / 2
                if x0 <= span_center_x <= x1:
                    cell_text.append(span.text)
            
            cells.append({
                'row': row_idx,
                'col': col_idx,
                'bounding_box': {'x0': x0, 'y0': y1, 'x1': x1, 'y1': y0},
                'text': ' '.join(cell_text),
                'border_present': {'top': False, 'bottom': False, 'left': False, 'right': False}
            })
    
    return Table(
        row_count=len(rows),
        col_count=len(col_boundaries) - 1,
        cells=cells,
        confidence=consistency
    )
```

---

## Algorithm: Cell Content Assignment

```python
def assign_cell_contents(cells, text_spans, images):
    """
    Assign text spans and images to table cells based on centroid containment.
    
    Args:
        cells: List of cell bounding boxes from grid reconstruction
        text_spans: All text spans on the page
        images: All image bounding boxes on the page
    
    Returns:
        Cells with populated 'text' and 'images' fields
    """
    for cell in cells:
        cell_bbox = cell['bounding_box']
        cell_center_x = (cell_bbox['x0'] + cell_bbox['x1']) / 2
        cell_center_y = (cell_bbox['y0'] + cell_bbox['y1']) / 2
        
        # Collect text spans whose centroid is inside the cell
        cell_texts = []
        for span in text_spans:
            span_center_x = (span.bbox.x0 + span.bbox.x1) / 2
            span_center_y = (span.bbox.y0 + span.bbox.y1) / 2
            
            if (cell_bbox['x0'] <= span_center_x <= cell_bbox['x1'] and
                cell_bbox['y1'] <= span_center_y <= cell_bbox['y0']):  # PDF y is inverted
                cell_texts.append((span_center_x, span_center_y, span.text))
        
        # Sort by reading order (left-to-right, top-to-bottom)
        cell_texts.sort(key=lambda t: (-t[1], t[0]))  # Sort by y desc, then x asc
        cell['text'] = ' '.join(t[2] for t in cell_texts)
        
        # Collect images whose centroid is inside the cell
        cell_images = []
        for img in images:
            img_center_x = (img.bbox.x0 + img.bbox.x1) / 2
            img_center_y = (img.bbox.y0 + img.bbox.y1) / 2
            
            if (cell_bbox['x0'] <= img_center_x <= cell_bbox['x1'] and
                cell_bbox['y1'] <= img_center_y <= cell_bbox['y0']):
                cell_images.append(img)
        
        cell['images'] = cell_images
    
    return cells
```

---

## Version History

- **v1.0** (2025-01-24): Final-pass with complete pseudo-code listings for line-based grid reconstruction, borderless table detection, and cell content assignment algorithms. Added merged cell handling, multi-page table continuation detection, and StructTree TH detection specification.
- **v0.9** (2024-12-15): Initial 218-line draft covering line-based detection fundamentals, whitespace gap analysis, and output representation.

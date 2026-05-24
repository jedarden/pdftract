//! Cell representation and span-to-cell assignment (7.2.3).
//!
//! This module implements span-to-cell assignment using centroid containment:
//! - For each span, compute its centroid ((x0+x1)/2, (y0+y1)/2)
//! - Assign the span to the cell whose bbox contains the centroid
//! - Use half-open interval [x0, x1) to avoid double-counting border cases
//! - Spans not contained in any cell become orphans
//! - Within each cell, sort spans by (round(y0/2), x0) for reading order

use serde::{Deserialize, Serialize};

/// Y-bucket size for span ordering within cells (2 pt).
///
/// Spans with y-coordinates within 2 pt of each other are considered
/// on the same line for sorting purposes. This prevents tiny y noise
/// from reordering spans on the same line.
const Y_BUCKET_SIZE: f64 = 2.0;

/// Edge presence threshold for merged cell detection (80%).
///
/// An interior edge is considered "present" if at least 80% of its
/// expected length is covered by clustered segments. This tolerates
/// broken/dashed rules typical in PDFs exported from spreadsheets.
const EDGE_PRESENCE_THRESHOLD: f32 = 0.8;

/// Bold indicator patterns in PostScript font names.
///
/// These patterns are used to detect bold fonts when the ForceBold flag
/// is not available or authoritative.
const BOLD_PATTERNS: &[&str] = &[
    "Bold",
    "Bd",
    "Black",
    "Heavy",
    "ExtraBold",
    "Extrabold",
    "UltraBold",
    "Ultrabold",
];

/// Check if a font name indicates a bold font.
///
/// This function uses heuristics based on PostScript naming conventions:
/// - Font name contains "Bold", "Bd", "Black", "Heavy", "ExtraBold", etc.
/// - Subset prefixes are stripped before checking (e.g., "ABCDEF+Helvetica-Bold")
///
/// Note: The ForceBold flag (bit 19) in FontDescriptor flags is authoritative
/// when present, but this heuristic is used when that information is unavailable.
///
/// # Arguments
///
/// * `font_name` - The PostScript font name (may include subset prefix)
///
/// # Returns
///
/// `true` if the font name indicates a bold font, `false` otherwise.
pub fn is_bold_font(font_name: &str) -> bool {
    // Strip subset prefix if present (e.g., "ABCDEF+Helvetica-Bold" -> "Helvetica-Bold")
    let base_name = crate::font::strip_subset_prefix(font_name);

    // Check for bold indicators in the font name
    BOLD_PATTERNS
        .iter()
        .any(|pattern| base_name.contains(pattern))
}

/// Check if all text spans in a cell use bold fonts.
///
/// A cell is considered "bold" if 100% of its non-whitespace glyphs
/// are in bold fonts. Whitespace-only cells are excluded from bold checks.
///
/// # Arguments
///
/// * `cell` - The cell to check
///
/// # Returns
///
/// `true` if all non-whitespace text in the cell uses bold fonts.
pub fn is_cell_bold(cell: &Cell) -> bool {
    // Count non-whitespace spans
    let non_whitespace_spans: Vec<_> = cell
        .content
        .iter()
        .filter(|s| !s.text.trim().is_empty())
        .collect();

    if non_whitespace_spans.is_empty() {
        // Empty or whitespace-only cells don't count as bold
        return false;
    }

    // All non-whitespace spans must use bold fonts
    non_whitespace_spans
        .iter()
        .all(|span| is_bold_font(&span.font_name))
}

/// Check if a row is a header row based on bold font detection.
///
/// A row is a bold-header if:
/// - It has at least 2 cells with content (single-cell rows don't qualify)
/// - 100% of its non-empty cells are bold
///
/// # Arguments
///
/// * `row_cells` - All cells in the row
///
/// # Returns
///
/// `true` if the row qualifies as a header row based on bold detection.
pub fn is_bold_header_row(row_cells: &[&Cell]) -> bool {
    // Filter cells with content
    let non_empty_cells: Vec<_> = row_cells
        .iter()
        .filter(|c| !c.content.is_empty() && c.content.iter().any(|s| !s.text.trim().is_empty()))
        .collect();

    // Must have at least 2 cells with content
    if non_empty_cells.len() < 2 {
        return false;
    }

    // All non-empty cells must be bold
    non_empty_cells.iter().all(|c| is_cell_bold(c))
}

/// Check if a row is a header row based on StructTree TH detection.
///
/// A row is a TH-header if every cell in the row maps to a TH StructElem
/// (TR > TH chain in the structure tree). This requires:
/// 1. MCID tracking on spans (not yet implemented - TableSpan needs mcid field)
/// 2. ParentTree lookup to find StructElem for each MCID
/// 3. Verification that the StructElem is a TH within a TR
///
/// # Arguments
///
/// * `row_cells` - All cells in the row
///
/// # Returns
///
/// `true` if the row qualifies as a header row based on StructTree TH detection.
///
/// # Note
///
/// This function currently returns `false` for all rows because MCID tracking
/// on TableSpan is not yet implemented. When MCID tracking is added, this function
/// should:
/// 1. Collect all MCIDs from spans in each cell
/// 2. Look up the StructElem for each MCID via ParentTree
/// 3. Check if each cell's StructElem is a TH within a TR
/// 4. Return true only if all cells in the row are TH elements
pub fn is_th_header_row(_row_cells: &[&Cell]) -> bool {
    // TODO: Implement TH detection when MCID tracking is available on TableSpan
    // This requires:
    // 1. Add `mcid: Option<u32>` field to TableSpan
    // 2. Track MCIDs during span extraction from content stream
    // 3. Pass ParentTreeResolver to enable MCID -> StructElem lookup
    // 4. Verify each cell's StructElem is TH within a TR structure
    false
}

/// Check if a row is a header row using both bold and TH signals.
///
/// A row is considered a header if either:
/// - All cells are bold (bold signal)
/// - All cells map to TH StructElems (TH signal from StructTree)
///
/// If both signals are present, they confirm each other.
/// If there's a conflict (e.g., bold body row without TH tag), bold wins
/// per the body data design principle.
///
/// # Arguments
///
/// * `row_cells` - All cells in the row
///
/// # Returns
///
/// `true` if the row qualifies as a header row based on either signal.
pub fn is_header_row(row_cells: &[&Cell]) -> bool {
    is_bold_header_row(row_cells) || is_th_header_row(row_cells)
}

/// Count contiguous header rows starting from row 0.
///
/// This function detects multi-row headers by checking contiguous
/// rows from the top of the table that are headers (either bold or TH).
///
/// # Arguments
///
/// * `cells` - All cells in the table
/// * `row_count` - Number of rows in the table
///
/// # Returns
///
/// The number of contiguous header rows from the top (0 if none).
pub fn count_header_rows(cells: &[Cell], row_count: usize) -> u32 {
    let mut header_count = 0;

    for row_idx in 0..row_count {
        // Get all cells in this row
        let row_cells: Vec<_> = cells.iter().filter(|c| c.row == row_idx).collect();

        if row_cells.is_empty() {
            break;
        }

        // Check if this row is a header row (bold or TH)
        if is_header_row(&row_cells) {
            header_count += 1;
        } else {
            // Stop at first non-header row (headers must be contiguous)
            break;
        }
    }

    header_count
}

/// Detect and apply merged cells (rowspan/colspan) by examining missing interior edges.
///
/// This function implements merged cell detection (7.2.5) by checking which interior
/// grid edges are present vs. missing. When the interior edge between two adjacent
/// grid cells is absent, the cells are merged.
///
/// # Algorithm
///
/// 1. For each interior cell (not on the grid boundary), enumerate the four edges
///    that should bound it (top, bottom, left, right).
/// 2. An edge is "present" if at least 80% of its expected length is covered by
///    clustered segments from the grid.
/// 3. Missing right edge between cells (i, j) and (i+1, j) -> colspan extension.
/// 4. Missing bottom edge between cells (i, j) and (i, j+1) -> rowspan extension.
/// 5. Iterate until no more merges can be applied (transitive merges).
/// 6. Absorbed cells are excluded from the final Vec<Cell>.
///
/// # Arguments
///
/// * `cells` - The cells to merge (from `assign_spans_to_cells`)
/// * `grid` - The grid candidate with row/col boundaries and segments
///
/// # Returns
///
/// A tuple of (merged_cells, diagnostics):
/// - `merged_cells`: Cells with rowspan/colspan applied, absorbed cells removed
/// - `diagnostics`: Diagnostic messages about merge operations
///
/// # Borderless Tables
///
/// For borderless tables (grid.segments is empty), this function returns the
/// original cells unchanged with a diagnostic indicating that merged cell
/// detection is a NO-OP for borderless tables.
pub fn detect_merged_cells(
    mut cells: Vec<Cell>,
    grid: &super::GridCandidate,
) -> (Vec<Cell>, Vec<String>) {
    let mut diagnostics = Vec::new();

    // Borderless tables have no segments to infer from - NO-OP with diagnostic
    if grid.segments.is_empty() {
        diagnostics.push(
            "merged_cell_detection_skipped: borderless table has no segments for edge inference"
                .to_string(),
        );
        return (cells, diagnostics);
    }

    let row_count = grid.row_count();
    let col_count = grid.col_count();

    // Track which cells have been absorbed (removed from output)
    // Index is row * col_count + col
    let mut absorbed = vec![vec![false; col_count]; row_count];

    // Track merges in a loop until no more merges can be applied
    let mut merges_applied = true;
    while merges_applied {
        merges_applied = false;

        // Check each cell for merge opportunities
        for row in 0..row_count {
            for col in 0..col_count {
                // Skip if this cell was already absorbed
                if absorbed[row][col] {
                    continue;
                }

                // Find the cell at this position to get current colspan/rowspan
                let cell_idx = cells.iter().position(|c| c.row == row && c.col == col);
                let cell_colspan = cell_idx
                    .and_then(|idx| Some(cells[idx].colspan as usize))
                    .unwrap_or(1);
                let cell_rowspan = cell_idx
                    .and_then(|idx| Some(cells[idx].rowspan as usize))
                    .unwrap_or(1);

                // Check right edge (colspan) - check at the merged boundary
                let next_col = col + cell_colspan;
                if next_col < col_count && !absorbed[row][next_col] {
                    if !is_vertical_edge_present(grid, next_col, row, row + 1) {
                        // Missing right edge - merge with cell to the right
                        merge_cells_right(
                            &mut cells,
                            &mut absorbed,
                            row,
                            col,
                            col_count,
                            &mut diagnostics,
                        );
                        merges_applied = true;
                        // After merging, this cell may have absorbed more, so continue
                        // but don't check other directions for this cell in this iteration
                        continue;
                    }
                }

                // Check bottom edge (rowspan) - check at the merged boundary
                let next_row = row + cell_rowspan;
                if next_row < row_count && !absorbed[next_row][col] {
                    if !is_horizontal_edge_present(grid, next_row, col, col + 1) {
                        // Missing bottom edge - merge with cell below
                        merge_cells_down(
                            &mut cells,
                            &mut absorbed,
                            row,
                            col,
                            col_count,
                            &mut diagnostics,
                        );
                        merges_applied = true;
                        continue;
                    }
                }
            }
        }
    }

    // Remove absorbed cells from the output
    let merged_cells: Vec<Cell> = cells
        .into_iter()
        .filter(|c| !absorbed[c.row][c.col])
        .collect();

    (merged_cells, diagnostics)
}

/// Check if a vertical edge at a given x coordinate is present between two rows.
///
/// The edge is present if at least 80% of its length is covered by vertical segments.
fn is_vertical_edge_present(
    grid: &super::GridCandidate,
    edge_x_idx: usize, // Index of the vertical line in col_xs
    row_start: usize,  // Starting row index (inclusive)
    row_end: usize,    // Ending row index (exclusive)
) -> bool {
    let x = grid.col_xs[edge_x_idx];
    let y_top = grid.row_ys[row_start];
    let y_bottom = grid.row_ys[row_end];
    let expected_length = (y_top - y_bottom).abs();

    if expected_length < 0.1 {
        return true; // Degenerate edge, consider present
    }

    // Find all vertical segments that are collinear with this edge
    let mut covered_length = 0.0;
    const EPSILON: f32 = 1.0;

    for segment in &grid.segments {
        if segment.orientation != super::SegmentOrientation::Vertical {
            continue;
        }

        // Check if segment is collinear (same x within epsilon)
        if (segment.x0 - x).abs() > EPSILON {
            continue;
        }

        // Check if segment overlaps with the expected edge range
        let seg_y0 = segment.y0.max(y_bottom);
        let seg_y1 = segment.y1.min(y_top);

        if seg_y1 > seg_y0 {
            covered_length += seg_y1 - seg_y0;
        }
    }

    covered_length / expected_length >= EDGE_PRESENCE_THRESHOLD
}

/// Check if a horizontal edge at a given y coordinate is present between two columns.
///
/// The edge is present if at least 80% of its length is covered by horizontal segments.
fn is_horizontal_edge_present(
    grid: &super::GridCandidate,
    edge_y_idx: usize, // Index of the horizontal line in row_ys
    col_start: usize,  // Starting column index (inclusive)
    col_end: usize,    // Ending column index (exclusive)
) -> bool {
    let y = grid.row_ys[edge_y_idx];
    let x_left = grid.col_xs[col_start];
    let x_right = grid.col_xs[col_end];
    let expected_length = x_right - x_left;

    if expected_length < 0.1 {
        return true; // Degenerate edge, consider present
    }

    // Find all horizontal segments that are collinear with this edge
    let mut covered_length = 0.0;
    const EPSILON: f32 = 1.0;

    for segment in &grid.segments {
        if segment.orientation != super::SegmentOrientation::Horizontal {
            continue;
        }

        // Check if segment is collinear (same y within epsilon)
        if (segment.y0 - y).abs() > EPSILON {
            continue;
        }

        // Check if segment overlaps with the expected edge range
        let seg_x0 = segment.x0.max(x_left);
        let seg_x1 = segment.x1.min(x_right);

        if seg_x1 > seg_x0 {
            covered_length += seg_x1 - seg_x0;
        }
    }

    covered_length / expected_length >= EDGE_PRESENCE_THRESHOLD
}

/// Merge cell at (row, col) with cell to its right at the merged boundary.
///
/// Updates the surviving cell's colspan and bbox, marks the absorbed cell.
fn merge_cells_right(
    cells: &mut Vec<Cell>,
    absorbed: &mut Vec<Vec<bool>>,
    row: usize,
    col: usize,
    col_count: usize,
    diagnostics: &mut Vec<String>,
) {
    // Find the surviving cell
    let survivor_idx = cells
        .iter()
        .position(|c| c.row == row && c.col == col && !absorbed[row][col]);

    if let Some(s_idx) = survivor_idx {
        // Find the furthest column this cell already spans to
        let current_colspan = cells[s_idx].colspan as usize;
        let next_col = col + current_colspan;

        if next_col >= col_count || absorbed[row][next_col] {
            return; // Already absorbed or out of bounds
        }

        // Find the cell to absorb at the merged boundary
        let target_idx = cells
            .iter()
            .position(|c| c.row == row && c.col == next_col && !absorbed[row][next_col]);
        if let Some(t_idx) = target_idx {
            // Clone data before mutating cells
            let absorbed_content = cells[t_idx].content.clone();
            let absorbed_bbox = cells[t_idx].bbox[2];
            let absorbed_colspan = cells[t_idx].colspan;

            // Update survivor's colspan and bbox (add the target's colspan, not just 1)
            cells[s_idx].colspan += absorbed_colspan;
            cells[s_idx].bbox[2] = absorbed_bbox; // Expand x1

            // Transfer content from absorbed cell to survivor
            cells[s_idx].content.extend(absorbed_content);

            // Mark absorbed cell
            absorbed[row][next_col] = true;

            diagnostics.push(format!(
                "merged_cells: cell ({},{}) colspan={} absorbed cell ({},{})",
                row, col, cells[s_idx].colspan, row, next_col
            ));
        }
    }
}

/// Merge cell at (row, col) with cell below it at the merged boundary.
///
/// Updates the surviving cell's rowspan and bbox, marks the absorbed cell.
fn merge_cells_down(
    cells: &mut Vec<Cell>,
    absorbed: &mut Vec<Vec<bool>>,
    row: usize,
    col: usize,
    col_count: usize,
    diagnostics: &mut Vec<String>,
) {
    // Find the surviving cell
    let survivor_idx = cells
        .iter()
        .position(|c| c.row == row && c.col == col && !absorbed[row][col]);

    if let Some(s_idx) = survivor_idx {
        // Find the furthest row this cell already spans to
        let current_rowspan = cells[s_idx].rowspan as usize;
        let next_row = row + current_rowspan;

        if next_row >= absorbed.len() || absorbed[next_row][col] {
            return; // Already absorbed or out of bounds
        }

        // Find the cell to absorb at the merged boundary
        let target_idx = cells
            .iter()
            .position(|c| c.row == next_row && c.col == col && !absorbed[next_row][col]);
        if let Some(t_idx) = target_idx {
            // Clone data before mutating cells
            let absorbed_content = cells[t_idx].content.clone();
            let absorbed_bbox_y0 = cells[t_idx].bbox[1];
            let absorbed_rowspan = cells[t_idx].rowspan;

            // Update survivor's rowspan and bbox (add the target's rowspan, not just 1)
            cells[s_idx].rowspan += absorbed_rowspan;
            cells[s_idx].bbox[1] = absorbed_bbox_y0; // Expand y0 downward

            // Transfer content from absorbed cell to survivor
            cells[s_idx].content.extend(absorbed_content);

            // Mark absorbed cell
            absorbed[next_row][col] = true;

            diagnostics.push(format!(
                "merged_cells: cell ({},{}) rowspan={} absorbed cell ({},{})",
                row, col, cells[s_idx].rowspan, next_row, col
            ));
        }
    }
}

/// A text span for table cell assignment.
///
/// Minimal span representation used during cell assignment.
/// This is independent of the hybrid::Span type used in OCR processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSpan {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f64; 4],
    /// The extracted text.
    pub text: String,
    /// Font name (PostScript name, may include subset prefix).
    pub font_name: String,
}

impl TableSpan {
    /// Create a new table span.
    pub fn new(bbox: [f64; 4], text: String, font_name: String) -> Self {
        Self {
            bbox,
            text,
            font_name,
        }
    }

    /// Get the centroid of this span's bbox.
    fn centroid(&self) -> (f32, f32) {
        let cx = ((self.bbox[0] + self.bbox[2]) / 2.0) as f32;
        let cy = ((self.bbox[1] + self.bbox[3]) / 2.0) as f32;
        (cx, cy)
    }

    /// Get the width of this span.
    #[inline]
    fn width(&self) -> f64 {
        self.bbox[2] - self.bbox[0]
    }

    /// Get the height of this span.
    #[inline]
    fn height(&self) -> f64 {
        self.bbox[3] - self.bbox[1]
    }

    /// Get the area of this span.
    #[inline]
    fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// A table cell with its assigned content.
///
/// Represents a single cell in a detected table grid, including
/// its position and the text spans assigned to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Text spans assigned to this cell, sorted in reading order.
    pub content: Vec<TableSpan>,
    /// Row index (0-based, 0 = top row).
    pub row: usize,
    /// Column index (0-based, 0 = leftmost column).
    pub col: usize,
    /// Row span (default 1, >1 for merged cells).
    pub rowspan: u32,
    /// Column span (default 1, >1 for merged cells).
    pub colspan: u32,
    /// Whether this cell is in a header row.
    pub is_header_row: bool,
}

impl Cell {
    /// Create a new empty cell.
    pub fn new(bbox: [f32; 4], row: usize, col: usize) -> Self {
        Self {
            bbox,
            content: Vec::new(),
            row,
            col,
            rowspan: 1,
            colspan: 1,
            is_header_row: false,
        }
    }

    /// Mark header rows on a set of cells based on bold detection.
    ///
    /// This function counts contiguous header rows from the top of the table
    /// and sets the `is_header_row` flag on all cells in those rows.
    ///
    /// # Arguments
    ///
    /// * `cells` - Mutable slice of cells to update
    /// * `row_count` - Number of rows in the table
    ///
    /// # Returns
    ///
    /// The number of header rows detected (0 if none).
    pub fn mark_header_rows(cells: &mut [Cell], row_count: usize) -> u32 {
        let header_rows = count_header_rows(cells, row_count);

        // Mark all cells in header rows
        for cell in cells.iter_mut() {
            if cell.row < header_rows as usize {
                cell.is_header_row = true;
            }
        }

        header_rows
    }

    /// Check if this cell contains a point (centroid).
    ///
    /// Uses half-open interval [x0, x1) × [y0, y1) to avoid
    /// double-counting when a point falls exactly on a shared border.
    ///
    /// # Arguments
    ///
    /// * `px, py` - Point coordinates in PDF user space
    ///
    /// # Returns
    ///
    /// `true` if the point is contained, `false` otherwise.
    fn contains_point(&self, px: f32, py: f32) -> bool {
        // Half-open interval: x0 <= px < x1, y0 <= py < y1
        // Note: edge cells have their bbox extended by 0.5 pt in extend_bbox_for_edges
        px >= self.bbox[0] && px < self.bbox[2] && py >= self.bbox[1] && py < self.bbox[3]
    }

    /// Assign spans to cells based on centroid containment.
    ///
    /// # Arguments
    ///
    /// * `grid` - The grid candidate with row/col boundaries
    /// * `spans` - All text spans on the page
    ///
    /// # Returns
    ///
    /// A tuple of (cells, orphan_spans, diagnostics):
    /// - `cells`: Vector of cells with their assigned content
    /// - `orphan_spans`: Spans not assigned to any cell
    /// - `diagnostics`: Diagnostic messages about edge cases
    pub fn assign_spans_to_cells(
        grid: &super::GridCandidate,
        spans: Vec<TableSpan>,
    ) -> (Vec<Cell>, Vec<TableSpan>, Vec<String>) {
        let mut cells = Vec::new();
        let mut orphans = Vec::new();
        let mut diagnostics = Vec::new();

        // Create empty cells for the grid
        for row in 0..grid.row_count() {
            for col in 0..grid.col_count() {
                if let Some(bbox) = grid.cell_bbox(row, col) {
                    // Extend bbox by 0.5 pt for edge cells to capture spans flush to border
                    let bbox = extend_bbox_for_edges(bbox, row, col, grid);
                    cells.push(Cell::new(bbox, row, col));
                }
            }
        }

        // Assign each span to a cell based on centroid containment
        for span in spans {
            let (centroid_x, centroid_y) = span.centroid();

            // Find the cell index containing this centroid
            let mut assigned_cell_idx = None;
            for (idx, cell) in cells.iter().enumerate() {
                if cell.contains_point(centroid_x, centroid_y) {
                    assigned_cell_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = assigned_cell_idx {
                // Check if span overlaps multiple cells significantly
                check_overlap_and_diagnose(&span, &cells[idx], &cells, &mut diagnostics);
                cells[idx].content.push(span);
            } else {
                orphans.push(span);
            }
        }

        // Sort content within each cell by reading order
        for cell in &mut cells {
            sort_cell_content(cell);
        }

        (cells, orphans, diagnostics)
    }
}

/// Extend bbox by 0.5 pt for cells touching grid edges.
///
/// This captures spans that are flush to the table border.
/// Edge cells (top row, bottom row, leftmost col, rightmost col)
/// get their outer boundary extended by 0.5 pt.
fn extend_bbox_for_edges(
    mut bbox: [f32; 4],
    row: usize,
    col: usize,
    grid: &super::GridCandidate,
) -> [f32; 4] {
    // Top row: extend y1 upward
    if row == 0 {
        bbox[3] += 0.5;
    }
    // Bottom row: extend y0 downward
    if row == grid.row_count() - 1 {
        bbox[1] -= 0.5;
    }
    // Leftmost column: extend x0 leftward
    if col == 0 {
        bbox[0] -= 0.5;
    }
    // Rightmost column: extend x1 rightward
    if col == grid.col_count() - 1 {
        bbox[2] += 0.5;
    }

    bbox
}

/// Check if a span overlaps multiple cells and emit diagnostic.
///
/// If a span's centroid is in cell A but its bbox overlaps cell B
/// by more than 40%, emit a diagnostic.
///
/// Note: Due to the geometry of half-open intervals, it's mathematically
/// impossible for a span to have > 50% overlap with a cell while its
/// centroid is in a different cell. The maximum is 50% (achieved when
/// the centroid is exactly on the boundary, which falls in the right cell
/// due to half-open interval). We use 40% as a practical threshold.
fn check_overlap_and_diagnose(
    span: &TableSpan,
    assigned_cell: &Cell,
    all_cells: &[Cell],
    diagnostics: &mut Vec<String>,
) {
    let span_area = span.area() as f32;

    for other_cell in all_cells {
        if other_cell.row == assigned_cell.row && other_cell.col == assigned_cell.col {
            continue;
        }

        // Compute overlap area
        let overlap_x0 = (span.bbox[0] as f32).max(other_cell.bbox[0]);
        let overlap_y0 = (span.bbox[1] as f32).max(other_cell.bbox[1]);
        let overlap_x1 = (span.bbox[2] as f32).min(other_cell.bbox[2]);
        let overlap_y1 = (span.bbox[3] as f32).min(other_cell.bbox[3]);

        if overlap_x1 > overlap_x0 && overlap_y1 > overlap_y0 {
            let overlap_area = (overlap_x1 - overlap_x0) * (overlap_y1 - overlap_y0);
            let overlap_ratio = overlap_area / span_area;

            if overlap_ratio > 0.4 {
                let text_preview: String = span.text.chars().take(20).collect();
                diagnostics.push(format!(
                    "span_bbox_overlaps_multiple_cells: text='{}' centroid in ({},{}) but {:.1}% overlaps ({},{})",
                    text_preview,
                    assigned_cell.row, assigned_cell.col,
                    overlap_ratio * 100.0,
                    other_cell.row, other_cell.col
                ));
            }
        }
    }
}

/// Sort spans within a cell by reading order.
///
/// Uses (round(y0/2), x0) ordering with a 2-pt y bucket.
/// This groups spans on the same line and sorts left-to-right.
fn sort_cell_content(cell: &mut Cell) {
    // Use sort_by with index to ensure stability
    // We attach the original index to each element to preserve order for equal keys
    let mut indexed: Vec<_> = cell.content.iter().enumerate().collect();
    indexed.sort_by(|(ia, a), (ib, b)| {
        // Y-bucket: round to nearest 2 pt
        let y_bucket_a = (a.bbox[1] / Y_BUCKET_SIZE).round() as i64;
        let y_bucket_b = (b.bbox[1] / Y_BUCKET_SIZE).round() as i64;

        // Primary sort by y-bucket (descending - PDF y increases upward)
        match y_bucket_b.cmp(&y_bucket_a) {
            std::cmp::Ordering::Equal => {
                // Secondary sort by x0 (ascending - left to right)
                match a.bbox[0].partial_cmp(&b.bbox[0]) {
                    Some(std::cmp::Ordering::Equal) => {
                        // Tertiary sort by original index (ascending) for stability
                        ia.cmp(ib)
                    }
                    Some(ord) => ord,
                    None => ia.cmp(ib),
                }
            }
            other => other,
        }
    });

    // Reconstruct the sorted vector
    cell.content = indexed.into_iter().map(|(_, span)| span.clone()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::GridCandidate;

    fn make_span(x0: f64, y0: f64, x1: f64, y1: f64, text: &str) -> TableSpan {
        TableSpan::new([x0, y0, x1, y1], text.to_string(), "Helvetica".to_string())
    }

    fn make_bold_span(x0: f64, y0: f64, x1: f64, y1: f64, text: &str) -> TableSpan {
        TableSpan::new(
            [x0, y0, x1, y1],
            text.to_string(),
            "Helvetica-Bold".to_string(),
        )
    }

    #[test]
    fn test_cell_new() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);
        assert_eq!(cell.rowspan, 1);
        assert_eq!(cell.colspan, 1);
        assert!(cell.content.is_empty());
    }

    #[test]
    fn test_cell_contains_point_inside() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        // Point inside
        assert!(cell.contains_point(100.0, 150.0));
    }

    #[test]
    fn test_cell_contains_point_on_boundary() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        // Points on boundaries - half-open interval
        assert!(cell.contains_point(50.0, 150.0)); // x0 included
        assert!(cell.contains_point(100.0, 100.0)); // y0 included
        assert!(!cell.contains_point(150.0, 150.0)); // x1 excluded
        assert!(!cell.contains_point(100.0, 200.0)); // y1 excluded
    }

    #[test]
    fn test_cell_contains_point_outside() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        assert!(!cell.contains_point(49.0, 150.0)); // Left of cell
        assert!(!cell.contains_point(151.0, 150.0)); // Right of cell
        assert!(!cell.contains_point(100.0, 99.0)); // Below cell
        assert!(!cell.contains_point(100.0, 201.0)); // Above cell
    }

    #[test]
    fn test_cell_contains_point_with_epsilon() {
        // Test that edge extension works for cells on grid boundaries
        // Create a grid and check that edge cells have extended bounds
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create spans and assign to cells (which triggers edge extension)
        let spans = vec![
            make_span(49.8, 210.0, 60.0, 220.0, "edge_left"), // x0=49.8, just outside left border
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        // The span should be captured by the edge-extended cell
        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "edge_left");
    }

    #[test]
    fn test_assign_spans_to_cells_simple() {
        // Create a simple 2x2 grid
        // Horizontal lines at y = 100, 200, 300 (3 lines = 2 rows)
        // Vertical lines at x = 50, 150, 250 (3 lines = 2 cols)
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create spans with centroids in each cell
        let spans = vec![
            make_span(60.0, 210.0, 90.0, 240.0, "R0C0"), // Top row, left col
            make_span(160.0, 210.0, 190.0, 240.0, "R0C1"), // Top row, right col
            make_span(60.0, 110.0, 90.0, 140.0, "R1C0"), // Bottom row, left col
            make_span(160.0, 110.0, 190.0, 140.0, "R1C1"), // Bottom row, right col
        ];

        let (cells, orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(cells.len(), 4);
        assert_eq!(orphans.len(), 0);
        assert!(diagnostics.is_empty());

        // Check that each cell has the correct span
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "R0C0");

        let cell_r0c1 = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
        assert_eq!(cell_r0c1.content.len(), 1);
        assert_eq!(cell_r0c1.content[0].text, "R0C1");

        let cell_r1c0 = cells.iter().find(|c| c.row == 1 && c.col == 0).unwrap();
        assert_eq!(cell_r1c0.content.len(), 1);
        assert_eq!(cell_r1c0.content[0].text, "R1C0");

        let cell_r1c1 = cells.iter().find(|c| c.row == 1 && c.col == 1).unwrap();
        assert_eq!(cell_r1c1.content.len(), 1);
        assert_eq!(cell_r1c1.content[0].text, "R1C1");
    }

    #[test]
    fn test_assign_spans_centroid_on_border() {
        // Test that centroids exactly on borders are assigned deterministically
        // due to half-open interval [x0, x1)
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with centroid exactly on vertical border at x=150
        // Bbox: [140, 210, 160, 240] -> centroid at (150, 225)
        // Due to half-open interval [x0, x1), x=150 falls in cell (0, 1) because [150, 250) includes 150
        // but [50, 150) excludes 150 (upper bound is exclusive)
        let spans = vec![make_span(140.0, 210.0, 160.0, 240.0, "border_x")];

        let (cells, _orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        // Should be assigned to cell (0, 1) because x=150 falls in [150, 250) not [50, 150)
        let cell_r0c1 = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
        assert_eq!(cell_r0c1.content.len(), 1);
        assert_eq!(cell_r0c1.content[0].text, "border_x");
    }

    #[test]
    fn test_assign_orphan_spans() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span outside the grid
        let spans = vec![make_span(300.0, 210.0, 350.0, 240.0, "outside")];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].text, "outside");

        // No cell should have this span
        for cell in &cells {
            assert!(!cell.content.iter().any(|s| s.text == "outside"));
        }
    }

    #[test]
    fn test_span_overlaps_multiple_cells_diagnostic() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with centroid in cell (0, 0) but bbox extending into cell (0, 1)
        // Bbox: [100, 210, 199, 240] -> centroid at (149.5, 225)
        // Due to half-open interval [x0, x1), x=149.5 falls in cell (0, 0) [50, 150)
        // Overlap with cell (0, 1) which covers x=[150, 250)
        // Overlap area = (199 - 150) * (240 - 210) = 49 * 30 = 1470
        // Span area = 99 * 30 = 2970
        // Overlap ratio = 1470 / 2970 = 49.5% > 40%, should trigger diagnostic
        let spans = vec![make_span(100.0, 210.0, 199.0, 240.0, "overlap")];

        let (cells, _orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        // Verify span is assigned to cell (0, 0)
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "overlap");

        // Should have a diagnostic about overlapping multiple cells
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].contains("span_bbox_overlaps_multiple_cells"));
    }

    #[test]
    fn test_sort_cell_content_by_line() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);

        // Add spans in random order
        cell.content = vec![
            make_span(70.0, 110.0, 90.0, 120.0, "line2_right"), // Lower y, right
            make_span(60.0, 210.0, 90.0, 220.0, "line1_left"),  // Higher y, left
            make_span(60.0, 109.0, 80.0, 119.0, "line2_left"), // Lower y, left (same line as line2_right within 2pt)
        ];

        sort_cell_content(&mut cell);

        // Should be sorted by y (descending), then x (ascending)
        assert_eq!(cell.content[0].text, "line1_left"); // Highest y
        assert_eq!(cell.content[1].text, "line2_left"); // Same line bucket, leftmost
        assert_eq!(cell.content[2].text, "line2_right"); // Same line bucket, rightmost
    }

    #[test]
    fn test_5x3_bordered_table_critical_test() {
        // Critical test from plan: 5 columns × 3 rows = 15 cells
        // Horizontal lines at y = 100, 200, 300, 400 (4 lines = 3 rows)
        // Vertical lines at x = 50, 150, 250, 350, 450, 550 (6 lines = 5 columns)
        let mut intersections = Vec::new();
        for &y in &[400.0, 300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0, 550.0] {
                intersections.push((x, y));
            }
        }

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create a span for each of the 15 cells
        // Row 0 is top row (highest y), row 2 is bottom row (lowest y)
        let mut spans = Vec::new();
        for row in 0..3 {
            for col in 0..5 {
                let x0 = 50.0 + (col as f64) * 100.0 + 10.0;
                let x1 = x0 + 80.0;
                // Grid rows: row 0 has y in [300, 400], row 1 has y in [200, 300], row 2 has y in [100, 200]
                let y0 = 300.0 - ((row as f64) * 100.0) + 10.0;
                let y1 = y0 + 80.0;
                spans.push(make_span(x0, y0, x1, y1, &format!("R{}C{}", row, col)));
            }
        }

        let (cells, orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(cells.len(), 15);
        assert_eq!(orphans.len(), 0);
        assert!(diagnostics.is_empty());

        // Verify each cell has correct content
        for row in 0..3 {
            for col in 0..5 {
                let cell = cells.iter().find(|c| c.row == row && c.col == col).unwrap();
                assert_eq!(cell.content.len(), 1);
                assert_eq!(cell.content[0].text, format!("R{}C{}", row, col));
            }
        }
    }

    #[test]
    fn test_extend_bbox_for_top_row() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Top row cell (row 0) should have extended y1
        let bbox = grid.cell_bbox(0, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 0, &grid);
        assert_eq!(extended[3], bbox[3] + 0.5); // y1 extended
    }

    #[test]
    fn test_extend_bbox_for_bottom_row() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Bottom row cell (row 1) should have extended y0
        let bbox = grid.cell_bbox(1, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 1, 0, &grid);
        assert_eq!(extended[1], bbox[1] - 0.5); // y0 extended
    }

    #[test]
    fn test_extend_bbox_for_leftmost_column() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Leftmost column (col 0) should have extended x0
        let bbox = grid.cell_bbox(0, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 0, &grid);
        assert_eq!(extended[0], bbox[0] - 0.5); // x0 extended
    }

    #[test]
    fn test_extend_bbox_for_rightmost_column() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Rightmost column (col 1) should have extended x1
        let bbox = grid.cell_bbox(0, 1).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 1, &grid);
        assert_eq!(extended[2], bbox[2] + 0.5); // x1 extended
    }

    #[test]
    fn test_span_flush_to_border_captured() {
        // Test that spans flush to the table border are captured by edge extension
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with bbox flush to the left border (x0 = 50.0)
        // Centroid at (65, 250) - this is well inside the cell
        // But even if it were closer, the edge extension would capture it
        let spans = vec![make_span(50.0, 210.0, 80.0, 240.0, "flush_left")];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "flush_left");
    }

    #[test]
    fn test_multiple_spans_in_same_cell_sorted() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Multiple spans in the same cell, out of order
        // Cell (0, 0) has y in [200, 300], so all spans should be in that range
        let spans = vec![
            make_span(60.0, 210.0, 90.0, 220.0, "third"),  // Lower y
            make_span(60.0, 280.0, 90.0, 290.0, "first"),  // Higher y
            make_span(60.0, 245.0, 90.0, 255.0, "second"), // Middle y
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 3);

        // Should be sorted by y descending (reading order)
        assert_eq!(cell_r0c0.content[0].text, "first");
        assert_eq!(cell_r0c0.content[1].text, "second");
        assert_eq!(cell_r0c0.content[2].text, "third");
    }

    #[test]
    fn test_y_bucket_sorting() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);

        // Spans with tiny y differences (< 2 pt) should be on same line
        // y0 = 210, 210.5, 210.9 all round to same bucket: 210/2=105.0, 210.5/2=105.25, 210.9/2=105.45 -> all round to 105
        cell.content = vec![
            make_span(60.0, 210.0, 90.0, 220.0, "a"),  // y0 = 210
            make_span(60.0, 210.5, 90.0, 220.5, "b"),  // y0 = 210.5 (same 2-pt bucket as 210)
            make_span(70.0, 210.9, 100.0, 220.9, "c"), // y0 = 210.9 (same bucket, right of b)
        ];

        sort_cell_content(&mut cell);

        // All in same y-bucket, sorted by x
        assert_eq!(cell.content[0].text, "a"); // x0 = 60
        assert_eq!(cell.content[1].text, "b"); // x0 = 60 (same as a, stable order)
        assert_eq!(cell.content[2].text, "c"); // x0 = 70
    }

    #[test]
    fn test_table_span_centroid() {
        let span = make_span(100.0, 200.0, 200.0, 300.0, "test");
        let (cx, cy) = span.centroid();
        assert_eq!(cx, 150.0);
        assert_eq!(cy, 250.0);
    }

    #[test]
    fn test_table_span_area() {
        let span = make_span(100.0, 200.0, 200.0, 300.0, "test");
        assert_eq!(span.width(), 100.0);
        assert_eq!(span.height(), 100.0);
        assert_eq!(span.area(), 10000.0);
    }

    // Bold detection tests

    #[test]
    fn test_is_bold_font_helvetica_bold() {
        assert!(is_bold_font("Helvetica-Bold"));
        assert!(is_bold_font("ABCDEF+Helvetica-Bold")); // With subset prefix
    }

    #[test]
    fn test_is_bold_font_times_bold() {
        assert!(is_bold_font("Times-Bold"));
        assert!(is_bold_font("TimesNewRomanBold"));
    }

    #[test]
    fn test_is_bold_font_heavy_black() {
        assert!(is_bold_font("Arial-Black"));
        assert!(is_bold_font("Roboto-Heavy"));
        assert!(is_bold_font("Georgia-ExtraBold"));
    }

    #[test]
    fn test_is_bold_font_short_form() {
        assert!(is_bold_font("ArialBd"));
        assert!(is_bold_font("CalibriBd"));
    }

    #[test]
    fn test_is_bold_font_negative() {
        assert!(!is_bold_font("Helvetica"));
        assert!(!is_bold_font("Times-Italic"));
        assert!(!is_bold_font("Arial-Regular"));
        assert!(!is_bold_font("Georgia"));
    }

    #[test]
    fn test_is_cell_bold_all_bold() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        cell.content = vec![
            make_bold_span(60.0, 110.0, 90.0, 120.0, "Header"),
            make_bold_span(60.0, 125.0, 90.0, 135.0, "Text"),
        ];
        assert!(is_cell_bold(&cell));
    }

    #[test]
    fn test_is_cell_bold_mixed() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        cell.content = vec![
            make_bold_span(60.0, 110.0, 90.0, 120.0, "Bold"),
            make_span(60.0, 125.0, 90.0, 135.0, "Plain"), // Not bold
        ];
        assert!(!is_cell_bold(&cell));
    }

    #[test]
    fn test_is_cell_bold_all_plain() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        cell.content = vec![
            make_span(60.0, 110.0, 90.0, 120.0, "Plain"),
            make_span(60.0, 125.0, 90.0, 135.0, "Text"),
        ];
        assert!(!is_cell_bold(&cell));
    }

    #[test]
    fn test_is_cell_bold_empty() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        assert!(!is_cell_bold(&cell)); // Empty cell is not bold
    }

    #[test]
    fn test_is_cell_bold_whitespace_only() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        cell.content = vec![
            make_bold_span(60.0, 110.0, 90.0, 120.0, "   "),
            make_bold_span(60.0, 125.0, 90.0, 135.0, "\t"),
        ];
        assert!(!is_cell_bold(&cell)); // Whitespace-only cells don't count
    }

    #[test]
    fn test_is_bold_header_row_two_bold_cells() {
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        assert!(is_bold_header_row(&[&cell1, &cell2]));
    }

    #[test]
    fn test_is_bold_header_row_single_cell() {
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header")];

        // Single cell rows don't qualify as headers
        assert!(!is_bold_header_row(&[&cell1]));
    }

    #[test]
    fn test_is_bold_header_row_mixed_boldness() {
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Bold")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_span(160.0, 310.0, 190.0, 320.0, "Plain")];

        // Not all cells are bold
        assert!(!is_bold_header_row(&[&cell1, &cell2]));
    }

    #[test]
    fn test_is_bold_header_row_with_empty_cell() {
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        let cell3 = Cell::new([250.0, 300.0, 350.0, 400.0], 0, 2); // Empty

        // Empty cells are ignored, so 2 bold cells still qualify
        assert!(is_bold_header_row(&[&cell1, &cell2, &cell3]));
    }

    #[test]
    fn test_count_header_rows_single_header() {
        // Row 0: bold header
        // Row 1: plain data
        let mut cells = Vec::new();

        // Header row (0)
        let mut cell_r0c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell_r0c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        // Data row (1)
        let mut cell_r1c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 1, 0);
        cell_r1c0.content = vec![make_span(60.0, 210.0, 90.0, 220.0, "Data1")];

        let mut cell_r1c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 1, 1);
        cell_r1c1.content = vec![make_span(160.0, 210.0, 190.0, 220.0, "Data2")];

        cells.extend([cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1]);

        assert_eq!(count_header_rows(&cells, 2), 1);
    }

    #[test]
    fn test_count_header_rows_multi_row_header() {
        // Row 0: bold header
        // Row 1: bold subheader
        // Row 2: plain data
        let mut cells = Vec::new();

        // Header row 1 (0)
        let mut cell_r0c0 = Cell::new([50.0, 400.0, 150.0, 500.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 410.0, 90.0, 420.0, "Header1")];

        let mut cell_r0c1 = Cell::new([150.0, 400.0, 250.0, 500.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 410.0, 190.0, 420.0, "Header2")];

        // Header row 2 (1)
        let mut cell_r1c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 1, 0);
        cell_r1c0.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Sub1")];

        let mut cell_r1c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 1, 1);
        cell_r1c1.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Sub2")];

        // Data row (2)
        let mut cell_r2c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 2, 0);
        cell_r2c0.content = vec![make_span(60.0, 210.0, 90.0, 220.0, "Data1")];

        let mut cell_r2c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 2, 1);
        cell_r2c1.content = vec![make_span(160.0, 210.0, 190.0, 220.0, "Data2")];

        cells.extend([
            cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1, cell_r2c0, cell_r2c1,
        ]);

        assert_eq!(count_header_rows(&cells, 3), 2);
    }

    #[test]
    fn test_count_header_rows_no_header() {
        // All rows are plain
        let mut cells = Vec::new();

        for row in 0..2 {
            for col in 0..2 {
                let mut cell = Cell::new(
                    [
                        50.0,
                        300.0 - (row as f32) * 100.0,
                        150.0,
                        400.0 - (row as f32) * 100.0,
                    ],
                    row,
                    col,
                );
                cell.content = vec![make_span(
                    60.0,
                    310.0 - (row as f64) * 100.0,
                    90.0,
                    320.0 - (row as f64) * 100.0,
                    "Data",
                )];
                cells.push(cell);
            }
        }

        assert_eq!(count_header_rows(&cells, 2), 0);
    }

    #[test]
    fn test_count_header_rows_non_contiguous() {
        // Row 0: bold header
        // Row 1: plain data
        // Row 2: bold (but not contiguous, so not counted)
        let mut cells = Vec::new();

        // Header row (0)
        let mut cell_r0c0 = Cell::new([50.0, 400.0, 150.0, 500.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 410.0, 90.0, 420.0, "Header1")];

        let mut cell_r0c1 = Cell::new([150.0, 400.0, 250.0, 500.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 410.0, 190.0, 420.0, "Header2")];

        // Plain row (1)
        let mut cell_r1c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 1, 0);
        cell_r1c0.content = vec![make_span(60.0, 310.0, 90.0, 320.0, "Data1")];

        let mut cell_r1c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 1, 1);
        cell_r1c1.content = vec![make_span(160.0, 310.0, 190.0, 320.0, "Data2")];

        // Bold row (2) - not contiguous, should not be counted
        let mut cell_r2c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 2, 0);
        cell_r2c0.content = vec![make_bold_span(60.0, 210.0, 90.0, 220.0, "Total")];

        let mut cell_r2c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 2, 1);
        cell_r2c1.content = vec![make_bold_span(160.0, 210.0, 190.0, 220.0, "100")];

        cells.extend([
            cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1, cell_r2c0, cell_r2c1,
        ]);

        // Only row 0 is counted (row 2 is not contiguous)
        assert_eq!(count_header_rows(&cells, 3), 1);
    }

    #[test]
    fn test_mark_header_rows_single_header() {
        let mut cells = Vec::new();

        // Header row (0)
        let mut cell_r0c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell_r0c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        // Data row (1)
        let mut cell_r1c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 1, 0);
        cell_r1c0.content = vec![make_span(60.0, 210.0, 90.0, 220.0, "Data1")];

        let mut cell_r1c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 1, 1);
        cell_r1c1.content = vec![make_span(160.0, 210.0, 190.0, 220.0, "Data2")];

        cells.extend([cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1]);

        let header_count = Cell::mark_header_rows(&mut cells, 2);

        assert_eq!(header_count, 1);
        assert!(cells[0].is_header_row); // r0c0
        assert!(cells[1].is_header_row); // r0c1
        assert!(!cells[2].is_header_row); // r1c0
        assert!(!cells[3].is_header_row); // r1c1
    }

    #[test]
    fn test_mark_header_rows_multi_row_header() {
        let mut cells = Vec::new();

        // Header row 1 (0)
        let mut cell_r0c0 = Cell::new([50.0, 400.0, 150.0, 500.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 410.0, 90.0, 420.0, "H1")];

        let mut cell_r0c1 = Cell::new([150.0, 400.0, 250.0, 500.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 410.0, 190.0, 420.0, "H2")];

        // Header row 2 (1)
        let mut cell_r1c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 1, 0);
        cell_r1c0.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Sub1")];

        let mut cell_r1c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 1, 1);
        cell_r1c1.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Sub2")];

        // Data row (2)
        let mut cell_r2c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 2, 0);
        cell_r2c0.content = vec![make_span(60.0, 210.0, 90.0, 220.0, "D1")];

        let mut cell_r2c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 2, 1);
        cell_r2c1.content = vec![make_span(160.0, 210.0, 190.0, 220.0, "D2")];

        cells.extend([
            cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1, cell_r2c0, cell_r2c1,
        ]);

        let header_count = Cell::mark_header_rows(&mut cells, 3);

        assert_eq!(header_count, 2);
        assert!(cells[0].is_header_row); // r0c0
        assert!(cells[1].is_header_row); // r0c1
        assert!(cells[2].is_header_row); // r1c0
        assert!(cells[3].is_header_row); // r1c1
        assert!(!cells[4].is_header_row); // r2c0
        assert!(!cells[5].is_header_row); // r2c1
    }

    #[test]
    fn test_mark_header_rows_none() {
        let mut cells = Vec::new();

        // All plain rows
        for row in 0..2 {
            for col in 0..2 {
                let mut cell = Cell::new(
                    [
                        50.0,
                        300.0 - (row as f32) * 100.0,
                        150.0,
                        400.0 - (row as f32) * 100.0,
                    ],
                    row,
                    col,
                );
                cell.content = vec![make_span(
                    60.0,
                    310.0 - (row as f64) * 100.0,
                    90.0,
                    320.0 - (row as f64) * 100.0,
                    "Data",
                )];
                cells.push(cell);
            }
        }

        let header_count = Cell::mark_header_rows(&mut cells, 2);

        assert_eq!(header_count, 0);
        assert!(!cells[0].is_header_row);
        assert!(!cells[1].is_header_row);
        assert!(!cells[2].is_header_row);
        assert!(!cells[3].is_header_row);
    }

    // TH detection tests (placeholder for future MCID tracking implementation)

    #[test]
    fn test_is_th_header_row_not_implemented() {
        // TH detection is not yet implemented - requires MCID tracking on spans
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        // Currently returns false for all rows until MCID tracking is implemented
        assert!(!is_th_header_row(&[&cell1, &cell2]));
    }

    #[test]
    fn test_is_header_row_bold_signal() {
        // Combined header detection should work with bold signal
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        // Bold signal should work
        assert!(is_header_row(&[&cell1, &cell2]));
    }

    #[test]
    fn test_is_header_row_plain_row() {
        // Combined header detection should return false for plain rows
        let mut cell1 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell1.content = vec![make_span(60.0, 310.0, 90.0, 320.0, "Data1")];

        let mut cell2 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell2.content = vec![make_span(160.0, 310.0, 190.0, 320.0, "Data2")];

        // Plain row (no bold, no TH) should not be a header
        assert!(!is_header_row(&[&cell1, &cell2]));
    }

    #[test]
    fn test_count_header_rows_uses_combined_signal() {
        // Verify that count_header_rows uses the combined is_header_row function
        let mut cells = Vec::new();

        // Bold header row (0)
        let mut cell_r0c0 = Cell::new([50.0, 300.0, 150.0, 400.0], 0, 0);
        cell_r0c0.content = vec![make_bold_span(60.0, 310.0, 90.0, 320.0, "Header1")];

        let mut cell_r0c1 = Cell::new([150.0, 300.0, 250.0, 400.0], 0, 1);
        cell_r0c1.content = vec![make_bold_span(160.0, 310.0, 190.0, 320.0, "Header2")];

        // Plain data row (1)
        let mut cell_r1c0 = Cell::new([50.0, 200.0, 150.0, 300.0], 1, 0);
        cell_r1c0.content = vec![make_span(60.0, 210.0, 90.0, 220.0, "Data1")];

        let mut cell_r1c1 = Cell::new([150.0, 200.0, 250.0, 300.0], 1, 1);
        cell_r1c1.content = vec![make_span(160.0, 210.0, 190.0, 220.0, "Data2")];

        cells.extend([cell_r0c0, cell_r0c1, cell_r1c0, cell_r1c1]);

        // Should count 1 header row (bold signal)
        assert_eq!(count_header_rows(&cells, 2), 1);
    }

    // Merged cell detection tests (7.2.5)

    #[test]
    fn test_detect_merged_cells_borderless_table_noop() {
        // Borderless tables have no segments - should NO-OP with diagnostic
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let mut grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();
        // Borderless table has no segments
        grid.segments = vec![];

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
        ];

        let (merged, diagnostics) = detect_merged_cells(cells, &grid);

        // All cells should remain (no merges)
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[0].colspan, 1);
        assert_eq!(merged[0].rowspan, 1);

        // Should have diagnostic about borderless table
        assert!(diagnostics
            .iter()
            .any(|d| d.contains("merged_cell_detection_skipped")));
    }

    #[test]
    fn debug_test_colspan_3() {
        // Debug test to understand what's happening
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0] {
                intersections.push((x, y));
            }
        }

        let segments = vec![
            crate::table::Segment::horizontal(300.0, 50.0, 450.0),
            crate::table::Segment::horizontal(200.0, 50.0, 450.0),
            crate::table::Segment::horizontal(100.0, 50.0, 450.0),
            crate::table::Segment::vertical(50.0, 100.0, 300.0),
            crate::table::Segment::vertical(450.0, 100.0, 300.0),
            crate::table::Segment::vertical(350.0, 100.0, 300.0), // Full height
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        println!(
            "Grid: {} rows x {} cols",
            grid.row_count(),
            grid.col_count()
        );
        println!("row_ys: {:?}", grid.row_ys);
        println!("col_xs: {:?}", grid.col_xs);

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([350.0, 200.0, 450.0, 300.0], 0, 3),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
            Cell::new([350.0, 100.0, 450.0, 200.0], 1, 3),
        ];

        let (merged, diagnostics) = detect_merged_cells(cells, &grid);

        println!("\nMerged cells: {}", merged.len());
        for cell in &merged {
            println!(
                "  cell ({},{}) colspan={} rowspan={}",
                cell.row, cell.col, cell.colspan, cell.rowspan
            );
        }
        println!("\nDiagnostics:");
        for d in diagnostics {
            println!("  {}", d);
        }
    }

    #[test]
    fn test_detect_merged_cells_colspan_3_critical_test() {
        // Critical test from plan: merged header cell spanning 3 columns
        // Grid: 4 columns x 2 rows
        // Top row has merged cell (colspan=3) and one normal cell
        // Vertical edge at col_xs[1] and col_xs[2] are missing in row 0

        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0] {
                intersections.push((x, y));
            }
        }

        // Create segments: all grid edges EXCEPT the vertical edges at x=150 and x=250 in row 0
        // This creates a merged cell from col 0 to col 2 (colspan=3) in row 0 only
        let segments = vec![
            // Horizontal edges (all present)
            crate::table::Segment::horizontal(300.0, 50.0, 450.0), // Top edge
            crate::table::Segment::horizontal(200.0, 50.0, 450.0), // Middle edge
            crate::table::Segment::horizontal(100.0, 50.0, 450.0), // Bottom edge
            // Vertical edges
            crate::table::Segment::vertical(50.0, 100.0, 300.0), // Left edge (full height)
            crate::table::Segment::vertical(450.0, 100.0, 300.0), // Right edge (full height)
            crate::table::Segment::vertical(350.0, 100.0, 300.0), // Edge between cols 2-3 (full height)
            crate::table::Segment::vertical(150.0, 100.0, 200.0), // Edge between cols 0-1 (row 1 only)
            crate::table::Segment::vertical(250.0, 100.0, 200.0), // Edge between cols 1-2 (row 1 only)
                                                                  // MISSING: vertical edges at x=150 and x=250 in row 0 (creates merged cell in row 0)
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([350.0, 200.0, 450.0, 300.0], 0, 3),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
            Cell::new([350.0, 100.0, 450.0, 200.0], 1, 3),
        ];

        let (merged, diagnostics) = detect_merged_cells(cells, &grid);

        // Should have 6 cells (3 absorbed in top row)
        assert_eq!(merged.len(), 6);

        // Find the merged cell
        let merged_cell = merged.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(merged_cell.colspan, 3);
        assert_eq!(merged_cell.rowspan, 1);
        assert_eq!(merged_cell.bbox[2], 350.0); // x1 expanded to cover absorbed cells

        // Other cells should be normal
        let cell_r0c3 = merged.iter().find(|c| c.row == 0 && c.col == 3).unwrap();
        assert_eq!(cell_r0c3.colspan, 1);

        // Should have diagnostic messages about merges
        assert!(diagnostics.iter().any(|d| d.contains("merged_cells")));
    }

    #[test]
    fn test_detect_merged_cells_pure_rowspan() {
        // Test pure rowspan (vertical merge)
        // Grid: 3 columns x 3 rows
        // Left column has merged cell (rowspan=2)

        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0] {
                intersections.push((x, y));
            }
        }

        // Create segments: all edges EXCEPT the horizontal edge at y=200 in column 0
        let segments = vec![
            // Horizontal edges
            crate::table::Segment::horizontal(300.0, 50.0, 350.0), // Top edge
            crate::table::Segment::horizontal(200.0, 150.0, 350.0), // Middle edge (missing in col 0)
            crate::table::Segment::horizontal(100.0, 50.0, 350.0),  // Bottom edge
            // Vertical edges
            crate::table::Segment::vertical(50.0, 100.0, 300.0), // Left edge
            crate::table::Segment::vertical(150.0, 100.0, 300.0), // Col divider 1
            crate::table::Segment::vertical(250.0, 100.0, 300.0), // Col divider 2
            crate::table::Segment::vertical(350.0, 100.0, 300.0), // Right edge
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
        ];

        let (merged, _diagnostics) = detect_merged_cells(cells, &grid);

        // Should have 5 cells (1 absorbed)
        assert_eq!(merged.len(), 5);

        // Find the merged cell
        let merged_cell = merged.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(merged_cell.rowspan, 2);
        assert_eq!(merged_cell.colspan, 1);
        assert_eq!(merged_cell.bbox[1], 100.0); // y0 expanded downward
    }

    #[test]
    fn test_detect_merged_cells_diagonal_merge() {
        // Test diagonal merge (rowspan=2, colspan=2)
        // Grid: 3 columns x 2 rows
        // Top-left has merged cell covering 2x2 region

        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0] {
                intersections.push((x, y));
            }
        }

        // Create segments: missing interior edges in top-left 2x2 region
        // Row 0: [200, 300], Row 1: [100, 200]
        // Col 0: [50, 150], Col 1: [150, 250], Col 2: [250, 350]
        let segments = vec![
            // Horizontal edges (missing middle divider in top-left)
            crate::table::Segment::horizontal(300.0, 50.0, 350.0), // Top edge (y=300)
            crate::table::Segment::horizontal(200.0, 250.0, 350.0), // Middle edge (y=200, missing in cols 0-1)
            crate::table::Segment::horizontal(100.0, 50.0, 350.0),  // Bottom edge (y=100)
            // Vertical edges (missing middle divider in top-left)
            crate::table::Segment::vertical(50.0, 100.0, 300.0), // Left edge (x=50)
            crate::table::Segment::vertical(250.0, 200.0, 300.0), // Middle vertical (x=250, missing in rows 0-1)
            crate::table::Segment::vertical(350.0, 100.0, 300.0), // Right edge (x=350)
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
        ];

        let (merged, _diagnostics) = detect_merged_cells(cells, &grid);

        // Should have 3 cells:
        // - (0,0) with rowspan=2, colspan=2 (absorbs (0,1), (1,0), (1,1))
        // - (0,2) normal
        // - (1,2) normal
        assert_eq!(merged.len(), 3);

        // Find the diagonal merged cell
        let merged_cell = merged.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(merged_cell.rowspan, 2);
        assert_eq!(merged_cell.colspan, 2);
        assert_eq!(merged_cell.bbox[1], 100.0); // y0 expanded
        assert_eq!(merged_cell.bbox[2], 250.0); // x1 expanded
    }

    #[test]
    fn test_detect_merged_cells_no_merges_complete_grid() {
        // Test that a complete grid with all edges present results in no merges
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0] {
                intersections.push((x, y));
            }
        }

        // All edges present
        let segments = vec![
            crate::table::Segment::horizontal(300.0, 50.0, 350.0),
            crate::table::Segment::horizontal(200.0, 50.0, 350.0),
            crate::table::Segment::horizontal(100.0, 50.0, 350.0),
            crate::table::Segment::vertical(50.0, 100.0, 300.0),
            crate::table::Segment::vertical(150.0, 100.0, 300.0),
            crate::table::Segment::vertical(250.0, 100.0, 300.0),
            crate::table::Segment::vertical(350.0, 100.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
        ];

        let (merged, diagnostics) = detect_merged_cells(cells, &grid);

        // All cells should remain with no merges
        assert_eq!(merged.len(), 6);
        for cell in &merged {
            assert_eq!(cell.rowspan, 1);
            assert_eq!(cell.colspan, 1);
        }

        // No merge diagnostics
        assert!(!diagnostics.iter().any(|d| d.contains("merged_cells")));
    }

    #[test]
    fn test_is_vertical_edge_present_full_coverage() {
        // Test that a fully covered edge is detected as present
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0] {
                intersections.push((x, y));
            }
        }

        // Full coverage vertical edge at x=150
        let segments = vec![crate::table::Segment::vertical(150.0, 100.0, 300.0)];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        // Edge at x=150 between rows 0-1 should be present (100% coverage)
        assert!(is_vertical_edge_present(&grid, 1, 0, 1));
    }

    #[test]
    fn test_is_vertical_edge_present_partial_coverage_below_threshold() {
        // Test that a partially covered edge (<80%) is detected as absent
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0] {
                intersections.push((x, y));
            }
        }

        // Partial coverage (50% of edge length)
        let segments = vec![
            crate::table::Segment::vertical(150.0, 200.0, 250.0), // Only covers 50pt of 100pt edge
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        // Edge at x=150 between rows 0-1 should be absent (50% < 80% threshold)
        assert!(!is_vertical_edge_present(&grid, 1, 0, 1));
    }

    #[test]
    fn test_is_horizontal_edge_present_full_coverage() {
        // Test that a fully covered horizontal edge is detected as present
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0] {
                intersections.push((x, y));
            }
        }

        // Full coverage horizontal edge at y=200
        let segments = vec![crate::table::Segment::horizontal(200.0, 50.0, 250.0)];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        // Edge at y=200 between cols 0-1 should be present (100% coverage)
        assert!(is_horizontal_edge_present(&grid, 1, 0, 1));
    }

    #[test]
    fn test_is_horizontal_edge_present_partial_coverage_above_threshold() {
        // Test that a partially covered edge (>80%) is detected as present
        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0] {
                intersections.push((x, y));
            }
        }

        // Partial coverage (85% of edge length - 85pt of 100pt)
        let segments = vec![
            crate::table::Segment::horizontal(200.0, 50.0, 185.0), // Covers 85% of edge
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        // Edge at y=200 between cols 0-1 should be present (85% >= 80% threshold)
        assert!(is_horizontal_edge_present(&grid, 1, 0, 1));
    }

    #[test]
    fn test_detect_merged_cells_transitive_merge() {
        // Test transitive merges: cell (0,0) absorbs (0,1), then absorbs (0,2), then absorbs (0,3)
        // Grid: 4 columns x 2 rows
        // NO interior vertical edges (all cells in each row should merge)

        let mut intersections = Vec::new();
        for &y in &[300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0] {
                intersections.push((x, y));
            }
        }

        // Missing ALL interior vertical edges (no edges at x=150, 250, 350)
        let segments = vec![
            crate::table::Segment::horizontal(300.0, 50.0, 450.0),
            crate::table::Segment::horizontal(200.0, 50.0, 450.0),
            crate::table::Segment::horizontal(100.0, 50.0, 450.0),
            crate::table::Segment::vertical(50.0, 100.0, 300.0), // Left edge only
            crate::table::Segment::vertical(450.0, 100.0, 300.0), // Right edge only
        ];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();

        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
            Cell::new([250.0, 200.0, 350.0, 300.0], 0, 2),
            Cell::new([350.0, 200.0, 450.0, 300.0], 0, 3),
            Cell::new([50.0, 100.0, 150.0, 200.0], 1, 0),
            Cell::new([150.0, 100.0, 250.0, 200.0], 1, 1),
            Cell::new([250.0, 100.0, 350.0, 200.0], 1, 2),
            Cell::new([350.0, 100.0, 450.0, 200.0], 1, 3),
        ];

        let (merged, _diagnostics) = detect_merged_cells(cells, &grid);

        // Should have 2 cells (6 absorbed: 3 in row 0, 3 in row 1)
        // - (0,0) colspan=4
        // - (1,0) colspan=4
        assert_eq!(merged.len(), 2);

        let merged_cell_r0 = merged.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(merged_cell_r0.colspan, 4);
        assert_eq!(merged_cell_r0.bbox[2], 450.0); // x1 expanded to cover all 4 columns

        let merged_cell_r1 = merged.iter().find(|c| c.row == 1 && c.col == 0).unwrap();
        assert_eq!(merged_cell_r1.colspan, 4);
        assert_eq!(merged_cell_r1.bbox[2], 450.0);
    }
}

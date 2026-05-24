//! Table JSON output conversion (7.2.6).
//!
//! This module handles the conversion from detected table structures
//! (GridCandidate, Cell) to the JSON output format (TableJson, RowJson, CellJson).

use crate::schema::{TableJson, RowJson, CellJson};
use crate::table::{GridCandidate, Cell};
use crate::table::cell::TableSpan;
use anyhow::Result;

/// Distance from page edge to consider a table as "continued" (50 pt).
const CONTINUED_THRESHOLD: f32 = 50.0;

/// Maximum RMSE for column alignment similarity (5 pt).
const COLUMN_SIMILARITY_RMSE: f32 = 5.0;

/// Convert a detected table (grid + cells) to TableJson output format.
///
/// # Arguments
///
/// * `grid` - The grid candidate representing the table geometry
/// * `cells` - The cells with their assigned content
/// * `page_index` - The page index where this table appears
/// * `detection_method` - Either "line_based" or "borderless"
/// * `continued` - Whether this table continues on the next page
/// * `continued_from_prev` - Whether this table is a continuation from the previous page
///
/// # Returns
///
/// A `TableJson` ready for serialization.
pub fn grid_to_table_json(
    grid: &GridCandidate,
    cells: &[Cell],
    page_index: usize,
    detection_method: &str,
    continued: bool,
    continued_from_prev: bool,
) -> TableJson {
    // Build rows from cells
    let rows = build_rows_from_cells(cells, grid);

    // Count header rows (should already be set on cells)
    let header_rows = cells.iter()
        .filter(|c| c.is_header_row)
        .map(|c| c.row)
        .collect::<std::collections::HashSet<_>>()
        .len() as u32;

    TableJson {
        id: format!("table_{}", page_index),
        bbox: [
            grid.bbox[0] as f64,
            grid.bbox[1] as f64,
            grid.bbox[2] as f64,
            grid.bbox[3] as f64,
        ],
        rows,
        header_rows,
        detection_method: detection_method.to_string(),
        continued,
        continued_from_prev,
        page_index,
    }
}

/// Build RowJson structures from cells.
///
/// Groups cells by row index and creates RowJson for each.
fn build_rows_from_cells(cells: &[Cell], grid: &GridCandidate) -> Vec<RowJson> {
    let mut row_map: std::collections::HashMap<usize, Vec<&Cell>> = std::collections::HashMap::new();

    // Group cells by row
    for cell in cells {
        row_map.entry(cell.row).or_insert_with(Vec::new).push(cell);
    }

    // Create rows in order (top to bottom = row 0 to row_count-1)
    let mut rows = Vec::new();
    for row_idx in 0..grid.row_count() {
        if let Some(row_cells) = row_map.get(&row_idx) {
            // Convert cells to CellJson and sort by column
            let mut cells_json: Vec<CellJson> = row_cells.iter()
                .map(|c| cell_to_cell_json(c, grid))
                .collect();

            // Sort by column index
            cells_json.sort_by_key(|c| c.col);

            // Compute row bbox from all cells
            let row_bbox = compute_row_bbox(&cells_json);

            // Check if this is a header row (all cells are header cells or first cell is header)
            let is_header = !cells_json.is_empty() &&
                cells_json.iter().all(|c| c.is_header_row);

            rows.push(RowJson {
                bbox: row_bbox,
                cells: cells_json,
                is_header,
            });
        }
    }

    rows
}

/// Convert a Cell to CellJson.
fn cell_to_cell_json(cell: &Cell, _grid: &GridCandidate) -> CellJson {
    // Build span references (indices into the page-level spans array)
    // For now, use empty vec since we don't have the span indices here
    let spans = Vec::new();

    // Concatenate text from all spans in the cell
    let text = cell.content.iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    CellJson {
        bbox: [
            cell.bbox[0] as f64,
            cell.bbox[1] as f64,
            cell.bbox[2] as f64,
            cell.bbox[3] as f64,
        ],
        text,
        spans,
        row: cell.row,
        col: cell.col,
        rowspan: cell.rowspan,
        colspan: cell.colspan,
        is_header_row: cell.is_header_row,
    }
}

/// Compute the bounding box for a row from its cells.
fn compute_row_bbox(cells: &[CellJson]) -> [f64; 4] {
    if cells.is_empty() {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let mut x0 = cells[0].bbox[0];
    let mut y0 = cells[0].bbox[1];
    let mut x1 = cells[0].bbox[2];
    let mut y1 = cells[0].bbox[3];

    for cell in &cells[1..] {
        x0 = x0.min(cell.bbox[0]);
        y0 = y0.min(cell.bbox[1]);
        x1 = x1.max(cell.bbox[2]);
        y1 = y1.max(cell.bbox[3]);
    }

    [x0, y0, x1, y1]
}

/// Detect two-page table continuation between adjacent pages.
///
/// This function examines tables on adjacent pages and determines if they
/// represent a single table split across pages.
///
/// # Algorithm
///
/// For each pair of tables on page N and page N+1:
/// 1. Check if the table on page N ends within CONTINUED_THRESHOLD (50 pt) of page bottom
/// 2. Check if the table on page N+1 starts within CONTINUED_THRESHOLD (50 pt) of page top
/// 3. Verify both tables have the same column count
/// 4. Verify column x-positions are similar (RMSE < COLUMN_SIMILARITY_RMSE)
///
/// If all conditions are met, set:
/// - page N table: `continued = true`
/// - page N+1 table: `continued_from_prev = true`
///
/// # Arguments
///
/// * `all_tables` - Slice of tables for all pages, indexed by page_index
/// * `page_heights` - Page heights in points, to determine page edges
///
/// # Returns
///
/// A vector of (page_index, continued, continued_from_prev) tuples for each table.
pub fn detect_two_page_tables(
    all_tables: &[Vec<GridCandidate>],
    page_heights: &[f64],
) -> Vec<Vec<(bool, bool)>> {
    let mut results = Vec::new();

    for (page_idx, page_tables) in all_tables.iter().enumerate() {
        let page_flags = if page_tables.is_empty() {
            Vec::new()
        } else {
            page_tables.iter().map(|_| (false, false)).collect()
        };
        results.push(page_flags);
    }

    // Check adjacent page pairs
    for page_idx in 0..all_tables.len().saturating_sub(1) {
        let current_page_height = page_heights.get(page_idx).copied().unwrap_or(792.0);
        let next_page_height = page_heights.get(page_idx + 1).copied().unwrap_or(792.0);

        let current_tables = &all_tables[page_idx];
        let next_tables = &all_tables.get(page_idx + 1);

        if let Some(next_page_tables) = next_tables {
            // For each table on current page, check if any table on next page continues it
            for (table_idx, current_table) in current_tables.iter().enumerate() {
                // Check if this table ends near page bottom
                let table_y0 = current_table.bbox[1] as f64;
                let is_near_bottom = table_y0 <= CONTINUED_THRESHOLD as f64;

                if !is_near_bottom {
                    continue;
                }

                // Look for a continuing table on the next page
                for (next_table_idx, next_table) in next_page_tables.iter().enumerate() {
                    // Check if next table starts near page top
                    let next_table_y1 = next_table.bbox[3] as f64;
                    let page_top = next_page_height - CONTINUED_THRESHOLD as f64;
                    let is_near_top = next_table_y1 >= page_top;

                    if !is_near_top {
                        continue;
                    }

                    // Check column count match
                    if current_table.col_count() != next_table.col_count() {
                        continue;
                    }

                    // Check column position similarity
                    if columns_similar(current_table, next_table) {
                        // Match! Set flags
                        results[page_idx][table_idx].0 = true; // continued
                        results[page_idx + 1][next_table_idx].1 = true; // continued_from_prev
                    }
                }
            }
        }
    }

    results
}

/// Check if two grids have similar column positions.
///
/// Computes RMSE between column x-positions and checks if it's below threshold.
fn columns_similar(grid1: &GridCandidate, grid2: &GridCandidate) -> bool {
    if grid1.col_xs.len() != grid2.col_xs.len() {
        return false;
    }

    // Compute RMSE
    let sum_sq_error: f32 = grid1.col_xs.iter()
        .zip(grid2.col_xs.iter())
        .map(|(x1, x2)| (x1 - x2).powi(2))
        .sum();

    let mse = sum_sq_error / grid1.col_xs.len() as f32;
    let rmse = mse.sqrt();

    rmse < COLUMN_SIMILARITY_RMSE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Segment;

    #[test]
    fn test_grid_to_table_json_basic() {
        // Create a simple 2x2 grid
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create some cells
        let cells = vec![
            Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0),
            Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1),
        ];

        let table_json = grid_to_table_json(&grid, &cells, 0, "line_based", false, false);

        assert_eq!(table_json.id, "table_0");
        assert_eq!(table_json.page_index, 0);
        assert_eq!(table_json.detection_method, "line_based");
        assert!(!table_json.continued);
        assert!(!table_json.continued_from_prev);
        assert_eq!(table_json.rows.len(), 1);
    }

    #[test]
    fn test_build_rows_from_cells() {
        let grid = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ], vec![]).unwrap();

        let mut cell1 = Cell::new([50.0, 200.0, 150.0, 300.0], 0, 0);
        cell1.content = vec![
            TableSpan::new([50.0, 210.0, 90.0, 220.0], "Row1Col1".to_string(), "Helvetica".to_string())
        ];

        let mut cell2 = Cell::new([150.0, 200.0, 250.0, 300.0], 0, 1);
        cell2.content = vec![
            TableSpan::new([160.0, 210.0, 190.0, 220.0], "Row1Col2".to_string(), "Helvetica".to_string())
        ];

        let rows = build_rows_from_cells(&[cell1, cell2], &grid);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 2);
        assert_eq!(rows[0].cells[0].text, "Row1Col1");
        assert_eq!(rows[0].cells[1].text, "Row1Col2");
    }

    #[test]
    fn test_columns_similar_identical() {
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
        ], vec![]).unwrap();

        let grid2 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
        ], vec![]).unwrap();

        assert!(columns_similar(&grid1, &grid2));
    }

    #[test]
    fn test_columns_similar_small_difference() {
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
        ], vec![]).unwrap();

        // 2 pt shift in column positions
        let grid2 = GridCandidate::from_intersections(vec![
            (52.0, 100.0), (152.0, 100.0), (252.0, 100.0),
            (52.0, 200.0), (152.0, 200.0), (252.0, 200.0),
        ], vec![]).unwrap();

        // RMSE = 2.0 < 5.0, should be similar
        assert!(columns_similar(&grid1, &grid2));
    }

    #[test]
    fn test_columns_similar_large_difference() {
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
        ], vec![]).unwrap();

        // 10 pt shift in column positions
        let grid2 = GridCandidate::from_intersections(vec![
            (60.0, 100.0), (160.0, 100.0), (260.0, 100.0),
            (60.0, 200.0), (160.0, 200.0), (260.0, 200.0),
        ], vec![]).unwrap();

        // RMSE = 10.0 > 5.0, should NOT be similar
        assert!(!columns_similar(&grid1, &grid2));
    }

    #[test]
    fn test_columns_similar_different_count() {
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
        ], vec![]).unwrap();

        let grid2 = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
        ], vec![]).unwrap();

        assert!(!columns_similar(&grid1, &grid2));
    }

    #[test]
    fn test_detect_two_page_tables_basic() {
        // Page 0: table ending at y=40 (within 50 pt of page bottom at 0)
        let grid0 = GridCandidate::from_intersections(vec![
            (50.0, 40.0), (150.0, 40.0),
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 150.0), (150.0, 150.0),
        ], vec![]).unwrap();

        // Page 1: table starting at y=750 (within 50 pt of page top at 792)
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 750.0), (150.0, 750.0),
            (50.0, 800.0), (150.0, 800.0),
            (50.0, 850.0), (150.0, 850.0),
        ], vec![]).unwrap();

        let all_tables = vec![vec![grid0], vec![grid1]];
        let page_heights = vec![792.0, 792.0];

        let results = detect_two_page_tables(&all_tables, &page_heights);

        // Page 0 table should be marked as continued
        assert!(results[0][0].0); // continued = true

        // Page 1 table should be marked as continued_from_prev
        assert!(results[1][0].1); // continued_from_prev = true
    }

    #[test]
    fn test_detect_two_page_tables_no_continuation() {
        // Page 0: table ending at y=200 (NOT within 50 pt of page bottom)
        let grid0 = GridCandidate::from_intersections(vec![
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ], vec![]).unwrap();

        // Page 1: table starting at y=700 (NOT within 50 pt of page top)
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 700.0), (150.0, 700.0),
            (50.0, 800.0), (150.0, 800.0),
        ], vec![]).unwrap();

        let all_tables = vec![vec![grid0], vec![grid1]];
        let page_heights = vec![792.0, 792.0];

        let results = detect_two_page_tables(&all_tables, &page_heights);

        // Neither table should be marked as continuation
        assert!(!results[0][0].0); // continued = false
        assert!(!results[1][0].1); // continued_from_prev = false
    }

    #[test]
    fn test_detect_two_page_tables_different_column_count() {
        // Page 0: 2-column table ending near page bottom
        let grid0 = GridCandidate::from_intersections(vec![
            (50.0, 40.0), (150.0, 40.0), (250.0, 40.0),
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
        ], vec![]).unwrap();

        // Page 1: 3-column table starting near page top
        let grid1 = GridCandidate::from_intersections(vec![
            (50.0, 750.0), (150.0, 750.0), (250.0, 750.0), (350.0, 750.0),
            (50.0, 800.0), (150.0, 800.0), (250.0, 800.0), (350.0, 800.0),
        ], vec![]).unwrap();

        let all_tables = vec![vec![grid0], vec![grid1]];
        let page_heights = vec![792.0, 792.0];

        let results = detect_two_page_tables(&all_tables, &page_heights);

        // Different column counts, should not be marked as continuation
        assert!(!results[0][0].0);
        assert!(!results[1][0].1);
    }

    #[test]
    fn test_cell_to_cell_json_text_concatenation() {
        let grid = GridCandidate::from_intersections(vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
        ], vec![]).unwrap();

        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        cell.content = vec![
            TableSpan::new([50.0, 150.0, 90.0, 160.0], "Hello".to_string(), "Helvetica".to_string()),
            TableSpan::new([50.0, 140.0, 90.0, 150.0], "World".to_string(), "Helvetica".to_string()),
        ];

        let cell_json = cell_to_cell_json(&cell, &grid);

        assert_eq!(cell_json.text, "Hello World");
    }
}

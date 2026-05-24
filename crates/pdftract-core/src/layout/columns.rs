//! Column label assignment for Phase 4.3.
//!
//! This module implements assigning column indices to spans and lines
//! based on confirmed column x_ranges.

use std::collections::HashMap;

/// A confirmed column with its x_range and index.
///
/// The x_range is [x0, x1] in PDF user space coordinates.
/// Spans whose bbox[0] falls within this range are assigned to this column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// Column index (0-based, monotonic left-to-right).
    pub index: u32,
    /// X range [x0, x1] defining the column bounds.
    pub x_range: [f32; 2],
}

impl Column {
    /// Create a new column with the given index and x_range.
    #[inline]
    pub fn new(index: u32, x_range: [f32; 2]) -> Self {
        Self { index, x_range }
    }

    /// Check if a given x coordinate falls within this column's x_range.
    #[inline]
    pub fn contains(&self, x: f32) -> bool {
        x >= self.x_range[0] && x < self.x_range[1]
    }
}

/// Assign column indices to spans based on confirmed columns.
///
/// For each span, finds the confirmed column whose x_range contains
/// span.bbox[0]. Spans outside any column get column = None.
///
/// # Arguments
///
/// * `spans` - Spans to assign columns to (must have bbox and column fields)
/// * `columns` - Confirmed columns with x_ranges
///
/// # Behavior
///
/// - Spans are assigned by their x0 coordinate (bbox[0])
/// - Spans outside all columns get `column = None`
/// - Column indices are monotonic left-to-right (INV)
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::columns::{assign_columns_to_spans, Column};
///
/// let columns = vec![
///     Column::new(0, [0.0, 300.0]),
///     Column::new(1, [320.0, 600.0]),
/// ];
///
/// // Span at x0=50 -> column 0
/// // Span at x0=350 -> column 1
/// // Span at x0=310 (gap) -> None
/// ```
pub fn assign_columns_to_spans<S>(spans: &mut [S], columns: &[Column])
where
    S: HasBBoxAndColumn,
{
    for span in spans.iter_mut() {
        let x0 = span.bbox()[0] as f32;
        let assigned = columns.iter().find(|c| c.contains(x0));
        span.set_column(assigned.map(|c| c.index));
    }
}

/// Propagate column indices from spans to lines via mode.
///
/// For each line, computes the mode (most common value) of member spans'
/// columns. If a single column dominates (>50% of spans), assign it.
/// Otherwise, assign None (mixed or no dominant column).
///
/// # Arguments
///
/// * `lines` - Lines to assign columns to
///
/// # Behavior
///
/// - Lines with all spans in same column: that column
/// - Lines with >50% spans in one column: that column
/// - Lines with no clear dominant column: None (e.g., full-width headings)
/// - Empty lines: None
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::columns::assign_columns_to_lines;
///
/// // Line with 3 spans in column 0, 1 span in column 1 -> column 0
/// // Line with 2 spans in column 0, 2 spans in column 1 -> None (mixed)
/// ```
pub fn assign_columns_to_lines<L>(lines: &mut [L])
where
    L: HasSpansWithColumn,
{
    for line in lines.iter_mut() {
        let column_counts = line.count_columns();
        let total_spans = line.span_count();

        if total_spans == 0 {
            line.set_column(None);
            continue;
        }

        // Find the column with maximum count
        let max_entry = column_counts.into_iter().max_by_key(|&(_, count)| count);

        if let Some((col, count)) = max_entry {
            // Assign column only if it dominates (>50% of spans)
            if count * 2 > total_spans {
                line.set_column(Some(col));
            } else {
                line.set_column(None);
            }
        } else {
            line.set_column(None);
        }
    }
}

/// Trait for types that have a bbox and column field.
///
/// This trait allows the column assignment code to work with different
/// span representations (internal, JSON, etc.).
pub trait HasBBoxAndColumn {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f64; 4];

    /// Set the column index.
    fn set_column(&mut self, column: Option<u32>);
}

/// Trait for types that contain spans with column information.
///
/// This trait allows the column propagation code to work with different
/// line representations.
pub trait HasSpansWithColumn {
    /// Count occurrences of each column among member spans.
    ///
    /// Returns a HashMap mapping column index to count.
    /// Spans with column=None are excluded.
    fn count_columns(&self) -> HashMap<u32, usize>;

    /// Get the total number of spans in this line.
    fn span_count(&self) -> usize;

    /// Set the column index for this line.
    fn set_column(&mut self, column: Option<u32>);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test span with bbox and column.
    #[derive(Debug, Clone)]
    struct TestSpan {
        bbox: [f64; 4],
        column: Option<u32>,
    }

    impl TestSpan {
        fn new(bbox: [f64; 4]) -> Self {
            Self { bbox, column: None }
        }
    }

    impl HasBBoxAndColumn for TestSpan {
        fn bbox(&self) -> [f64; 4] {
            self.bbox
        }

        fn set_column(&mut self, column: Option<u32>) {
            self.column = column;
        }
    }

    /// Test line with spans.
    #[derive(Debug, Clone)]
    struct TestLine {
        spans: Vec<TestSpan>,
        column: Option<u32>,
    }

    impl TestLine {
        fn new(spans: Vec<TestSpan>) -> Self {
            Self {
                spans,
                column: None,
            }
        }
    }

    impl HasSpansWithColumn for TestLine {
        fn count_columns(&self) -> HashMap<u32, usize> {
            let mut counts = HashMap::new();
            for span in &self.spans {
                if let Some(col) = span.column {
                    *counts.entry(col).or_insert(0) += 1;
                }
            }
            counts
        }

        fn span_count(&self) -> usize {
            self.spans.len()
        }

        fn set_column(&mut self, column: Option<u32>) {
            self.column = column;
        }
    }

    #[test]
    fn test_column_new() {
        let col = Column::new(0, [0.0, 300.0]);
        assert_eq!(col.index, 0);
        assert_eq!(col.x_range, [0.0, 300.0]);
    }

    #[test]
    fn test_column_contains_within() {
        let col = Column::new(0, [0.0, 300.0]);
        assert!(col.contains(50.0));
        assert!(col.contains(0.0));
        assert!(!col.contains(300.0)); // x1 is exclusive
    }

    #[test]
    fn test_column_contains_outside() {
        let col = Column::new(0, [0.0, 300.0]);
        assert!(!col.contains(-10.0));
        assert!(!col.contains(350.0));
    }

    #[test]
    fn test_assign_columns_to_spans_two_column() {
        let columns = vec![Column::new(0, [0.0, 300.0]), Column::new(1, [320.0, 600.0])];

        let mut spans = vec![
            TestSpan::new([50.0, 100.0, 200.0, 120.0]), // x0=50 -> col 0
            TestSpan::new([350.0, 100.0, 450.0, 120.0]), // x0=350 -> col 1
            TestSpan::new([310.0, 100.0, 320.0, 120.0]), // x0=310 (gap) -> None
        ];

        assign_columns_to_spans(&mut spans, &columns);

        assert_eq!(spans[0].column, Some(0));
        assert_eq!(spans[1].column, Some(1));
        assert_eq!(spans[2].column, None);
    }

    #[test]
    fn test_assign_columns_to_spans_empty() {
        let columns = vec![Column::new(0, [0.0, 300.0])];
        let mut spans: Vec<TestSpan> = vec![];
        assign_columns_to_spans(&mut spans, &columns);
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_assign_columns_to_spans_single_column() {
        let columns = vec![Column::new(0, [0.0, 600.0])];
        let mut spans = vec![
            TestSpan::new([50.0, 100.0, 200.0, 120.0]),
            TestSpan::new([350.0, 100.0, 450.0, 120.0]),
        ];

        assign_columns_to_spans(&mut spans, &columns);

        assert_eq!(spans[0].column, Some(0));
        assert_eq!(spans[1].column, Some(0));
    }

    #[test]
    fn test_assign_columns_to_lines_unanimous() {
        // Line with all spans in column 0 -> column 0
        let spans = vec![
            {
                let mut s = TestSpan::new([0.0, 0.0, 100.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([100.0, 0.0, 200.0, 10.0]);
                s.column = Some(0);
                s
            },
        ];
        let mut lines = vec![TestLine::new(spans)];

        assign_columns_to_lines(&mut lines);

        assert_eq!(lines[0].column, Some(0));
    }

    #[test]
    fn test_assign_columns_to_lines_dominant() {
        // Line with 3 spans in col 0, 1 span in col 1 -> col 0 (>50%)
        let spans = vec![
            {
                let mut s = TestSpan::new([0.0, 0.0, 100.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([100.0, 0.0, 200.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([200.0, 0.0, 300.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([400.0, 0.0, 500.0, 10.0]);
                s.column = Some(1);
                s
            },
        ];
        let mut lines = vec![TestLine::new(spans)];

        assign_columns_to_lines(&mut lines);

        assert_eq!(lines[0].column, Some(0));
    }

    #[test]
    fn test_assign_columns_to_lines_mixed() {
        // Line with 2 spans in col 0, 2 spans in col 1 -> None (no >50%)
        let spans = vec![
            {
                let mut s = TestSpan::new([0.0, 0.0, 100.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([100.0, 0.0, 200.0, 10.0]);
                s.column = Some(0);
                s
            },
            {
                let mut s = TestSpan::new([400.0, 0.0, 500.0, 10.0]);
                s.column = Some(1);
                s
            },
            {
                let mut s = TestSpan::new([500.0, 0.0, 600.0, 10.0]);
                s.column = Some(1);
                s
            },
        ];
        let mut lines = vec![TestLine::new(spans)];

        assign_columns_to_lines(&mut lines);

        assert_eq!(lines[0].column, None);
    }

    #[test]
    fn test_assign_columns_to_lines_full_width_heading() {
        // Full-width heading: all spans None -> line None
        let spans = vec![{
            let mut s = TestSpan::new([0.0, 0.0, 600.0, 10.0]);
            s.column = None;
            s
        }];
        let mut lines = vec![TestLine::new(spans)];

        assign_columns_to_lines(&mut lines);

        assert_eq!(lines[0].column, None);
    }

    #[test]
    fn test_assign_columns_to_lines_empty() {
        let mut lines = vec![TestLine::new(vec![])];

        assign_columns_to_lines(&mut lines);

        assert_eq!(lines[0].column, None);
    }

    #[test]
    fn test_column_index_monotonic_left_to_right() {
        // INV: column index monotonic left-to-right
        let columns = vec![
            Column::new(0, [0.0, 200.0]),
            Column::new(1, [200.0, 400.0]),
            Column::new(2, [400.0, 600.0]),
        ];

        assert!(columns[0].x_range[0] < columns[1].x_range[0]);
        assert!(columns[1].x_range[0] < columns[2].x_range[0]);
        assert!(columns[0].index < columns[1].index);
        assert!(columns[1].index < columns[2].index);
    }

    #[test]
    fn test_span_straddling_gap_assigned_by_x0() {
        // Span straddling gap: assigned by x0
        let columns = vec![Column::new(0, [0.0, 300.0]), Column::new(1, [320.0, 600.0])];

        // Span starts at 290 (in col 0) but extends to 350 (into gap/col 1)
        let mut spans = vec![TestSpan::new([290.0, 100.0, 350.0, 120.0])];

        assign_columns_to_spans(&mut spans, &columns);

        // Should be assigned to col 0 based on x0
        assert_eq!(spans[0].column, Some(0));
    }
}

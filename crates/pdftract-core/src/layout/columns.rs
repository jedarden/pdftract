//! Column label assignment for Phase 4.3.
//!
//! This module implements assigning column indices to spans and lines
//! based on confirmed column x_ranges.

use std::collections::HashMap;
use tracing::warn;

/// Build a histogram of x0 coordinates for column detection.
///
/// Returns a `Vec<u32>` of length `ceil(page_width)`, indexed by x0 (rounded to
/// nearest integer point). Each span contributes 1 to the bucket at its x0.
///
/// # Arguments
///
/// * `spans` - Spans to histogram (must have bbox accessible)
/// * `page_width` - Page width in points
///
/// # Returns
///
/// A histogram where `hist[i]` is the count of spans whose x0 rounds to i.
///
/// # Behavior
///
/// - For each span: `idx = span.bbox\[0\].round() as usize`
/// - Clamp idx to `\[0, hist.len() - 1\]`
/// - x0 < 0: clamped to 0, diagnostic logged
/// - x0 > page_width: clamped to last bucket, diagnostic logged
/// - Empty spans: returns Vec of zeros
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::columns::build_x0_histogram;
///
/// let spans: Vec<[f32; 4]> = vec![
///     [100.0, 0.0, 200.0, 10.0], // x0=100
///     [100.0, 0.0, 200.0, 10.0], // x0=100
///     [200.0, 0.0, 300.0, 10.0], // x0=200
///     [200.0, 0.0, 300.0, 10.0], // x0=200
///     [300.0, 0.0, 400.0, 10.0], // x0=300
/// ];
/// let hist = build_x0_histogram(&spans, 612.0);
/// assert_eq!(hist[100], 2);
/// assert_eq!(hist[200], 2);
/// assert_eq!(hist[300], 1);
/// ```
pub fn build_x0_histogram<S>(spans: &[S], page_width: f32) -> Vec<u32>
where
    S: HasBBox,
{
    let hist_len = page_width.ceil() as usize;
    let mut hist = vec![0u32; hist_len];

    for span in spans {
        let x0 = span.bbox()[0];
        let idx = x0.round() as usize;

        // Clamp and emit diagnostics for out-of-bounds x0
        if idx >= hist_len {
            if x0 < 0.0 {
                warn!("build_x0_histogram: x0={} < 0, clamping to bucket 0", x0);
                hist[0] += 1;
            } else {
                // x0 >= page_width
                warn!(
                    "build_x0_histogram: x0={} >= page_width={}, clamping to bucket {}",
                    x0,
                    page_width,
                    hist_len.saturating_sub(1)
                );
                if !hist.is_empty() {
                    hist[hist_len - 1] += 1;
                }
            }
        } else {
            hist[idx] += 1;
        }
    }

    hist
}

/// A gap in the x0 histogram representing a candidate column boundary.
///
/// The gap spans from bucket `lo` to `hi` (inclusive), where all buckets
/// have zero coverage (no spans start at these x positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnGap {
    /// Start index of the gap (inclusive).
    pub lo: usize,
    /// End index of the gap (inclusive).
    pub hi: usize,
}

impl ColumnGap {
    /// Create a new column gap.
    #[inline]
    pub fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }

    /// Return the width of this gap in buckets.
    #[inline]
    pub fn width(&self) -> usize {
        self.hi - self.lo + 1
    }

    /// Return the midpoint of this gap in points.
    ///
    /// This is useful for setting column boundaries at the center of gaps.
    #[inline]
    pub fn midpoint(&self) -> f32 {
        (self.lo + self.hi) as f32 / 2.0
    }
}

/// Detect column gaps in the x0 histogram.
///
/// Finds all contiguous spans of zero-coverage buckets longer than
/// `0.03 * page_width`. Each such gap is a candidate column boundary.
///
/// # Arguments
///
/// * `hist` - The x0 histogram from `build_x0_histogram`
/// * `page_width` - Page width in points (used for threshold calculation)
///
/// # Returns
///
/// A `Vec<ColumnGap>` listing boundary indices. Each gap spans from
/// `lo` to `hi` (inclusive), where all buckets have zero coverage.
///
/// # Behavior
///
/// - Threshold = `(page_width * 0.03).ceil() as usize` (~18pt on 612pt page)
/// - Leading zeros (page left margin) are included if >= threshold
/// - Trailing zeros (page right margin) are included if >= threshold
/// - All-zero histogram (empty page) returns no gaps
/// - Adjacent gaps are merged (no two gaps can be adjacent)
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::columns::detect_column_gaps;
///
/// // Histogram with 20 zeros in the middle, page_width=600
/// let mut hist = vec![1u32; 100]; // [0..100] = 1
/// hist.extend(vec![0u32; 20]);    // [100..120] = 0 (gap)
/// hist.extend(vec![1u32; 100]);   // [120..220] = 1
///
/// let gaps = detect_column_gaps(&hist, 600.0);
/// assert_eq!(gaps.len(), 1);
/// assert_eq!(gaps[0].lo, 100);
/// assert_eq!(gaps[0].hi, 119);
/// ```
pub fn detect_column_gaps(hist: &[u32], page_width: f32) -> Vec<ColumnGap> {
    let threshold = (page_width * 0.03_f32).ceil() as usize;

    if hist.is_empty() {
        return Vec::new();
    }

    // Edge case: all zeros (empty page) - no gaps
    // We need at least some non-zero buckets to have meaningful column detection
    if hist.iter().all(|&count| count == 0) {
        return Vec::new();
    }

    let mut gaps = Vec::new();
    let mut run_start: Option<usize> = None;

    for (i, &count) in hist.iter().enumerate() {
        if count == 0 {
            // Start a new run if we're not in one
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else {
            // End of run - check if it meets threshold
            if let Some(start) = run_start {
                let end = i.saturating_sub(1);
                let run_length = end - start + 1;
                if run_length >= threshold {
                    gaps.push(ColumnGap::new(start, end));
                }
                run_start = None;
            }
        }
    }

    // Handle trailing zeros (page right margin)
    if let Some(start) = run_start {
        let end = hist.len().saturating_sub(1);
        let run_length = end - start + 1;
        if run_length >= threshold {
            gaps.push(ColumnGap::new(start, end));
        }
    }

    gaps
}

/// A candidate column region for confirmation.
///
/// Represents a potential column with its x_range and line count.
/// Used during the column confirmation phase.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CandidateColumn {
    /// X range [x0, x1] defining the candidate column bounds.
    x_range: [f32; 2],
    /// Number of unique lines whose first span starts in this column.
    line_count: usize,
}

/// Confirm column boundaries by counting lines per candidate column.
///
/// Partitions the page into candidate columns (regions between consecutive
/// gaps + before-first + after-last). For each candidate, counts unique lines
/// whose first span's x0 falls within the column's x-range. Promotes columns
/// with line_count >= 3 to confirmed columns.
///
/// # Arguments
///
/// * `gaps` - Candidate column gaps from `detect_column_gaps`
/// * `page_width` - Page width in points
/// * `lines` - Lines to count (must have spans sorted left-to-right)
///
/// # Returns
///
/// A `Vec<Column>` of confirmed columns with x_ranges and indices.
/// Columns are returned left-to-right with monotonic indices.
///
/// # Behavior
///
/// - No gaps detected: entire page is one column (if >= 3 lines)
/// - Gaps detected: candidate columns are (0, gap_0.lo), (gap_i.hi, gap_i+1.lo), (gap_last.hi, page_width)
/// - Lines whose first span is in a GAP region remain unassigned (column = None)
/// - "First span" = leftmost post-sort (within-line sorting already done)
/// - INV: 3-line minimum from plan
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::columns::confirm_columns;
///
/// // Two gaps detected on a 612pt page
/// let gaps = vec![ColumnGap::new(200, 215), ColumnGap::new(400, 415)];
///
/// // Candidate columns: [0, 200), [216, 400), [416, 612)
/// // Only columns with >= 3 lines are confirmed
/// ```
pub fn confirm_columns<L>(gaps: &[ColumnGap], page_width: f32, lines: &[L]) -> Vec<Column>
where
    L: HasFirstSpan,
{
    // Handle no gaps: entire page is one candidate column
    if gaps.is_empty() {
        // Count lines whose first span is within page bounds
        let line_count = lines
            .iter()
            .filter(|line| {
                line
                    .first_span_bbox()
                    .map_or(false, |bbox| bbox[0] >= 0.0 && bbox[0] < page_width)
            })
            .count();

        // Confirm single column if >= 3 lines
        if line_count >= 3 {
            return vec![Column::new(0, [0.0, page_width])];
        } else {
            return Vec::new();
        }
    }

    // Build candidate columns from gaps
    let mut candidates = Vec::new();

    // Before-first gap: (0, gap_0.lo)
    if let Some(first_gap) = gaps.first() {
        if first_gap.lo > 0 {
            candidates.push(CandidateColumn {
                x_range: [0.0, first_gap.lo as f32],
                line_count: 0,
            });
        }
    }

    // Between consecutive gaps: (gap_i.hi, gap_i+1.lo)
    for window in gaps.windows(2) {
        let prev_gap = &window[0];
        let next_gap = &window[1];
        candidates.push(CandidateColumn {
            x_range: [(prev_gap.hi + 1) as f32, next_gap.lo as f32],
            line_count: 0,
        });
    }

    // After-last gap: (gap_last.hi, page_width)
    if let Some(last_gap) = gaps.last() {
        let x0 = (last_gap.hi + 1) as f32;
        if x0 < page_width {
            candidates.push(CandidateColumn {
                x_range: [x0, page_width],
                line_count: 0,
            });
        }
    }

    // Count lines whose first span's x0 falls in each candidate column
    for line in lines {
        if let Some(bbox) = line.first_span_bbox() {
            let x0 = bbox[0];
            for candidate in &mut candidates {
                if x0 >= candidate.x_range[0] && x0 < candidate.x_range[1] {
                    candidate.line_count += 1;
                    break; // Each line counted once
                }
            }
        }
    }

    // Promote candidates with >= 3 lines to confirmed columns
    let confirmed: Vec<Column> = candidates
        .into_iter()
        .filter(|c| c.line_count >= 3)
        .enumerate()
        .map(|(i, c)| Column::new(i as u32, c.x_range))
        .collect();

    confirmed
}

/// Trait for types that can provide the first span's bounding box.
///
/// This trait allows the column confirmation code to work with different
/// line representations while accessing the leftmost span's position.
pub trait HasFirstSpan {
    /// Get the bounding box [x0, y0, x1, y1] of the first (leftmost) span.
    ///
    /// Returns None if the line has no spans.
    fn first_span_bbox(&self) -> Option<[f32; 4]>;
}

/// Trait for types with a bounding box for histogram building.
///
/// This is a simplified version of the trait used in column assignment,
/// returning `[f32; 4]` for compatibility with the histogram function.
pub trait HasBBox {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f32; 4];
}

// Implement HasBBox for common types
impl HasBBox for [f32; 4] {
    fn bbox(&self) -> [f32; 4] {
        *self
    }
}

impl HasBBox for [f64; 4] {
    fn bbox(&self) -> [f32; 4] {
        [
            self[0] as f32,
            self[1] as f32,
            self[2] as f32,
            self[3] as f32,
        ]
    }
}

/// A confirmed column with its x_range and index.
///
/// The x_range is \[x0, x1\] in PDF user space coordinates.
/// Spans whose bbox\[0\] falls within this range are assigned to this column.
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
/// `span.bbox\[0\]`. Spans outside any column get column = None.
///
/// # Arguments
///
/// * `spans` - Spans to assign columns to (must have bbox and column fields)
/// * `columns` - Confirmed columns with x_ranges
///
/// # Behavior
///
/// - Spans are assigned by their x0 coordinate (`bbox\[0\]`)
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

    #[test]
    fn test_build_x0_histogram_single_span() {
        // 1 span at x0=100, page_width=612: hist[100] == 1
        let spans: Vec<[f32; 4]> = vec![[100.0, 0.0, 200.0, 10.0]];
        let hist = build_x0_histogram(&spans, 612.0);

        assert_eq!(hist.len(), 612);
        assert_eq!(hist[100], 1);
        // All other buckets should be 0
        assert_eq!(hist[0], 0);
        assert_eq!(hist[99], 0);
        assert_eq!(hist[101], 0);
    }

    #[test]
    fn test_build_x0_histogram_multiple_spans() {
        // 5 spans at x0=100,100,200,200,300: hist[100]==2, hist[200]==2, hist[300]==1
        let spans: Vec<[f32; 4]> = vec![
            [100.0, 0.0, 200.0, 10.0],
            [100.0, 0.0, 200.0, 10.0],
            [200.0, 0.0, 300.0, 10.0],
            [200.0, 0.0, 300.0, 10.0],
            [300.0, 0.0, 400.0, 10.0],
        ];
        let hist = build_x0_histogram(&spans, 612.0);

        assert_eq!(hist[100], 2);
        assert_eq!(hist[200], 2);
        assert_eq!(hist[300], 1);
        // Other buckets should be 0
        assert_eq!(hist[0], 0);
        assert_eq!(hist[99], 0);
        assert_eq!(hist[101], 0);
        assert_eq!(hist[299], 0);
    }

    #[test]
    fn test_build_x0_histogram_clamp_negative_x0() {
        // Span at x0=-5: clamped to hist[0], diagnostic
        let spans: Vec<[f32; 4]> = vec![[-5.0, 0.0, 100.0, 10.0]];
        let hist = build_x0_histogram(&spans, 612.0);

        // Should be clamped to bucket 0
        assert_eq!(hist[0], 1);
        assert_eq!(hist[1], 0);
    }

    #[test]
    fn test_build_x0_histogram_clamp_overflow_x0() {
        // Span at x0 > page_width: clamped to last bucket, diagnostic
        let spans: Vec<[f32; 4]> = vec![[700.0, 0.0, 800.0, 10.0]];
        let hist = build_x0_histogram(&spans, 612.0);

        // Should be clamped to last bucket (611)
        assert_eq!(hist[611], 1);
    }

    #[test]
    fn test_build_x0_histogram_empty_spans() {
        // Empty spans: returns Vec of zeros
        let spans: Vec<[f32; 4]> = vec![];
        let hist = build_x0_histogram(&spans, 612.0);

        assert_eq!(hist.len(), 612);
        // All buckets should be 0
        for &count in &hist {
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn test_build_x0_histogram_rounding() {
        // Test that x0 is rounded to nearest integer
        let spans: Vec<[f32; 4]> = vec![
            [100.4, 0.0, 200.0, 10.0], // rounds to 100
            [100.6, 0.0, 200.0, 10.0], // rounds to 101
            [99.5, 0.0, 200.0, 10.0],  // rounds to 100 (round half to even in Rust)
            [99.6, 0.0, 200.0, 10.0],  // rounds to 100
        ];
        let hist = build_x0_histogram(&spans, 612.0);

        // 100.4 -> 100, 100.6 -> 101, 99.5 -> 100, 99.6 -> 100
        assert_eq!(hist[100], 3);
        assert_eq!(hist[101], 1);
    }

    #[test]
    fn test_build_x0_histogram_a4_page() {
        // Test with A4 page width (595pt)
        let spans: Vec<[f32; 4]> = vec![[100.0, 0.0, 200.0, 10.0]];
        let hist = build_x0_histogram(&spans, 595.0);

        assert_eq!(hist.len(), 595);
        assert_eq!(hist[100], 1);
    }

    // ColumnGap tests

    #[test]
    fn test_column_gap_new() {
        let gap = ColumnGap::new(10, 20);
        assert_eq!(gap.lo, 10);
        assert_eq!(gap.hi, 20);
    }

    #[test]
    fn test_column_gap_width() {
        let gap = ColumnGap::new(10, 20);
        assert_eq!(gap.width(), 11); // 20 - 10 + 1
    }

    #[test]
    fn test_column_gap_midpoint() {
        let gap = ColumnGap::new(10, 20);
        assert_eq!(gap.midpoint(), 15.0); // (10 + 20) / 2
    }

    #[test]
    fn test_column_gap_single_bucket() {
        let gap = ColumnGap::new(10, 10);
        assert_eq!(gap.width(), 1);
        assert_eq!(gap.midpoint(), 10.0);
    }

    // detect_column_gaps tests

    #[test]
    fn test_detect_column_gaps_short_zeros_no_gap() {
        // Histogram with 8 contiguous zeros, page_width=600 (threshold=18): NO gap (8 < 18)
        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; 8]);
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_detect_column_gaps_middle_gap() {
        // Histogram with 20 zeros middle, page_width=600: 1 gap
        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; 20]);
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].lo, 50);
        assert_eq!(gaps[0].hi, 69);
        assert_eq!(gaps[0].width(), 20);
    }

    #[test]
    fn test_detect_column_gaps_leading_gap() {
        // Leading zeros >= threshold: 1 leading gap
        let mut hist = vec![0u32; 25];
        hist.extend(vec![1u32; 100]);

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].lo, 0);
        assert_eq!(gaps[0].hi, 24);
    }

    #[test]
    fn test_detect_column_gaps_trailing_gap() {
        // Trailing zeros >= threshold: 1 trailing gap
        let mut hist = vec![1u32; 100];
        hist.extend(vec![0u32; 25]);

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].lo, 100);
        assert_eq!(gaps[0].hi, 124);
    }

    #[test]
    fn test_detect_column_gaps_all_zeros_no_gaps() {
        // All-zero histogram: 0 gaps (empty page)
        let hist = vec![0u32; 600];

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_detect_column_gaps_multiple_gaps() {
        // Multiple gaps separated by non-zero regions
        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; 20]);  // gap 1
        hist.extend(vec![1u32; 30]);
        hist.extend(vec![0u32; 25]);  // gap 2
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].lo, 50);
        assert_eq!(gaps[0].hi, 69);
        assert_eq!(gaps[1].lo, 100);
        assert_eq!(gaps[1].hi, 124);
    }

    #[test]
    fn test_detect_column_gaps_threshold_exact() {
        // Gap exactly at threshold should be included
        let page_width = 600.0_f32;
        let threshold = (page_width * 0.03_f32).ceil() as usize; // 18

        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; threshold]); // exactly threshold
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, page_width);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].width(), threshold);
    }

    #[test]
    fn test_detect_column_gaps_threshold_minus_one() {
        // Gap at threshold-1 should NOT be included
        let page_width = 600.0_f32;
        let threshold = (page_width * 0.03_f32).ceil() as usize; // 18

        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; threshold - 1]); // just below threshold
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, page_width);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_detect_column_gaps_empty_histogram() {
        let hist: Vec<u32> = vec![];
        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_detect_column_gaps_no_zeros() {
        // Histogram with no zeros: no gaps
        let hist = vec![1u32; 600];
        let gaps = detect_column_gaps(&hist, 600.0_f32);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_detect_column_gaps_small_page() {
        // Test with smaller page (threshold scales proportionally)
        let page_width = 300.0_f32;
        let threshold = (page_width * 0.03_f32).ceil() as usize; // 9

        let mut hist = vec![1u32; 50];
        hist.extend(vec![0u32; 12]); // > 9, should be a gap
        hist.extend(vec![1u32; 50]);

        let gaps = detect_column_gaps(&hist, page_width);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].width(), 12);
    }

    #[test]
    fn test_detect_column_gaps_leading_and_trailing() {
        // Both leading and trailing gaps
        let mut hist = vec![0u32; 20];  // leading
        hist.extend(vec![1u32; 100]);
        hist.extend(vec![0u32; 20]);  // trailing

        let page_width = 600.0_f32;
        let threshold = (page_width * 0.03_f32).ceil() as usize; // 18

        // Only the trailing gap should be detected (20 >= 18)
        // Leading is only 20 which is >= 18, so it should also be detected
        let gaps = detect_column_gaps(&hist, page_width);
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].lo, 0);
        assert_eq!(gaps[0].hi, 19);
        assert_eq!(gaps[1].lo, 120);
        assert_eq!(gaps[1].hi, 139);
    }

    // confirm_columns tests

    /// Test line with first span bbox.
    #[derive(Debug, Clone)]
    struct TestLineWithSpans {
        first_span_bbox: Option<[f32; 4]>,
    }

    impl TestLineWithSpans {
        fn new(bbox: Option<[f32; 4]>) -> Self {
            Self { first_span_bbox: bbox }
        }
    }

    impl HasFirstSpan for TestLineWithSpans {
        fn first_span_bbox(&self) -> Option<[f32; 4]> {
            self.first_span_bbox
        }
    }

    #[test]
    fn test_confirm_columns_two_column_both_confirmed() {
        // 2-column page with 30 lines each: both confirmed
        let gaps = vec![ColumnGap::new(300, 319)]; // gap at 300-319

        let mut lines = Vec::new();
        // Column 0: lines at x0=50 (30 lines)
        for _ in 0..30 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }
        // Column 1: lines at x0=350 (30 lines)
        for _ in 0..30 {
            lines.push(TestLineWithSpans::new(Some([350.0, 0.0, 500.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 2);
        assert_eq!(confirmed[0].index, 0);
        assert_eq!(confirmed[0].x_range, [0.0, 300.0]);
        assert_eq!(confirmed[1].index, 1);
        assert_eq!(confirmed[1].x_range, [320.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_two_column_one_confirmed() {
        // 2-column page with 30 lines + 2 lines: only 30-line column confirmed
        let gaps = vec![ColumnGap::new(300, 319)];

        let mut lines = Vec::new();
        // Column 0: lines at x0=50 (30 lines)
        for _ in 0..30 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }
        // Column 1: lines at x0=350 (2 lines)
        for _ in 0..2 {
            lines.push(TestLineWithSpans::new(Some([350.0, 0.0, 500.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].index, 0);
        assert_eq!(confirmed[0].x_range, [0.0, 300.0]);
    }

    #[test]
    fn test_confirm_columns_single_column_confirmed() {
        // Single column with 5 lines -> confirmed
        let gaps = vec![];

        let mut lines = Vec::new();
        for _ in 0..5 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].index, 0);
        assert_eq!(confirmed[0].x_range, [0.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_single_column_insufficient_lines() {
        // Single column with only 2 lines -> not confirmed
        let gaps = vec![];

        let mut lines = Vec::new();
        for _ in 0..2 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 0);
    }

    #[test]
    fn test_confirm_columns_empty_page() {
        // Empty page: 0 confirmed
        let gaps = vec![];
        let lines: Vec<TestLineWithSpans> = vec![];

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 0);
    }

    #[test]
    fn test_confirm_columns_no_gaps_insufficient_lines() {
        // No gaps but only 2 lines: 0 confirmed (below 3-line threshold)
        let gaps = vec![];

        let mut lines = Vec::new();
        for _ in 0..2 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 0);
    }

    #[test]
    fn test_confirm_columns_exactly_three_lines() {
        // Exactly 3 lines: confirmed (>= threshold)
        let gaps = vec![];

        let mut lines = Vec::new();
        for _ in 0..3 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 1);
    }

    #[test]
    fn test_confirm_columns_three_column_all_confirmed() {
        // Three-column page with 10 lines each: all confirmed
        let gaps = vec![ColumnGap::new(200, 219), ColumnGap::new(400, 419)];

        let mut lines = Vec::new();
        // Column 0: lines at x0=50 (10 lines)
        for _ in 0..10 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 150.0, 10.0])));
        }
        // Column 1: lines at x0=250 (10 lines)
        for _ in 0..10 {
            lines.push(TestLineWithSpans::new(Some([250.0, 0.0, 350.0, 10.0])));
        }
        // Column 2: lines at x0=450 (10 lines)
        for _ in 0..10 {
            lines.push(TestLineWithSpans::new(Some([450.0, 0.0, 550.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 3);
        assert_eq!(confirmed[0].index, 0);
        assert_eq!(confirmed[0].x_range, [0.0, 200.0]);
        assert_eq!(confirmed[1].index, 1);
        assert_eq!(confirmed[1].x_range, [220.0, 400.0]);
        assert_eq!(confirmed[2].index, 2);
        assert_eq!(confirmed[2].x_range, [420.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_three_column_middle_insufficient() {
        // Three-column with middle column having only 2 lines: only outer confirmed
        let gaps = vec![ColumnGap::new(200, 219), ColumnGap::new(400, 419)];

        let mut lines = Vec::new();
        // Column 0: lines at x0=50 (10 lines)
        for _ in 0..10 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 150.0, 10.0])));
        }
        // Column 1: lines at x0=250 (2 lines)
        for _ in 0..2 {
            lines.push(TestLineWithSpans::new(Some([250.0, 0.0, 350.0, 10.0])));
        }
        // Column 2: lines at x0=450 (10 lines)
        for _ in 0..10 {
            lines.push(TestLineWithSpans::new(Some([450.0, 0.0, 550.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 2);
        assert_eq!(confirmed[0].index, 0);
        assert_eq!(confirmed[0].x_range, [0.0, 200.0]);
        assert_eq!(confirmed[1].index, 1); // Note: index is reassigned after filtering
        assert_eq!(confirmed[1].x_range, [420.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_lines_in_gap_unassigned() {
        // Lines whose first span is in a gap region are not counted
        let gaps = vec![ColumnGap::new(200, 219)];

        let mut lines = Vec::new();
        // Column 0: lines at x0=50 (5 lines)
        for _ in 0..5 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 150.0, 10.0])));
        }
        // Gap region: lines at x0=210 (3 lines) - should NOT be counted
        for _ in 0..3 {
            lines.push(TestLineWithSpans::new(Some([210.0, 0.0, 250.0, 10.0])));
        }
        // Column 1: lines at x0=250 (5 lines)
        for _ in 0..5 {
            lines.push(TestLineWithSpans::new(Some([250.0, 0.0, 350.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 2);
        // Both columns have 5 lines each (gap lines not counted)
        assert_eq!(confirmed[0].x_range, [0.0, 200.0]);
        assert_eq!(confirmed[1].x_range, [220.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_lines_with_no_spans() {
        // Lines with no spans (first_span_bbox = None) are not counted
        let gaps = vec![];

        let mut lines = Vec::new();
        // 3 valid lines
        for _ in 0..3 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 150.0, 10.0])));
        }
        // 2 lines with no spans
        for _ in 0..2 {
            lines.push(TestLineWithSpans::new(None));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        // Only 3 valid lines -> confirmed
        assert_eq!(confirmed.len(), 1);
    }

    #[test]
    fn test_confirm_columns_leading_gap() {
        // Leading gap (page margin) creates first column after gap
        let gaps = vec![ColumnGap::new(0, 49)]; // leading margin

        let mut lines = Vec::new();
        // Lines in column starting at x0=50 (5 lines)
        for _ in 0..5 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].x_range, [50.0, 600.0]);
    }

    #[test]
    fn test_confirm_columns_trailing_gap() {
        // Trailing gap (page margin) creates last column before gap
        let gaps = vec![ColumnGap::new(550, 599)]; // trailing margin

        let mut lines = Vec::new();
        // Lines in column ending before x0=550 (5 lines)
        for _ in 0..5 {
            lines.push(TestLineWithSpans::new(Some([50.0, 0.0, 200.0, 10.0])));
        }

        let confirmed = confirm_columns(&gaps, 600.0, &lines);

        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].x_range, [0.0, 550.0]);
    }
}

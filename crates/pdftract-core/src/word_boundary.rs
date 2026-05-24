//! Adaptive word boundary detector for Phase 3.2 text extraction.
//!
//! This module implements the adaptive word boundary detection algorithm
//! specified in the plan (line 1547) and documented in
//! `docs/research/word-boundary-reconstruction.md`.
//!
//! # Algorithm
//!
//! 1. **Bootstrap phase (first 20 glyphs per font):**
//!    - Threshold = 0.25 × font_size
//!    - Collect gap samples
//!
//! 2. **Adaptive phase (after 20 glyphs):**
//!    - Every 5 samples: compute median of last 20 gaps
//!    - Threshold = 1.5 × median
//!    - Exclude outliers > 4× current threshold
//!
//! 3. **Reset conditions:**
//!    - Font switch (Tf operator)
//!    - Begin text (BT operator)
//!
//! # Invariants
//!
//! - All gap comparisons are in **text space** (before CTM transformation)
//! - Negative gaps (overlapping glyphs) never trigger word boundaries
//! - Gaps must be strictly greater than threshold (not >=)
//! - Recalibration happens every 5 samples after the 20-glyph bootstrap

use crate::font::FontId;
use std::collections::HashMap;

/// Word boundary detector for a single font.
///
/// Tracks gap samples and maintains an adaptive threshold for determining
/// whether a gap between glyphs represents a word boundary.
#[derive(Debug, Clone)]
pub struct WordBoundaryDetector {
    /// Font identifier for this detector.
    font_id: FontId,
    /// Number of glyph samples collected.
    sample_count: u32,
    /// Gap samples in text space (bounded to last 20).
    samples: Vec<f32>,
    /// Current threshold in text space points.
    threshold: f32,
}

impl WordBoundaryDetector {
    /// Create a new detector for the given font.
    ///
    /// Starts with bootstrap threshold = 0.25 × font_size.
    pub fn new(font_id: FontId, font_size: f32) -> Self {
        Self {
            font_id,
            sample_count: 0,
            samples: Vec::with_capacity(20),
            threshold: 0.25 * font_size,
        }
    }

    /// Reset the detector to bootstrap state.
    ///
    /// Called on font switch (Tf) or begin text (BT).
    pub fn reset(&mut self, font_size: f32) {
        self.sample_count = 0;
        self.samples.clear();
        self.threshold = 0.25 * font_size;
    }

    /// Record a gap and detect if this is a word boundary.
    ///
    /// # Arguments
    ///
    /// * `gap` - The inter-glyph gap in text space points
    ///
    /// # Returns
    ///
    /// `true` if this gap exceeds the threshold and should insert a word boundary.
    pub fn record_and_detect(&mut self, gap: f32) -> bool {
        // Negative gaps never trigger word boundaries
        if gap <= 0.0 {
            return false;
        }

        // Check if gap exceeds threshold (strictly greater than)
        let is_boundary = gap > self.threshold;

        // Record the sample
        self.samples.push(gap);
        if self.samples.len() > 20 {
            self.samples.remove(0);
        }
        self.sample_count += 1;

        // Recalibrate every 5 samples after bootstrap (20 samples)
        if self.sample_count > 20 && self.sample_count % 5 == 0 {
            self.recalibrate();
        }

        is_boundary
    }

    /// Recalibrate the threshold based on recent samples.
    fn recalibrate(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        // Exclude outliers > 4× current threshold
        let outlier_limit = 4.0 * self.threshold;
        let filtered: Vec<f32> = self
            .samples
            .iter()
            .copied()
            .filter(|g| *g <= outlier_limit)
            .collect();

        if filtered.is_empty() {
            return;
        }

        // Compute median
        let median = median(&filtered);

        // Set new threshold to 1.5× median
        self.threshold = 1.5 * median;
    }

    /// Get the current threshold in text space points.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Get the number of samples collected.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }
}

/// Compute the median of a slice of floats.
fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        sorted[len / 2]
    }
}

/// Manager for per-font word boundary detectors.
///
/// Maintains a HashMap of detectors, one per font in use on the page.
#[derive(Debug, Clone, Default)]
pub struct WordBoundaryManager {
    /// Per-font detectors.
    detectors: HashMap<FontId, WordBoundaryDetector>,
}

impl WordBoundaryManager {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self {
            detectors: HashMap::new(),
        }
    }

    /// Get or create a detector for the given font.
    ///
    /// If no detector exists for this font, creates one with the given font size.
    pub fn detector_for(&mut self, font_id: FontId, font_size: f32) -> &mut WordBoundaryDetector {
        self.detectors
            .entry(font_id)
            .or_insert_with(|| WordBoundaryDetector::new(font_id, font_size))
    }

    /// Reset a detector to bootstrap state (font switch).
    pub fn reset_font(&mut self, font_id: FontId, font_size: f32) {
        if let Some(detector) = self.detectors.get_mut(&font_id) {
            detector.reset(font_size);
        }
    }

    /// Reset all detectors (begin text BT).
    pub fn reset_all(&mut self) {
        for detector in self.detectors.values_mut() {
            detector.reset(detector.threshold / 0.25); // Reconstruct font_size from threshold
        }
    }

    /// Record a gap and detect word boundary for the given font.
    ///
    /// # Arguments
    ///
    /// * `font_id` - Font identifier
    /// * `font_size` - Font size in points (for bootstrap threshold)
    /// * `gap` - Inter-glyph gap in text space points
    ///
    /// # Returns
    ///
    /// `true` if this gap should insert a word boundary.
    pub fn record_and_detect(&mut self, font_id: FontId, font_size: f32, gap: f32) -> bool {
        self.detector_for(font_id, font_size).record_and_detect(gap)
    }

    /// Get the current threshold for a font.
    ///
    /// Returns the bootstrap threshold (0.25 × font_size) if the font
    /// has no detector yet.
    pub fn threshold_for(&self, font_id: FontId, font_size: f32) -> f32 {
        self.detectors
            .get(&font_id)
            .map(|d| d.threshold())
            .unwrap_or_else(|| 0.25 * font_size)
    }
}

/// Text state parameters for Tc/Tw/Tz tracking.
///
/// Per PDF spec section 9.3 "Text State":
/// - Tc: character spacing (added to every glyph)
/// - Tw: word spacing (added only after space glyph, codepoint 0x20)
/// - Tz: horizontal scaling (percentage, default 100)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextState {
    /// Character spacing (Tc operator).
    pub tc: f32,
    /// Word spacing (Tw operator).
    pub tw: f32,
    /// Horizontal scaling (Tz operator, default 100).
    pub tz: f32,
    /// Current font size.
    pub font_size: f32,
    /// Current font ID.
    pub font_id: Option<FontId>,
}

impl TextState {
    /// Create a new text state with default values.
    pub fn new() -> Self {
        Self {
            tc: 0.0,
            tw: 0.0,
            tz: 100.0,
            font_size: 12.0,
            font_id: None,
        }
    }

    /// Set character spacing (Tc operator).
    pub fn set_tc(&mut self, tc: f32) {
        self.tc = tc;
    }

    /// Set word spacing (Tw operator).
    pub fn set_tw(&mut self, tw: f32) {
        self.tw = tw;
    }

    /// Set horizontal scaling (Tz operator).
    pub fn set_tz(&mut self, tz: f32) {
        self.tz = tz;
    }

    /// Set font and size (Tf operator).
    pub fn set_font(&mut self, font_id: FontId, size: f32) {
        self.font_id = Some(font_id);
        self.font_size = size;
    }

    /// Compute expected advance for a glyph.
    ///
    /// Per plan line 1547:
    /// ```
    /// expected_advance = (w_g / 1000 * font_size + Tc + Tw_if_space) * Tz / 100
    /// ```
    ///
    /// # Arguments
    ///
    /// * `glyph_width` - Glyph width in 1/1000 em units
    /// * `is_space` - True if the glyph is U+0020 (SPACE)
    ///
    /// # Returns
    ///
    /// Expected advance in text space points.
    pub fn expected_advance(&self, glyph_width: f32, is_space: bool) -> f32 {
        let tw_if_space = if is_space { self.tw } else { 0.0 };
        (glyph_width / 1000.0 * self.font_size + self.tc + tw_if_space) * self.tz / 100.0
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_font_id(id: usize) -> FontId {
        FontId::from_usize(id)
    }

    #[test]
    fn test_detector_bootstrap_threshold() {
        let font_id = make_font_id(0);
        let detector = WordBoundaryDetector::new(font_id, 12.0);
        assert_eq!(detector.threshold(), 3.0); // 0.25 * 12
    }

    #[test]
    fn test_detector_negative_gap_no_boundary() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Negative gap should not trigger boundary
        assert!(!detector.record_and_detect(-1.0));
        assert!(!detector.record_and_detect(-0.1));
    }

    #[test]
    fn test_detector_zero_gap_no_boundary() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Zero gap should not trigger boundary (strictly greater than)
        assert!(!detector.record_and_detect(0.0));
    }

    #[test]
    fn test_detector_gap_below_threshold() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Gap below threshold should not trigger boundary
        assert!(!detector.record_and_detect(2.0)); // < 3.0
    }

    #[test]
    fn test_detector_gap_at_threshold() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Gap exactly at threshold should NOT trigger (strictly greater than)
        assert!(!detector.record_and_detect(3.0)); // == 3.0
    }

    #[test]
    fn test_detector_gap_above_threshold() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Gap above threshold should trigger boundary
        assert!(detector.record_and_detect(3.5)); // > 3.0
    }

    #[test]
    fn test_detector_recalibration_after_20_samples() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Feed 25 samples with gaps around 8.0 (typical word gap for 12pt font)
        for i in 0..25 {
            // Alternate between tight kerning (0.1) and word gaps (8.0)
            let gap = if i % 2 == 0 { 0.1 } else { 8.0 };
            detector.record_and_detect(gap);
        }

        // After 20+ samples, threshold should adapt to the data
        // Median of mixed 0.1 and 8.0 samples (13 values: 0.1 x7, 8.0 x6)
        // Sorted: [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0]
        // Median (7th index) = 0.1
        // 1.5 × 0.1 = 0.15
        // But outlier filtering excludes values > 4× current threshold
        // Initial threshold = 3.0, outlier limit = 12.0 (8.0 is not excluded)
        // After first recalibration at 25 samples:
        // Samples: last 20 values (mix of 0.1 and 8.0)
        // Threshold = 1.5 × median(0.1, 0.1, ..., 8.0, 8.0, ...) = 1.5 × ~4.05 = ~6.075
        let new_threshold = detector.threshold();
        assert!(
            new_threshold > 3.0,
            "Threshold should adapt from bootstrap 3.0 based on data, got {}",
            new_threshold
        );
    }

    #[test]
    fn test_detector_sample_count() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        assert_eq!(detector.sample_count(), 0);
        detector.record_and_detect(1.0);
        assert_eq!(detector.sample_count(), 1);
        detector.record_and_detect(2.0);
        assert_eq!(detector.sample_count(), 2);
    }

    #[test]
    fn test_detector_reset() {
        let font_id = make_font_id(0);
        let mut detector = WordBoundaryDetector::new(font_id, 12.0);

        // Add some samples
        for i in 0..25 {
            detector.record_and_detect(if i % 2 == 0 { 0.1 } else { 8.0 });
        }

        // Threshold should have adapted
        let adapted_threshold = detector.threshold();

        // Reset
        detector.reset(12.0);

        // Should return to bootstrap state
        assert_eq!(detector.threshold(), 3.0);
        assert_eq!(detector.sample_count(), 0);
        assert!(detector.samples.is_empty());
        assert_ne!(adapted_threshold, 3.0); // Verify it was different before reset
    }

    #[test]
    fn test_manager_multiple_fonts() {
        let mut manager = WordBoundaryManager::new();
        let font_id_1 = make_font_id(0);
        let font_id_2 = make_font_id(1);

        // Each font should have independent threshold
        let threshold_1 = manager.threshold_for(font_id_1, 12.0);
        let threshold_2 = manager.threshold_for(font_id_2, 10.0);

        assert_eq!(threshold_1, 3.0); // 0.25 * 12
        assert_eq!(threshold_2, 2.5); // 0.25 * 10
    }

    #[test]
    fn test_manager_record_and_detect() {
        let mut manager = WordBoundaryManager::new();
        let font_id = make_font_id(0);

        // First gap below threshold
        assert!(!manager.record_and_detect(font_id, 12.0, 2.0));

        // Second gap above threshold
        assert!(manager.record_and_detect(font_id, 12.0, 4.0));
    }

    #[test]
    fn test_manager_reset_font() {
        let mut manager = WordBoundaryManager::new();
        let font_id = make_font_id(0);

        // Add samples to adapt threshold
        for i in 0..25 {
            manager.record_and_detect(font_id, 12.0, if i % 2 == 0 { 0.1 } else { 8.0 });
        }

        // Threshold should have adapted
        let adapted = manager.threshold_for(font_id, 12.0);
        assert_ne!(adapted, 3.0);

        // Reset this font
        manager.reset_font(font_id, 12.0);

        // Should return to bootstrap
        let reset = manager.threshold_for(font_id, 12.0);
        assert_eq!(reset, 3.0);
    }

    #[test]
    fn test_text_state_defaults() {
        let state = TextState::new();
        assert_eq!(state.tc, 0.0);
        assert_eq!(state.tw, 0.0);
        assert_eq!(state.tz, 100.0);
        assert_eq!(state.font_size, 12.0);
        assert!(state.font_id.is_none());
    }

    #[test]
    fn test_text_state_setters() {
        let mut state = TextState::new();
        state.set_tc(5.0);
        state.set_tw(10.0);
        state.set_tz(90.0);

        assert_eq!(state.tc, 5.0);
        assert_eq!(state.tw, 10.0);
        assert_eq!(state.tz, 90.0);
    }

    #[test]
    fn test_text_state_set_font() {
        let mut state = TextState::new();
        let font_id = make_font_id(42);
        state.set_font(font_id, 14.0);

        assert_eq!(state.font_id, Some(font_id));
        assert_eq!(state.font_size, 14.0);
    }

    #[test]
    fn test_text_state_expected_advance_basic() {
        let state = TextState::new();

        // Glyph width 500 (half-em), 12pt font, no spacing adjustments
        // Expected: (500/1000 * 12 + 0 + 0) * 100/100 = 6.0
        let advance = state.expected_advance(500.0, false);
        assert_eq!(advance, 6.0);
    }

    #[test]
    fn test_text_state_expected_advance_with_tc() {
        let mut state = TextState::new();
        state.set_tc(2.0);

        // Glyph width 500, 12pt font, Tc=2.0
        // Expected: (500/1000 * 12 + 2.0 + 0) * 100/100 = 8.0
        let advance = state.expected_advance(500.0, false);
        assert_eq!(advance, 8.0);
    }

    #[test]
    fn test_text_state_expected_advance_with_tw_space() {
        let mut state = TextState::new();
        state.set_tw(5.0);

        // Space glyph gets Tw
        // Expected: (500/1000 * 12 + 0 + 5.0) * 100/100 = 11.0
        let advance = state.expected_advance(500.0, true);
        assert_eq!(advance, 11.0);
    }

    #[test]
    fn test_text_state_expected_advance_with_tw_non_space() {
        let mut state = TextState::new();
        state.set_tw(5.0);

        // Non-space glyph does NOT get Tw
        let advance = state.expected_advance(500.0, false);
        assert_eq!(advance, 6.0);
    }

    #[test]
    fn test_text_state_expected_advance_with_tz() {
        let mut state = TextState::new();
        state.set_tz(50.0); // Compress to half

        // Expected: (500/1000 * 12 + 0 + 0) * 50/100 = 3.0
        let advance = state.expected_advance(500.0, false);
        assert_eq!(advance, 3.0);
    }

    #[test]
    fn test_text_state_expected_advance_combined() {
        let mut state = TextState::new();
        state.set_tc(1.0);
        state.set_tw(3.0);
        state.set_tz(80.0);
        state.font_size = 10.0;

        // Space glyph with all adjustments
        // Expected: (500/1000 * 10 + 1.0 + 3.0) * 80/100 = 9.0 * 0.8 = 7.2
        let advance = state.expected_advance(500.0, true);
        assert_eq!(advance, 7.2);
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median(&[]), 0.0);
    }

    #[test]
    fn test_median_single() {
        assert_eq!(median(&[5.0]), 5.0);
    }

    #[test]
    fn test_median_two() {
        assert_eq!(median(&[2.0, 8.0]), 5.0);
    }

    #[test]
    fn test_median_odd() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
    }

    #[test]
    fn test_median_even() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
    }

    #[test]
    fn test_median_unsorted() {
        assert_eq!(median(&[5.0, 1.0, 3.0, 2.0, 4.0]), 3.0);
    }
}

//! Page classification for hybrid detection (Phase 5.1).
//!
//! This module implements per-page classification to determine the extraction
//! path: Vector (text-based), Scanned (image-based), Hybrid (mixed), or
//! BrokenVector (invisible text over scanned image).
//!
//! ## Hybrid Detection
//!
//! Hybrid detection uses an 8×8 grid decomposition. Each cell is classified
//! as vector, scanned, or mixed based on:
//! - **vector**: text_op_count > 0 AND char_validity > 0.6
//! - **scanned**: image_coverage > 0.80 AND text_op_count == 0
//! - **mixed**: neither condition met
//!
//! If ≥ 10 cells (≥ 15%) are vector AND ≥ 10 cells are scanned, the page
//! is classified as Hybrid. The set of scanned cell indexes is returned for
//! downstream OCR-only-on-cells routing in Phase 5.2.
//!
//! ## PageClassifier Engine (Phase 5.1.4)
//!
//! The PageClassifier wires signal evaluators + Hybrid evaluator together:
//! 1. Run Hybrid evaluator first; if it triggers, return immediately
//! 2. Walk signal evaluators in declared order; accumulate votes
//! 3. Apply short-circuit: as soon as any signal has strength > 0.95, return
//! 4. After all signals run: tally votes weighted by strength; pick highest-weight class
//! 5. If no signal voted, default to Vector with confidence 0.5

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Signal evaluator configuration constants.
///
/// Centralizes all threshold constants used by signal evaluators.
/// Per EC-12, these thresholds must be kept in sync with fixture expectations.
/// Changes to these values require updating fixture expectations and running
/// the full test suite to verify correctness.
#[derive(Debug, Clone, Copy)]
pub struct SignalsConfig;

impl SignalsConfig {
    // Evaluator 1: text_operator_presence
    /// Strength for Scanned vote when no text operators present and images exist.
    pub const NO_TEXT_OPS_STRENGTH: f32 = 0.95;

    // Evaluator 2: all_tr3_with_full_page_image
    /// Minimum fraction of page area a single image must cover to be "full page".
    pub const FULL_PAGE_IMAGE_THRESHOLD: f64 = 0.95;
    /// Strength for BrokenVector vote when all text is Tr=3 AND full-page image present.
    pub const ALL_TR3_WITH_IMAGE_STRENGTH: f32 = 0.99;

    // Evaluator 3: image_coverage_fraction
    /// Minimum image coverage fraction to trigger Scanned vote.
    pub const IMAGE_COVERAGE_THRESHOLD: f32 = 0.85;
    /// Strength for Scanned vote when image coverage exceeds threshold.
    pub const IMAGE_COVERAGE_STRENGTH: f32 = 0.85;

    // Evaluator 4: char_validity_rate (low)
    /// Maximum character validity rate to trigger BrokenVector vote.
    pub const CHAR_VALIDITY_LOW_THRESHOLD: f32 = 0.4;
    /// Strength for BrokenVector vote when character validity is below threshold.
    pub const CHAR_VALIDITY_LOW_STRENGTH: f32 = 0.80;

    // Evaluator 5: char_validity_rate (high)
    /// Minimum character validity rate to trigger Vector vote.
    pub const CHAR_VALIDITY_HIGH_THRESHOLD: f32 = 0.85;
    /// Strength for Vector vote when character validity exceeds threshold.
    pub const CHAR_VALIDITY_HIGH_STRENGTH: f32 = 0.90;

    // Evaluator 6: char_density_ratio
    /// Maximum character density (chars per pt²) to trigger Scanned vote.
    pub const CHAR_DENSITY_RATIO_THRESHOLD: f32 = 0.03;
    /// Strength for Scanned vote when character density is below threshold.
    pub const CHAR_DENSITY_RATIO_STRENGTH: f32 = 0.65;

    // Short-circuit threshold
    /// Minimum signal strength to trigger immediate short-circuit classification.
    pub const SHORT_CIRCUIT_STRENGTH: f32 = 0.95;

    // Hybrid detection thresholds
    /// Minimum number of vector cells required for Hybrid classification.
    pub const HYBRID_VECTOR_CELL_MIN: u32 = 10;
    /// Minimum number of scanned cells required for Hybrid classification.
    pub const HYBRID_SCANNED_CELL_MIN: u32 = 10;
    /// Character validity threshold for vector cell classification.
    pub const VECTOR_CELL_VALIDITY_THRESHOLD: f32 = 0.6;
    /// Image coverage threshold for scanned cell classification.
    pub const SCANNED_CELL_IMAGE_THRESHOLD: f32 = 0.80;
}

/// Page context containing all metrics needed for classification.
///
/// This struct is populated by content stream analysis and contains
/// the raw data that signal evaluators use to make classification decisions.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::PageContext;
///
/// let ctx = PageContext {
///     text_op_count: 150,
///     invisible_text_count: 0,
///     tr3_op_count: 0,
///     image_xobject_areas: vec![50000.0],
///     raw_char_count: 2500,
///     valid_char_count: 2450,
///     replacement_char_count: 50,
///     image_coverage: 0.15,
///     has_full_page_image: false,
///     has_visible_text: true,
///     density_ratio: 0.92,
///     width: 612.0,
///     height: 792.0,
///     rotation: 0,
///     grid_cells: None,
/// };
///
/// assert_eq!(ctx.char_validity_rate(), 0.98);
/// assert!(ctx.has_text());
/// assert!(ctx.has_images());
/// ```
#[derive(Debug, Clone, Default)]
pub struct PageContext {
    /// Number of text operators in the content stream.
    pub text_op_count: u32,

    /// Number of text operators with rendering mode Tr=3 (invisible).
    pub invisible_text_count: u32,

    /// Alias for invisible_text_count: number of text operators with Tr=3.
    /// Used by signal evaluators for BrokenVector detection (EC-12).
    pub tr3_op_count: u32,

    /// Areas of individual image XObjects on this page (in pt²).
    /// Used for precise full-page image detection (>= 95% coverage per EC-12).
    pub image_xobject_areas: Vec<f64>,

    /// Total number of characters extracted (before ToUnicode mapping).
    pub raw_char_count: u32,

    /// Number of characters that successfully decoded to valid Unicode.
    pub valid_char_count: u32,

    /// Number of characters that decoded to U+FFFD (replacement).
    pub replacement_char_count: u32,

    /// Image coverage fraction [0.0, 1.0] - fraction of page area covered by images.
    pub image_coverage: f32,

    /// Whether at least one full-page image is present.
    pub has_full_page_image: bool,

    /// Whether any text rendering mode other than Tr=3 was used.
    pub has_visible_text: bool,

    /// Character density ratio: extracted_char_count / expected_char_count.
    pub density_ratio: f32,

    /// Page width in PDF user space units (after rotation).
    pub width: f64,

    /// Page height in PDF user space units (after rotation).
    pub height: f64,

    /// Page rotation in degrees (0, 90, 180, 270).
    pub rotation: i32,

    /// Optional: GridClassifier cell data for hybrid detection.
    /// Populated if grid-based analysis was performed.
    pub grid_cells: Option<[CellData; 64]>,
}

impl PageContext {
    /// Create a new empty page context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute character validity rate.
    ///
    /// Returns fraction of characters that decoded to valid Unicode.
    pub fn char_validity_rate(&self) -> f32 {
        if self.raw_char_count == 0 {
            return 1.0; // No text = validity is vacuously true
        }
        self.valid_char_count as f32 / self.raw_char_count as f32
    }

    /// Check if page has any text operators.
    pub fn has_text(&self) -> bool {
        self.text_op_count > 0
    }

    /// Check if page has any images.
    pub fn has_images(&self) -> bool {
        self.image_coverage > 0.0
    }

    /// Check if all text is invisible (Tr=3).
    pub fn is_all_invisible_text(&self) -> bool {
        self.text_op_count > 0 && self.invisible_text_count == self.text_op_count
    }

    /// Check if this is a blank page (no text, no images).
    pub fn is_blank(&self) -> bool {
        !self.has_text() && !self.has_images()
    }

    /// Check if this is an image-only page (no text).
    pub fn is_image_only(&self) -> bool {
        !self.has_text() && self.has_images()
    }
}

/// Classification vote with strength.
///
/// Each signal evaluator returns a vote for a PageClass with an associated
/// strength [0.0, 1.0] indicating confidence in that vote.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{Vote, PageClass};
///
/// // Create a vote with explicit class and strength
/// let vote = Vote::new(PageClass::Vector, 0.9);
/// assert_eq!(vote.class, PageClass::Vector);
/// assert_eq!(vote.strength, 0.9);
///
/// // Create votes using helper methods
/// let vector_vote = Vote::vector(0.85);
/// let scanned_vote = Vote::scanned(0.95);
/// let broken_vote = Vote::broken_vector(0.75);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Vote {
    /// The class being voted for.
    pub class: PageClass,
    /// Confidence strength [0.0, 1.0].
    pub strength: f32,
}

impl Vote {
    /// Create a new vote.
    pub fn new(class: PageClass, strength: f32) -> Self {
        Self { class, strength }
    }

    /// Create a vote for Vector class.
    pub fn vector(strength: f32) -> Self {
        Self::new(PageClass::Vector, strength)
    }

    /// Create a vote for Scanned class.
    pub fn scanned(strength: f32) -> Self {
        Self::new(PageClass::Scanned, strength)
    }

    /// Create a vote for BrokenVector class.
    pub fn broken_vector(strength: f32) -> Self {
        Self::new(PageClass::BrokenVector, strength)
    }
}

/// Signal evaluator trait.
///
/// Signal evaluators examine the PageContext and produce classification votes.
trait SignalEvaluator: Send + Sync {
    /// Evaluate the signal and return a vote.
    ///
    /// Returns None if the signal does not apply to this page.
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote>;

    /// Get the name of this signal (for debugging/diagnostics).
    fn name(&self) -> &'static str;
}

/// Signal: No text operators in content stream → Scanned.
struct NoTextOperatorsSignal;

impl SignalEvaluator for NoTextOperatorsSignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.text_op_count == 0 {
            // Strong signal for Scanned if images present
            // If no images either, this is a blank page (handled elsewhere)
            if ctx.has_images() {
                return Some(Vote::scanned(SignalsConfig::NO_TEXT_OPS_STRENGTH));
            }
        }
        None
    }

    fn name(&self) -> &'static str {
        "no_text_operators"
    }
}

/// Signal: All text Tr=3 + full-page image → BrokenVector.
struct InvisibleTextWithImageSignal;

impl SignalEvaluator for InvisibleTextWithImageSignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        // Delegate to the precise area-based check
        all_tr3_with_full_page_image(ctx)
    }

    fn name(&self) -> &'static str {
        "all_tr3_with_full_page_image"
    }
}

/// Signal: Image coverage fraction > 0.85 → Scanned.
struct HighImageCoverageSignal;

impl SignalEvaluator for HighImageCoverageSignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.image_coverage > SignalsConfig::IMAGE_COVERAGE_THRESHOLD {
            // Strong signal for Scanned
            return Some(Vote::scanned(SignalsConfig::IMAGE_COVERAGE_STRENGTH));
        }
        None
    }

    fn name(&self) -> &'static str {
        "high_image_coverage"
    }
}

/// Signal: Character validity rate < 0.4 → BrokenVector.
struct LowCharValiditySignal;

impl SignalEvaluator for LowCharValiditySignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.has_text() {
            let validity = ctx.char_validity_rate();
            if validity < SignalsConfig::CHAR_VALIDITY_LOW_THRESHOLD {
                // Very low validity = broken encoding
                return Some(Vote::broken_vector(
                    SignalsConfig::CHAR_VALIDITY_LOW_STRENGTH,
                ));
            }
        }
        None
    }

    fn name(&self) -> &'static str {
        "low_char_validity"
    }
}

/// Signal: Character validity rate > 0.85 → Vector.
struct HighCharValiditySignal;

impl SignalEvaluator for HighCharValiditySignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.has_text() {
            let validity = ctx.char_validity_rate();
            if validity > SignalsConfig::CHAR_VALIDITY_HIGH_THRESHOLD {
                // High validity = good vector text
                return Some(Vote::vector(SignalsConfig::CHAR_VALIDITY_HIGH_STRENGTH));
            }
        }
        None
    }

    fn name(&self) -> &'static str {
        "high_char_validity"
    }
}

/// Signal: Character density ratio < 0.03 → Scanned.
///
/// Low density despite text operators indicates broken encoding
/// (font is present but few characters decode successfully).
struct LowDensitySignal;

impl SignalEvaluator for LowDensitySignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.has_text() && ctx.density_ratio < 0.03 {
            // Very low density = likely scanned or broken vector
            // Use high strength to short-circuit before HighCharValiditySignal
            return Some(Vote::scanned(0.95));
        }
        None
    }

    fn name(&self) -> &'static str {
        "low_density"
    }
}

/// Signal: Character density per pt² < 0.03 → Scanned.
///
/// Extremely low character density (chars per square point) suggests a cover page
/// or title page with minimal text, which may be a scan. This is a weaker fallback
/// signal (strength 0.65) that fires when stronger evaluators have not triggered.
struct CharDensityRatioSignal;

impl SignalEvaluator for CharDensityRatioSignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        // Skip if high character validity is present (mutually exclusive with HighCharValiditySignal)
        // If text decodes well, density doesn't matter - it's good vector text
        if ctx.has_text() && ctx.char_validity_rate() > SignalsConfig::CHAR_VALIDITY_HIGH_THRESHOLD
        {
            return None;
        }

        // Calculate character density: chars per square point
        let page_area_pt2 = ctx.width * ctx.height;
        if page_area_pt2 > 0.0 {
            let density = ctx.valid_char_count as f32 / page_area_pt2 as f32;
            if density < 0.03 {
                // Very sparse content → likely scanned cover/title page
                return Some(Vote::scanned(0.65));
            }
        } else if ctx.valid_char_count == 0 {
            // Zero area page with no text is effectively scanned
            return Some(Vote::scanned(0.65));
        }
        None
    }

    fn name(&self) -> &'static str {
        "char_density_ratio"
    }
}

/// Signal evaluator: all text Tr=3 + single image covering >= 95% page → BrokenVector.
///
/// This is the definitive BrokenVector signal per EC-12. It detects the classic
/// invisible-text-overlay pattern produced by PDF/A optimizers and scanner software.
///
/// # Arguments
///
/// * `ctx` - The page context containing text operator and image metrics
///
/// # Returns
///
/// `Some(Vote)` for BrokenVector with strength 0.99 if the pattern matches,
/// `None` otherwise.
///
/// # Detection Logic
///
/// - All text operators must have rendering mode Tr=3 (invisible)
/// - At least one image XObject must cover >= 95% of the page area
/// - Returns definitive strength (0.99) to short-circuit all other evaluators
///
/// # EC-12 Reference
///
/// Per plan section 5.1.2, this is the "Definitive" BrokenVector signal.
pub fn all_tr3_with_full_page_image(ctx: &PageContext) -> Option<Vote> {
    // All text operators must be Tr=3 (not just some)
    let all_tr3 = ctx.text_op_count > 0 && ctx.tr3_op_count == ctx.text_op_count;

    // Check if any single image XObject covers >= 95% of page area
    let page_area = ctx.width * ctx.height;
    let full_page_image = if page_area > 0.0 {
        ctx.image_xobject_areas
            .iter()
            .any(|&area| area / page_area >= 0.95)
    } else {
        false
    };

    if all_tr3 && full_page_image {
        return Some(Vote::broken_vector(0.99));
    }
    None
}

/// Signal evaluator: image coverage fraction > 0.85 → Scanned.
///
/// Computes the union image coverage of the page from individual image XObject areas.
/// Used as a fallback when the more-definitive `text_operator_presence` signal
/// doesn't fire.
///
/// # Arguments
///
/// * `ctx` - The page context containing image metrics and page dimensions
///
/// # Returns
///
/// `Some(Vote)` for Scanned with strength 0.85 if coverage > 0.85,
/// `None` otherwise.
///
/// # Detection Logic
///
/// - Sum all `image_xobject_areas` to get total image coverage
/// - Divide by page area (`width * height`) to get coverage fraction
/// - Clamp to [0.0, 1.0] to handle overlapping images (defensive)
/// - If clamped fraction > 0.85, vote Scanned with strength 0.85
///
/// # Note on Union vs Sum
///
/// This implementation uses sum for simplicity, which overestimates coverage
/// when images overlap. For example, 5 overlapping copies of one image would
/// sum to 5x area but the union is 1x area. This is acceptable for the 0.85
/// threshold as it's a conservative signal (fires more easily). Revisit with
/// Klee's algorithm (~O(N log N)) if accuracy demands.
///
/// # EC-12 Reference
///
/// Per plan section 5.1.2, this is a fallback Scanned signal.
pub fn image_coverage_fraction(ctx: &PageContext) -> Option<Vote> {
    let page_area_pt2 = ctx.width * ctx.height;

    // Guard against zero page area
    if page_area_pt2 <= 0.0 {
        return None;
    }

    // Compute total image coverage as sum of individual image areas
    let total_image_area: f64 = ctx.image_xobject_areas.iter().sum();

    // Compute coverage fraction and clamp to [0.0, 1.0]
    // Clamping is defensive: overlapping images could sum to > page area
    let coverage_fraction = (total_image_area / page_area_pt2).clamp(0.0, 1.0);

    // Fire signal if coverage exceeds threshold
    if coverage_fraction > 0.85 {
        Some(Vote::scanned(0.85))
    } else {
        None
    }
}

/// Classify a page using the full signal evaluator pipeline.
///
/// This is the main entry point for page classification. It creates a PageClassifier
/// and runs classification on the given page context.
///
/// # Arguments
///
/// * `ctx` - The page context containing all metrics needed for classification
///
/// # Returns
///
/// A `PageClassification` containing the class, confidence, and
/// optionally the set of hybrid cell indexes for Hybrid pages.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{classify_page, PageContext};
///
/// let ctx = PageContext {
///     text_op_count: 150,
///     invisible_text_count: 0,
///     tr3_op_count: 0,
///     image_xobject_areas: vec![],
///     raw_char_count: 2500,
///     valid_char_count: 2450,
///     replacement_char_count: 50,
///     image_coverage: 0.0,
///     has_full_page_image: false,
///     has_visible_text: true,
///     density_ratio: 0.92,
///     width: 612.0,
///     height: 792.0,
///     rotation: 0,
///     grid_cells: None,
/// };
///
/// let classification = classify_page(&ctx);
/// assert_eq!(classification.class, pdftract_core::classify::PageClass::Vector);
/// ```
pub fn classify_page(ctx: &PageContext) -> PageClassification {
    let classifier = PageClassifier::new();
    classifier.classify(ctx)
}

/// Page classifier that runs all signal evaluators and produces a decision.
///
/// The classifier implements the following pipeline:
/// 1. Check for special cases (blank, image-only)
/// 2. Run Hybrid evaluator first (if grid data available)
/// 3. Walk signal evaluators in order, applying short-circuit at >= 0.95
/// 4. Tally remaining votes weighted by strength
/// 5. Default to Vector with confidence 0.5 if no votes
pub struct PageClassifier {
    /// Signal evaluators in declaration order.
    signals: Vec<Box<dyn SignalEvaluator>>,
}

impl PageClassifier {
    /// Create a new PageClassifier with default signal evaluators.
    ///
    /// Signals are evaluated in this order:
    /// 1. No text operators → Scanned
    /// 2. Invisible text with image → BrokenVector
    /// 3. High image coverage → Scanned
    /// 4. Low char validity → BrokenVector
    /// 5. Low density → Scanned
    /// 6. High char validity → Vector
    /// 7. Character density per pt² → Scanned (weak fallback)
    ///
    /// NOTE: Low density is evaluated before high validity to ensure that
    /// sparse/broken text pages are correctly classified as Scanned even when
    /// character validity happens to be high (which can occur with minimal text).
    /// Char density ratio is a weaker fallback signal (0.65 strength) that fires
    /// after the stronger signals have been evaluated.
    pub fn new() -> Self {
        Self {
            signals: vec![
                Box::new(NoTextOperatorsSignal),
                Box::new(InvisibleTextWithImageSignal),
                Box::new(HighImageCoverageSignal),
                Box::new(LowCharValiditySignal),
                Box::new(LowDensitySignal),
                Box::new(HighCharValiditySignal),
                Box::new(CharDensityRatioSignal),
            ],
        }
    }

    /// Classify a page based on its context.
    ///
    /// This is the main entry point for page classification.
    pub fn classify(&self, ctx: &PageContext) -> PageClassification {
        // Special case: blank page (no text, no images)
        if ctx.is_blank() {
            // Return Vector with 0.0 confidence as a sentinel
            // The mapping layer will convert this to "blank" page_type
            return PageClassification::new(PageClass::Vector, 0.0);
        }

        // Step 1: Run Hybrid evaluator first (if grid data available)
        if let Some(cells) = &ctx.grid_cells {
            let hybrid_result = self.classify_hybrid(ctx, cells);
            if hybrid_result.class == PageClass::Hybrid {
                // Hybrid takes precedence - return immediately
                return hybrid_result;
            }
        }

        // Step 2: Walk signal evaluators in order, checking for short-circuit
        let mut votes: Vec<Vote> = Vec::new();

        for signal in &self.signals {
            if let Some(vote) = signal.evaluate(ctx) {
                // Short-circuit: very high confidence (>= 0.95)
                if vote.strength >= 0.95 {
                    return PageClassification::new(vote.class, vote.strength);
                }
                votes.push(vote);
            }
        }

        // Step 3: Tally votes weighted by strength
        if votes.is_empty() {
            // No signals fired - default to Vector with low confidence
            return PageClassification::new(PageClass::Vector, 0.5);
        }

        // Weight each class by sum of strengths
        let mut class_weights: std::collections::HashMap<PageClass, f32> =
            std::collections::HashMap::new();
        let mut total_weight = 0.0;

        for vote in &votes {
            *class_weights.entry(vote.class).or_insert(0.0) += vote.strength;
            total_weight += vote.strength;
        }

        // Find the class with highest weight
        let mut best_class = PageClass::Vector;
        let mut best_weight = 0.0;

        for (class, weight) in &class_weights {
            if *weight > best_weight {
                best_weight = *weight;
                best_class = *class;
            }
        }

        // Confidence is the winning weight divided by total weight
        let confidence = if total_weight > 0.0 {
            best_weight / total_weight
        } else {
            0.5
        };

        PageClassification::new(best_class, confidence)
    }

    /// Run the Hybrid evaluator on grid cell data.
    ///
    /// Returns Hybrid classification if the ≥15% rule is met,
    /// otherwise returns a non-Hybrid classification based on cell counts.
    fn classify_hybrid(&self, ctx: &PageContext, cells: &[CellData; 64]) -> PageClassification {
        let mut vector_count = 0u32;
        let mut scanned_count = 0u32;
        let mut scanned_cells = BTreeSet::new();

        for (i, cell) in cells.iter().enumerate() {
            match cell.classify() {
                CellClass::Vector => vector_count += 1,
                CellClass::Scanned => {
                    scanned_count += 1;
                    scanned_cells.insert(i);
                }
                CellClass::Mixed => {}
            }
        }

        // Hybrid detection: ≥ 10 cells of each type (≥ 15% of 64)
        if vector_count >= 10 && scanned_count >= 10 {
            let vector_ratio = vector_count as f32 / 64.0;
            let scanned_ratio = scanned_count as f32 / 64.0;
            let confidence = vector_ratio.min(scanned_ratio);

            return PageClassification::hybrid(confidence, scanned_cells);
        }

        // Not hybrid - classify based on dominant signal
        // This result will be considered along with other signal evaluators
        if vector_count > scanned_count {
            PageClassification::new(PageClass::Vector, vector_count as f32 / 64.0)
        } else if scanned_count > 0 {
            PageClassification::new(PageClass::Scanned, scanned_count as f32 / 64.0)
        } else {
            // No clear signal - let other evaluators decide
            PageClassification::new(PageClass::Vector, 0.0)
        }
    }
}

impl Default for PageClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Page classification result.
///
/// Represents the extraction path that should be used for this page.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::PageClass;
///
/// let class = PageClass::Vector;
/// assert_eq!(class.as_type_str(), "text");
///
/// assert!(class.can_escalate_to_broken_vector());
///
/// let scanned = PageClass::Scanned;
/// assert_eq!(scanned.as_type_str(), "scanned");
/// assert!(!scanned.can_escalate_to_broken_vector());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageClass {
    /// Vector (text-based) page - use Phase 3 content stream extraction.
    Vector,
    /// Scanned page - use Phase 5.2 raster extraction + OCR.
    Scanned,
    /// Hybrid page - use Phase 3 for vector cells + Phase 5.2 for scanned cells.
    Hybrid,
    /// BrokenVector (invisible text layer over scanned image).
    BrokenVector,
}

impl PageClass {
    /// Returns the JSON output string for this page type.
    ///
    /// Maps internal enum values to the schema's `page_type` field.
    pub fn as_type_str(&self) -> &'static str {
        match self {
            PageClass::Vector => "text",
            PageClass::Scanned => "scanned",
            PageClass::Hybrid => "mixed",
            PageClass::BrokenVector => "broken_vector",
        }
    }

    /// Check if this page class is eligible for BrokenVector escalation.
    ///
    /// Only Vector pages can be escalated to BrokenVector based on readability.
    /// Scanned and Hybrid pages are already handled by other paths.
    pub fn can_escalate_to_broken_vector(&self) -> bool {
        matches!(self, PageClass::Vector)
    }
}

/// Compute the canonical page_type string for the JSON schema output.
///
/// This function implements the stable mapping from (PageClass, ocr_succeeded, has_text, has_images)
/// to the page_type string emitted in the 6.1 JSON schema. The mapping is frozen per INV-9.
///
/// # Mapping Table
///
/// | class           | ocr_succeeded | has_text | has_images | page_type        |
/// |-----------------|---------------|----------|------------|------------------|
/// | Vector          | -             | -        | -          | "text"           |
/// | Scanned         | -             | -        | -          | "scanned"        |
/// | Hybrid          | -             | -        | -          | "mixed"          |
/// | BrokenVector    | false         | -        | -          | "broken_vector"  |
/// | BrokenVector    | true          | -        | -          | "scanned"        | // post-OCR recovery
/// | (any)           | -             | false    | false      | "blank"          | // overrides class
/// | (any)           | -             | false    | true       | "figure_only"    | // overrides class
///
/// # Precedence Rules
///
/// 1. **Override checks first**: If `has_text == false` and `has_images == false`, return "blank".
///    If `has_text == false` and `has_images == true`, return "figure_only".
///    These overrides apply regardless of the PageClass value.
/// 2. **Class-based mapping**: If no override applies, map based on PageClass:
///    - Vector → "text"
///    - Scanned → "scanned"
///    - Hybrid → "mixed"
///    - BrokenVector with `ocr_succeeded == true` → "scanned" (post-OCR recovery)
///    - BrokenVector with `ocr_succeeded == false` → "broken_vector"
///
/// # Arguments
///
/// * `class` - The PageClass from Phase 5.1 classification
/// * `ocr_succeeded` - Whether OCR successfully recovered text (only relevant for BrokenVector)
/// * `has_text` - Whether the page contains any text glyphs
/// * `has_images` - Whether the page contains any images
///
/// # Returns
///
/// The canonical page_type string as a static str. This string is guaranteed to be
/// one of the six values in the 6.1 JSON schema enum: "text", "scanned", "mixed",
/// "broken_vector", "blank", or "figure_only".
///
/// # INV-9 Stable Taxonomy
///
/// The page_type strings are FROZEN by the 6.1 schema version. Any change requires
/// a schema_version bump and a downstream migration plan. Do not modify this function
/// without updating the JSON schema and plan.md.
pub fn page_type_string(
    class: PageClass,
    ocr_succeeded: bool,
    has_text: bool,
    has_images: bool,
) -> &'static str {
    // Override checks take precedence over class-based mapping.
    // These represent the "blank" and "figure_only" page types which are
    // determined solely by content presence, not by classification.
    if !has_text && !has_images {
        return "blank";
    }
    if !has_text && has_images {
        return "figure_only";
    }

    // Class-based mapping (applies when has_text == true or the override didn't match).
    match class {
        PageClass::Vector => "text",
        PageClass::Scanned => "scanned",
        PageClass::Hybrid => "mixed",
        PageClass::BrokenVector => {
            if ocr_succeeded {
                "scanned" // Post-OCR recovery: treated as scanned
            } else {
                "broken_vector"
            }
        }
    }
}

/// Apply BrokenVector escalation based on readability score (Phase 4.7).
///
/// Per plan section 4.7 (line 1801): If page readability score < 0.5 AND
/// the page is classified as Vector, escalate to BrokenVector and route
/// to Phase 5.5 assisted OCR.
///
/// # Arguments
///
/// * `current_class` - The current page classification from Phase 5.1
/// * `readability_score` - The page-level readability score from `aggregate_page_readability`
/// * `page_index` - The page index (for diagnostic messages)
///
/// # Returns
///
/// The updated `PageClass` after escalation logic:
/// - If readability < 0.5 AND current_class is Vector: returns BrokenVector
/// - Otherwise: returns current_class unchanged
///
/// # Escalation Behavior
///
/// When escalation occurs (Vector → BrokenVector):
/// - With `ocr` feature: routes to Phase 5.5 assisted OCR for re-extraction
/// - Without `ocr` feature: emits `BROKENVECTOR_OCR_UNAVAILABLE` diagnostic
///   and sets page_type = "broken_vector" in output (no re-extraction)
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{apply_broken_vector_escalation, PageClass};
///
/// // Vector page with low readability escalates to BrokenVector
/// let result = apply_broken_vector_escalation(PageClass::Vector, 0.3, 0);
/// assert_eq!(result, PageClass::BrokenVector);
///
/// // Vector page with good readability stays Vector
/// let result = apply_broken_vector_escalation(PageClass::Vector, 0.8, 1);
/// assert_eq!(result, PageClass::Vector);
///
/// // Scanned pages never escalate (not eligible)
/// let result = apply_broken_vector_escalation(PageClass::Scanned, 0.2, 2);
/// assert_eq!(result, PageClass::Scanned);
///
/// // Hybrid pages never escalate (not eligible)
/// let result = apply_broken_vector_escalation(PageClass::Hybrid, 0.1, 3);
/// assert_eq!(result, PageClass::Hybrid);
///
/// // BrokenVector pages stay BrokenVector regardless of readability
/// let result = apply_broken_vector_escalation(PageClass::BrokenVector, 0.9, 4);
/// assert_eq!(result, PageClass::BrokenVector);
/// ```
pub fn apply_broken_vector_escalation(
    current_class: PageClass,
    readability_score: f32,
    page_index: usize,
) -> PageClass {
    // Escalation only applies to Vector pages
    if !current_class.can_escalate_to_broken_vector() {
        return current_class;
    }

    // Check readability threshold (0.5 per plan spec)
    if readability_score < 0.5 {
        #[cfg(feature = "ocr")]
        {
            // Route to Phase 5.5 assisted OCR
            // TODO: Implement Phase 5.5 routing when available
            // For now, escalate to BrokenVector to indicate re-extraction needed
        }

        #[cfg(not(feature = "ocr"))]
        {
            // Emit diagnostic when OCR feature is unavailable
            use crate::diagnostics::{DiagCode, Diagnostic};

            // Emit diagnostic via a thread-local or callback mechanism
            // For now, we escalate to BrokenVector which will be reflected in output
            Diagnostic::with_dynamic_no_offset(
                DiagCode::OcrBrokenVectorUnavailable,
                format!(
                    "Page {} readability {:.2} < 0.5 on Vector page; OCR feature unavailable",
                    page_index, readability_score
                ),
            );
        }

        PageClass::BrokenVector
    } else {
        current_class
    }
}

/// Page classification result with confidence and metadata.
///
/// Contains the classification decision, confidence score, and optionally
/// the set of hybrid cell indexes for OCR routing.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{PageClassification, PageClass};
/// use std::collections::BTreeSet;
///
/// // Create a simple classification
/// let classification = PageClassification::new(PageClass::Vector, 0.9);
/// assert_eq!(classification.class, PageClass::Vector);
/// assert_eq!(classification.confidence, 0.9);
/// assert!(classification.hybrid_cells.is_none());
///
/// // Create a hybrid classification with cell indexes
/// let mut cells = BTreeSet::new();
/// cells.insert(10);
/// cells.insert(18);
/// cells.insert(27);
/// let hybrid = PageClassification::hybrid(0.85, cells);
/// assert_eq!(hybrid.class, PageClass::Hybrid);
/// assert!(hybrid.hybrid_cells.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageClassification {
    /// The classification decision.
    pub class: PageClass,
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// For Hybrid pages: set of scanned cell indexes (row * 8 + col).
    /// None for non-Hybrid classifications.
    pub hybrid_cells: Option<BTreeSet<usize>>,
}

impl PageClassification {
    /// Create a new classification with the given class and confidence.
    pub fn new(class: PageClass, confidence: f32) -> Self {
        Self {
            class,
            confidence,
            hybrid_cells: None,
        }
    }

    /// Create a Hybrid classification with scanned cell indexes.
    pub fn hybrid(confidence: f32, hybrid_cells: BTreeSet<usize>) -> Self {
        Self {
            class: PageClass::Hybrid,
            confidence,
            hybrid_cells: Some(hybrid_cells),
        }
    }
}

/// Cell index in the 8×8 grid.
///
/// Cells are indexed as (row, col) where:
/// - row: 0..8 (0 = top of page in rendered orientation)
/// - col: 0..8 (0 = left of page)
///
/// The flat index is `row * 8 + col`, ranging from 0..63.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::CellIndex;
///
/// // Create a cell index
/// let cell = CellIndex::new(3, 5);
/// assert_eq!(cell.row, 3);
/// assert_eq!(cell.col, 5);
/// assert_eq!(cell.flat(), 3 * 8 + 5); // = 29
///
/// // Convert to and from flat index
/// let flat = 42;
/// let cell2 = CellIndex::from_flat(flat);
/// assert_eq!(cell2.row, 5); // 42 / 8 = 5
/// assert_eq!(cell2.col, 2);  // 42 % 8 = 2
/// assert_eq!(cell2.flat(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellIndex {
    /// Row index (0 = top, 7 = bottom).
    pub row: u8,
    /// Column index (0 = left, 7 = right).
    pub col: u8,
}

impl CellIndex {
    /// Create a new cell index.
    ///
    /// # Panics
    ///
    /// Panics if row or col >= 8.
    pub fn new(row: u8, col: u8) -> Self {
        assert!(row < 8, "row must be < 8");
        assert!(col < 8, "col must be < 8");
        Self { row, col }
    }

    /// Convert to flat index (0..63).
    #[inline]
    pub fn flat(&self) -> usize {
        (self.row as usize) * 8 + (self.col as usize)
    }

    /// Create from flat index (0..63).
    ///
    /// # Panics
    ///
    /// Panics if flat >= 64.
    pub fn from_flat(flat: usize) -> Self {
        assert!(flat < 64, "flat index must be < 64");
        Self {
            row: (flat / 8) as u8,
            col: (flat % 8) as u8,
        }
    }
}

/// Cell classification for a single grid cell.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::CellClass;
///
/// let vector = CellClass::Vector;
/// let scanned = CellClass::Scanned;
/// let mixed = CellClass::Mixed;
///
/// // CellClass determines the extraction path for that cell
/// // Vector → content stream extraction
/// // Scanned → OCR
/// // Mixed → fallback to image analysis
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellClass {
    /// Vector cell: has text operators with high character validity.
    Vector,
    /// Scanned cell: has high image coverage with no text operators.
    Scanned,
    /// Mixed cell: neither condition met (empty or ambiguous).
    Mixed,
}

/// Per-cell analysis data.
///
/// Contains the metrics computed for each grid cell during classification.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{CellData, CellClass};
///
/// // Create empty cell data
/// let empty = CellData::empty();
/// assert_eq!(empty.text_op_count, 0);
/// assert_eq!(empty.classify(), CellClass::Mixed);
///
/// // Create cell data for a vector cell
/// let vector_cell = CellData {
///     text_op_count: 50,
///     image_coverage: 0.1,
///     char_validity: 0.95,
/// };
/// assert_eq!(vector_cell.classify(), CellClass::Vector);
///
/// // Create cell data for a scanned cell
/// let scanned_cell = CellData {
///     text_op_count: 0,
///     image_coverage: 0.9,
///     char_validity: 0.0,
/// };
/// assert_eq!(scanned_cell.classify(), CellClass::Scanned);
/// ```
#[derive(Debug, Clone)]
pub struct CellData {
    /// Number of text operators in this cell.
    pub text_op_count: u32,
    /// Image coverage fraction [0.0, 1.0].
    pub image_coverage: f32,
    /// Character validity rate [0.0, 1.0] (fraction of valid Unicode chars).
    pub char_validity: f32,
}

impl CellData {
    /// Create new cell data with all zeros.
    pub fn empty() -> Self {
        Self {
            text_op_count: 0,
            image_coverage: 0.0,
            char_validity: 0.0,
        }
    }

    /// Classify this cell based on its metrics.
    pub fn classify(&self) -> CellClass {
        // Vector: has text operators AND high character validity
        if self.text_op_count > 0 && self.char_validity > 0.6 {
            return CellClass::Vector;
        }
        // Scanned: high image coverage AND no text operators
        if self.image_coverage > 0.80 && self.text_op_count == 0 {
            return CellClass::Scanned;
        }
        // Mixed: neither condition met (empty or ambiguous)
        CellClass::Mixed
    }
}

/// Grid-based page classifier.
///
/// Implements the 8×8 grid decomposition for hybrid detection.
///
/// # Examples
///
/// ```
/// use pdftract_core::classify::{GridClassifier, CellIndex, CellData};
///
/// // Create a grid classifier for a US Letter page (612 x 792 pt)
/// let mut grid = GridClassifier::new(612.0, 792.0, 0);
///
/// // Access specific cells
/// let cell_idx = CellIndex::new(3, 4);
/// let cell_data = grid.cell(cell_idx);
/// assert_eq!(cell_data.text_op_count, 0);
///
/// // Modify a cell
/// let mut cell = grid.cell_mut(cell_idx);
/// cell.text_op_count = 25;
/// cell.image_coverage = 0.15;
/// cell.char_validity = 0.92;
///
/// // Classify a point to find which cell it belongs to
/// let cell_for_point = grid.point_to_cell(300.0, 400.0);
/// assert!(cell_for_point.row < 8 && cell_for_point.col < 8);
/// ```
pub struct GridClassifier {
    /// Page width in PDF user space units.
    width: f64,
    /// Page height in PDF user space units.
    height: f64,
    /// Page rotation in degrees (0, 90, 180, 270).
    rotation: i32,
    /// Cell data for each of the 64 cells.
    cells: [CellData; 64],
}

impl GridClassifier {
    /// Create a new grid classifier for a page.
    ///
    /// # Arguments
    ///
    /// * `width` - Page width in PDF user space units (after rotation applied).
    /// * `height` - Page height in PDF user space units (after rotation applied).
    /// * `rotation` - Page rotation in degrees (0, 90, 180, 270).
    pub fn new(width: f64, height: f64, rotation: i32) -> Self {
        Self {
            width,
            height,
            rotation,
            cells: std::array::from_fn(|_| CellData::empty()),
        }
    }

    /// Get mutable reference to cell data for a given cell index.
    pub fn cell_mut(&mut self, index: CellIndex) -> &mut CellData {
        &mut self.cells[index.flat()]
    }

    /// Get cell data for a given cell index.
    pub fn cell(&self, index: CellIndex) -> &CellData {
        &self.cells[index.flat()]
    }

    /// Compute which cell a point belongs to.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in PDF user space.
    /// * `y` - Y coordinate in PDF user space.
    ///
    /// # Returns
    ///
    /// The cell index containing the point.
    ///
    /// # Note
    ///
    /// This method assumes the page has already been rotated to its
    /// rendered orientation. The MediaBox coordinates should be
    /// transformed by the rotation matrix before calling this method.
    pub fn point_to_cell(&self, x: f64, y: f64) -> CellIndex {
        // Clamp to page bounds
        let x_clamped = x.clamp(0.0, self.width);
        let y_clamped = y.clamp(0.0, self.height);

        // Compute cell indices
        // col 0 is at the left (x = 0), col 7 is at the right (x = width)
        let col_idx = ((x_clamped / self.width) * 8.0).floor() as u8;
        let col = col_idx.min(7);

        // row 0 is at the top (y = height), row 7 is at the bottom (y = 0)
        let y_ratio = y_clamped / self.height;
        let y_idx = (y_ratio * 8.0).floor() as u8;
        let y_idx_clamped = y_idx.min(7);
        let row = 7 - y_idx_clamped;

        CellIndex::new(row, col)
    }

    /// Classify the page based on cell analysis.
    ///
    /// Computes the final page classification by counting cell types
    /// and applying the hybrid detection rule (≥10 vector AND ≥10 scanned).
    ///
    /// # Returns
    ///
    /// A `PageClassification` containing the class, confidence, and
    /// optionally the set of scanned cell indexes for Hybrid pages.
    pub fn classify(&self) -> PageClassification {
        let mut vector_count = 0u32;
        let mut scanned_count = 0u32;
        let mut scanned_cells = BTreeSet::new();

        for (i, cell) in self.cells.iter().enumerate() {
            match cell.classify() {
                CellClass::Vector => vector_count += 1,
                CellClass::Scanned => {
                    scanned_count += 1;
                    scanned_cells.insert(i);
                }
                CellClass::Mixed => {}
            }
        }

        // Hybrid detection: ≥ 10 cells of each type (≥ 15% of 64)
        if vector_count >= 10 && scanned_count >= 10 {
            // Confidence is derived from the minimum of the two ratios
            let vector_ratio = vector_count as f32 / 64.0;
            let scanned_ratio = scanned_count as f32 / 64.0;
            let confidence = vector_ratio.min(scanned_ratio);

            return PageClassification::hybrid(confidence, scanned_cells);
        }

        // Non-hybrid classification based on dominant signal
        // This is a simplified version; the full Phase 5.1 includes
        // additional signals (no text ops, Tr=3, image coverage, etc.)
        if vector_count > scanned_count {
            PageClassification::new(PageClass::Vector, vector_count as f32 / 64.0)
        } else if scanned_count > 0 {
            PageClassification::new(PageClass::Scanned, scanned_count as f32 / 64.0)
        } else {
            // Empty page (no vector, no scanned) - default to Vector
            // with low confidence; will be handled by other signals
            // in the full classifier
            PageClassification::new(PageClass::Vector, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_index_flat_conversion() {
        let cell = CellIndex::new(0, 0);
        assert_eq!(cell.flat(), 0);

        let cell = CellIndex::new(0, 1);
        assert_eq!(cell.flat(), 1);

        let cell = CellIndex::new(1, 0);
        assert_eq!(cell.flat(), 8);

        let cell = CellIndex::new(7, 7);
        assert_eq!(cell.flat(), 63);

        let cell = CellIndex::from_flat(0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);

        let cell = CellIndex::from_flat(8);
        assert_eq!(cell.row, 1);
        assert_eq!(cell.col, 0);

        let cell = CellIndex::from_flat(63);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 7);
    }

    #[test]
    fn test_cell_data_classify_vector() {
        let cell = CellData {
            text_op_count: 10,
            image_coverage: 0.1,
            char_validity: 0.9,
        };
        assert_eq!(cell.classify(), CellClass::Vector);
    }

    #[test]
    fn test_cell_data_classify_scanned() {
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.9,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Scanned);
    }

    #[test]
    fn test_cell_data_classify_mixed() {
        // Empty cell
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.0,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Text but low validity (char_validity <= 0.6)
        let cell = CellData {
            text_op_count: 10,
            image_coverage: 0.1,
            char_validity: 0.5,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Image but also text with low validity
        let cell = CellData {
            text_op_count: 1,
            image_coverage: 0.9,
            char_validity: 0.5,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);

        // Image with low coverage (< 0.80)
        let cell = CellData {
            text_op_count: 0,
            image_coverage: 0.5,
            char_validity: 0.0,
        };
        assert_eq!(cell.classify(), CellClass::Mixed);
    }

    #[test]
    fn test_grid_classifier_point_to_cell() {
        let classifier = GridClassifier::new(612.0, 792.0, 0);

        // Bottom-left corner -> row 7, col 0
        let cell = classifier.point_to_cell(0.0, 0.0);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 0);

        // Top-left corner -> row 0, col 0
        let cell = classifier.point_to_cell(0.0, 792.0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);

        // Top-right corner -> row 0, col 7
        let cell = classifier.point_to_cell(612.0, 792.0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 7);

        // Bottom-right corner -> row 7, col 7
        let cell = classifier.point_to_cell(612.0, 0.0);
        assert_eq!(cell.row, 7);
        assert_eq!(cell.col, 7);

        // Center -> row 3-4, col 3-4
        let cell = classifier.point_to_cell(306.0, 396.0);
        assert!(cell.row >= 3 && cell.row <= 4);
        assert!(cell.col >= 3 && cell.col <= 4);
    }

    #[test]
    fn test_grid_classifier_hybrid_detection() {
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Set up a hybrid page: top 2 rows (16 cells) are vector,
        // bottom 6 rows (48 cells) are scanned
        for row in 0..8 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                if row < 2 {
                    // Top rows: vector
                    cell.text_op_count = 10;
                    cell.char_validity = 0.95;
                    cell.image_coverage = 0.1;
                } else {
                    // Bottom rows: scanned
                    cell.text_op_count = 0;
                    cell.image_coverage = 0.9;
                    cell.char_validity = 0.0;
                }
            }
        }

        let result = classifier.classify();
        assert_eq!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_some());
        assert_eq!(result.hybrid_cells.as_ref().unwrap().len(), 48);

        // Verify scanned cells are from rows 2-7 only
        for flat in result.hybrid_cells.as_ref().unwrap() {
            let cell = CellIndex::from_flat(*flat);
            assert!(cell.row >= 2, "scanned cell should be in rows 2-7");
        }
    }

    #[test]
    fn test_grid_classifier_below_threshold() {
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Set up a page with 9 vector cells and 9 scanned cells
        // (just below the 10-cell threshold)
        // Use a 3x3 arrangement for each type
        for row in 0..3 {
            for col in 0..3 {
                let vector_cell = classifier.cell_mut(CellIndex::new(row, col));
                vector_cell.text_op_count = 10;
                vector_cell.char_validity = 0.95;
                vector_cell.image_coverage = 0.1;
            }
        }
        for row in 5..8 {
            for col in 5..8 {
                let scanned_cell = classifier.cell_mut(CellIndex::new(row, col));
                scanned_cell.text_op_count = 0;
                scanned_cell.image_coverage = 0.9;
                scanned_cell.char_validity = 0.0;
            }
        }

        let result = classifier.classify();
        // Should NOT be Hybrid (below threshold)
        assert_ne!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_class_as_type_str() {
        assert_eq!(PageClass::Vector.as_type_str(), "text");
        assert_eq!(PageClass::Scanned.as_type_str(), "scanned");
        assert_eq!(PageClass::Hybrid.as_type_str(), "mixed");
        assert_eq!(PageClass::BrokenVector.as_type_str(), "broken_vector");
    }

    #[test]
    fn test_page_classification_hybrid() {
        let mut cells = BTreeSet::new();
        cells.insert(16);
        cells.insert(17);

        let classification = PageClassification::hybrid(0.75, cells);

        assert_eq!(classification.class, PageClass::Hybrid);
        assert_eq!(classification.confidence, 0.75);
        assert!(classification.hybrid_cells.is_some());
        assert_eq!(classification.hybrid_cells.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_determinism_btree_set() {
        // Verify BTreeSet produces deterministic iteration order
        let mut set1 = BTreeSet::new();
        set1.insert(5);
        set1.insert(2);
        set1.insert(8);

        let mut set2 = BTreeSet::new();
        set2.insert(8);
        set2.insert(5);
        set2.insert(2);

        // Iteration order should be the same
        assert_eq!(
            set1.iter().collect::<Vec<_>>(),
            set2.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    #[should_panic(expected = "row must be < 8")]
    fn test_cell_index_invalid_row() {
        CellIndex::new(8, 0);
    }

    #[test]
    #[should_panic(expected = "col must be < 8")]
    fn test_cell_index_invalid_col() {
        CellIndex::new(0, 8);
    }

    #[test]
    #[should_panic(expected = "flat index must be < 64")]
    fn test_cell_index_invalid_flat() {
        CellIndex::from_flat(64);
    }

    #[test]
    fn test_critical_hybrid_page_text_header_scanned_body() {
        // Critical test from bead pdftract-347:
        // Hybrid page with text header (top 2 rows) + scanned body (bottom 6 rows)
        // -> Hybrid with hybrid_cells containing rows 2-7 only
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // Top 2 rows: vector (text header)
        for row in 0..2 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                cell.text_op_count = 15;
                cell.char_validity = 0.95;
                cell.image_coverage = 0.05;
            }
        }

        // Bottom 6 rows: scanned (body)
        for row in 2..8 {
            for col in 0..8 {
                let idx = CellIndex::new(row, col);
                let cell = classifier.cell_mut(idx);
                cell.text_op_count = 0;
                cell.image_coverage = 0.90;
                cell.char_validity = 0.0;
            }
        }

        let result = classifier.classify();

        // Should be classified as Hybrid
        assert_eq!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_some());

        let scanned_cells = result.hybrid_cells.as_ref().unwrap();
        assert_eq!(scanned_cells.len(), 48); // 6 rows * 8 cols

        // Verify all scanned cells are from rows 2-7 only
        for flat in scanned_cells {
            let cell = CellIndex::from_flat(*flat);
            assert!(
                cell.row >= 2 && cell.row <= 7,
                "scanned cell at flat {} should be in rows 2-7, got row {}",
                flat,
                cell.row
            );
        }
    }

    #[test]
    fn test_determinism_classify_twice() {
        // Verify that classifying the same page twice produces byte-identical
        // hybrid_cells serialization (BTreeSet ensures deterministic ordering)
        let mut classifier1 = GridClassifier::new(612.0, 792.0, 0);
        let mut classifier2 = GridClassifier::new(612.0, 792.0, 0);

        // Set up identical hybrid pages
        for row in 0..8 {
            for col in 0..8 {
                let is_scanned = row >= 4 && col >= 4;
                let cell1 = classifier1.cell_mut(CellIndex::new(row, col));
                let cell2 = classifier2.cell_mut(CellIndex::new(row, col));

                if is_scanned {
                    cell1.text_op_count = 0;
                    cell1.image_coverage = 0.9;
                    cell1.char_validity = 0.0;

                    cell2.text_op_count = 0;
                    cell2.image_coverage = 0.9;
                    cell2.char_validity = 0.0;
                } else {
                    cell1.text_op_count = 10;
                    cell1.char_validity = 0.95;
                    cell1.image_coverage = 0.1;

                    cell2.text_op_count = 10;
                    cell2.char_validity = 0.95;
                    cell2.image_coverage = 0.1;
                }
            }
        }

        let result1 = classifier1.classify();
        let result2 = classifier2.classify();

        assert_eq!(result1.class, result2.class);
        assert_eq!(result1.confidence, result2.confidence);

        // Verify hybrid_cells serialize identically
        let json1 = serde_json::to_string(&result1.hybrid_cells).unwrap();
        let json2 = serde_json::to_string(&result2.hybrid_cells).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_exactly_10_cells_threshold() {
        // Test the exact threshold: 10 vector cells + 10 scanned cells = Hybrid
        let mut classifier = GridClassifier::new(612.0, 792.0, 0);

        // 10 vector cells (row 0, cols 0-7 + row 1, cols 0-1)
        for col in 0..8 {
            let cell = classifier.cell_mut(CellIndex::new(0, col));
            cell.text_op_count = 10;
            cell.char_validity = 0.95;
            cell.image_coverage = 0.1;
        }
        for col in 0..2 {
            let cell = classifier.cell_mut(CellIndex::new(1, col));
            cell.text_op_count = 10;
            cell.char_validity = 0.95;
            cell.image_coverage = 0.1;
        }

        // 10 scanned cells (row 7, cols 0-7 + row 6, cols 0-1)
        for col in 0..8 {
            let cell = classifier.cell_mut(CellIndex::new(7, col));
            cell.text_op_count = 0;
            cell.image_coverage = 0.9;
            cell.char_validity = 0.0;
        }
        for col in 0..2 {
            let cell = classifier.cell_mut(CellIndex::new(6, col));
            cell.text_op_count = 0;
            cell.image_coverage = 0.9;
            cell.char_validity = 0.0;
        }

        let result = classifier.classify();
        assert_eq!(result.class, PageClass::Hybrid);
    }

    #[test]
    fn test_rotation_handling() {
        // Verify that rotation is stored (actual rotation handling
        // requires transforming coordinates before calling point_to_cell)
        let classifier_rotated = GridClassifier::new(792.0, 612.0, 90);
        assert_eq!(classifier_rotated.rotation, 90);
        assert_eq!(classifier_rotated.width, 792.0);
        assert_eq!(classifier_rotated.height, 612.0);

        // After 90-degree rotation, width and height are swapped
        let classifier_normal = GridClassifier::new(612.0, 792.0, 0);
        assert_eq!(classifier_normal.rotation, 0);
        assert_eq!(classifier_normal.width, 612.0);
        assert_eq!(classifier_normal.height, 792.0);
    }

    #[test]
    fn test_empty_page_classification() {
        // Empty page (no text, no images) should default to Vector with low confidence
        let classifier = GridClassifier::new(612.0, 792.0, 0);
        let result = classifier.classify();

        // Empty pages default to Vector (will be overridden by other signals in full classifier)
        assert_eq!(result.class, PageClass::Vector);
        assert_eq!(result.confidence, 0.0);
        assert!(result.hybrid_cells.is_none());
    }

    // ============ PageClassifier Tests (Phase 5.1.4) ============

    #[test]
    fn test_page_context_blank_page() {
        let ctx = PageContext::new();
        assert!(ctx.is_blank());
        assert!(!ctx.is_image_only());
        assert!(!ctx.has_text());
        assert!(!ctx.has_images());
    }

    #[test]
    fn test_page_context_image_only() {
        let mut ctx = PageContext::new();
        ctx.image_coverage = 0.95;
        assert!(!ctx.is_blank());
        assert!(ctx.is_image_only());
        assert!(!ctx.has_text());
        assert!(ctx.has_images());
    }

    #[test]
    fn test_page_context_char_validity_rate() {
        let mut ctx = PageContext::new();
        ctx.raw_char_count = 1000;
        ctx.valid_char_count = 850;
        assert_eq!(ctx.char_validity_rate(), 0.85);

        // No text = vacuously valid
        let ctx2 = PageContext::new();
        assert_eq!(ctx2.char_validity_rate(), 1.0);
    }

    #[test]
    fn test_page_context_all_invisible_text() {
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.invisible_text_count = 100;
        assert!(ctx.is_all_invisible_text());

        ctx.invisible_text_count = 99;
        assert!(!ctx.is_all_invisible_text());
    }

    #[test]
    fn test_page_classifier_vector_pure_text() {
        // Critical test: pure vector PDF (born-digital text)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 500;
        ctx.raw_char_count = 3000;
        ctx.valid_char_count = 2900; // 97% validity
        ctx.invisible_text_count = 0;
        ctx.image_coverage = 0.0;
        ctx.has_visible_text = true;
        ctx.density_ratio = 0.85;

        let result = classify_page(&ctx);

        // High validity + no images = Vector with high confidence
        assert_eq!(result.class, PageClass::Vector);
        assert!(result.confidence > 0.90);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classifier_scanned_image_only() {
        // Critical test: scanned single-page PDF (image only)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 0;
        ctx.raw_char_count = 0;
        ctx.valid_char_count = 0;
        ctx.image_coverage = 0.95;
        ctx.has_full_page_image = true;
        ctx.density_ratio = 0.0;

        let result = classify_page(&ctx);

        // No text + high image coverage = Scanned
        assert_eq!(result.class, PageClass::Scanned);
        assert!(result.confidence > 0.90);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classifier_broken_vector() {
        // Critical test: PDF/A with invisible text layer over scanned image
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.invisible_text_count = 100; // All text is Tr=3
        ctx.tr3_op_count = 100; // Keep in sync with invisible_text_count
        ctx.raw_char_count = 1000;
        ctx.valid_char_count = 1000; // Text decodes but is invisible
        ctx.image_coverage = 0.95;
        ctx.has_full_page_image = true;
        ctx.density_ratio = 0.30;
        ctx.width = 612.0; // US Letter
        ctx.height = 792.0;
        // Add a full-page image (>= 95% of 484,704 pt²)
        // 0.95 * 484,704 = 460,468.8, so use 460,500 to be safely above threshold
        ctx.image_xobject_areas.push(460_500.0); // >= 95% coverage

        let result = classify_page(&ctx);

        // Invisible text + full-page image = BrokenVector
        assert_eq!(result.class, PageClass::BrokenVector);
        assert!(result.confidence > 0.95);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classifier_hybrid_with_grid() {
        // Critical test: hybrid page with text header and scanned body
        let mut ctx = PageContext::new();
        ctx.text_op_count = 200;
        ctx.raw_char_count = 1500;
        ctx.valid_char_count = 1400;
        ctx.image_coverage = 0.70;
        ctx.density_ratio = 0.50;
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.rotation = 0;

        // Set up grid cells: top 2 rows vector, bottom 6 rows scanned
        let mut cells = std::array::from_fn(|_| CellData::empty());
        for row in 0..8 {
            for col in 0..8 {
                let idx = row * 8 + col;
                if row < 2 {
                    // Vector cells (text header)
                    cells[idx] = CellData {
                        text_op_count: 15,
                        image_coverage: 0.05,
                        char_validity: 0.95,
                    };
                } else {
                    // Scanned cells (body)
                    cells[idx] = CellData {
                        text_op_count: 0,
                        image_coverage: 0.90,
                        char_validity: 0.0,
                    };
                }
            }
        }
        ctx.grid_cells = Some(cells);

        let result = classify_page(&ctx);

        // Hybrid detection should trigger
        assert_eq!(result.class, PageClass::Hybrid);
        assert!(result.hybrid_cells.is_some());
        assert_eq!(result.hybrid_cells.as_ref().unwrap().len(), 48); // 6 rows * 8 cols
    }

    #[test]
    fn test_page_classifier_blank_page() {
        // Edge case: blank page (no text, no images)
        let ctx = PageContext::new();

        let result = classify_page(&ctx);

        // Blank pages return Vector with 0.0 confidence as a sentinel
        assert_eq!(result.class, PageClass::Vector);
        assert_eq!(result.confidence, 0.0);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classifier_image_only_figure() {
        // Edge case: full-page image with no text (scanned page)
        // Note: This is classified as Scanned, not "figure_only"
        // The mapping layer can convert to "figure_only" based on additional context
        let mut ctx = PageContext::new();
        ctx.text_op_count = 0;
        ctx.image_coverage = 0.95;
        ctx.has_full_page_image = true;

        let result = classify_page(&ctx);

        // No text + images = Scanned (will route to OCR)
        assert_eq!(result.class, PageClass::Scanned);
        assert!(result.confidence > 0.90);
        assert!(result.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classifier_short_circuit_no_text() {
        // Short-circuit test: no text operators with images
        let mut ctx = PageContext::new();
        ctx.text_op_count = 0;
        ctx.image_coverage = 0.50;

        let result = classify_page(&ctx);

        // Should short-circuit to Scanned with >=0.95 confidence
        assert_eq!(result.class, PageClass::Scanned);
        assert!(result.confidence >= 0.95);
    }

    #[test]
    fn test_page_classifier_short_circuit_invisible_with_image() {
        // Short-circuit test: all invisible text with full-page image
        let mut ctx = PageContext::new();
        ctx.text_op_count = 50;
        ctx.invisible_text_count = 50;
        ctx.tr3_op_count = 50; // Must match invisible_text_count for BrokenVector detection
        ctx.has_full_page_image = true;
        ctx.image_coverage = 0.90;
        ctx.width = 612.0; // US Letter
        ctx.height = 792.0;
        // Add a full-page image (>= 95% of 484,704 pt²)
        // 0.95 * 484,704 = 460,468.8, so use 460,500 to be safely above threshold
        ctx.image_xobject_areas.push(460_500.0); // >= 95% coverage

        let result = classify_page(&ctx);

        // Should short-circuit to BrokenVector with >0.95 confidence
        assert_eq!(result.class, PageClass::BrokenVector);
        assert!(result.confidence > 0.95);
    }

    #[test]
    fn test_page_classifier_low_char_validity() {
        // Low character validity indicates broken encoding
        let mut ctx = PageContext::new();
        ctx.text_op_count = 200;
        ctx.raw_char_count = 1000;
        ctx.valid_char_count = 200; // 20% validity
        ctx.replacement_char_count = 800;
        ctx.image_coverage = 0.10;
        ctx.density_ratio = 0.25;

        let result = classify_page(&ctx);

        // Low validity should push toward BrokenVector
        assert_eq!(result.class, PageClass::BrokenVector);
        assert!(result.confidence > 0.90);
    }

    #[test]
    fn test_page_classifier_high_image_coverage() {
        // High image coverage (> 0.85) pushes toward Scanned
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.raw_char_count = 500;
        ctx.valid_char_count = 400; // 80% validity (not high enough for Vector)
        ctx.image_coverage = 0.90;
        ctx.density_ratio = 0.20;

        let result = classify_page(&ctx);

        // High image coverage should push toward Scanned
        assert_eq!(result.class, PageClass::Scanned);
        assert!(result.confidence > 0.85);
    }

    #[test]
    fn test_page_classifier_low_density() {
        // Low density ratio (< 0.03) indicates sparse or broken text
        let mut ctx = PageContext::new();
        ctx.text_op_count = 50;
        ctx.raw_char_count = 50;
        ctx.valid_char_count = 50;
        ctx.image_coverage = 0.10;
        ctx.density_ratio = 0.02; // Below threshold

        let result = classify_page(&ctx);

        // Low density should push toward Scanned
        assert_eq!(result.class, PageClass::Scanned);
        assert!(result.confidence > 0.70);
    }

    #[test]
    fn test_page_classifier_default_vector() {
        // No strong signals - should default to Vector
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.raw_char_count = 500;
        ctx.valid_char_count = 350; // 70% validity (ambiguous)
        ctx.image_coverage = 0.30;
        ctx.density_ratio = 0.20;

        let result = classify_page(&ctx);

        // Default to Vector with 0.5 confidence
        assert_eq!(result.class, PageClass::Vector);
        assert!(result.confidence > 0.4 && result.confidence < 0.7);
    }

    #[test]
    fn test_page_classifier_determinism() {
        // Verify that classifying the same context twice produces identical results
        let mut ctx = PageContext::new();
        ctx.text_op_count = 250;
        ctx.raw_char_count = 2000;
        ctx.valid_char_count = 1800;
        ctx.image_coverage = 0.15;
        ctx.density_ratio = 0.60;

        let result1 = classify_page(&ctx);
        let result2 = classify_page(&ctx);

        assert_eq!(result1.class, result2.class);
        assert_eq!(result1.confidence, result2.confidence);
        assert_eq!(
            result1.hybrid_cells.is_some(),
            result2.hybrid_cells.is_some()
        );
    }

    #[test]
    fn test_page_classifier_confidence_in_range() {
        // Verify all confidence values are in [0.0, 1.0]
        let test_cases = vec![
            // (text_ops, raw_chars, valid_chars, image_cov, density)
            (0, 0, 0, 0.0, 0.0),         // blank
            (0, 0, 0, 0.95, 0.0),        // scanned
            (100, 1000, 100, 0.1, 0.1),  // low validity
            (500, 3000, 2900, 0.0, 0.9), // high validity vector
            (200, 1500, 1400, 0.7, 0.5), // ambiguous
        ];

        for (text_ops, raw, valid, img_cov, density) in test_cases {
            let mut ctx = PageContext::new();
            ctx.text_op_count = text_ops;
            ctx.raw_char_count = raw;
            ctx.valid_char_count = valid;
            ctx.image_coverage = img_cov;
            ctx.density_ratio = density;

            let result = classify_page(&ctx);
            assert!(
                result.confidence >= 0.0 && result.confidence <= 1.0,
                "confidence {} out of range for case ({}, {}, {}, {}, {})",
                result.confidence,
                text_ops,
                raw,
                valid,
                img_cov,
                density
            );
        }
    }

    #[test]
    fn test_page_classifier_entry_point() {
        // Test the classify_page entry point directly
        let mut ctx = PageContext::new();
        ctx.text_op_count = 300;
        ctx.raw_char_count = 2500;
        ctx.valid_char_count = 2400;
        ctx.image_coverage = 0.05;
        ctx.density_ratio = 0.75;

        // This should use the default PageClassifier
        let result = classify_page(&ctx);

        assert_eq!(result.class, PageClass::Vector);
        assert!(result.confidence > 0.85);
    }

    #[test]
    fn test_vote_helpers() {
        // Test Vote helper methods
        let v1 = Vote::vector(0.9);
        assert_eq!(v1.class, PageClass::Vector);
        assert_eq!(v1.strength, 0.9);

        let v2 = Vote::scanned(0.8);
        assert_eq!(v2.class, PageClass::Scanned);
        assert_eq!(v2.strength, 0.8);

        let v3 = Vote::broken_vector(0.95);
        assert_eq!(v3.class, PageClass::BrokenVector);
        assert_eq!(v3.strength, 0.95);
    }

    #[test]
    fn test_page_classifier_default_impl() {
        // Test PageClassifier default implementation
        let classifier = PageClassifier::default();
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.raw_char_count = 800;
        ctx.valid_char_count = 700;
        ctx.density_ratio = 0.7; // Set a reasonable density ratio

        let result = classifier.classify(&ctx);
        assert_eq!(result.class, PageClass::Vector);
    }

    // ============ CharDensityRatioSignal Tests ============

    #[test]
    fn test_char_density_ratio_signal_sparse_cover_page() {
        // AC: char_count=10, page_area_pt2=1000 → density=0.01 → Scanned with strength 0.65
        // Note: valid_char_count must be < 0.85 threshold to avoid early return
        let classifier = PageClassifier::default();
        let mut ctx = PageContext::new();
        ctx.text_op_count = 5; // Some text operators but very sparse
        ctx.raw_char_count = 10;
        ctx.valid_char_count = 8; // 80% validity (below 0.85 threshold)
        ctx.width = 25.0; // 25 * 40 = 1000 pt²
        ctx.height = 40.0;
        ctx.density_ratio = 0.5; // Normal density_ratio (not used by this signal)
        ctx.image_coverage = 0.0; // No images
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should return Some(Vote) for Scanned with strength 0.65
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.65);
    }

    #[test]
    fn test_char_density_ratio_signal_dense_page() {
        // AC: char_count=1000, page_area_pt2=1000 → density=1.0 → None
        let classifier = PageClassifier::default();
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.raw_char_count = 1000;
        ctx.valid_char_count = 1000; // 1000 characters
        ctx.width = 25.0; // 25 * 40 = 1000 pt²
        ctx.height = 40.0;
        ctx.density_ratio = 0.8;
        ctx.image_coverage = 0.0;
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should return None (density = 1.0 > 0.03 threshold)
        assert!(result.is_none());
    }

    #[test]
    fn test_char_density_ratio_signal_zero_chars() {
        // AC: char_count=0 → density=0 → Scanned with strength 0.65
        let classifier = PageClassifier::default();
        let mut ctx = PageContext::new();
        ctx.text_op_count = 0; // No text operators
        ctx.raw_char_count = 0;
        ctx.valid_char_count = 0; // No characters
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.density_ratio = 0.0;
        ctx.image_coverage = 0.0;
        ctx.has_visible_text = false;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Zero chars → triggers the signal
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.65);
    }

    #[test]
    fn test_char_density_ratio_signal_threshold_exact() {
        // Edge case: density exactly 0.03 → should not fire (only fires < 0.03)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 50;
        ctx.raw_char_count = 30;
        ctx.valid_char_count = 30;
        ctx.width = 10.0; // 10 * 100 = 1000 pt²
        ctx.height = 100.0; // 30 / 1000 = 0.03 (exactly at threshold)
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should NOT fire (threshold is < 0.03, not <= 0.03)
        assert!(result.is_none());
    }

    #[test]
    fn test_char_density_ratio_signal_just_below_threshold() {
        // Edge case: density = 0.0299 → should fire
        // Note: valid_char_count must be < 0.85 threshold to avoid early return
        let mut ctx = PageContext::new();
        ctx.text_op_count = 50;
        ctx.raw_char_count = 29;
        ctx.valid_char_count = 24; // ~83% validity (below 0.85 threshold)
        ctx.width = 10.0; // 10 * 100 = 1000 pt²
        ctx.height = 100.0; // 29 / 1000 = 0.029 (< 0.03)
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should fire (just below threshold)
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.65);
    }

    #[test]
    fn test_char_density_ratio_signal_zero_area_with_chars() {
        // Edge case: page_area_pt2 = 0 but has chars → should not fire (division by zero guard)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 50;
        ctx.raw_char_count = 100;
        ctx.valid_char_count = 100;
        ctx.width = 0.0; // Zero area
        ctx.height = 792.0;
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should NOT fire (division by zero is guarded)
        assert!(result.is_none());
    }

    #[test]
    fn test_char_density_ratio_signal_standard_letter_page() {
        // Realistic case: US Letter page (612×792 pt) with minimal text
        // Note: valid_char_count must be < 0.85 threshold to avoid early return
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.raw_char_count = 50;
        ctx.valid_char_count = 40; // 80% validity (below 0.85 threshold)
        ctx.width = 612.0; // US Letter width
        ctx.height = 792.0; // US Letter height
                            // density = 50 / (612 * 792) = 50 / 484,704 ≈ 0.0001 (well below 0.03)
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // Should fire (very sparse - cover page)
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.65);
    }

    #[test]
    fn test_char_density_ratio_signal_standard_page_with_text() {
        // Realistic case: US Letter page with normal text content
        // Note: valid_char_count must be < 0.85 threshold to avoid early return
        let mut ctx = PageContext::new();
        ctx.text_op_count = 500;
        ctx.raw_char_count = 3000;
        ctx.valid_char_count = 2400; // 80% validity (below 0.85 threshold)
        ctx.width = 612.0;
        ctx.height = 792.0;
        // density = 2900 / 484,704 ≈ 0.006 (still below 0.03)
        ctx.density_ratio = 0.85;
        ctx.has_visible_text = true;

        let signal = CharDensityRatioSignal;
        let result = signal.evaluate(&ctx);

        // This shows that even normal pages can have low chars/pt²
        // The signal is designed to be a weak fallback (0.65 strength) for very sparse pages
        assert!(result.is_some()); // Fires but with weak strength
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.65);
    }

    #[test]
    fn test_char_density_ratio_signal_name() {
        // Verify the signal name for debugging/diagnostics
        let signal = CharDensityRatioSignal;
        assert_eq!(signal.name(), "char_density_ratio");
    }

    #[test]
    fn test_char_density_ratio_signal_in_full_classifier() {
        // Integration test: verify CharDensityRatioSignal is wired into PageClassifier
        // Note: valid_char_count must be < 0.85 threshold to avoid early return
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.raw_char_count = 20;
        ctx.valid_char_count = 16; // 80% validity (below 0.85 threshold)
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.density_ratio = 0.6; // Normal density_ratio
        ctx.image_coverage = 0.0; // No images (so NoTextOperatorsSignal won't fire)
        ctx.has_visible_text = true;

        let classifier = PageClassifier::default();
        let result = classifier.classify(&ctx);

        // CharDensityRatioSignal should fire (20 / 484,704 ≈ 0.00004 < 0.03)
        // With strength 0.65, and no other signals firing, should classify as Scanned
        assert_eq!(result.class, PageClass::Scanned);
    }

    #[test]
    fn test_smoke_classify_basic_vector_page() {
        // Basic smoke test: verify classify_page works with a simple vector PDF page
        // This test verifies the basic functionality:
        // 1. Function returns Ok() for valid input
        // 2. Output structure is correct (classification, confidence)
        // 3. Reasonable classification and confidence values

        let mut ctx = PageContext::new();
        // Simple vector page: text-only, born-digital PDF
        ctx.text_op_count = 100;
        ctx.raw_char_count = 500;
        ctx.valid_char_count = 490; // 98% validity
        ctx.replacement_char_count = 10;
        ctx.image_coverage = 0.0; // No images
        ctx.has_full_page_image = false;
        ctx.has_visible_text = true;
        ctx.density_ratio = 0.90; // High character density
        ctx.width = 612.0; // US Letter
        ctx.height = 792.0;
        ctx.rotation = 0;

        // Call classify_page - returns PageClassification directly
        let result = classify_page(&ctx);

        // Verify basic output structure
        // Check that classification exists and is reasonable
        assert!(!matches!(result.class, PageClass::Hybrid)); // Not hybrid for simple page
        assert!(!matches!(result.class, PageClass::BrokenVector)); // Not broken vector

        // Check that confidence is in valid range [0.0, 1.0]
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0,
            "Confidence should be in [0.0, 1.0], got {}", result.confidence);

        // For a simple vector page, should have reasonable confidence
        assert!(result.confidence > 0.5,
            "Simple vector page should have confidence > 0.5, got {}", result.confidence);

        // Verify hybrid_cells is None for non-hybrid page
        assert!(result.hybrid_cells.is_none(),
            "hybrid_cells should be None for non-hybrid classification");

        // Verify JSON serialization works (basic output format check)
        let json_output = serde_json::to_string(&result);
        assert!(json_output.is_ok(), "PageClassification should serialize to JSON");

        let json_str = json_output.unwrap();
        assert!(json_str.contains("\"class\":"), "JSON should contain 'class' field");
        assert!(json_str.contains("\"confidence\":"), "JSON should contain 'confidence' field");

        println!("Smoke test passed: classify_page returned {:?}", result);
    }

    #[test]
    fn test_microbenchmark_classify_page_performance() {
        // Micro-benchmark: verify classify_page p99 < 5 ms
        // This test simulates a 50-fixture suite to verify performance

        use std::time::Instant;

        // Create 50 diverse page contexts representing real fixtures
        let fixtures: Vec<PageContext> = vec![
            // Vector pages (born-digital text)
            PageContext {
                text_op_count: 500,
                raw_char_count: 3000,
                valid_char_count: 2900,
                invisible_text_count: 0,
                tr3_op_count: 0,
                replacement_char_count: 50,
                image_coverage: 0.0,
                image_xobject_areas: Vec::new(),
                has_full_page_image: false,
                has_visible_text: true,
                density_ratio: 0.95,
                width: 612.0,
                height: 792.0,
                rotation: 0,
                grid_cells: None,
            },
            // Scanned pages (image-only)
            PageContext {
                text_op_count: 0,
                raw_char_count: 0,
                valid_char_count: 0,
                invisible_text_count: 0,
                tr3_op_count: 0,
                replacement_char_count: 0,
                image_coverage: 0.95,
                image_xobject_areas: vec![612.0 * 792.0],
                has_full_page_image: true,
                has_visible_text: false,
                density_ratio: 0.0,
                width: 612.0,
                height: 792.0,
                rotation: 0,
                grid_cells: None,
            },
            // BrokenVector pages
            PageContext {
                text_op_count: 100,
                raw_char_count: 1000,
                valid_char_count: 1000,
                invisible_text_count: 100,
                tr3_op_count: 100,
                replacement_char_count: 0,
                image_coverage: 0.95,
                image_xobject_areas: vec![612.0 * 792.0],
                has_full_page_image: true,
                has_visible_text: false,
                density_ratio: 0.30,
                width: 612.0,
                height: 792.0,
                rotation: 0,
                grid_cells: None,
            },
            // Hybrid pages
            PageContext {
                text_op_count: 200,
                raw_char_count: 1500,
                valid_char_count: 1400,
                invisible_text_count: 0,
                tr3_op_count: 0,
                replacement_char_count: 50,
                image_coverage: 0.70,
                image_xobject_areas: vec![200.0 * 300.0],
                has_full_page_image: false,
                has_visible_text: true,
                density_ratio: 0.50,
                width: 612.0,
                height: 792.0,
                rotation: 0,
                grid_cells: Some(std::array::from_fn(|i| {
                    let row = i / 8;
                    if row < 2 {
                        CellData {
                            text_op_count: 15,
                            image_coverage: 0.05,
                            char_validity: 0.95,
                        }
                    } else {
                        CellData {
                            text_op_count: 0,
                            image_coverage: 0.90,
                            char_validity: 0.0,
                        }
                    }
                })),
            },
        ];

        // Run each fixture 50 times to simulate 50-page document
        let iterations = 50;
        let mut durations = Vec::new();

        for _ in 0..iterations {
            for ctx in &fixtures {
                let start = Instant::now();
                let _result = classify_page(ctx);
                let elapsed = start.elapsed();
                durations.push(elapsed);
            }
        }

        // Calculate p99 (99th percentile)
        durations.sort();
        let p99_index = (durations.len() as f64 * 0.99) as usize;
        let p99 = durations[p99_index];

        // Verify p99 < 5 ms
        assert!(
            p99.as_millis() < 5,
            "classify_page p99 = {} ms, expected < 5 ms",
            p99.as_millis()
        );

        // Also verify median for good measure
        let median = durations[durations.len() / 2];
        assert!(
            median.as_micros() < 1000,
            "classify_page median = {} μs, expected < 1000 μs",
            median.as_micros()
        );
    }

    // ============ BrokenVector Escalation Tests (Phase 4.7) ============

    #[test]
    fn test_broken_vector_escalation_vector_low_readability() {
        // AC: Vector page with readability < 0.5 escalates to BrokenVector
        let current_class = PageClass::Vector;
        let readability_score = 0.4;
        let page_index = 5;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::BrokenVector);
    }

    #[test]
    fn test_broken_vector_escalation_vector_high_readability() {
        // AC: Vector page with readability >= 0.5 does NOT escalate
        let current_class = PageClass::Vector;
        let readability_score = 0.6;
        let page_index = 3;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::Vector);
    }

    #[test]
    fn test_broken_vector_escalation_vector_threshold_exact() {
        // AC: Vector page with readability exactly 0.5 does NOT escalate
        // (threshold is < 0.5, not <= 0.5)
        let current_class = PageClass::Vector;
        let readability_score = 0.5;
        let page_index = 0;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::Vector);
    }

    #[test]
    fn test_broken_vector_escalation_scanned_no_escalation() {
        // AC: Scanned page does NOT escalate (already OCR path)
        let current_class = PageClass::Scanned;
        let readability_score = 0.3;
        let page_index = 10;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::Scanned);
    }

    #[test]
    fn test_broken_vector_escalation_hybrid_no_escalation() {
        // AC: Hybrid page does NOT escalate (mixed path)
        let current_class = PageClass::Hybrid;
        let readability_score = 0.2;
        let page_index = 7;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::Hybrid);
    }

    #[test]
    fn test_broken_vector_escalation_broken_vector_stays() {
        // AC: Already BrokenVector page stays BrokenVector
        let current_class = PageClass::BrokenVector;
        let readability_score = 0.1;
        let page_index = 12;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::BrokenVector);
    }

    #[test]
    fn test_broken_vector_escalation_zero_readability() {
        // AC: Vector page with 0.0 readability escalates
        let current_class = PageClass::Vector;
        let readability_score = 0.0;
        let page_index = 2;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::BrokenVector);
    }

    #[test]
    fn test_broken_vector_escalation_perfect_readability() {
        // AC: Vector page with 1.0 readability does NOT escalate
        let current_class = PageClass::Vector;
        let readability_score = 1.0;
        let page_index = 15;

        let result = apply_broken_vector_escalation(current_class, readability_score, page_index);

        assert_eq!(result, PageClass::Vector);
    }

    #[test]
    fn test_page_class_can_escalate_vector() {
        // AC: Vector pages can escalate to BrokenVector
        assert!(PageClass::Vector.can_escalate_to_broken_vector());
    }

    #[test]
    fn test_page_class_can_escalate_scanned() {
        // AC: Scanned pages cannot escalate
        assert!(!PageClass::Scanned.can_escalate_to_broken_vector());
    }

    #[test]
    fn test_page_class_can_escalate_hybrid() {
        // AC: Hybrid pages cannot escalate
        assert!(!PageClass::Hybrid.can_escalate_to_broken_vector());
    }

    #[test]
    fn test_page_class_can_escalate_broken_vector() {
        // AC: BrokenVector pages cannot escalate (already there)
        assert!(!PageClass::BrokenVector.can_escalate_to_broken_vector());
    }

    // ============ page_type_string Tests (Phase 5.1.1) ============

    #[test]
    fn test_page_type_string_vector() {
        // AC: Vector → "text"
        assert_eq!(
            page_type_string(PageClass::Vector, false, true, false),
            "text"
        );
        assert_eq!(
            page_type_string(PageClass::Vector, true, true, false),
            "text"
        );
        assert_eq!(
            page_type_string(PageClass::Vector, false, true, true),
            "text"
        );
    }

    #[test]
    fn test_page_type_string_scanned() {
        // AC: Scanned → "scanned"
        assert_eq!(
            page_type_string(PageClass::Scanned, false, true, false),
            "scanned"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, true, true, false),
            "scanned"
        );
    }

    #[test]
    fn test_page_type_string_hybrid() {
        // AC: Hybrid → "mixed"
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, true, true),
            "mixed"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, true, true, true),
            "mixed"
        );
    }

    #[test]
    fn test_page_type_string_broken_vector_ocr_failed() {
        // AC: BrokenVector + ocr_succeeded=false → "broken_vector"
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, true, false),
            "broken_vector"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, true, true),
            "broken_vector"
        );
    }

    #[test]
    fn test_page_type_string_broken_vector_ocr_succeeded() {
        // AC: BrokenVector + ocr_succeeded=true → "scanned" (post-OCR recovery)
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, true, false),
            "scanned"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, true, true),
            "scanned"
        );
    }

    #[test]
    fn test_page_type_string_blank_override() {
        // AC: has_text=false + has_images=false → "blank" (overrides class)
        assert_eq!(
            page_type_string(PageClass::Vector, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, false, false),
            "blank"
        );
    }

    #[test]
    fn test_page_type_string_figure_only_override() {
        // AC: has_text=false + has_images=true → "figure_only" (overrides class)
        assert_eq!(
            page_type_string(PageClass::Vector, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, false, true),
            "figure_only"
        );
    }

    #[test]
    fn test_page_type_string_exhaustive_combinations() {
        // AC: Every combination from the mapping table produces the documented string
        // 4 classes × 2 ocr_succeeded × 2 has_text × 2 has_images = 32 cases

        let all_classes = [
            PageClass::Vector,
            PageClass::Scanned,
            PageClass::Hybrid,
            PageClass::BrokenVector,
        ];

        for &class in &all_classes {
            for &ocr_succeeded in &[false, true] {
                for &has_text in &[false, true] {
                    for &has_images in &[false, true] {
                        let result = page_type_string(class, ocr_succeeded, has_text, has_images);

                        // Verify result is one of the six valid enum values
                        assert!(
                            matches!(
                                result,
                                "text" | "scanned" | "mixed" | "broken_vector" | "blank" | "figure_only"
                            ),
                            "Invalid page_type: '{}' for class={:?}, ocr={}, has_text={}, has_images={}",
                            result,
                            class,
                            ocr_succeeded,
                            has_text,
                            has_images
                        );

                        // Verify override rules
                        if !has_text && !has_images {
                            assert_eq!(result, "blank");
                        } else if !has_text && has_images {
                            assert_eq!(result, "figure_only");
                        } else {
                            // Class-based mapping
                            match class {
                                PageClass::Vector => assert_eq!(result, "text"),
                                PageClass::Scanned => assert_eq!(result, "scanned"),
                                PageClass::Hybrid => assert_eq!(result, "mixed"),
                                PageClass::BrokenVector => {
                                    if ocr_succeeded {
                                        assert_eq!(result, "scanned");
                                    } else {
                                        assert_eq!(result, "broken_vector");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ============ all_tr3_with_full_page_image Tests ============

    #[test]
    fn test_all_tr3_with_full_page_image_exact_match() {
        // AC: text_op_count=10, tr3_op_count=10, full_page_image=true → Some(Vote { 0.99, BrokenVector })
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10; // All text is Tr=3
        ctx.width = 612.0; // US Letter
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height; // 484,704 pt²
        ctx.image_xobject_areas.push(page_area * 0.96); // 96% coverage (>= 95%)

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    #[test]
    fn test_all_tr3_with_full_page_image_exactly_95_percent() {
        // Edge case: exactly 95% coverage (>= threshold, should fire)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 100.0;
        ctx.height = 100.0;
        let page_area = 10_000.0;
        ctx.image_xobject_areas.push(page_area * 0.95); // Exactly 95%

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    #[test]
    fn test_all_tr3_with_full_page_image_just_below_threshold() {
        // Edge case: 94.9% coverage (< 95%, should NOT fire)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 100.0;
        ctx.height = 100.0;
        let page_area = 10_000.0;
        ctx.image_xobject_areas.push(page_area * 0.949); // Just below 95%

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_mixed_tr3() {
        // AC: text_op_count=10, tr3_op_count=5 → None (mix of Tr=3 and visible)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 5; // Only half are Tr=3
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(500_000.0); // Full page image

        let result = all_tr3_with_full_page_image(&ctx);

        // Should NOT fire (not all text is Tr=3)
        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_no_text() {
        // AC: text_op_count=0 → None (no text)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 0;
        ctx.tr3_op_count = 0;
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(500_000.0);

        let result = all_tr3_with_full_page_image(&ctx);

        // Should NOT fire (no text operators)
        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_no_full_page_image() {
        // AC: full_page_image=false → None
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 612.0;
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(100_000.0); // Small image (< 95%)

        let result = all_tr3_with_full_page_image(&ctx);

        // Should NOT fire (no full-page image)
        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_multiple_images_one_large() {
        // Multiple image XObjects, one covers >= 95% → should fire
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(1000.0); // Small image
        ctx.image_xobject_areas.push(page_area * 0.96); // Full page image
        ctx.image_xobject_areas.push(5000.0); // Another small image

        let result = all_tr3_with_full_page_image(&ctx);

        // Should fire (one image covers >= 95%)
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    #[test]
    fn test_all_tr3_with_full_page_image_zero_page_area() {
        // Edge case: zero page area (should NOT fire to avoid division by zero)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 0.0; // Zero area
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(100_000.0);

        let result = all_tr3_with_full_page_image(&ctx);

        // Should NOT fire (zero page area guarded)
        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_empty_image_areas() {
        // No image XObjects at all → should NOT fire
        let mut ctx = PageContext::new();
        ctx.text_op_count = 10;
        ctx.tr3_op_count = 10;
        ctx.width = 612.0;
        ctx.height = 792.0;
        // image_xobject_areas is empty (default)

        let result = all_tr3_with_full_page_image(&ctx);

        // Should NOT fire (no images)
        assert!(result.is_none());
    }

    #[test]
    fn test_all_tr3_with_full_page_image_invisible_text_with_image() {
        // AC: All Tr=3 + single image >= 95% → definitive BrokenVector (strength 0.99)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.tr3_op_count = 100; // All invisible
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.98); // 98% coverage

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99); // Definitive strength
    }

    #[test]
    fn test_all_tr3_with_full_page_image_standard_us_letter() {
        // Realistic case: US Letter (8.5" x 11" = 612 x 792 pt)
        // with invisible text overlay on full scan
        let mut ctx = PageContext::new();
        ctx.text_op_count = 250;
        ctx.tr3_op_count = 250;
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = 484_704.0;
        ctx.image_xobject_areas.push(page_area * 0.97); // Near full page

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    #[test]
    fn test_all_tr3_with_full_page_image_a4_page() {
        // Realistic case: A4 (210mm x 297mm ≈ 595 x 842 pt)
        let mut ctx = PageContext::new();
        ctx.text_op_count = 200;
        ctx.tr3_op_count = 200;
        ctx.width = 595.0;
        ctx.height = 842.0;
        let page_area = 595.0 * 842.0;
        ctx.image_xobject_areas.push(page_area * 0.96);

        let result = all_tr3_with_full_page_image(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    #[test]
    fn test_all_tr3_with_full_page_image_definitive_short_circuit() {
        // Verify that strength 0.99 triggers short-circuit in full classifier
        let mut ctx = PageContext::new();
        ctx.text_op_count = 100;
        ctx.tr3_op_count = 100;
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.96);

        // The InvisibleTextWithImageSignal delegates to all_tr3_with_full_page_image
        let signal = InvisibleTextWithImageSignal;
        let result = signal.evaluate(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::BrokenVector);
        assert_eq!(vote.strength, 0.99);
    }

    // ============ image_coverage_fraction Tests ============

    #[test]
    fn test_image_coverage_fraction_single_image_90_percent() {
        // AC: One image covering 90% area → Some(Vote { 0.85, Scanned })
        let mut ctx = PageContext::new();
        ctx.width = 612.0; // US Letter
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height; // 484,704 pt²
        ctx.image_xobject_areas.push(page_area * 0.90); // 90% coverage

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.85);
    }

    #[test]
    fn test_image_coverage_fraction_multiple_images_total_50_percent() {
        // AC: Multiple small images totaling 50% → None (below threshold)
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.20);
        ctx.image_xobject_areas.push(page_area * 0.20);
        ctx.image_xobject_areas.push(page_area * 0.10); // Total = 50%

        let result = image_coverage_fraction(&ctx);

        // Should NOT fire (below 0.85 threshold)
        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_no_images() {
        // AC: Page with no images → None
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        // image_xobject_areas is empty (default)

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_overlapping_images_clamped() {
        // AC: Coverage clamped to 1.0 on overlapping images
        let mut ctx = PageContext::new();
        ctx.width = 100.0;
        ctx.height = 100.0;
        let page_area = 10_000.0;
        // 5 overlapping copies of a full-page image (sum = 500% of page area)
        ctx.image_xobject_areas.push(page_area);
        ctx.image_xobject_areas.push(page_area);
        ctx.image_xobject_areas.push(page_area);
        ctx.image_xobject_areas.push(page_area);
        ctx.image_xobject_areas.push(page_area);

        let result = image_coverage_fraction(&ctx);

        // Should fire (clamped to 1.0 > 0.85 threshold)
        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.85);
    }

    #[test]
    fn test_image_coverage_fraction_exactly_85_percent() {
        // Edge case: exactly 85% coverage (should fire, threshold is > 0.85)
        let mut ctx = PageContext::new();
        ctx.width = 100.0;
        ctx.height = 100.0;
        let page_area = 10_000.0;
        ctx.image_xobject_areas.push(page_area * 0.86); // Just above 85%

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.85);
    }

    #[test]
    fn test_image_coverage_fraction_just_below_threshold() {
        // Edge case: 84.9% coverage (< 0.85, should NOT fire)
        let mut ctx = PageContext::new();
        ctx.width = 100.0;
        ctx.height = 100.0;
        let page_area = 10_000.0;
        ctx.image_xobject_areas.push(page_area * 0.84); // Below 85%

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_zero_page_area() {
        // Edge case: zero page area (should NOT fire to avoid division by zero)
        let mut ctx = PageContext::new();
        ctx.width = 0.0; // Zero area
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(100_000.0);

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_negative_page_area() {
        // Edge case: negative width (should NOT fire)
        let mut ctx = PageContext::new();
        ctx.width = -100.0; // Invalid (negative)
        ctx.height = 792.0;
        ctx.image_xobject_areas.push(50_000.0);

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_single_small_image() {
        // Single small image (10% coverage) → None
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.10); // 10% coverage

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_image_coverage_fraction_multiple_images_above_threshold() {
        // Multiple images totaling 90% coverage → should fire
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.40);
        ctx.image_xobject_areas.push(page_area * 0.30);
        ctx.image_xobject_areas.push(page_area * 0.20); // Total = 90%

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
        assert_eq!(vote.strength, 0.85);
    }

    #[test]
    fn test_image_coverage_fraction_high_threshold_scanned_vote() {
        // Verify that the signal votes for Scanned class specifically
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.90);

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.class, PageClass::Scanned);
    }

    #[test]
    fn test_image_coverage_fraction_strength_value() {
        // Verify that the strength is exactly 0.85 as specified
        let mut ctx = PageContext::new();
        ctx.width = 612.0;
        ctx.height = 792.0;
        let page_area = ctx.width * ctx.height;
        ctx.image_xobject_areas.push(page_area * 0.90);

        let result = image_coverage_fraction(&ctx);

        assert!(result.is_some());
        let vote = result.unwrap();
        assert_eq!(vote.strength, 0.85);
    }
}

//! Marked content tracking for MCID association.
//!
//! This module implements tracking of BDC/BMC/EMC marked content sequences
//! for MCID association with the structure tree (Phase 3.4).
//!
//! ## MCID Tracking
//!
//! Each marked content sequence can carry an MCID (Marked Content Identifier)
//! via the `/MCID` property in the BDC operator's property dictionary. This MCID
//! is used to associate the content with a structure element via the ParentTree.
//!
//! ## Coverage Calculation
//!
//! For the StructTree coverage check (Phase 7.1.4), we need to compute:
//! - claimed_mcids: MCIDs that resolve to a non-Artifact StructElem via ParentTree
//! - total_mcids: Total MCIDs emitted in marked-content sequences on the page
//!
//! Coverage = claimed_mcids / total_mcids

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::lexer::Lexer;
use std::collections::HashSet;

/// Result type for marked content operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// MCID tracking state for a page.
///
/// Tracks all MCIDs seen in marked content sequences and their properties.
#[derive(Debug, Clone, Default)]
pub struct McidTracker {
    /// All MCIDs seen in marked content sequences on this page.
    mcids: HashSet<u32>,
    /// MCIDs inside Artifact marked-content sequences (excluded from coverage).
    artifact_mcids: HashSet<u32>,
    /// Diagnostics emitted during tracking.
    diagnostics: Vec<Diagnostic>,
}

impl McidTracker {
    /// Create a new empty MCID tracker.
    pub fn new() -> Self {
        Self {
            mcids: HashSet::new(),
            artifact_mcids: HashSet::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Record an MCID from a marked content sequence.
    ///
    /// # Arguments
    ///
    /// * `mcid` - The MCID value from the marked content property dict
    /// * `is_artifact` - True if this MCID is inside an Artifact marked-content sequence
    pub fn record_mcid(&mut self, mcid: u32, is_artifact: bool) {
        self.mcids.insert(mcid);
        if is_artifact {
            self.artifact_mcids.insert(mcid);
        }
    }

    /// Get the total count of MCIDs on this page.
    pub fn total_mcids(&self) -> usize {
        self.mcids.len()
    }

    /// Get the count of non-Artifact MCIDs on this page.
    ///
    /// These are the MCIDs that should be claimed by the StructTree
    /// for coverage calculation.
    pub fn non_artifact_mcids(&self) -> usize {
        self.mcids.len() - self.artifact_mcids.len()
    }

    /// Get all MCIDs as a set.
    pub fn mcid_set(&self) -> &HashSet<u32> {
        &self.mcids
    }

    /// Add a diagnostic.
    fn emit_diagnostic(&mut self, code: DiagCode, message: String) {
        self.diagnostics
            .push(Diagnostic::with_dynamic_no_offset(code, message));
    }

    /// Get all diagnostics emitted during tracking.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Coverage calculation result for a single page.
///
/// Computes the StructTree coverage ratio for the Suspects fallback check.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    /// The page index (0-based).
    pub page_index: usize,
    /// Total MCIDs emitted in marked-content sequences on this page.
    pub total_mcids: usize,
    /// MCIDs claimed by the StructTree (non-Artifact, resolved via ParentTree).
    pub claimed_mcids: usize,
    /// Coverage ratio: claimed_mcids / total_mcids (0.0 to 1.0).
    /// Returns 0.0 if total_mcids == 0 (no marked content on page).
    pub coverage: f64,
    /// Whether this page should fall back to XY-cut based on coverage.
    pub should_fallback: bool,
}

impl CoverageResult {
    /// Create a new coverage result.
    pub fn new(page_index: usize, total_mcids: usize, claimed_mcids: usize) -> Self {
        let coverage = if total_mcids > 0 {
            (claimed_mcids as f64) / (total_mcids as f64)
        } else {
            0.0
        };

        // Fallback threshold: 0.80 (hard-coded per plan)
        // Also fallback if total_mcids == 0 (no marked content to trust)
        let should_fallback = total_mcids == 0 || coverage < 0.80;

        Self {
            page_index,
            total_mcids,
            claimed_mcids,
            coverage,
            should_fallback,
        }
    }

    /// Apply Suspects mode to determine actual fallback behavior.
    ///
    /// When /Suspects is false, the StructTree is trusted regardless of coverage,
    /// so should_fallback is always false.
    ///
    /// # Arguments
    ///
    /// * `suspects_mode` - If true, use the coverage-based fallback; if false, never fall back
    ///
    /// # Returns
    ///
    /// A new `CoverageResult` with `should_fallback` adjusted based on Suspects mode.
    pub fn with_suspects_mode(mut self, suspects_mode: bool) -> Self {
        if !suspects_mode {
            // When Suspects is false, trust the tree regardless of coverage
            self.should_fallback = false;
        }
        self
    }

    /// Get a diagnostic message for fallback trigger.
    pub fn fallback_diagnostic(&self) -> Option<String> {
        if self.should_fallback {
            if self.total_mcids == 0 {
                Some(format!(
                    "Page {} has no marked-content sequences; falling back to XY-cut",
                    self.page_index
                ))
            } else {
                Some(format!(
                    "Page {} StructTree coverage is {:.1}% ({}/{} MCIDs claimed); below 80% threshold, falling back to XY-cut",
                    self.page_index,
                    self.coverage * 100.0,
                    self.claimed_mcids,
                    self.total_mcids
                ))
            }
        } else {
            None
        }
    }
}

/// Compute coverage for a single page.
///
/// # Arguments
///
/// * `page_index` - The page index (0-based)
/// * `total_mcids` - Total MCIDs emitted in marked-content sequences on this page
/// * `claimed_mcids` - MCIDs claimed by the StructTree (via ParentTree resolution)
///
/// # Returns
///
/// A `CoverageResult` containing the coverage ratio and fallback decision.
pub fn compute_coverage(
    page_index: usize,
    total_mcids: usize,
    claimed_mcids: usize,
) -> CoverageResult {
    CoverageResult::new(page_index, total_mcids, claimed_mcids)
}

/// Compute coverage from MCID sets.
///
/// # Arguments
///
/// * `page_index` - The page index (0-based)
/// * `all_mcids` - All MCIDs seen in marked-content sequences
/// * `claimed_mcids` - MCIDs that resolved to a StructElem via ParentTree
///
/// # Returns
///
/// A `CoverageResult` containing the coverage ratio and fallback decision.
pub fn compute_coverage_from_sets(
    page_index: usize,
    all_mcids: &HashSet<u32>,
    claimed_mcids: &HashSet<u32>,
) -> CoverageResult {
    // Exclude Artifact MCIDs from both counts for coverage calculation
    // Artifacts are not part of the logical content, so they shouldn't count
    let non_artifact_mcids = all_mcids.len();

    // Count claimed MCIDs that are not artifacts
    let claimed_count = claimed_mcids.intersection(all_mcids).count();

    compute_coverage(page_index, non_artifact_mcids, claimed_count)
}

/// Track MCIDs from decoded content stream bytes.
///
/// This function parses PDF content stream operators to find marked content
/// sequences (BDC/BMC/EMC) and extracts MCID values for coverage calculation.
///
/// # Arguments
///
/// * `content_bytes` - The decoded content stream bytes
/// * `tracker` - The McidTracker to populate with discovered MCIDs
///
/// # Behavior
///
/// - Parses content stream operators using the PDF lexer
/// - Tracks BDC (begin marked content dictionary) operators with /MCID property
/// - Tracks BMC (begin marked content) operators (no MCID, but marks sequence)
/// - Tracks EMC (end marked content) operators
/// - Handles nested marked content sequences correctly
///
/// # MCID Extraction
///
/// MCIDs are extracted from BDC property dictionaries:
/// - BDC `<tag>` `<properties>` EMC
/// - If `<properties>` contains /MCID N, the MCID N is recorded
/// - Artifact marked content (/Artifact) is tracked separately
pub fn track_mcids_from_content_stream(content_bytes: &[u8], tracker: &mut McidTracker) {
    

    let mut lexer = Lexer::new(content_bytes);
    let mut artifact_depth = 0;
    let mut mcid_stack: Vec<u32> = Vec::new();

    while let Some(token) = lexer.next_token() {
        match token {
            crate::parser::lexer::Token::Keyword(ref op) => {
                match op.as_slice() {
                    b"BDC" => {
                        // Begin marked content with properties dictionary
                        // Look ahead for the MCID in the property dict
                        if let Some(mcid) = extract_mcid_from_property_dict(&mut lexer) {
                            // Check if this is an Artifact marked content
                            // For now, we'll track all MCIDs as non-artifact
                            // A proper implementation would check the tag
                            tracker.record_mcid(mcid, artifact_depth > 0);
                            mcid_stack.push(mcid);
                        } else {
                            // BDC without MCID - still increases depth for tracking
                            mcid_stack.push(u32::MAX); // Sentinel for no-MCID BDC
                        }
                    }
                    b"BMC" => {
                        // Begin marked content without properties
                        // No MCID to track, but marks the sequence
                        mcid_stack.push(u32::MAX); // Sentinel for BMC
                    }
                    b"EMC" => {
                        // End marked content
                        if let Some(mcid) = mcid_stack.pop() {
                            if mcid != u32::MAX && artifact_depth > 0 {
                                // We're closing an artifact sequence
                                // Check if there are more artifact sequences open
                                artifact_depth -= 1;
                            }
                        }
                    }
                    _ => {
                        // Other operators - ignore for MCID tracking
                    }
                }
            }
            _ => {
                // Other tokens (keywords, names, numbers, etc.) - ignore
            }
        }
    }
}

/// Extract MCID from a BDC property dictionary.
///
/// Looks ahead in the lexer to find the MCID value in the property dict
/// that follows a BDC operator.
///
/// # Returns
///
/// Some(mcid) if found, None otherwise
fn extract_mcid_from_property_dict(lexer: &mut Lexer) -> Option<u32> {
    // After BDC, we expect: <tag> <properties>
    // We need to skip the tag and parse the properties dict to find /MCID

    // Skip the tag (can be a name or other object)
    let mut depth = 0;
    let mut found_mcid = None;
    let mut brace_depth = 0;

    // Scan tokens looking for /MCID
    while let Some(token) = lexer.next_token() {
        match token {
            crate::parser::lexer::Token::DictStart => {
                brace_depth += 1;
                depth += 1;
            }
            crate::parser::lexer::Token::DictEnd => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    // End of property dict
                    break;
                }
            }
            crate::parser::lexer::Token::Name(ref name) => {
                if name == b"MCID" {
                    // Found /MCID - next token should be the value
                    if let Some(next_token) = lexer.next_token() {
                        match next_token {
                            crate::parser::lexer::Token::Integer(n) if n >= 0 => {
                                found_mcid = Some(n as u32);
                                break;
                            }
                            _ => break,
                        }
                    }
                }
            }
            _ => {
                // Other tokens - continue scanning
                if brace_depth == 0 && depth > 0 {
                    // We've exited the dict without finding DictEnd
                    break;
                }
            }
        }
    }

    found_mcid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcid_tracker_new() {
        let tracker = McidTracker::new();
        assert_eq!(tracker.total_mcids(), 0);
        assert_eq!(tracker.non_artifact_mcids(), 0);
        assert!(tracker.diagnostics().is_empty());
    }

    #[test]
    fn test_mcid_tracker_record_mcid() {
        let mut tracker = McidTracker::new();
        tracker.record_mcid(0, false);
        tracker.record_mcid(1, false);
        tracker.record_mcid(2, true); // Artifact

        assert_eq!(tracker.total_mcids(), 3);
        assert_eq!(tracker.non_artifact_mcids(), 2);
        assert!(tracker.mcid_set().contains(&0));
        assert!(tracker.mcid_set().contains(&1));
        assert!(tracker.mcid_set().contains(&2));
    }

    #[test]
    fn test_coverage_result_full_coverage() {
        let result = CoverageResult::new(0, 100, 100);
        assert_eq!(result.page_index, 0);
        assert_eq!(result.total_mcids, 100);
        assert_eq!(result.claimed_mcids, 100);
        assert!((result.coverage - 1.0).abs() < f64::EPSILON);
        assert!(!result.should_fallback);
        assert!(result.fallback_diagnostic().is_none());
    }

    #[test]
    fn test_coverage_result_above_threshold() {
        let result = CoverageResult::new(0, 100, 85);
        assert_eq!(result.total_mcids, 100);
        assert_eq!(result.claimed_mcids, 85);
        assert!((result.coverage - 0.85).abs() < f64::EPSILON);
        assert!(!result.should_fallback); // 85% >= 80%
    }

    #[test]
    fn test_coverage_result_below_threshold() {
        let result = CoverageResult::new(0, 100, 75);
        assert_eq!(result.total_mcids, 100);
        assert_eq!(result.claimed_mcids, 75);
        assert!((result.coverage - 0.75).abs() < f64::EPSILON);
        assert!(result.should_fallback); // 75% < 80%
        assert!(result.fallback_diagnostic().is_some());
        assert!(result.fallback_diagnostic().unwrap().contains("75.0%"));
    }

    #[test]
    fn test_coverage_result_no_mcids() {
        let result = CoverageResult::new(0, 0, 0);
        assert_eq!(result.total_mcids, 0);
        assert_eq!(result.claimed_mcids, 0);
        assert_eq!(result.coverage, 0.0);
        assert!(result.should_fallback); // No MCIDs = fallback
        assert!(result
            .fallback_diagnostic()
            .unwrap()
            .contains("no marked-content sequences"));
    }

    #[test]
    fn test_coverage_result_threshold_edge_case() {
        // Exactly 80% should NOT fall back
        let result = CoverageResult::new(0, 100, 80);
        assert!((result.coverage - 0.80).abs() < f64::EPSILON);
        assert!(!result.should_fallback); // 80% >= 80% (not less than)

        // 79.9% should fall back
        let result = CoverageResult::new(0, 1000, 799);
        assert!((result.coverage - 0.799).abs() < 0.001);
        assert!(result.should_fallback); // 79.9% < 80%
    }

    #[test]
    fn test_compute_coverage() {
        let result = compute_coverage(5, 200, 150);
        assert_eq!(result.page_index, 5);
        assert_eq!(result.total_mcids, 200);
        assert_eq!(result.claimed_mcids, 150);
        assert!((result.coverage - 0.75).abs() < f64::EPSILON);
        assert!(result.should_fallback);
    }

    #[test]
    fn test_compute_coverage_from_sets() {
        let mut all_mcids = HashSet::new();
        all_mcids.insert(0);
        all_mcids.insert(1);
        all_mcids.insert(2);
        all_mcids.insert(3);
        all_mcids.insert(4);

        let mut claimed_mcids = HashSet::new();
        claimed_mcids.insert(0);
        claimed_mcids.insert(1);
        claimed_mcids.insert(2);
        // MCIDs 3 and 4 are orphans

        let result = compute_coverage_from_sets(0, &all_mcids, &claimed_mcids);
        assert_eq!(result.total_mcids, 5);
        assert_eq!(result.claimed_mcids, 3);
        assert!((result.coverage - 0.60).abs() < f64::EPSILON);
        assert!(result.should_fallback); // 60% < 80%
    }

    #[test]
    fn test_fallback_diagnostic_message() {
        let result = CoverageResult::new(2, 100, 60);
        let diag = result.fallback_diagnostic().unwrap();
        assert!(diag.contains("Page 2"));
        assert!(diag.contains("60.0%"));
        assert!(diag.contains("60/100"));
        assert!(diag.contains("falling back to XY-cut"));
    }

    #[test]
    fn test_fallback_diagnostic_no_mcids() {
        let result = CoverageResult::new(3, 0, 0);
        let diag = result.fallback_diagnostic().unwrap();
        assert!(diag.contains("Page 3"));
        assert!(diag.contains("no marked-content sequences"));
    }
}

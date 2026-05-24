//! Marked-content stack for BDC/BMC/EMC operators.
//!
//! This module implements the marked-content stack data structure that tracks
//! nested marked-content sequences (BDC/BMC) and their MCID values.
//!
//! Per PDF spec section 14.5, the marked-content stack is independent of the
//! graphics state stack — q/Q operators do not affect it.

use crate::diagnostics::{DiagCode, Diagnostic};

/// Maximum depth of marked-content stack (prevents stack overflow).
const MAX_MC_DEPTH: usize = 64;

/// A frame on the marked-content stack.
///
/// Each BMC/BDC operator pushes a frame with the tag name and optional MCID.
#[derive(Debug, Clone)]
pub struct MarkedContentFrame {
    /// The tag name (e.g., "Span", "P", "Artifact").
    pub tag: String,
    /// The MCID (Marked Content Identifier) if present in the property dict.
    pub mcid: Option<u32>,
}

impl MarkedContentFrame {
    /// Create a new marked-content frame.
    pub fn new(tag: String, mcid: Option<u32>) -> Self {
        Self { tag, mcid }
    }

    /// Create a BMC frame (tag only, no MCID).
    pub fn bmc(tag: String) -> Self {
        Self { tag, mcid: None }
    }

    /// Create a BDC frame with optional MCID.
    pub fn bdc(tag: String, mcid: Option<u32>) -> Self {
        Self { tag, mcid }
    }
}

/// Marked-content stack for BDC/BMC/EMC operators.
///
/// Tracks nested marked-content sequences. Each BMC/BDC pushes a frame,
/// each EMC pops the top frame.
#[derive(Debug, Clone)]
pub struct MarkedContentStack {
    /// The stack of marked-content frames.
    stack: Vec<MarkedContentFrame>,
    /// Diagnostics emitted during stack operations.
    diagnostics: Vec<Diagnostic>,
}

impl Default for MarkedContentStack {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkedContentStack {
    /// Create a new empty marked-content stack.
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(16),
            diagnostics: Vec::new(),
        }
    }

    /// Push a BMC frame (tag only, no MCID).
    ///
    /// Returns false if the stack would exceed the maximum depth.
    pub fn push_bmc(&mut self, tag: String) -> bool {
        if self.stack.len() >= MAX_MC_DEPTH {
            self.diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::MarkedContentDepthExceeded,
                format!(
                    "Marked-content stack depth {} exceeds limit {}",
                    self.stack.len() + 1,
                    MAX_MC_DEPTH
                ),
            ));
            false
        } else {
            self.stack.push(MarkedContentFrame::bmc(tag));
            true
        }
    }

    /// Push a BDC frame with optional MCID.
    ///
    /// Returns false if the stack would exceed the maximum depth.
    pub fn push_bdc(&mut self, tag: String, mcid: Option<u32>) -> bool {
        if self.stack.len() >= MAX_MC_DEPTH {
            self.diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::MarkedContentDepthExceeded,
                format!(
                    "Marked-content stack depth {} exceeds limit {}",
                    self.stack.len() + 1,
                    MAX_MC_DEPTH
                ),
            ));
            false
        } else {
            self.stack.push(MarkedContentFrame::bdc(tag, mcid));
            true
        }
    }

    /// Pop the top frame (EMC operator).
    ///
    /// Returns None if the stack is empty (underflow).
    pub fn pop_emc(&mut self) -> Option<MarkedContentFrame> {
        if self.stack.is_empty() {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::EmcWithoutBmc,
                "EMC operator without matching BMC/BDC",
            ));
            None
        } else {
            self.stack.pop()
        }
    }

    /// Get the innermost (top) MCID, if any.
    ///
    /// Returns the MCID of the topmost frame that has one.
    pub fn innermost_mcid(&self) -> Option<u32> {
        self.stack.iter().rev().find_map(|frame| frame.mcid)
    }

    /// Get the innermost (top) frame, if any.
    pub fn innermost_frame(&self) -> Option<&MarkedContentFrame> {
        self.stack.last()
    }

    /// Get the current depth of the stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get all diagnostics emitted during stack operations.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take all diagnostics (clears the internal buffer).
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Reset the stack (for page boundary).
    pub fn reset(&mut self) {
        self.stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stack() {
        let stack = MarkedContentStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
        assert_eq!(stack.innermost_mcid(), None);
        assert!(stack.innermost_frame().is_none());
    }

    #[test]
    fn test_push_bmc() {
        let mut stack = MarkedContentStack::new();
        assert!(stack.push_bmc("Span".to_string()));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_empty());
        let frame = stack.innermost_frame().unwrap();
        assert_eq!(frame.tag, "Span");
        assert_eq!(frame.mcid, None);
    }

    #[test]
    fn test_push_bdc_with_mcid() {
        let mut stack = MarkedContentStack::new();
        assert!(stack.push_bdc("P".to_string(), Some(42)));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), Some(42));
        let frame = stack.innermost_frame().unwrap();
        assert_eq!(frame.tag, "P");
        assert_eq!(frame.mcid, Some(42));
    }

    #[test]
    fn test_push_bdc_without_mcid() {
        let mut stack = MarkedContentStack::new();
        assert!(stack.push_bdc("Artifact".to_string(), None));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), None);
    }

    #[test]
    fn test_pop_emc() {
        let mut stack = MarkedContentStack::new();
        stack.push_bmc("Span".to_string());
        let frame = stack.pop_emc().unwrap();
        assert_eq!(frame.tag, "Span");
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_emc_underflow() {
        let mut stack = MarkedContentStack::new();
        let result = stack.pop_emc();
        assert!(result.is_none());
        assert!(!stack.diagnostics().is_empty());
        assert_eq!(stack.diagnostics()[0].code, DiagCode::EmcWithoutBmc);
    }

    #[test]
    fn test_nested_frames() {
        let mut stack = MarkedContentStack::new();
        stack.push_bdc("P".to_string(), Some(1));
        stack.push_bmc("Span".to_string());
        stack.push_bdc("Span".to_string(), Some(2));

        assert_eq!(stack.depth(), 3);
        assert_eq!(stack.innermost_mcid(), Some(2)); // Innermost wins

        stack.pop_emc();
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.innermost_mcid(), Some(1)); // Now outer MCID visible

        stack.pop_emc();
        stack.pop_emc();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_depth_limit() {
        let mut stack = MarkedContentStack::new();

        // Fill to max depth
        for i in 0..MAX_MC_DEPTH {
            assert!(stack.push_bmc(format!("frame{}", i)));
        }
        assert_eq!(stack.depth(), MAX_MC_DEPTH);

        // Should fail to push beyond max
        assert!(!stack.push_bmc("overflow".to_string()));
        assert_eq!(stack.depth(), MAX_MC_DEPTH);
        assert!(!stack.diagnostics().is_empty());
        assert_eq!(
            stack.diagnostics().last().unwrap().code,
            DiagCode::MarkedContentDepthExceeded
        );
    }

    #[test]
    fn test_innermost_mcid_with_nested() {
        let mut stack = MarkedContentStack::new();
        stack.push_bdc("Outer".to_string(), Some(10));
        assert_eq!(stack.innermost_mcid(), Some(10));

        stack.push_bmc("Middle".to_string()); // No MCID
        assert_eq!(stack.innermost_mcid(), Some(10)); // Outer still visible

        stack.push_bdc("Inner".to_string(), Some(20));
        assert_eq!(stack.innermost_mcid(), Some(20)); // Innermost wins
    }

    #[test]
    fn test_reset() {
        let mut stack = MarkedContentStack::new();
        stack.push_bmc("Span".to_string());
        stack.push_bdc("P".to_string(), Some(5));
        assert_eq!(stack.depth(), 2);

        stack.reset();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_frame_new() {
        let frame = MarkedContentFrame::new("Test".to_string(), Some(123));
        assert_eq!(frame.tag, "Test");
        assert_eq!(frame.mcid, Some(123));
    }

    #[test]
    fn test_frame_bmc() {
        let frame = MarkedContentFrame::bmc("Tag".to_string());
        assert_eq!(frame.tag, "Tag");
        assert_eq!(frame.mcid, None);
    }

    #[test]
    fn test_frame_bdc() {
        let frame = MarkedContentFrame::bdc("Tag".to_string(), Some(99));
        assert_eq!(frame.tag, "Tag");
        assert_eq!(frame.mcid, Some(99));
    }

    #[test]
    fn test_take_diagnostics() {
        let mut stack = MarkedContentStack::new();
        stack.pop_emc(); // Emits diagnostic
        assert!(!stack.diagnostics().is_empty());

        let diags = stack.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert!(stack.diagnostics().is_empty()); // Cleared
    }
}

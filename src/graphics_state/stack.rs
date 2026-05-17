//! Graphics state stack for q/Q operators.
//!
//! Implements the PDF graphics state stack with a maximum depth of 64
/// as specified in the PDF specification.

use super::state::GraphicsState;

const MAX_STACK_DEPTH: usize = 64;

/// Graphics state stack for q/Q operators.
///
/// PDF specifies a maximum depth of 64. Pushing beyond this limit
/// emits a diagnostic and discards the push (safe failure).
#[derive(Debug, Clone)]
pub struct GraphicsStateStack {
    stack: Vec<GraphicsState>,
    diagnostics: Vec<String>,
}

impl GraphicsStateStack {
    /// Create a new empty stack.
    pub fn new() -> Self {
        GraphicsStateStack {
            stack: Vec::with_capacity(MAX_STACK_DEPTH),
            diagnostics: Vec::new(),
        }
    }

    /// Push a copy of the current state onto the stack (q operator).
    ///
    /// Returns true if the push succeeded, false if the stack is full.
    /// When the stack is full (depth 64), a diagnostic is emitted.
    pub fn push(&mut self, state: &GraphicsState) -> bool {
        if self.stack.len() >= MAX_STACK_DEPTH {
            self.diagnostics
                .push("GSTATE_STACK_OVERFLOW".to_string());
            return false;
        }
        self.stack.push(state.clone());
        true
    }

    /// Pop and return the previous state (Q operator).
    ///
    /// Returns None if the stack is empty (should not happen in valid PDFs).
    pub fn pop(&mut self) -> Option<GraphicsState> {
        self.stack.pop()
    }

    /// Get the current depth of the stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Take all diagnostics emitted so far.
    pub fn take_diagnostics(&mut self) -> Vec<String> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Clear all diagnostics.
    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }
}

impl Default for GraphicsStateStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stack() {
        let mut stack = GraphicsStateStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_push_pop() {
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        assert!(stack.push(&state));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_empty());

        let popped = stack.pop();
        assert!(popped.is_some());
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_depth_64() {
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        // Push 64 times - should all succeed
        for i in 0..64 {
            assert!(stack.push(&state), "Push {} failed", i);
            assert_eq!(stack.depth(), i + 1);
        }

        // 65th push should fail
        assert!(!stack.push(&state));
        assert_eq!(stack.depth(), 64);

        // Check diagnostic was emitted
        let diags = stack.take_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0], "GSTATE_STACK_OVERFLOW");
    }

    #[test]
    fn test_state_clone_on_push() {
        let mut stack = GraphicsStateStack::new();
        let mut state = GraphicsState::new();
        state.set_char_spacing(5.0);

        stack.push(&state);

        // Modify original state
        state.set_char_spacing(10.0);

        // Popped state should have original value
        let popped = stack.pop().unwrap();
        assert_eq!(popped.char_spacing, 5.0);
    }

    #[test]
    fn test_multiple_diagnostics() {
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        // Fill the stack
        for _ in 0..64 {
            stack.push(&state);
        }

        // Try to push multiple times
        for _ in 0..3 {
            stack.push(&state);
        }

        let diags = stack.take_diagnostics();
        assert_eq!(diags.len(), 3);
        assert!(diags.iter().all(|d| d == "GSTATE_STACK_OVERFLOW"));
    }

    #[test]
    fn test_clear_diagnostics() {
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        // Fill and overflow
        for _ in 0..65 {
            stack.push(&state);
        }

        assert!(!stack.diagnostics.is_empty());
        stack.clear_diagnostics();
        assert!(stack.diagnostics.is_empty());
    }

    #[test]
    fn test_nested_q_q() {
        let mut stack = GraphicsStateStack::new();
        let mut state = GraphicsState::new();

        // q
        state.set_char_spacing(1.0);
        stack.push(&state);

        // q
        state.set_char_spacing(2.0);
        stack.push(&state);

        assert_eq!(stack.depth(), 2);

        // Q
        let popped = stack.pop().unwrap();
        assert_eq!(popped.char_spacing, 2.0);

        // Q
        let popped = stack.pop().unwrap();
        assert_eq!(popped.char_spacing, 1.0);
    }
}

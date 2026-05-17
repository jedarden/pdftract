//! Diagnostic messages emitted during PDF processing.

/// Diagnostic message emitted during PDF processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    GraphicsStateStackOverflow,
}

impl Diagnostic {
    pub fn severity(&self) -> Severity {
        match self {
            Diagnostic::GraphicsStateStackOverflow => Severity::Warning,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Diagnostic::GraphicsStateStackOverflow => "GSTATE_STACK_OVERFLOW",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Diagnostic::GraphicsStateStackOverflow => {
                "Graphics state stack depth exceeded limit of 64".to_string()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

//! Diagnostic messages emitted during PDF processing.

/// Diagnostic message emitted during PDF processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    GraphicsStateStackOverflow,
    /// Stream bomb: decompressed bytes exceeded max_decompress_bytes limit
    StreamBomb { bytes: u64, limit: u64 },
    /// Unknown filter name in /Filter array
    StructUnknownFilter { filter: String },
    /// /DecodeParms array length doesn't match /Filter array length
    StructInvalidFilterParams { filter_len: usize, params_len: usize },
    /// Stream decoding error mid-stream (corrupt data, truncated)
    StreamDecodeError { filter: String, details: String },
}

impl Diagnostic {
    pub fn severity(&self) -> Severity {
        match self {
            Diagnostic::GraphicsStateStackOverflow => Severity::Warning,
            Diagnostic::StreamBomb { .. } => Severity::Error,
            Diagnostic::StructUnknownFilter { .. } => Severity::Warning,
            Diagnostic::StructInvalidFilterParams { .. } => Severity::Warning,
            Diagnostic::StreamDecodeError { .. } => Severity::Warning,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Diagnostic::GraphicsStateStackOverflow => "GSTATE_STACK_OVERFLOW",
            Diagnostic::StreamBomb { .. } => "STREAM_BOMB",
            Diagnostic::StructUnknownFilter { .. } => "STRUCT_UNKNOWN_FILTER",
            Diagnostic::StructInvalidFilterParams { .. } => "STRUCT_INVALID_FILTER_PARAMS",
            Diagnostic::StreamDecodeError { .. } => "STREAM_DECODE_ERROR",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Diagnostic::GraphicsStateStackOverflow => {
                "Graphics state stack depth exceeded limit of 64".to_string()
            }
            Diagnostic::StreamBomb { bytes, limit } => {
                format!(
                    "Decompressed bytes ({}) exceeded max_decompress_bytes limit ({}); partial data returned",
                    bytes, limit
                )
            }
            Diagnostic::StructUnknownFilter { filter } => {
                format!("Unknown filter '{}'; raw bytes passed through", filter)
            }
            Diagnostic::StructInvalidFilterParams { filter_len, params_len } => {
                format!(
                    "/Filter array has {} entries but /DecodeParms has {} entries; using defaults for missing params",
                    filter_len, params_len
                )
            }
            Diagnostic::StreamDecodeError { filter, details } => {
                format!("Error decoding {} filter: {}; partial data returned", filter, details)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

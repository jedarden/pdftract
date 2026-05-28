//! Multi-output specification types.
//!
//! This module defines the types for specifying multiple output formats
//! and destinations for a single extraction pass.

use std::path::PathBuf;

/// Output format type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Full JSON document format.
    Json,
    /// CommonMark Markdown format.
    Markdown,
    /// Plain text format.
    Text,
    /// Newline-delimited JSON streaming format.
    Ndjson,
}

impl Format {
    /// Returns the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Markdown => "md",
            Format::Text => "txt",
            Format::Ndjson => "ndjson",
        }
    }

    /// Returns the content-type MIME for this format.
    pub fn content_type(&self) -> &'static str {
        match self {
            Format::Json => "application/json",
            Format::Markdown => "text/markdown; charset=UTF-8",
            Format::Text => "text/plain; charset=UTF-8",
            Format::Ndjson => "application/x-ndjson",
        }
    }

    /// Parses a format name string.
    ///
    /// Accepts: "json", "markdown" (or "md"), "text", "ndjson"
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "json" => Some(Format::Json),
            "markdown" | "md" => Some(Format::Markdown),
            "text" | "txt" => Some(Format::Text),
            "ndjson" => Some(Format::Ndjson),
            _ => None,
        }
    }

    /// Returns true if this format can be combined with others in multi-output.
    ///
    /// NDJSON is mutually exclusive with other formats because it streams
    /// page-by-page rather than emitting a whole document.
    pub fn is_combinable(&self) -> bool {
        !matches!(self, Format::Ndjson)
    }
}

/// Output destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Write to a file at the given path.
    File(PathBuf),
    /// Write to stdout.
    Stdout,
}

impl Destination {
    /// Returns true if this destination is stdout.
    pub fn is_stdout(&self) -> bool {
        matches!(self, Destination::Stdout)
    }
}

/// A single output specification combining format and destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpec {
    /// The output format.
    pub format: Format,
    /// The output destination.
    pub dest: Destination,
}

impl OutputSpec {
    /// Creates a new OutputSpec for writing to a file.
    pub fn file(format: Format, path: PathBuf) -> Self {
        OutputSpec {
            format,
            dest: Destination::File(path),
        }
    }

    /// Creates a new OutputSpec for writing to stdout.
    pub fn stdout(format: Format) -> Self {
        OutputSpec {
            format,
            dest: Destination::Stdout,
        }
    }

    /// Returns true if this spec writes to stdout.
    pub fn is_stdout(&self) -> bool {
        self.dest.is_stdout()
    }

    /// Returns the file path if this spec writes to a file.
    pub fn file_path(&self) -> Option<&PathBuf> {
        match &self.dest {
            Destination::File(path) => Some(path),
            Destination::Stdout => None,
        }
    }

    /// Creates an OutputSpec from a base path and format by appending the format's extension.
    pub fn from_base(base: PathBuf, format: Format) -> Self {
        let mut path = base.clone();
        let ext = format.extension();
        if !path.as_os_str().to_str().unwrap_or("").ends_with(&format!(".{}", ext)) {
            path.set_extension(ext);
        }
        OutputSpec::file(format, path)
    }
}

/// Validation errors for multi-output specifications.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// Duplicate format flag specified.
    #[error("duplicate --{0} flag (each format may be specified at most once)")]
    DuplicateFormat(String),

    /// At most one output may go to stdout.
    #[error("at most one output may use '-' for stdout")]
    MultipleStdout,

    /// NDJSON cannot be combined with other formats.
    #[error("--ndjson cannot be combined with other output formats")]
    NdjsonExclusive,

    /// --format requires -o to specify the output base path.
    #[error("--format requires -o to specify the output base path")]
    FormatRequiresOutput,

    /// Invalid format name in --format list.
    #[error("invalid format '{0}' in --format list (valid: json, markdown, text, ndjson)")]
    InvalidFormat(String),
}

/// Validates a list of output specifications.
///
/// # Errors
///
/// Returns `Err` if any validation rule is violated:
/// - More than one spec writes to stdout
/// - NDJSON is combined with other formats
pub fn validate_outputs(specs: &[OutputSpec]) -> Result<(), ValidationError> {
    // Check for multiple stdout destinations
    let stdout_count = specs.iter().filter(|s| s.is_stdout()).count();
    if stdout_count > 1 {
        return Err(ValidationError::MultipleStdout);
    }

    // Check for NDJSON combined with other formats
    let has_ndjson = specs.iter().any(|s| s.format == Format::Ndjson);
    if has_ndjson && specs.len() > 1 {
        return Err(ValidationError::NdjsonExclusive);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_extension() {
        assert_eq!(Format::Json.extension(), "json");
        assert_eq!(Format::Markdown.extension(), "md");
        assert_eq!(Format::Text.extension(), "txt");
        assert_eq!(Format::Ndjson.extension(), "ndjson");
    }

    #[test]
    fn test_format_from_name() {
        assert_eq!(Format::from_name("json"), Some(Format::Json));
        assert_eq!(Format::from_name("JSON"), Some(Format::Json));
        assert_eq!(Format::from_name("markdown"), Some(Format::Markdown));
        assert_eq!(Format::from_name("md"), Some(Format::Markdown));
        assert_eq!(Format::from_name("text"), Some(Format::Text));
        assert_eq!(Format::from_name("ndjson"), Some(Format::Ndjson));
        assert_eq!(Format::from_name("invalid"), None);
    }

    #[test]
    fn test_output_spec_from_base() {
        let spec = OutputSpec::from_base(PathBuf::from("out"), Format::Json);
        assert_eq!(spec.file_path(), Some(&PathBuf::from("out.json")));

        let spec = OutputSpec::from_base(PathBuf::from("out"), Format::Markdown);
        assert_eq!(spec.file_path(), Some(&PathBuf::from("out.md")));

        let spec = OutputSpec::from_base(PathBuf::from("out.json"), Format::Json);
        // Already has .json extension, shouldn't double it
        assert_eq!(spec.file_path(), Some(&PathBuf::from("out.json")));
    }

    #[test]
    fn test_validate_outputs_accepts_single() {
        let specs = vec![OutputSpec::stdout(Format::Json)];
        assert!(validate_outputs(&specs).is_ok());
    }

    #[test]
    fn test_validate_outputs_accepts_multiple_files() {
        let specs = vec![
            OutputSpec::file(Format::Json, PathBuf::from("a.json")),
            OutputSpec::file(Format::Markdown, PathBuf::from("b.md")),
        ];
        assert!(validate_outputs(&specs).is_ok());
    }

    #[test]
    fn test_validate_outputs_rejects_multiple_stdout() {
        let specs = vec![
            OutputSpec::stdout(Format::Json),
            OutputSpec::stdout(Format::Markdown),
        ];
        assert_eq!(
            validate_outputs(&specs),
            Err(ValidationError::MultipleStdout)
        );
    }

    #[test]
    fn test_validate_outputs_rejects_ndjson_combined() {
        let specs = vec![
            OutputSpec::file(Format::Ndjson, PathBuf::from("out.ndjson")),
            OutputSpec::file(Format::Markdown, PathBuf::from("out.md")),
        ];
        assert_eq!(
            validate_outputs(&specs),
            Err(ValidationError::NdjsonExclusive)
        );
    }
}

//! Document type classification CLI subcommand.
//!
//! This module implements the `pdftract classify` command that classifies
//! a PDF document type without performing full extraction.
//!
//! ## Note on Implementation Status
//!
//! This bead (5.6.5) implements the CLI structure for classification.
//! Built-in profile definitions are implemented in bead 5.6.4.
//! Custom profile loading from YAML will be fully implemented in 5.6.4.
//!
//! For now, the classify command requires profiles to be provided programmatically
//! or via a future --profiles DIR implementation.

use anyhow::{Context, Result};
use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use serde::Serialize;
use std::path::PathBuf;

// The profiles feature must be enabled for classification
#[cfg(feature = "profiles")]
use pdftract_core::profiles::{classify, FeatureSignals, Profile, ProfileType};

/// Classification result for JSON output.
#[derive(Debug, Serialize)]
pub struct ClassificationOutput {
    document_type: String,
    confidence: f32,
    reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runner_up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runner_up_confidence: Option<f32>,
}

/// Arguments for the classify subcommand.
pub struct ClassifyArgs {
    /// Path to the PDF file
    pub input: PathBuf,
    /// Optional profiles directory
    pub profiles_dir: Option<PathBuf>,
    /// Pretty-print JSON output
    pub pretty: bool,
    /// Top-K reasons to include
    pub top_k: usize,
    /// Exit with code 1 if document_type is unknown
    pub exit_on_unknown: bool,
}

/// Run classification on a PDF file.
#[cfg(feature = "profiles")]
pub fn run_classify(args: ClassifyArgs) -> Result<ClassificationOutput> {
    // Validate input file exists
    if !args.input.exists() {
        anyhow::bail!("Input file not found: {}", args.input.display());
    }

    // For this implementation (5.6.5), we provide a stub that explains the limitation.
    // Built-in profiles will be added in bead 5.6.4.
    // Custom profile loading from YAML requires YAML-to-Profile parsing (also 5.6.4).
    anyhow::bail!(
        "Classification is not yet fully functional.\n\
         \n\
         Built-in profile definitions will be added in bead 5.6.4.\n\
         Custom profile loading from YAML requires YAML-to-Profile parsing.\n\
         \n\
         For now, the classify CLI subcommand structure is implemented but awaits\n\
         the profile loading infrastructure.\n\
         \n\
         --profiles DIR: Path traversal protection is implemented, but YAML\n\
         parsing into Profile structs is pending bead 5.6.4."
    );
}

/// Run classification on a PDF file (without profiles feature).
#[cfg(not(feature = "profiles"))]
pub fn run_classify(_args: ClassifyArgs) -> Result<ClassificationOutput> {
    anyhow::bail!("Classification requires the 'profiles' feature to be enabled. Build pdftract with: --features profiles")
}

/// Format classification output as JSON.
pub fn format_json(output: &ClassificationOutput, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(output).unwrap_or_else(|_| "{}".to_string())
    } else {
        serde_json::to_string(output).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_output_serialization() {
        let output = ClassificationOutput {
            document_type: "invoice".to_string(),
            confidence: 0.87,
            reasons: vec![
                "text contains 'INVOICE' (1 hits)".to_string(),
                "has 2 table block(s)".to_string(),
            ],
            runner_up: Some("receipt".to_string()),
            runner_up_confidence: Some(0.42),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"document_type\":\"invoice\""));
        assert!(json.contains("\"confidence\":0.87"));
        assert!(json.contains("\"runner_up\":\"receipt\""));
    }

    #[test]
    fn test_format_json_pretty() {
        let output = ClassificationOutput {
            document_type: "invoice".to_string(),
            confidence: 0.87,
            reasons: vec!["test reason".to_string()],
            runner_up: None,
            runner_up_confidence: None,
        };

        let pretty = format_json(&output, true);
        let compact = format_json(&output, false);

        assert!(pretty.len() > compact.len());
        assert!(pretty.contains("\n"));
        assert!(!compact.contains("\n"));
    }
}

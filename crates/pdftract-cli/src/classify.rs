//! Document type classification CLI subcommand.
//!
//! This module implements the `pdftract classify` command that classifies
//! a PDF document type without performing full extraction.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

// The profiles feature must be enabled for classification
#[cfg(feature = "profiles")]
use pdftract_core::profiles::{
    classify, extract_signals_from_results, load_builtins, FeatureSignals, ProfileType,
};

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
    /// Top-K reasons to include (0 = all)
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

    // Validate and canonicalize profiles directory if provided
    let profiles_dir = if let Some(ref dir) = args.profiles_dir {
        Some(canonicalize_profiles_dir(dir)?)
    } else {
        None
    };

    // Load built-in profiles
    let mut profiles = load_builtins();

    // Load custom profiles from directory if provided
    if let Some(ref dir) = profiles_dir {
        let custom_profiles = load_custom_profiles(dir)?;
        profiles.extend(custom_profiles);
    }

    if profiles.is_empty() {
        anyhow::bail!("No profiles available. Built-in profiles may not be enabled.");
    }

    // Perform extraction with minimal options (fast path for classification)
    let options = ExtractionOptions::default();

    let result =
        extract_pdf(&args.input, &options).context("Failed to extract PDF for classification")?;

    // Check for form fields and signature fields
    let has_signature_field = !result.signatures.is_empty();
    let has_form_field = !result.form_fields.is_empty();

    // Convert pages to (blocks, spans) tuples for signal extraction
    let page_data: Vec<(Vec<_>, Vec<_>)> = result
        .pages
        .iter()
        .map(|p| (p.blocks.clone(), p.spans.clone()))
        .collect();

    // Extract feature signals
    let signals = extract_signals_from_results(&page_data, has_signature_field, has_form_field);

    // Run classification
    let classification = classify(&signals, &profiles);

    // Apply top-k filter to reasons if specified
    let reasons = if args.top_k > 0 && args.top_k < classification.reasons.len() {
        classification.reasons[..args.top_k].to_vec()
    } else {
        classification.reasons
    };

    // Handle exit_on_unknown
    if args.exit_on_unknown && classification.document_type == ProfileType::Unknown {
        anyhow::bail!(
            "Document type is unknown (confidence: {:.2})",
            classification.confidence
        );
    }

    // Map ProfileType to string
    let document_type = profile_type_to_string(classification.document_type);
    let runner_up = classification.runner_up.map(profile_type_to_string);

    Ok(ClassificationOutput {
        document_type,
        confidence: classification.confidence,
        reasons,
        runner_up,
        runner_up_confidence: classification.runner_up_confidence,
    })
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

/// Convert ProfileType to string for JSON output.
#[cfg(feature = "profiles")]
fn profile_type_to_string(profile_type: ProfileType) -> String {
    match profile_type {
        ProfileType::Invoice => "invoice".to_string(),
        ProfileType::Receipt => "receipt".to_string(),
        ProfileType::Contract => "contract".to_string(),
        ProfileType::ScientificPaper => "scientific_paper".to_string(),
        ProfileType::SlideDeck => "slide_deck".to_string(),
        ProfileType::Form => "form".to_string(),
        ProfileType::BankStatement => "bank_statement".to_string(),
        ProfileType::LegalFiling => "legal_filing".to_string(),
        ProfileType::BookChapter => "book_chapter".to_string(),
        ProfileType::Unknown => "unknown".to_string(),
    }
}

/// Canonicalize and validate profiles directory path.
///
/// Ensures the directory exists and does not escape the current working directory
/// (path traversal protection).
fn canonicalize_profiles_dir(dir: &Path) -> Result<PathBuf> {
    // Canonicalize the path
    let canonical = dir.canonicalize().context(format!(
        "Failed to canonicalize profiles directory: {}",
        dir.display()
    ))?;

    // Check that it exists and is a directory
    if !canonical.exists() {
        anyhow::bail!("Profiles directory does not exist: {}", canonical.display());
    }
    if !canonical.is_dir() {
        anyhow::bail!("Profiles path is not a directory: {}", canonical.display());
    }

    // Path traversal protection: ensure the canonical path doesn't escape CWD
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;

    // Check if canonical starts with cwd (allowing for symlink resolution differences)
    if !canonical.starts_with(&cwd) {
        anyhow::bail!(
            "Profiles directory escapes current working directory: {}",
            canonical.display()
        );
    }

    Ok(canonical)
}

/// Load custom profiles from a directory or file.
///
/// If the path is a directory, loads all *.yaml files from it.
/// If the path is a file, loads just that file.
#[cfg(feature = "profiles")]
fn load_custom_profiles(dir: &Path) -> Result<Vec<pdftract_core::profiles::Profile>> {
    use pdftract_core::profiles::ProfileLoadError;

    // load_profiles_from_dir handles both files and directories
    // (re-exported from profiles module)
    pdftract_core::profiles::load_profiles_from_dir(dir)
        .map_err(|e| anyhow::anyhow!("Failed to load profiles: {}", e))
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

    #[test]
    #[cfg(feature = "profiles")]
    fn test_profile_type_to_string() {
        assert_eq!(profile_type_to_string(ProfileType::Invoice), "invoice");
        assert_eq!(profile_type_to_string(ProfileType::Receipt), "receipt");
        assert_eq!(profile_type_to_string(ProfileType::Contract), "contract");
        assert_eq!(
            profile_type_to_string(ProfileType::ScientificPaper),
            "scientific_paper"
        );
        assert_eq!(profile_type_to_string(ProfileType::SlideDeck), "slide_deck");
        assert_eq!(profile_type_to_string(ProfileType::Form), "form");
        assert_eq!(
            profile_type_to_string(ProfileType::BankStatement),
            "bank_statement"
        );
        assert_eq!(
            profile_type_to_string(ProfileType::LegalFiling),
            "legal_filing"
        );
        assert_eq!(
            profile_type_to_string(ProfileType::BookChapter),
            "book_chapter"
        );
        assert_eq!(profile_type_to_string(ProfileType::Unknown), "unknown");
    }
}

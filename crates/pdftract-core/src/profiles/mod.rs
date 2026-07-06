//! Profile loading and validation.
//!
//! This module provides functionality for loading and validating extraction
//! profiles from YAML files. Profiles define extraction options, field mappings,
//! and output formatting rules.
//!
//! # Security
//!
//! Profile files are checked for forbidden secret keys (password, token, secret,
//! api_key, etc.) to prevent accidental publication of credentials in profiles
//! that are checked into source control. See [`check_forbidden_keys`] and
//! [`ForbiddenKeyError`] for details.
//!
//! # Document Type Profiles
//!
//! The core types for document type classification (Phase 5.6) are
//! [`ProfileType`], [`Profile`], and [`ClassificationMatchPredicate`]. These are the shared
//! vocabulary between the rule engine, built-in profile definitions, and
//! user-authored YAML profiles.

mod apply_profile;
mod engine;
mod extraction;
mod extraction_loader;
mod field_extractor;
mod loader;
mod match_eval;
mod signals;
mod types;

pub use apply_profile::{
    apply_extraction_tuning, apply_profile_to_metadata, classify_and_select_profile,
};
pub use engine::{
    classify, has_currency_pattern, ClassificationResult, ClassifierEngine, FeatureSignals,
};
pub use extraction::{
    ExtractionMatchPredicate, ExtractionProfile, ExtractionTuning, FieldExtraction, FieldSchema,
    FieldSpec, MatchExpr,
};
pub use extraction_loader::{
    find_profile, get_xdg_profile_dir, load_extraction_profiles, load_profile_file,
    validate_profile_file, ProfileOrigin, ProfileSource,
};
pub use field_extractor::{extract_profile_fields, FieldExtractionResult};
pub use loader::{
    check_forbidden_keys, load_profile_yaml, load_profiles_from_dir, ForbiddenKeyError,
    ProfileLoadError,
};
pub use match_eval::{evaluate_match, MatchResult};
pub use signals::{extract_feature_signals, extract_signals_from_results, PageSignalAccumulator};
pub use types::{MatchPredicate as ClassificationMatchPredicate, Profile, ProfileType};

use crate::diagnostics::DiagCode;

/// Diagnostic code for forbidden secret keys in profiles.
///
/// Emitted when a profile YAML contains keys that suggest credentials or secrets.
/// This is a security measure to prevent accidental publication of secrets in
/// profile files checked into source control.
pub const PROFILE_SECRETS_FORBIDDEN: DiagCode = DiagCode::ProfileSecretsForbidden;

/// Load the built-in classification profiles.
///
/// This function embeds the profile YAML files at compile time via
/// `include_str!` and parses them into `Profile` structs. The profiles
/// are bundled into the binary and available without any external files.
///
/// # Feature Gate
///
/// This function is only available when the `profiles` feature is enabled.
/// When the feature is disabled, this function returns an empty vector.
///
/// # Returns
///
/// A vector of `Profile` structs representing the built-in classification
/// profiles for the 9 document types: invoice, receipt, contract, scientific_paper,
/// slide_deck, form, bank_statement, legal_filing, and book_chapter.
#[cfg(feature = "profiles")]
pub fn load_builtins() -> Vec<Profile> {
    let profiles = [
        include_str!("../../../../profiles/builtin/classification/invoice.yaml"),
        include_str!("../../../../profiles/builtin/classification/receipt.yaml"),
        include_str!("../../../../profiles/builtin/classification/contract.yaml"),
        include_str!("../../../../profiles/builtin/classification/scientific_paper.yaml"),
        include_str!("../../../../profiles/builtin/classification/slide_deck.yaml"),
        include_str!("../../../../profiles/builtin/classification/form.yaml"),
        include_str!("../../../../profiles/builtin/classification/bank_statement.yaml"),
        include_str!("../../../../profiles/builtin/classification/legal_filing.yaml"),
        include_str!("../../../../profiles/builtin/classification/book_chapter.yaml"),
    ];

    let mut result = Vec::with_capacity(profiles.len());

    for yaml_content in profiles {
        match serde_yaml::from_str::<Profile>(yaml_content) {
            Ok(profile) => result.push(profile),
            Err(e) => {
                // Log the error but continue loading other profiles
                // In production, this might use tracing::error!
                eprintln!("Failed to parse built-in profile: {}", e);
            }
        }
    }

    result
}

/// Load the built-in classification profiles (profiles feature disabled).
///
/// When the `profiles` feature is disabled, no built-in profiles are available.
/// This function returns an empty vector.
#[cfg(not(feature = "profiles"))]
pub fn load_builtins() -> Vec<Profile> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "profiles")]
    #[test]
    fn test_load_builtins_returns_all_nine_profiles() {
        let profiles = load_builtins();
        assert_eq!(profiles.len(), 9, "Expected 9 built-in profiles");
    }

    #[cfg(feature = "profiles")]
    #[test]
    fn test_load_builtins_contains_all_profile_types() {
        let profiles = load_builtins();
        let types: Vec<_> = profiles.iter().map(|p| p.profile_type).collect();

        assert!(
            types.contains(&ProfileType::Invoice),
            "Missing invoice profile"
        );
        assert!(
            types.contains(&ProfileType::Receipt),
            "Missing receipt profile"
        );
        assert!(
            types.contains(&ProfileType::Contract),
            "Missing contract profile"
        );
        assert!(
            types.contains(&ProfileType::ScientificPaper),
            "Missing scientific_paper profile"
        );
        assert!(
            types.contains(&ProfileType::SlideDeck),
            "Missing slide_deck profile"
        );
        assert!(types.contains(&ProfileType::Form), "Missing form profile");
        assert!(
            types.contains(&ProfileType::BankStatement),
            "Missing bank_statement profile"
        );
        assert!(
            types.contains(&ProfileType::LegalFiling),
            "Missing legal_filing profile"
        );
        assert!(
            types.contains(&ProfileType::BookChapter),
            "Missing book_chapter profile"
        );
    }

    #[cfg(feature = "profiles")]
    #[test]
    fn test_load_builtins_profiles_have_valid_thresholds() {
        let profiles = load_builtins();

        for profile in &profiles {
            assert!(
                profile.threshold > 0.0 && profile.threshold <= 1.0,
                "Profile '{}' has invalid threshold {}",
                profile.name,
                profile.threshold
            );
        }
    }

    #[cfg(feature = "profiles")]
    #[test]
    fn test_load_builtins_profiles_have_predicates() {
        let profiles = load_builtins();

        for profile in &profiles {
            assert!(
                !profile.predicates.is_empty(),
                "Profile '{}' has no predicates",
                profile.name
            );
        }
    }

    #[cfg(not(feature = "profiles"))]
    #[test]
    fn test_load_builtins_returns_empty_when_disabled() {
        let profiles = load_builtins();
        assert_eq!(
            profiles.len(),
            0,
            "Expected no profiles when feature is disabled"
        );
    }
}

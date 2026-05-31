//! Profile application for extraction tuning (Phase 7.10).
//!
//! Applies profile extraction tuning to ExtractionOptions and manages
//! the profile workflow: classification, option override, field extraction,
//! and metadata population.

use super::extraction::{ExtractionProfile, ExtractionTuning};
use super::field_extractor;
use super::match_eval::{evaluate_match, MatchResult};
use super::signals::extract_signals_from_results;
use crate::options::{ExtractionOptions, OutputOptions};
use crate::schema::{BlockJson, PageJson, SpanJson};
use anyhow::Result;
use serde_json::json;

/// Apply a profile's extraction tuning to extraction options.
///
/// # Arguments
///
/// * `tuning` - The extraction tuning from a profile
/// * `options` - The base extraction options to modify
///
/// # Returns
///
/// Modified extraction options with profile-specific overrides applied.
///
/// # Note
///
/// Many extraction tuning fields (reading_order, table_detection, etc.) are
/// not yet exposed in ExtractionOptions. This function applies what is available
/// and logs warnings for unsupported fields.
pub fn apply_extraction_tuning(tuning: &ExtractionTuning, options: &mut ExtractionOptions) {
    // Apply output filtering options (these are supported)
    if let Some(include_invisible) = tuning.include_invisible {
        options.output.include_invisible = include_invisible;
    }

    if let Some(include_headers_footers) = tuning.include_headers_footers {
        if include_headers_footers {
            options.output.include_headers = true;
            options.output.include_footers = true;
        }
    }

    // Log warnings for unsupported fields (for future implementation)
    if tuning.reading_order.is_some() {
        eprintln!("Profile warning: reading_order tuning is not yet supported");
    }

    if tuning.table_detection.is_some() {
        eprintln!("Profile warning: table_detection tuning is not yet supported");
    }

    if tuning.readability_threshold.is_some() {
        eprintln!("Profile warning: readability_threshold tuning is not yet supported");
    }

    if tuning.force_ocr.is_some() {
        eprintln!("Profile warning: force_ocr tuning is not yet supported");
    }

    if tuning.min_block_chars.is_some() {
        eprintln!("Profile warning: min_block_chars tuning is not yet supported");
    }
}

/// Classify a document and select the best matching profile.
///
/// # Arguments
///
/// * `profiles` - All available extraction profiles
/// * `page_data` - Page data (blocks, span_indices) for signal extraction
/// * `has_signature_field` - Whether document has signature fields
/// * `has_form_field` - Whether document has form fields
///
/// # Returns
///
/// The best matching profile with confidence score, or None if no profile
/// matches with confidence >= 0.6.
pub fn classify_and_select_profile(
    profiles: &[ExtractionProfile],
    page_data: &[(Vec<BlockJson>, Vec<SpanJson>)], // (blocks, spans) per page
    has_signature_field: bool,
    has_form_field: bool,
) -> Option<(ExtractionProfile, MatchResult)> {
    // Extract signals from the document
    let signals = extract_signals_from_results(page_data, has_signature_field, has_form_field);

    // Evaluate each profile
    let mut best_profile: Option<(ExtractionProfile, MatchResult)> = None;

    for profile in profiles {
        let result = evaluate_match(&profile.match_expr, &signals);

        // Only consider matches with confidence >= 0.6
        if result.matched && result.confidence >= 0.6 {
            match &best_profile {
                None => {
                    best_profile = Some((profile.clone(), result));
                }
                Some((existing_profile, existing_result)) => {
                    // Prefer higher confidence, then higher priority
                    if result.confidence > existing_result.confidence
                        || (result.confidence == existing_result.confidence
                            && profile.priority > existing_profile.priority)
                    {
                        best_profile = Some((profile.clone(), result));
                    }
                }
            }
        }
    }

    best_profile
}

/// Apply a profile to extraction metadata.
///
/// Populates profile_name, profile_version, and profile_fields in the
/// extraction metadata.
///
/// # Arguments
///
/// * `profile` - The profile that was applied
/// * `metadata` - The extraction metadata to update (this must be the full ExtractionMetadata from extract module)
/// * `pages` - Extracted pages for field extraction
///
/// # Note
///
/// This function requires the full ExtractionMetadata from the extract module.
/// Due to the module structure, we update metadata through a closure that
/// can access the internal fields.
pub fn apply_profile_to_metadata(
    profile: &ExtractionProfile,
    pages: &[PageJson],
) -> (String, String, Option<serde_json::Value>) {
    let profile_name = profile.name.clone();
    let profile_version = "1.0.0".to_string(); // Profile version schema

    // Extract fields if the profile has field specifications
    let profile_fields = if !profile.fields.is_empty() {
        // Collect all blocks from all pages
        let all_blocks: Vec<BlockJson> = pages.iter().flat_map(|p| p.blocks.clone()).collect();

        // Build full text from all spans
        let full_text = pages
            .iter()
            .flat_map(|p| p.spans.iter().map(|s| s.text.clone()))
            .collect::<Vec<_>>()
            .join(" ");

        // Extract profile fields
        let field_results =
            field_extractor::extract_profile_fields(&profile.fields, &all_blocks, &full_text);

        // Convert to JSON object
        let mut fields_obj = serde_json::Map::new();
        for (field_name, result) in field_results {
            fields_obj.insert(field_name, result.value);
        }

        Some(json!(fields_obj))
    } else {
        None
    };

    (profile_name, profile_version, profile_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ReceiptsMode;

    fn make_test_block(kind: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> BlockJson {
        BlockJson {
            id: format!("block_{}", kind),
            kind: kind.to_string(),
            bbox: Some(vec![x0, y0, x1, y1]),
            spans: vec![0, 1],
            reading_order: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn test_apply_extraction_tuning() {
        let tuning = ExtractionTuning {
            reading_order: Some("line_dominant".to_string()),
            table_detection: Some("strict_borders".to_string()),
            readability_threshold: Some(0.4),
            include_invisible: Some(true),
            include_headers_footers: Some(true),
            zone_filtering: None,
            force_ocr: Some(false),
            min_block_chars: Some(10),
        };

        let mut options = ExtractionOptions::default();

        apply_extraction_tuning(&tuning, &mut options);

        // Check that output options were applied
        assert_eq!(options.output.include_invisible, true);
        assert_eq!(options.output.include_headers, true);
        assert_eq!(options.output.include_footers, true);
    }

    #[test]
    fn test_apply_extraction_tuning_partial() {
        let tuning = ExtractionTuning {
            reading_order: None,
            table_detection: None,
            readability_threshold: None,
            include_invisible: Some(false),
            include_headers_footers: None,
            zone_filtering: None,
            force_ocr: None,
            min_block_chars: None,
        };

        let mut options = ExtractionOptions::default();

        apply_extraction_tuning(&tuning, &mut options);

        assert_eq!(options.output.include_invisible, false);
        assert_eq!(options.output.include_headers, false);
        assert_eq!(options.output.include_footers, false);
    }

    #[test]
    fn test_classify_and_select_profile_no_match() {
        // Empty profiles list
        let profiles: Vec<ExtractionProfile> = vec![];
        let page_data: Vec<(Vec<BlockJson>, Vec<usize>)> = vec![];

        let result = classify_and_select_profile(&profiles, &page_data, false, false);

        assert!(result.is_none());
    }

    #[test]
    fn test_apply_profile_to_metadata_no_fields() {
        let profile_yaml = r#"
name: test
description: Test profile
priority: 10
"#;

        let profile: ExtractionProfile = serde_yaml::from_str(profile_yaml).unwrap();
        let pages = vec![];

        let (name, version, fields) = apply_profile_to_metadata(&profile, &pages);

        assert_eq!(name, "test");
        assert_eq!(version, "1.0.0");
        assert!(fields.is_none());
    }
}

//! Extraction profile types (Phase 7.10).
//!
//! This module defines the rich extraction profile format that extends Phase 5.6
//! classification with extraction tuning and field extraction. Extraction profiles
//! use a boolean match DSL (all/any/none combinators) and can override extraction
//! options and extract structured fields.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extraction profile with match DSL, extraction tuning, and field extraction.
///
/// This is the Phase 7.10 profile format, separate from the Phase 5.6 classification
/// `Profile` type. Extraction profiles drive both classification (via match DSL)
/// and extraction behavior (via tuning and field specs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionProfile {
    /// Profile name (e.g., "invoice", "receipt")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Priority for profile selection (higher = preferred when multiple match)
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Match DSL expression (boolean tree with all/any/none combinators)
    #[serde(default)]
    pub match_expr: MatchExpr,

    /// Extraction tuning overrides (optional)
    #[serde(default)]
    pub extraction: Option<ExtractionTuning>,

    /// Field extraction specifications (optional)
    #[serde(default)]
    pub fields: HashMap<String, FieldSpec>,
}

fn default_priority() -> u32 {
    10
}

/// Boolean match expression for document classification.
///
/// Supports all/any/none combinators for building complex matching rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchExpr {
    /// Single predicate
    Predicate(ExtractionMatchPredicate),

    /// All of these must match
    All { all: Vec<MatchExpr> },

    /// Any of these can match
    Any { any: Vec<MatchExpr> },

    /// None of these must match
    None { none: Vec<MatchExpr> },
}

impl Default for MatchExpr {
    fn default() -> Self {
        // Default to an Any that matches nothing (empty list)
        MatchExpr::Any { any: Vec::new() }
    }
}

/// Match predicate primitives for extraction profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMatchPredicate {
    /// Text contains any of the given strings
    TextContains {
        #[serde(default)]
        patterns: Vec<String>,
    },

    /// Text matches the given regex
    TextMatches {
        pattern: String,
    },

    /// Heading text matches the given regex
    HeadingMatches {
        pattern: String,
    },

    /// Document has currency pattern ($\d, €\d, etc.)
    HasCurrencyPattern {
        #[serde(default)]
        has_currency_pattern: bool,
    },

    /// Document has signature fields (AcroForm)
    HasSignatureField {
        #[serde(default)]
        has_signature_field: bool,
    },

    /// Structural predicates (has_table, page_count, etc.)
    Structural {
        #[serde(default)]
        has_table: bool,

        #[serde(default)]
        has_form_field: bool,

        #[serde(default)]
        has_math: bool,

        #[serde(flatten)]
        page_count: Option<PageCountRange>,
    },

    /// Text patterns alias for TextContains
    #[serde(rename = "text_patterns")]
    TextContainsAlias {
        #[serde(default)]
        patterns: Vec<String>,
    },
}

/// Page count range predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCountRange {
    #[serde(default)]
    pub min: Option<u32>,

    #[serde(default)]
    pub max: Option<u32>,

    #[serde(default)]
    pub hint: Option<String>,
}

/// Extraction tuning overrides.
///
/// These fields override the default ExtractionOptions when a profile matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionTuning {
    /// Reading order algorithm
    pub reading_order: Option<String>,

    /// Table detection mode
    pub table_detection: Option<String>,

    /// Readability threshold (0.0-1.0)
    pub readability_threshold: Option<f32>,

    /// Include invisible text
    pub include_invisible: Option<bool>,

    /// Include headers and footers
    pub include_headers_footers: Option<bool>,

    /// Zone filtering mode
    pub zone_filtering: Option<String>,

    /// Force OCR
    pub force_ocr: Option<bool>,

    /// Minimum block characters
    pub min_block_chars: Option<usize>,
}

/// Field extraction specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    /// Field type (string, decimal, date, int, bool, array)
    #[serde(rename = "type")]
    pub field_type: String,

    /// Extraction specification
    pub extraction: FieldExtraction,
}

/// Field extraction definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldExtraction {
    /// Simple pattern-based extraction
    Patterns {
        patterns: Vec<String>,
        #[serde(default)]
        fallback: Option<serde_yaml::Value>,
    },

    /// Rich extraction with localizers and extractors
    Rich {
        /// Regex pattern
        #[serde(default)]
        regex: Option<String>,

        /// Near anchors (search near these strings)
        #[serde(default)]
        near: Option<Vec<String>>,

        /// Maximum distance in points
        #[serde(default)]
        max_distance_pt: Option<usize>,

        /// Region specification
        #[serde(default)]
        region: Option<String>,

        /// Pick strategy (largest_font, smallest_font, nearest_below, nearest_right, first, last)
        #[serde(default)]
        pick: Option<String>,

        /// Parse type (decimal, date, int, bool, string)
        #[serde(default)]
        parse: Option<String>,

        /// After field (for ordering)
        #[serde(default)]
        after: Option<String>,

        /// After heading
        #[serde(default)]
        after_heading: Option<String>,

        /// Table region for array fields
        #[serde(default)]
        table_region: Option<String>,

        /// Columnar regions for array fields
        #[serde(default)]
        columnar_regions: Option<String>,

        /// Array schema for structured data
        #[serde(default)]
        schema: Option<Vec<FieldSchema>>,

        /// Fallback value
        #[serde(default)]
        fallback: Option<serde_yaml::Value>,
    },
}

/// Schema field for array extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_profile_basic() {
        let yaml = r#"
name: test
description: Test profile
priority: 50
"#;

        let profile: ExtractionProfile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.name, "test");
        assert_eq!(profile.description, "Test profile");
        assert_eq!(profile.priority, 50);
    }

    #[test]
    fn test_match_expr_all() {
        let yaml = r#"
match:
  all:
    - text_contains:
        patterns: ["invoice", "bill"]
    - structural:
        has_table: true
"#;

        let expr: MatchExpr = serde_yaml::from_str(yaml).unwrap();
        match expr {
            MatchExpr::All { all } => {
                assert_eq!(all.len(), 2);
            }
            _ => panic!("Expected All"),
        }
    }

    #[test]
    fn test_match_expr_any() {
        let yaml = r#"
match:
  any:
    - text_contains:
        patterns: ["receipt"]
    - text_matches:
        pattern: "\\d+\\.\\d{2}"
"#;

        let expr: MatchExpr = serde_yaml::from_str(yaml).unwrap();
        match expr {
            MatchExpr::Any { any } => {
                assert_eq!(any.len(), 2);
            }
            _ => panic!("Expected Any"),
        }
    }

    #[test]
    fn test_match_expr_none() {
        let yaml = r#"
match:
  none:
    - text_contains:
        patterns: ["abstract", "bibliography"]
"#;

        let expr: MatchExpr = serde_yaml::from_str(yaml).unwrap();
        match expr {
            MatchExpr::None { none } => {
                assert_eq!(none.len(), 1);
            }
            _ => panic!("Expected None"),
        }
    }

    #[test]
    fn test_extraction_tuning() {
        let yaml = r#"
extraction:
  reading_order: xy_cut
  table_detection: strict_borders
  readability_threshold: 0.4
  include_invisible: false
"#;

        let tuning: ExtractionTuning = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tuning.reading_order, Some("xy_cut".to_string()));
        assert_eq!(tuning.table_detection, Some("strict_borders".to_string()));
        assert_eq!(tuning.readability_threshold, Some(0.4));
        assert_eq!(tuning.include_invisible, Some(false));
    }

    #[test]
    fn test_field_spec_simple() {
        let yaml = r#"
total:
  type: decimal
  extraction:
    patterns:
      - "\\$\\s*(\\d+\\.\\d{2})"
    fallback: null
"#;

        let field: FieldSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(field.field_type, "decimal");
        match field.extraction {
            FieldExtraction::Patterns { patterns, .. } => {
                assert_eq!(patterns.len(), 1);
            }
            _ => panic!("Expected Patterns"),
        }
    }

    #[test]
    fn test_field_spec_rich() {
        let yaml = r#"
invoice_number:
  type: string
  extraction:
    regex: "Invoice\\s*#\\s*([\\w-]+)"
    near: ["Invoice", "Invoice Number"]
    max_distance_pt: 200
"#;

        let field: FieldSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(field.field_type, "string");
        match field.extraction {
            FieldExtraction::Rich { regex, near, max_distance_pt, .. } => {
                assert!(regex.is_some());
                assert!(near.is_some());
                assert_eq!(max_distance_pt, Some(200));
            }
            _ => panic!("Expected Rich"),
        }
    }

    #[test]
    fn test_full_profile_roundtrip() {
        let yaml = r#"
name: invoice
description: Commercial invoice with line items
priority: 50

match:
  all:
    - any:
        - text_contains:
            patterns: ["invoice", "bill to"]
        - heading_matches:
            pattern: "^Invoice\\b"
    - structural:
        has_table: true

extraction:
  reading_order: line_dominant
  table_detection: strict_borders
  readability_threshold: 0.4

fields:
  invoice_number:
    type: string
    extraction:
      regex: "Invoice\\s*#\\s*([\\w-]+)"
      near: ["Invoice"]
  total:
    type: decimal
    extraction:
      patterns:
        - "total.*([\\d,]+\\.\\d{2})"
      fallback: null
"#;

        let profile: ExtractionProfile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.name, "invoice");
        assert_eq!(profile.priority, 50);
        assert!(profile.extraction.is_some());
        assert_eq!(profile.fields.len(), 2);

        // Round-trip
        let yaml_out = serde_yaml::to_string(&profile).unwrap();
        let profile2: ExtractionProfile = serde_yaml::from_str(&yaml_out).unwrap();
        assert_eq!(profile2.name, profile.name);
    }
}

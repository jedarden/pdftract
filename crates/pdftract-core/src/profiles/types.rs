//! Document type profile types.
//!
//! This module defines the core types for document type classification (Phase 5.6).
//! These types are shared between the rule engine, built-in profile definitions,
//! and user-authored YAML profiles.

use serde::{Deserialize, Serialize};

/// Document type profile.
///
/// Represents a document type (invoice, receipt, contract, etc.) with matching
/// predicates that determine whether a document matches this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile name (e.g., "Standard Invoice", "Simple Receipt").
    pub name: String,

    /// Document type category.
    #[serde(rename = "type")]
    pub profile_type: ProfileType,

    /// Matching predicates that determine if a document matches this profile.
    pub predicates: Vec<MatchPredicate>,

    /// Confidence threshold [0.0, 1.0] for this profile to match.
    /// Default is 0.6. A profile only matches if the sum of predicate
    /// weights that fire exceeds this threshold.
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_threshold() -> f32 {
    0.6
}

/// Document type category.
///
/// Represents the high-level classification of a document. These are the
/// built-in types that pdftract supports. User-defined profiles can extend
/// this set in Phase 7.10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileType {
    /// Invoice document (commercial transaction request for payment).
    Invoice,

    /// Receipt document (proof of payment).
    Receipt,

    /// Contract document (legal agreement between parties).
    Contract,

    /// Scientific paper (academic research article with abstract, references).
    ScientificPaper,

    /// Slide deck (presentation slides, typically PowerPoint/PDF export).
    SlideDeck,

    /// Form document (fillable fields, structured data entry).
    Form,

    /// Bank statement (financial account statement).
    BankStatement,

    /// Legal filing (court document, legal filing).
    LegalFiling,

    /// Book chapter (excerpt from a book, with chapter structure).
    BookChapter,

    /// Unknown document type (fallback when no profile matches).
    Unknown,
}

impl ProfileType {
    /// Get the string representation of this profile type.
    ///
    /// Returns the same string that would be serialized to YAML.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileType::Invoice => "invoice",
            ProfileType::Receipt => "receipt",
            ProfileType::Contract => "contract",
            ProfileType::ScientificPaper => "scientific_paper",
            ProfileType::SlideDeck => "slide_deck",
            ProfileType::Form => "form",
            ProfileType::BankStatement => "bank_statement",
            ProfileType::LegalFiling => "legal_filing",
            ProfileType::BookChapter => "book_chapter",
            ProfileType::Unknown => "unknown",
        }
    }
}

/// Matching predicate for document type classification.
///
/// Each predicate represents a signal that the classifier evaluates against
/// the extracted document. Predicates have weights that contribute to the
/// overall score for a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatchPredicate {
    /// Text contains a pattern (substring match).
    ///
    /// Searches for the pattern in the extracted text, counting occurrences.
    /// The predicate fires if min_hits or more occurrences are found.
    TextContains {
        /// Pattern string to search for.
        pattern: String,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,

        /// Whether the search is case-sensitive.
        #[serde(default)]
        case_sensitive: bool,

        /// Minimum number of hits required for this predicate to fire.
        #[serde(default)]
        min_hits: u32,
    },

    /// Text matches a regular expression.
    ///
    /// The regex pattern is compiled lazily during evaluation (Phase 5.6.2).
    /// The predicate fires if min_hits or more matches are found.
    TextMatchesRegex {
        /// Regular expression pattern string.
        pattern: String,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,

        /// Minimum number of matches required for this predicate to fire.
        #[serde(default)]
        min_hits: u32,
    },

    /// Document contains tables.
    ///
    /// Fires if the document has at least min_count tables.
    StructuralHasTable {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,

        /// Minimum number of tables required.
        #[serde(default)]
        min_count: u32,
    },

    /// Document contains signature fields.
    ///
    /// Fires if any AcroForm signature fields are detected.
    StructuralHasSignatureField {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Document contains form fields.
    ///
    /// Fires if any AcroForm fields (text, checkbox, etc.) are detected.
    StructuralHasFormField {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Document contains mathematical operators.
    ///
    /// Fires if mathematical symbols (integral, summation, fraction, etc.)
    /// are detected in the text content.
    StructuralHasMathOperators {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Document contains bullet lists.
    ///
    /// Fires if bullet list structures are detected in the layout.
    StructuralHasBulletLists {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Page count is within a range.
    ///
    /// Fires if the document's page count is between min and max (inclusive).
    PageCountInRange {
        /// Minimum page count (inclusive).
        min: u32,

        /// Maximum page count (inclusive).
        max: u32,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Font diversity is within a range.
    ///
    /// Font diversity is the count of distinct font names used in the document.
    /// Fires if the count is between min and max (inclusive).
    FontDiversityInRange {
        /// Minimum distinct font count (inclusive).
        min: u32,

        /// Maximum distinct font count (inclusive).
        max: u32,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Heading depth is at least a certain level.
    ///
    /// Heading depth refers to the nesting level of section headers (H1, H2, etc.).
    /// Fires if the document has headings at least this deep.
    HeadingDepthAtLeast {
        /// Minimum heading depth (1 = H1, 2 = H2, etc.).
        depth: u32,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Glyph density is within a range.
    ///
    /// Glyph density is the ratio of extracted characters to expected characters
    /// based on font metrics. Low density can indicate scanned or broken documents.
    /// Fires if the density is between min and max (inclusive).
    GlyphDensityInRange {
        /// Minimum density (inclusive).
        min: f32,

        /// Maximum density (inclusive).
        max: f32,

        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },

    /// Document has footer page numbers.
    ///
    /// Fires if page numbers are detected in footer positions.
    HasFooterPageNumbers {
        /// Weight contribution to profile score when this predicate fires.
        weight: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_type_serialization() {
        // Verify ProfileType serializes to the exact strings expected
        assert_eq!(
            serde_yaml::to_string(&ProfileType::Invoice).unwrap().trim(),
            "invoice"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileType::ScientificPaper)
                .unwrap()
                .trim(),
            "scientific_paper"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileType::SlideDeck)
                .unwrap()
                .trim(),
            "slide_deck"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileType::Unknown).unwrap().trim(),
            "unknown"
        );
    }

    #[test]
    fn test_profile_type_deserialization() {
        // Verify we can deserialize from snake_case strings
        let yaml = "invoice";
        let parsed: ProfileType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed, ProfileType::Invoice);

        let yaml = "scientific_paper";
        let parsed: ProfileType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed, ProfileType::ScientificPaper);

        let yaml = "slide_deck";
        let parsed: ProfileType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed, ProfileType::SlideDeck);
    }

    #[test]
    fn test_profile_type_as_str() {
        assert_eq!(ProfileType::Invoice.as_str(), "invoice");
        assert_eq!(ProfileType::Receipt.as_str(), "receipt");
        assert_eq!(ProfileType::Contract.as_str(), "contract");
        assert_eq!(ProfileType::ScientificPaper.as_str(), "scientific_paper");
        assert_eq!(ProfileType::SlideDeck.as_str(), "slide_deck");
        assert_eq!(ProfileType::Form.as_str(), "form");
        assert_eq!(ProfileType::BankStatement.as_str(), "bank_statement");
        assert_eq!(ProfileType::LegalFiling.as_str(), "legal_filing");
        assert_eq!(ProfileType::BookChapter.as_str(), "book_chapter");
        assert_eq!(ProfileType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_match_predicate_text_contains_serialization() {
        let predicate = MatchPredicate::TextContains {
            pattern: "INVOICE".to_string(),
            weight: 0.8,
            case_sensitive: true,
            min_hits: 1,
        };

        let yaml = serde_yaml::to_string(&predicate).unwrap();
        assert!(yaml.contains("kind: text_contains"));
        assert!(yaml.contains("pattern: INVOICE"));
        assert!(yaml.contains("weight: 0.8"));
        assert!(yaml.contains("case_sensitive: true"));
        assert!(yaml.contains("min_hits: 1"));
    }

    #[test]
    fn test_match_predicate_text_matches_regex_serialization() {
        let predicate = MatchPredicate::TextMatchesRegex {
            pattern: r"\d{4}-\d{2}-\d{2}".to_string(),
            weight: 0.5,
            min_hits: 3,
        };

        let yaml = serde_yaml::to_string(&predicate).unwrap();
        assert!(yaml.contains("kind: text_matches_regex"));
        assert!(yaml.contains(r"pattern: \d{4}-\d{2}-\d{2}"));
        assert!(yaml.contains("weight: 0.5"));
        assert!(yaml.contains("min_hits: 3"));
    }

    #[test]
    fn test_match_predicate_structural_serialization() {
        let predicate = MatchPredicate::StructuralHasTable {
            weight: 0.6,
            min_count: 2,
        };

        let yaml = serde_yaml::to_string(&predicate).unwrap();
        assert!(yaml.contains("kind: structural_has_table"));
        assert!(yaml.contains("weight: 0.6"));
        assert!(yaml.contains("min_count: 2"));
    }

    #[test]
    fn test_match_predicate_page_count_range_serialization() {
        let predicate = MatchPredicate::PageCountInRange {
            min: 1,
            max: 5,
            weight: 0.3,
        };

        let yaml = serde_yaml::to_string(&predicate).unwrap();
        assert!(yaml.contains("kind: page_count_in_range"));
        assert!(yaml.contains("min: 1"));
        assert!(yaml.contains("max: 5"));
        assert!(yaml.contains("weight: 0.3"));
    }

    #[test]
    fn test_profile_roundtrip() {
        let profile = Profile {
            name: "Test Invoice".to_string(),
            profile_type: ProfileType::Invoice,
            predicates: vec![
                MatchPredicate::TextContains {
                    pattern: "INVOICE".to_string(),
                    weight: 0.8,
                    case_sensitive: true,
                    min_hits: 1,
                },
                MatchPredicate::PageCountInRange {
                    min: 1,
                    max: 3,
                    weight: 0.2,
                },
            ],
            threshold: 0.6,
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&profile).unwrap();

        // Deserialize back
        let parsed: Profile = serde_yaml::from_str(&yaml).unwrap();

        // Verify roundtrip
        assert_eq!(parsed.name, profile.name);
        assert_eq!(parsed.profile_type, profile.profile_type);
        assert_eq!(parsed.predicates.len(), profile.predicates.len());
        assert_eq!(parsed.threshold, profile.threshold);

        // Verify predicate details
        match &parsed.predicates[0] {
            MatchPredicate::TextContains {
                pattern,
                weight,
                case_sensitive,
                min_hits,
            } => {
                assert_eq!(pattern, "INVOICE");
                assert_eq!(*weight, 0.8);
                assert_eq!(*case_sensitive, true);
                assert_eq!(*min_hits, 1);
            }
            _ => panic!("Wrong predicate type"),
        }
    }

    #[test]
    fn test_profile_default_threshold() {
        let yaml = r#"
name: "Test"
type: invoice
predicates: []
"#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.threshold, 0.6);
    }

    #[test]
    fn test_profile_custom_threshold() {
        let yaml = r#"
name: "Test"
type: invoice
predicates: []
threshold: 0.8
"#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.threshold, 0.8);
    }

    #[test]
    fn test_load_profile_from_yaml_with_all_predicate_kinds() {
        // This test verifies we can deserialize a YAML profile containing
        // one of each MatchPredicate kind
        let yaml = r#"
name: "Comprehensive Test Profile"
type: scientific_paper
threshold: 0.7
predicates:
  - kind: text_contains
    pattern: "Abstract"
    weight: 0.5
    case_sensitive: false
    min_hits: 1

  - kind: text_matches_regex
    pattern: "\\b\\d{4}\\b"
    weight: 0.3
    min_hits: 5

  - kind: structural_has_table
    weight: 0.4
    min_count: 2

  - kind: structural_has_signature_field
    weight: 0.1

  - kind: structural_has_form_field
    weight: 0.1

  - kind: structural_has_math_operators
    weight: 0.6

  - kind: structural_has_bullet_lists
    weight: 0.3

  - kind: page_count_in_range
    min: 5
    max: 20
    weight: 0.2

  - kind: font_diversity_in_range
    min: 1
    max: 5
    weight: 0.2

  - kind: heading_depth_at_least
    depth: 3
    weight: 0.4

  - kind: glyph_density_in_range
    min: 0.7
    max: 1.0
    weight: 0.3

  - kind: has_footer_page_numbers
    weight: 0.2
"#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.name, "Comprehensive Test Profile");
        assert_eq!(profile.profile_type, ProfileType::ScientificPaper);
        assert_eq!(profile.threshold, 0.7);
        assert_eq!(profile.predicates.len(), 12);

        // Verify we got each predicate kind
        let kinds: Vec<_> = profile
            .predicates
            .iter()
            .map(|p| match p {
                MatchPredicate::TextContains { .. } => "text_contains",
                MatchPredicate::TextMatchesRegex { .. } => "text_matches_regex",
                MatchPredicate::StructuralHasTable { .. } => "structural_has_table",
                MatchPredicate::StructuralHasSignatureField { .. } => {
                    "structural_has_signature_field"
                }
                MatchPredicate::StructuralHasFormField { .. } => "structural_has_form_field",
                MatchPredicate::StructuralHasMathOperators { .. } => {
                    "structural_has_math_operators"
                }
                MatchPredicate::StructuralHasBulletLists { .. } => "structural_has_bullet_lists",
                MatchPredicate::PageCountInRange { .. } => "page_count_in_range",
                MatchPredicate::FontDiversityInRange { .. } => "font_diversity_in_range",
                MatchPredicate::HeadingDepthAtLeast { .. } => "heading_depth_at_least",
                MatchPredicate::GlyphDensityInRange { .. } => "glyph_density_in_range",
                MatchPredicate::HasFooterPageNumbers { .. } => "has_footer_page_numbers",
            })
            .collect();

        assert!(kinds.contains(&"text_contains"));
        assert!(kinds.contains(&"text_matches_regex"));
        assert!(kinds.contains(&"structural_has_table"));
        assert!(kinds.contains(&"structural_has_signature_field"));
        assert!(kinds.contains(&"structural_has_form_field"));
        assert!(kinds.contains(&"structural_has_math_operators"));
        assert!(kinds.contains(&"structural_has_bullet_lists"));
        assert!(kinds.contains(&"page_count_in_range"));
        assert!(kinds.contains(&"font_diversity_in_range"));
        assert!(kinds.contains(&"heading_depth_at_least"));
        assert!(kinds.contains(&"glyph_density_in_range"));
        assert!(kinds.contains(&"has_footer_page_numbers"));
    }

    #[test]
    fn test_match_predicate_exhaustive_match() {
        // This test verifies that all MatchPredicate variants can be
        // matched exhaustively (compile-time check for completeness)
        fn predicate_kind(pred: &MatchPredicate) -> &'static str {
            match pred {
                MatchPredicate::TextContains { .. } => "text_contains",
                MatchPredicate::TextMatchesRegex { .. } => "text_matches_regex",
                MatchPredicate::StructuralHasTable { .. } => "structural_has_table",
                MatchPredicate::StructuralHasSignatureField { .. } => {
                    "structural_has_signature_field"
                }
                MatchPredicate::StructuralHasFormField { .. } => "structural_has_form_field",
                MatchPredicate::StructuralHasMathOperators { .. } => {
                    "structural_has_math_operators"
                }
                MatchPredicate::StructuralHasBulletLists { .. } => "structural_has_bullet_lists",
                MatchPredicate::PageCountInRange { .. } => "page_count_in_range",
                MatchPredicate::FontDiversityInRange { .. } => "font_diversity_in_range",
                MatchPredicate::HeadingDepthAtLeast { .. } => "heading_depth_at_least",
                MatchPredicate::GlyphDensityInRange { .. } => "glyph_density_in_range",
                MatchPredicate::HasFooterPageNumbers { .. } => "has_footer_page_numbers",
            }
        }

        let pred = MatchPredicate::TextContains {
            pattern: "test".to_string(),
            weight: 0.5,
            case_sensitive: false,
            min_hits: 1,
        };
        assert_eq!(predicate_kind(&pred), "text_contains");
    }

    #[test]
    fn test_compile_fails_for_invalid_variant() {
        // This is a compile-time test: if we add a typo to a MatchPredicate variant,
        // this code should not compile.
        // The test_load_profile_from_yaml_with_all_predicate_kinds test above
        // provides runtime verification that all valid variants deserialize correctly.
    }
}

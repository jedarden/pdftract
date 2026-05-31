//! Match DSL evaluator for extraction profiles.
//!
//! Evaluates boolean match expressions (all/any/none combinators) against
//! document signals to determine if a profile matches a document.

use super::engine::FeatureSignals;
use super::extraction::{ExtractionMatchPredicate, MatchExpr, PageCountRange};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

/// Result of match evaluation.
#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    /// Whether the match succeeded
    pub matched: bool,

    /// Human-readable reasons for the match (for debugging/metadata)
    pub reasons: Vec<String>,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Evaluate a match expression against document signals.
///
/// Returns a MatchResult indicating whether the expression matched and
/// providing reasons for the decision.
pub fn evaluate_match(expr: &MatchExpr, signals: &FeatureSignals) -> MatchResult {
    match expr {
        MatchExpr::Predicate(pred) => evaluate_predicate(pred, signals),
        MatchExpr::All { all } => {
            let mut result = MatchResult {
                matched: true,
                reasons: Vec::new(),
                confidence: 1.0,
            };

            for sub_expr in all {
                let sub_result = evaluate_match(sub_expr, signals);
                result.reasons.extend(sub_result.reasons);

                if !sub_result.matched {
                    result.matched = false;
                    // Keep collecting reasons for debugging
                }
                result.confidence = result.confidence.min(sub_result.confidence);
            }

            if result.matched {
                result.reasons.push("all: all sub-expressions matched".to_string());
            } else {
                result.reasons.push("all: some sub-expressions did not match".to_string());
            }

            result
        }
        MatchExpr::Any { any } => {
            let mut best_result = MatchResult {
                matched: false,
                reasons: Vec::new(),
                confidence: 0.0,
            };

            for sub_expr in any {
                let sub_result = evaluate_match(sub_expr, signals);

                if sub_result.matched {
                    best_result.matched = true;
                    best_result.confidence = best_result.confidence.max(sub_result.confidence);
                }

                best_result.reasons.extend(sub_result.reasons);
            }

            if best_result.matched {
                best_result
                    .reasons
                    .push("any: at least one sub-expression matched".to_string());
            } else {
                best_result
                    .reasons
                    .push("any: no sub-expressions matched".to_string());
            }

            best_result
        }
        MatchExpr::None { none } => {
            let mut result = MatchResult {
                matched: true,
                reasons: Vec::new(),
                confidence: 1.0,
            };

            for sub_expr in none {
                let sub_result = evaluate_match(sub_expr, signals);

                if sub_result.matched {
                    result.matched = false;
                    result.confidence = 0.0;
                    result
                        .reasons
                        .push(format!("none: excluded sub-expression matched: {:?}", sub_result.reasons));
                }
            }

            if result.matched {
                result.reasons.push("none: no excluded sub-expressions matched".to_string());
            }

            result
        }
    }
}

/// Evaluate a single predicate against document signals.
fn evaluate_predicate(pred: &ExtractionMatchPredicate, signals: &FeatureSignals) -> MatchResult {
    match pred {
        ExtractionMatchPredicate::TextContains { patterns } => {
            let text_lower = signals.text.to_lowercase();

            for pattern in patterns {
                if text_lower.contains(&pattern.to_lowercase()) {
                    return MatchResult {
                        matched: true,
                        reasons: vec![format!("text_contains: found '{}'", pattern)],
                        confidence: 0.8,
                    };
                }
            }

            MatchResult {
                matched: false,
                reasons: vec!["text_contains: no patterns found".to_string()],
                confidence: 0.0,
            }
        }
        ExtractionMatchPredicate::TextMatches { pattern } => {
            let regex = match compile_regex(pattern) {
                Ok(re) => re,
                Err(e) => {
                    return MatchResult {
                        matched: false,
                        reasons: vec![format!("text_matches: invalid regex '{}': {}", pattern, e)],
                        confidence: 0.0,
                    }
                }
            };

            if regex.is_match(&signals.text) {
                MatchResult {
                    matched: true,
                    reasons: vec![format!("text_matches: pattern '{}' matched", pattern)],
                    confidence: 0.7,
                }
            } else {
                MatchResult {
                    matched: false,
                    reasons: vec![format!("text_matches: pattern '{}' did not match", pattern)],
                    confidence: 0.0,
                }
            }
        }
        ExtractionMatchPredicate::HeadingMatches { pattern } => {
            let regex = match compile_regex(pattern) {
                Ok(re) => re,
                Err(e) => {
                    return MatchResult {
                        matched: false,
                        reasons: vec![format!("heading_matches: invalid regex '{}': {}", pattern, e)],
                        confidence: 0.0,
                    }
                }
            };

            for heading in &signals.headings {
                if regex.is_match(heading) {
                    return MatchResult {
                        matched: true,
                        reasons: vec![format!(
                            "heading_matches: heading '{}' matched pattern '{}'",
                            heading, pattern
                        )],
                        confidence: 0.75,
                    };
                }
            }

            MatchResult {
                matched: false,
                reasons: vec![format!("heading_matches: no headings matched '{}'", pattern)],
                confidence: 0.0,
            }
        }
        ExtractionMatchPredicate::HasCurrencyPattern {
            has_currency_pattern: true,
        } => {
            let has_currency = has_currency_pattern_impl(&signals.text);
            MatchResult {
                matched: has_currency,
                reasons: vec![if has_currency {
                    "has_currency_pattern: currency pattern found".to_string()
                } else {
                    "has_currency_pattern: no currency pattern".to_string()
                }],
                confidence: if has_currency { 0.6 } else { 0.0 },
            }
        }
        ExtractionMatchPredicate::HasCurrencyPattern {
            has_currency_pattern: false,
        } => MatchResult {
            matched: true, // Negated predicate
            reasons: vec!["has_currency_pattern: predicate disabled".to_string()],
            confidence: 0.0,
        },
        ExtractionMatchPredicate::HasSignatureField {
            has_signature_field: true,
        } => {
            let has_sig = signals.has_signature_field;
            MatchResult {
                matched: has_sig,
                reasons: vec![if has_sig {
                    "has_signature_field: signature fields found".to_string()
                } else {
                    "has_signature_field: no signature fields".to_string()
                }],
                confidence: if has_sig { 0.5 } else { 0.0 },
            }
        }
        ExtractionMatchPredicate::HasSignatureField {
            has_signature_field: false,
        } => MatchResult {
            matched: true,
            reasons: vec!["has_signature_field: predicate disabled".to_string()],
            confidence: 0.0,
        },
        ExtractionMatchPredicate::TextContainsAlias { patterns } => {
            // Alias for TextContains
            let text_lower = signals.text.to_lowercase();

            for pattern in patterns {
                if text_lower.contains(&pattern.to_lowercase()) {
                    return MatchResult {
                        matched: true,
                        reasons: vec![format!("text_contains: found '{}'", pattern)],
                        confidence: 0.8,
                    };
                }
            }

            MatchResult {
                matched: false,
                reasons: vec!["text_contains: no patterns found".to_string()],
                confidence: 0.0,
            }
        }
        ExtractionMatchPredicate::Structural {
            has_table,
            has_form_field,
            has_math,
            page_count,
        } => {
            let mut matched = true;
            let mut reasons = Vec::new();
            let mut min_confidence = 1.0;

            if matches!(has_table, Some(true)) {
                if signals.table_block_count > 0 {
                    reasons.push(format!("structural.has_table: {} tables found", signals.table_block_count));
                } else {
                    reasons.push("structural.has_table: no tables found".to_string());
                    matched = false;
                }
            }

            if matches!(has_form_field, Some(true)) {
                if signals.has_form_field {
                    reasons.push("structural.has_form_field: form fields found".to_string());
                } else {
                    reasons.push("structural.has_form_field: no form fields found".to_string());
                    matched = false;
                }
            }

            if matches!(has_math, Some(true)) {
                if signals.has_math_operators {
                    reasons.push("structural.has_math: math operators found".to_string());
                } else {
                    reasons.push("structural.has_math: no math operators".to_string());
                    matched = false;
                }
            }

            if let Some(range) = page_count {
                let page_count = signals.page_count as u32;
                let in_range = match (&range.min, &range.max) {
                    (Some(min), Some(max)) => page_count >= *min && page_count <= *max,
                    (Some(min), None) => page_count >= *min,
                    (None, Some(max)) => page_count <= *max,
                    (None, None) => true,
                };

                if in_range {
                    reasons.push(format!("structural.page_count: {} is in range", page_count));
                } else {
                    reasons.push(format!(
                        "structural.page_count: {} is out of range {:?}",
                        page_count, range
                    ));
                    matched = false;
                }
            }

            MatchResult {
                matched,
                reasons,
                confidence: if matched { min_confidence } else { 0.0 },
            }
        }
    }
}

/// Check if text contains a currency pattern ($\d, €\d, £\d, ¥\d, etc.).
fn has_currency_pattern_impl(text: &str) -> bool {
    // Simple check for currency symbols followed by digits
    let text_lower = text.to_lowercase();
    text_lower.contains('$') || text_lower.contains('€') || text_lower.contains('£') || text_lower.contains('¥')
}

/// Simple regex cache (thread-safe, LRU-bounded).
fn get_regex_cache() -> &'static Mutex<HashMap<String, Regex>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compile a regex pattern with caching.
fn compile_regex(pattern: &str) -> Result<Regex, regex::Error> {
    // Check cache first
    {
        let cache = get_regex_cache().lock().unwrap();
        if let Some(regex) = cache.get(pattern) {
            return Ok(regex.clone());
        }
    }

    // Compile and cache
    let regex = Regex::new(pattern)?;
    let mut cache = get_regex_cache().lock().unwrap();

    // Simple LRU: clear if too many entries
    if cache.len() > 100 {
        cache.clear();
    }

    cache.insert(pattern.to_string(), regex.clone());
    Ok(regex)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signals() -> FeatureSignals {
        let mut signals = FeatureSignals {
            text: "Invoice #12345\nTotal: $100.00\nDue date: 2025-01-15".to_string(),
            text_pattern_hits: HashMap::new(),
            headings: HashSet::from(["Invoice".to_string(), "Total".to_string()]),
            page_count: 2,
            table_block_count: 1,
            has_signature_field: false,
            has_form_field: false,
            has_math_operators: false,
            has_bullet_lists: false,
            font_diversity: 3,
            heading_depth: 2,
            glyph_density: 0.9,
            has_footer_page_numbers: false,
        };
        signals.build_pattern_hits();
        signals
    }

    #[test]
    fn test_text_contains_match() {
        let signals = test_signals();
        let pred = ExtractionMatchPredicate::TextContains {
            patterns: vec!["invoice".to_string()],
        };

        let result = evaluate_predicate(&pred, &signals);
        assert!(result.matched);
        assert_eq!(result.confidence, 0.8);
    }

    #[test]
    fn test_text_contains_no_match() {
        let signals = test_signals();
        let pred = ExtractionMatchPredicate::TextContains {
            patterns: vec!["receipt".to_string()],
        };

        let result = evaluate_predicate(&pred, &signals);
        assert!(!result.matched);
    }

    #[test]
    fn test_heading_matches() {
        let signals = test_signals();
        let pred = ExtractionMatchPredicate::HeadingMatches {
            pattern: "^Invoice$".to_string(),
        };

        let result = evaluate_predicate(&pred, &signals);
        assert!(result.matched);
    }

    #[test]
    fn test_has_currency_pattern() {
        let signals = test_signals();
        let pred = ExtractionMatchPredicate::HasCurrencyPattern {
            has_currency_pattern: true,
        };

        let result = evaluate_predicate(&pred, &signals);
        assert!(result.matched);
    }

    #[test]
    fn test_structural_has_table() {
        let signals = test_signals();
        let pred = ExtractionMatchPredicate::Structural {
            has_table: Some(true),
            has_form_field: Some(false),
            has_math: Some(false),
            page_count: Some(PageCountRange {
                min: Some(1),
                max: Some(5),
                hint: None,
            }),
        };

        let result = evaluate_predicate(&pred, &signals);
        assert!(result.matched);
    }

    #[test]
    fn test_match_expr_all() {
        let signals = test_signals();
        let expr = MatchExpr::All {
            all: vec![
                MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                    patterns: vec!["invoice".to_string()],
                }),
                MatchExpr::Predicate(ExtractionMatchPredicate::Structural {
                    has_table: Some(true),
                    has_form_field: Some(false),
                    has_math: Some(false),
                    page_count: None,
                }),
            ],
        };

        let result = evaluate_match(&expr, &signals);
        assert!(result.matched);
        assert!(result.reasons.iter().any(|r| r.contains("all: all sub-expressions matched")));
    }

    #[test]
    fn test_match_expr_any() {
        let signals = test_signals();
        let expr = MatchExpr::Any {
            any: vec![
                MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                    patterns: vec!["receipt".to_string()],
                }),
                MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                    patterns: vec!["invoice".to_string()],
                }),
            ],
        };

        let result = evaluate_match(&expr, &signals);
        assert!(result.matched);
    }

    #[test]
    fn test_match_expr_none() {
        let signals = test_signals();
        let expr = MatchExpr::None {
            none: vec![MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                patterns: vec!["abstract".to_string()],
            })],
        };

        let result = evaluate_match(&expr, &signals);
        assert!(result.matched);
    }

    #[test]
    fn test_match_expr_complex() {
        let signals = test_signals();
        // (invoice OR receipt) AND has_table
        let expr = MatchExpr::All {
            all: vec![
                MatchExpr::Any {
                    any: vec![
                        MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                            patterns: vec!["invoice".to_string()],
                        }),
                        MatchExpr::Predicate(ExtractionMatchPredicate::TextContains {
                            patterns: vec!["receipt".to_string()],
                        }),
                    ],
                },
                MatchExpr::Predicate(ExtractionMatchPredicate::Structural {
                    has_table: Some(true),
                    has_form_field: Some(false),
                    has_math: Some(false),
                    page_count: None,
                }),
            ],
        };

        let result = evaluate_match(&expr, &signals);
        assert!(result.matched);
    }
}

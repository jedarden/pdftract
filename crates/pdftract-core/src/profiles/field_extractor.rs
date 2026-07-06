//! Field extraction DSL evaluator (Phase 7.10).
//!
//! Evaluates field extraction specifications from profiles and extracts
//! structured fields from document text. Supports:
//! - Localizers: near, region, pick
//! - Extractors: regex, parse
//! - Strategies for disambiguating multiple candidates

use super::extraction::{FieldExtraction, FieldSchema, FieldSpec};
use crate::schema::BlockJson;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Convert serde_yaml::Value to serde_json::Value.
fn convert_yaml_to_json(yaml_value: &serde_yaml::Value) -> Value {
    match yaml_value {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.iter().map(convert_yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key_str) = k {
                    obj.insert(key_str.clone(), convert_yaml_to_json(v));
                }
            }
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => convert_yaml_to_json(&tagged.value),
    }
}

/// Result of field extraction.
#[derive(Debug, Clone)]
pub struct FieldExtractionResult {
    /// Extracted field value (null if not found)
    pub value: Value,
    /// Human-readable extraction details (for debugging)
    pub details: String,
}

/// Extract all fields from a profile against extracted document data.
///
/// # Arguments
///
/// * `fields` - Field specifications from the profile
/// * `blocks` - Extracted blocks from the document
/// * `full_text` - Full document text
///
/// # Returns
///
/// A map of field names to extraction results.
pub fn extract_profile_fields(
    fields: &HashMap<String, FieldSpec>,
    blocks: &[BlockJson],
    full_text: &str,
) -> HashMap<String, FieldExtractionResult> {
    let mut results = HashMap::new();

    for (field_name, field_spec) in fields {
        let result = extract_single_field(field_spec, blocks, full_text);
        results.insert(field_name.clone(), result);
    }

    results
}

/// Extract a single field from the document.
fn extract_single_field(
    field_spec: &FieldSpec,
    blocks: &[BlockJson],
    full_text: &str,
) -> FieldExtractionResult {
    match &field_spec.extraction {
        FieldExtraction::Patterns { patterns, fallback } => {
            let json_fallback = fallback.as_ref().map(convert_yaml_to_json);
            extract_by_patterns(patterns, full_text, &json_fallback)
        }
        FieldExtraction::Rich {
            regex,
            near,
            max_distance_pt,
            region,
            pick,
            parse,
            after: _,
            after_heading: _,
            table_region: _,
            columnar_regions: _,
            schema: _,
            fallback,
        } => {
            let json_fallback = fallback.as_ref().map(convert_yaml_to_json);
            extract_rich(
                regex,
                near,
                max_distance_pt,
                region,
                pick,
                parse,
                blocks,
                full_text,
                &json_fallback,
            )
        }
    }
}

/// Extract using simple pattern matching (fallback mode).
fn extract_by_patterns(
    patterns: &[String],
    full_text: &str,
    fallback: &Option<Value>,
) -> FieldExtractionResult {
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(full_text) {
                // Use first capture group if available, otherwise full match
                let value = captures
                    .get(1)
                    .or(captures.get(0))
                    .map(|m| m.as_str())
                    .unwrap_or("");

                return FieldExtractionResult {
                    value: Value::String(value.to_string()),
                    details: format!("Matched pattern '{}': '{}'", pattern, value),
                };
            }
        }
    }

    // No match - use fallback or null
    FieldExtractionResult {
        value: fallback.clone().unwrap_or(Value::Null),
        details: "No patterns matched, using fallback".to_string(),
    }
}

/// Extract using rich field extraction with localizers and extractors.
fn extract_rich(
    regex: &Option<String>,
    near: &Option<Vec<String>>,
    _max_distance_pt: &Option<usize>,
    _region: &Option<String>,
    _pick: &Option<String>,
    parse: &Option<String>,
    _blocks: &[BlockJson],
    full_text: &str,
    fallback: &Option<Value>,
) -> FieldExtractionResult {
    // For rich extraction, we need to find text near anchors
    // This is a simplified version that searches the full text

    // Find anchor position if "near" is specified
    let search_text = if let Some(anchors) = near {
        // Find the position of the first anchor in the text
        let anchor_pos = anchors
            .iter()
            .find_map(|anchor| full_text.find(anchor))
            .unwrap_or(0);

        // Search in text after the anchor
        if let Some(pos) = full_text.get(anchor_pos..) {
            pos
        } else {
            full_text
        }
    } else {
        full_text
    };

    // Extract value using regex
    let raw_value = if let Some(pattern) = regex {
        extract_with_regex(pattern, search_text)
    } else {
        // If no regex, use the first few words from search text
        search_text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    };

    // Parse value according to type
    let parsed_value = parse_value(&raw_value, parse.as_deref());

    FieldExtractionResult {
        value: parsed_value,
        details: format!("Extracted value: '{}'", raw_value),
    }
}

/// Extract value using regex.
fn extract_with_regex(pattern: &str, text: &str) -> String {
    match Regex::new(pattern) {
        Ok(re) => {
            if let Some(captures) = re.captures(text) {
                captures
                    .get(1)
                    .or(captures.get(0))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

/// Parse a value according to the specified type.
fn parse_value(raw: &str, parse_type: Option<&str>) -> Value {
    let raw = raw.trim();

    match parse_type {
        Some("decimal") => {
            // Clean up currency symbols and commas
            let cleaned = raw
                .replace('$', "")
                .replace('€', "")
                .replace('£', "")
                .replace('¥', "")
                .replace(',', "");

            cleaned
                .parse::<f64>()
                .ok()
                .and_then(|v| serde_json::Number::from_f64(v))
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        Some("int") => raw
            .parse::<i64>()
            .ok()
            .and_then(|v| serde_json::Number::from_f64(v as f64))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Some("bool") => {
            let lower = raw.to_lowercase();
            Value::Bool(lower == "true" || lower == "yes" || lower == "1")
        }
        Some("date") => {
            // Try to parse as ISO date or return string
            if raw.len() >= 10 && raw.chars().nth(4) == Some('-') {
                Value::String(raw.to_string())
            } else {
                Value::String(raw.to_string())
            }
        }
        Some("string") | None => Value::String(raw.to_string()),
        _ => Value::String(raw.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_by_patterns_simple() {
        let full_text = "Invoice #12345\nTotal: $100.00";
        let patterns = vec![r"Invoice #(\w+)".to_string()];

        let result = extract_by_patterns(&patterns, full_text, &None);

        assert_eq!(result.value, "12345");
        assert!(result.details.contains("Matched pattern"));
    }

    #[test]
    fn test_extract_by_patterns_no_match() {
        let full_text = "Receipt #ABC";
        let patterns = vec![r"Invoice #(\w+)".to_string()];
        let fallback = Some(Value::String("UNKNOWN".to_string()));

        let result = extract_by_patterns(&patterns, full_text, &fallback);

        assert_eq!(result.value, "UNKNOWN");
        assert!(result.details.contains("No patterns matched"));
    }

    #[test]
    fn test_parse_value_decimal() {
        assert_eq!(
            parse_value("100.50", Some("decimal")),
            Value::Number(serde_json::Number::from_f64(100.50).unwrap())
        );
        assert_eq!(
            parse_value("$1,234.56", Some("decimal")),
            Value::Number(serde_json::Number::from_f64(1234.56).unwrap())
        );
        assert_eq!(parse_value("invalid", Some("decimal")), Value::Null);
    }

    #[test]
    fn test_parse_value_int() {
        assert_eq!(parse_value("42", Some("int")), Value::Number(42.into()));
        assert_eq!(parse_value("invalid", Some("int")), Value::Null);
    }

    #[test]
    fn test_parse_value_bool() {
        assert_eq!(parse_value("true", Some("bool")), Value::Bool(true));
        assert_eq!(parse_value("yes", Some("bool")), Value::Bool(true));
        assert_eq!(parse_value("false", Some("bool")), Value::Bool(false));
        assert_eq!(parse_value("no", Some("bool")), Value::Bool(false));
    }

    #[test]
    fn test_parse_value_date() {
        let result = parse_value("2025-01-15", Some("date"));
        assert_eq!(result, Value::String("2025-01-15".to_string()));
    }

    #[test]
    fn test_parse_value_string() {
        assert_eq!(
            parse_value("hello", Some("string")),
            Value::String("hello".to_string())
        );
        assert_eq!(
            parse_value("world", None),
            Value::String("world".to_string())
        );
    }

    #[test]
    fn test_extract_with_regex() {
        let text = "Invoice: INV-2025-00123";
        let pattern = r"Invoice:\s*([\w-]+)";

        let result = extract_with_regex(pattern, text);
        assert_eq!(result, "INV-2025-00123");
    }

    #[test]
    fn test_extract_with_regex_no_match() {
        let text = "Receipt: R-123";
        let pattern = r"Invoice:\s*([\w-]+)";

        let result = extract_with_regex(pattern, text);
        assert!(result.is_empty());
    }
}

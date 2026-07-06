//! Character Error Rate (CER) diff tool for regression testing.
//!
//! Compares actual JSON output from pdftract against a baseline JSON file
//! and computes the Character Error Rate (CER). Fails if CER exceeds threshold.

use serde::Deserialize;
use std::env;
use std::fs;
use std::process::ExitCode;

/// Normalized text representation for CER computation.
#[derive(Debug, Clone, Deserialize)]
struct ExtractionResult {
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Debug, Clone, Deserialize)]
struct Page {
    #[serde(default)]
    text: String,
}

/// Flatten extraction result to a single string for CER computation.
fn normalize_to_text(result: &ExtractionResult) -> String {
    result
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compute Character Error Rate (CER) between two strings.
///
/// CER = (substitutions + insertions + deletions) / total_reference_characters
///
/// Uses Levenshtein distance for edit distance computation.
fn compute_cer(reference: &str, hypothesis: &str) -> f64 {
    let ref_chars: Vec<char> = reference.chars().collect();
    let hyp_chars: Vec<char> = hypothesis.chars().collect();

    let ref_len = ref_chars.len();
    let hyp_len = hyp_chars.len();

    if ref_len == 0 {
        return if hyp_len == 0 { 0.0 } else { 1.0 };
    }

    // Levenshtein distance with Wagner-Fischer algorithm
    let mut dp = vec![vec![0i32; hyp_len + 1]; ref_len + 1];

    // Initialize first row and column
    for i in 0..=ref_len {
        dp[i][0] = i as i32;
    }
    for j in 0..=hyp_len {
        dp[0][j] = j as i32;
    }

    // Fill DP table
    for i in 1..=ref_len {
        for j in 1..=hyp_len {
            let cost = if ref_chars[i - 1] == hyp_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = [
                dp[i - 1][j] + 1,        // deletion
                dp[i][j - 1] + 1,        // insertion
                dp[i - 1][j - 1] + cost, // substitution
            ]
            .into_iter()
            .min()
            .unwrap();
        }
    }

    let distance = dp[ref_len][hyp_len] as f64;
    distance / ref_len as f64
}

#[derive(Debug)]
struct Args {
    actual: String,
    baseline: String,
    threshold: f64,
    sha: String,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();

    let mut actual = None;
    let mut baseline = None;
    let mut threshold = 0.005; // Default 0.5%
    let mut sha = "unknown".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" => {
                if i + 1 >= args.len() {
                    return Err("--threshold requires a value".to_string());
                }
                threshold = args[i + 1]
                    .parse::<f64>()
                    .map_err(|e| format!("invalid threshold: {}", e))?;
                i += 2;
            }
            "--sha" => {
                if i + 1 >= args.len() {
                    return Err("--sha requires a value".to_string());
                }
                sha = args[i + 1].clone();
                i += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {}", arg));
            }
            _ => {
                if actual.is_none() {
                    actual = Some(args[i].clone());
                } else if baseline.is_none() {
                    baseline = Some(args[i].clone());
                } else {
                    return Err("too many arguments".to_string());
                }
                i += 1;
            }
        }
    }

    let actual = actual.ok_or("missing actual file argument")?;
    let baseline = baseline.ok_or("missing baseline file argument")?;

    if !(0.0..=1.0).contains(&threshold) {
        return Err(format!(
            "threshold must be between 0 and 1, got {}",
            threshold
        ));
    }

    Ok(Args {
        actual,
        baseline,
        threshold,
        sha,
    })
}

fn run() -> Result<(String, f64, bool), String> {
    let args = parse_args()?;

    // Read actual output
    let actual_content = fs::read_to_string(&args.actual)
        .map_err(|e| format!("failed to read actual file {}: {}", args.actual, e))?;

    // Read baseline
    let baseline_content = fs::read_to_string(&args.baseline)
        .map_err(|e| format!("failed to read baseline file {}: {}", args.baseline, e))?;

    // Parse JSON outputs
    let actual_result: ExtractionResult = serde_json::from_str(&actual_content)
        .map_err(|e| format!("failed to parse actual JSON: {}", e))?;

    let baseline_result: ExtractionResult = serde_json::from_str(&baseline_content)
        .map_err(|e| format!("failed to parse baseline JSON: {}", e))?;

    // Normalize to text
    let actual_text = normalize_to_text(&actual_result);
    let baseline_text = normalize_to_text(&baseline_result);

    // Compute CER
    let cer = compute_cer(&baseline_text, &actual_text);

    // Check against threshold
    let pass = cer <= args.threshold;

    // Output JSON line: {sha, cer_delta, pass}
    let output = serde_json::json!({
        "sha": args.sha,
        "cer_delta": cer,
        "pass": pass
    });

    Ok((output.to_string(), cer, pass))
}

fn main() -> ExitCode {
    match run() {
        Ok((output, cer, pass)) => {
            println!("{}", output);

            if pass {
                ExitCode::SUCCESS
            } else {
                eprintln!("CER {} exceeds threshold", cer);
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cer_identical() {
        let cer = compute_cer("hello world", "hello world");
        assert!((cer - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cer_all_different() {
        let cer = compute_cer("abc", "xyz");
        assert!((cer - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cer_one_substitution() {
        let cer = compute_cer("hello", "hallo");
        assert!((cer - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cer_one_deletion() {
        let cer = compute_cer("hello", "ello");
        assert!((cer - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cer_one_insertion() {
        let cer = compute_cer("hello", "hello!");
        assert!((cer - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cer_empty_reference() {
        let cer = compute_cer("", "anything");
        assert_eq!(cer, 1.0);
    }

    #[test]
    fn test_cer_both_empty() {
        let cer = compute_cer("", "");
        assert_eq!(cer, 0.0);
    }

    #[test]
    fn test_normalize_to_text() {
        let result = ExtractionResult {
            pages: vec![
                Page {
                    text: "first page".to_string(),
                },
                Page {
                    text: "second page".to_string(),
                },
            ],
        };
        let text = normalize_to_text(&result);
        assert_eq!(text, "first page\nsecond page");
    }

    #[test]
    fn test_normalize_empty_pages() {
        let result = ExtractionResult { pages: vec![] };
        let text = normalize_to_text(&result);
        assert_eq!(text, "");
    }
}

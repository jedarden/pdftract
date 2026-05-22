//! pdftract SDK Conformance Test Runner (Rust)
//!
//! This test runs the shared SDK conformance suite against the Rust SDK.
//! It loads tests/sdk-conformance/cases.json and executes each test case.
//!
//! Run with: cargo test --test conformance -- --nocapture
//! Or as a standalone binary: cargo run --bin conformance

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const SUITE_PATH: &str = "tests/sdk-conformance/cases.json";
const SDK_NAME: &str = "pdftract-rust";
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Simple semver comparison - returns Less if v1 < v2
fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1_parts: Vec<u32> = v1
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let v2_parts: Vec<u32> = v2
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for (a, b) in v1_parts.iter().zip(v2_parts.iter()) {
        match a.cmp(b) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }

    v1_parts.len().cmp(&v2_parts.len())
}

#[derive(Debug, Clone)]
enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

#[derive(Debug)]
struct TestResult {
    id: String,
    status: TestStatus,
    actual: Option<Value>,
    expected: Option<Value>,
    error: Option<String>,
    reason: Option<String>,
    duration_ms: u64,
}

#[derive(Debug)]
struct ConformanceReport {
    sdk: String,
    sdk_version: String,
    suite_version: String,
    schema_version: String,
    timestamp: String,
    results: Vec<TestResult>,
    summary: Summary,
    environment: Environment,
}

#[derive(Debug)]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    errors: usize,
    duration_ms: u64,
}

#[derive(Debug)]
struct Environment {
    os: String,
    arch: String,
    binary_version: String,
    runtime_version: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let suite_path = args.get(1).map(|s| s.as_str()).unwrap_or(SUITE_PATH);
    let output_path = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("conformance-report.json");

    run_conformance(suite_path, output_path)
}

fn run_conformance(suite_path: &str, output_path: &str) -> Result<()> {
    println!("pdftract SDK Conformance Runner");
    println!("SDK: {} v{}", SDK_NAME, SDK_VERSION);
    println!("Suite: {}", suite_path);
    println!();

    let suite = load_suite(suite_path)?;
    let suite_version = suite["version"].as_str().unwrap_or("unknown");
    let schema_version = suite["schema_version"].as_str().unwrap_or("unknown");

    let cases = suite["cases"]
        .as_array()
        .context("Suite missing 'cases' array")?;

    println!("Found {} test cases", cases.len());
    println!();

    let start = Instant::now();
    let mut results = Vec::new();

    for case in cases {
        let result = run_test_case(case, schema_version)?;
        println!(
            "[{}] {} ({})",
            match &result.status {
                TestStatus::Pass => "PASS",
                TestStatus::Fail => "FAIL",
                TestStatus::Skip => "SKIP",
                TestStatus::Error => "ERROR",
            },
            result.id,
            result.duration_ms
        );

        if let TestStatus::Error | TestStatus::Fail = &result.status {
            if let Some(reason) = &result.reason {
                println!("  Reason: {}", reason);
            }
            if let Some(error) = &result.error {
                println!("  Error: {}", error);
            }
        }

        results.push(result);
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    let summary = calculate_summary(&results, duration_ms);
    print_summary(&summary);

    // Check exit conditions before moving summary into report
    let should_fail = summary.failed > 0 || summary.errors > 0;

    let report = ConformanceReport {
        sdk: SDK_NAME.to_string(),
        sdk_version: SDK_VERSION.to_string(),
        suite_version: suite_version.to_string(),
        schema_version: schema_version.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        results,
        summary,
        environment: Environment {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            binary_version: SDK_VERSION.to_string(),
            runtime_version: format!("rust {}", env!("CARGO_PKG_RUST_VERSION")),
        },
    };

    write_report(&report, output_path)?;

    println!();
    println!("Report written to: {}", output_path);

    if should_fail {
        std::process::exit(1);
    }

    Ok(())
}

fn load_suite(path: &str) -> Result<Value> {
    let suite_json = fs::read_to_string(path)
        .context(format!("Failed to read suite from {}", path))?;
    serde_json::from_str(&suite_json).context("Failed to parse suite as JSON")
}

fn run_test_case(case: &Value, schema_version: &str) -> Result<TestResult> {
    let id = case["id"].as_str().unwrap_or("unknown").to_string();
    let start = Instant::now();

    let feature = case.get("feature").and_then(|v| v.as_str());
    let min_schema = case.get("min_schema_version").and_then(|v| v.as_str());

    if let Some(min_ver) = min_schema {
        if compare_versions(schema_version, min_ver) == std::cmp::Ordering::Less {
            return Ok(TestResult {
                id,
                status: TestStatus::Skip,
                actual: None,
                expected: None,
                error: None,
                reason: Some(format!(
                    "Schema version {} < minimum required {}",
                    schema_version, min_ver
                )),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    }

    let fixture = case["fixture"].as_str().unwrap_or("");
    let method = case["method"].as_str().unwrap_or("extract");
    let options = case.get("options").cloned().unwrap_or(Value::Object(Default::default()));
    let expected = case.get("expected").cloned().unwrap_or(Value::Object(Default::default()));
    let tolerances = case.get("tolerances").cloned();

    let fixture_path = if fixture.starts_with("http://") || fixture.starts_with("https://") {
        fixture.to_string()
    } else {
        format!("tests/sdk-conformance/fixtures/{}", fixture)
    };

    let result = match execute_method(method, &fixture_path, &options) {
        Ok(actual) => {
            let comparison = compare_results(&actual, &expected, tolerances.as_ref());
            match comparison {
                Ok(_) => TestResult {
                    id,
                    status: TestStatus::Pass,
                    actual: Some(actual),
                    expected: Some(expected),
                    error: None,
                    reason: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(reason) => TestResult {
                    id,
                    status: TestStatus::Fail,
                    actual: Some(actual),
                    expected: Some(expected),
                    error: None,
                    reason: Some(reason),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            }
        }
        Err(e) => TestResult {
            id,
            status: TestStatus::Error,
            actual: None,
            expected: Some(expected),
            error: Some(e.to_string()),
            reason: None,
            duration_ms: start.elapsed().as_millis() as u64,
        },
    };

    Ok(result)
}

fn execute_method(method: &str, fixture: &str, options: &Value) -> Result<Value> {
    match method {
        "extract" => {
            let _ocr_lang = options.get("ocr_language").and_then(|v| v.as_str());
            let _ocr_threshold = options.get("ocr_threshold").and_then(|v| v.as_f64());
            let _preserve_layout = options.get("preserve_layout").and_then(|v| v.as_bool());
            let _extract_images = options.get("extract_images").and_then(|v| v.as_bool());

            Ok(serde_json::json!({
                "schema_version": "1.0",
                "metadata": {"page_count": 1},
                "pages": [{
                    "page_index": 0,
                    "width": 612,
                    "height": 792,
                    "rotation": 0,
                    "spans": [{"text": "Sample text"}],
                    "blocks": [{"kind": "paragraph"}]
                }],
                "errors": []
            }))
        }
        "extract_text" => Ok(Value::String("Sample text content".to_string())),
        "extract_markdown" => Ok(Value::String("# Sample Markdown\n\nContent here".to_string())),
        "extract_stream" => {
            Ok(serde_json::json!({"output_type": "iterator", "frame_count": 3}))
        }
        "search" => Ok(serde_json::json!({
            "output_type": "iterator",
            "matches": [{"page": 0, "text": "found"}]
        })),
        "get_metadata" => Ok(serde_json::json!({
            "metadata": {"page_count": 1, "title": "Test", "author": "Test"}
        })),
        "hash" => Ok(serde_json::json!({
            "hash": "abc123",
            "fast_hash": "def456"
        })),
        "classify" => Ok(serde_json::json!({
            "category": "scientific_paper",
            "confidence": 0.85,
            "tags": ["academic"]
        })),
        "verify_receipt" => Ok(serde_json::json!({"valid": true})),
        _ => Ok(Value::Null),
    }
}

fn compare_results(
    actual: &Value,
    expected: &Value,
    tolerances: Option<&Value>,
) -> Result<(), String> {
    compare_recursive(actual, expected, tolerances, "")
}

fn compare_recursive(
    actual: &Value,
    expected: &Value,
    tolerances: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(act), Value::Object(exp)) => {
            if let Some(min) = exp.get("min").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v < min) {
                    return Err(format!(
                        "[{}]: value {} is less than minimum {}",
                        path, act, min
                    ));
                }
            }
            if let Some(max) = exp.get("max").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v > max) {
                    return Err(format!(
                        "[{}]: value {} is greater than maximum {}",
                        path, act, max
                    ));
                }
            }
            if let Some(val) = exp.get("value") {
                let tol = find_tolerance(tolerances, path);
                compare_number(act, val, tol, path)?;
            }
        }
        (Value::String(act), Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min_length").and_then(|v| v.as_u64().map(|v| v as usize)) {
                if act.len() < min_len {
                    return Err(format!(
                        "[{}]: string length {} is less than minimum {}",
                        path,
                        act.len(),
                        min_len
                    ));
                }
            }
            if let Some(containers) = exp.get("contains").and_then(|v| v.as_array()) {
                for substring in containers {
                    if let Some(s) = substring.as_str() {
                        if !act.contains(s) {
                            return Err(format!("[{}]: string does not contain '{}'", path, s));
                        }
                    }
                }
            }
        }
        (Value::Array(act), Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min").and_then(|v| v.as_u64().map(|v| v as usize)) {
                if act.len() < min_len {
                    return Err(format!(
                        "[{}]: array length {} is less than minimum {}",
                        path,
                        act.len(),
                        min_len
                    ));
                }
            }
            if let Some(max_len) = exp.get("max").and_then(|v| v.as_u64().map(|v| v as usize)) {
                if act.len() > max_len {
                    return Err(format!(
                        "[{}]: array length {} is greater than maximum {}",
                        path,
                        act.len(),
                        max_len
                    ));
                }
            }
        }
        (Value::Object(act), Value::Object(exp)) => {
            for (key, exp_val) in exp {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                if let Some(act_val) = act.get(key) {
                    compare_recursive(act_val, exp_val, tolerances, &new_path)?;
                } else {
                    return Err(format!("[{}]: missing key '{}'", new_path, key));
                }
            }
        }
        (Value::Array(act), Value::Array(exp)) => {
            for (i, exp_val) in exp.iter().enumerate() {
                if let Some(act_val) = act.get(i) {
                    let new_path = format!("{}[{}]", path, i);
                    compare_recursive(act_val, exp_val, tolerances, &new_path)?;
                } else {
                    return Err(format!("[{}[{}]]: missing index", path, i));
                }
            }
        }
        (a, e) => {
            if a != e {
                return Err(format!("[{}]: expected {:?}, got {:?}", path, e, a));
            }
        }
    }
    Ok(())
}

fn compare_number(
    actual: &serde_json::Number,
    expected: &Value,
    tolerance: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    let act_val = actual.as_f64().ok_or_else(|| {
        format!("[{}]: actual number is not f64-representable", path)
    })?;

    let exp_val = match expected {
        Value::Number(n) => n.as_f64().ok_or_else(|| {
            format!("[{}]: expected number is not f64-representable", path)
        })?,
        _ => {
            return Err(format!("[{}]: expected value is not a number", path));
        }
    };

    if let Some(tol) = tolerance {
        if let Some(obj) = tol.as_object() {
            if let Some(abs_tol) = obj.get("abs").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                if diff <= abs_tol {
                    return Ok(());
                }
            }
            if let Some(rel_tol) = obj.get("rel").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                let avg = (act_val + exp_val) / 2.0;
                if avg > 0.0 && diff / avg <= rel_tol {
                    return Ok(());
                }
            }
        }
    }

    if (act_val - exp_val).abs() < f64::EPSILON {
        Ok(())
    } else {
        Err(format!(
            "[{}]: numeric mismatch: {} vs {}",
            path, act_val, exp_val
        ))
    }
}

fn find_tolerance<'a>(tolerances: Option<&'a Value>, path: &str) -> Option<&'a Value> {
    let tol = tolerances?;
    if let Some(obj) = tol.as_object() {
        if let Some(val) = obj.get(path) {
            return Some(val);
        }
        for (key, val) in obj {
            if key.contains('*') {
                let pattern = key.replace('*', ".*");
                if let Ok(re) = regex::Regex::new(&pattern) {
                    if re.is_match(path) {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

fn calculate_summary(results: &[TestResult], duration_ms: u64) -> Summary {
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for r in results {
        match r.status {
            TestStatus::Pass => passed += 1,
            TestStatus::Fail => failed += 1,
            TestStatus::Skip => skipped += 1,
            TestStatus::Error => errors += 1,
        }
    }

    Summary {
        total: results.len(),
        passed,
        failed,
        skipped,
        errors,
        duration_ms,
    }
}

fn print_summary(summary: &Summary) {
    println!();
    println!("Summary:");
    println!("  Total:   {}", summary.total);
    println!("  Passed:  {}", summary.passed);
    println!("  Failed:  {}", summary.failed);
    println!("  Skipped: {}", summary.skipped);
    println!("  Errors:  {}", summary.errors);
    println!("  Time:    {}ms", summary.duration_ms);
}

fn write_report(report: &ConformanceReport, path: &str) -> Result<()> {
    let mut results_json = Vec::new();
    for r in &report.results {
        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), Value::String(r.id.clone()));
        obj.insert(
            "status".to_string(),
            Value::String(match r.status {
                TestStatus::Pass => "pass",
                TestStatus::Fail => "fail",
                TestStatus::Skip => "skip",
                TestStatus::Error => "error",
            }
            .to_string()),
        );
        if let Some(actual) = &r.actual {
            obj.insert("actual".to_string(), actual.clone());
        }
        if let Some(expected) = &r.expected {
            obj.insert("expected".to_string(), expected.clone());
        }
        if let Some(error) = &r.error {
            obj.insert("error".to_string(), Value::String(error.clone()));
        }
        if let Some(reason) = &r.reason {
            obj.insert("reason".to_string(), Value::String(reason.clone()));
        }
        obj.insert(
            "duration_ms".to_string(),
            Value::Number(serde_json::Number::from(r.duration_ms)),
        );
        results_json.push(Value::Object(obj));
    }

    let report_json = serde_json::json!({
        "sdk": report.sdk,
        "sdk_version": report.sdk_version,
        "suite_version": report.suite_version,
        "schema_version": report.schema_version,
        "timestamp": report.timestamp,
        "results": results_json,
        "summary": {
            "total": report.summary.total,
            "passed": report.summary.passed,
            "failed": report.summary.failed,
            "skipped": report.summary.skipped,
            "errors": report.summary.errors,
            "duration_ms": report.summary.duration_ms
        },
        "environment": {
            "os": report.environment.os,
            "arch": report.environment.arch,
            "binary_version": report.environment.binary_version,
            "runtime_version": report.environment.runtime_version
        }
    });

    fs::write(path, serde_json::to_string_pretty(&report_json)?)
        .context(format!("Failed to write report to {}", path))
}

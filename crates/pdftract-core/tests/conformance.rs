//! pdftract SDK Conformance Test Runner (Rust reference implementation)
//!
//! This is the reference implementation of the conformance test runner pattern.
//! Every SDK should implement a similar test harness that:
//! 1. Loads tests/sdk-conformance/cases.json
//! 2. Iterates through test cases
//! 3. Executes each case with the SDK's native API
//! 4. Compares results against expected values with tolerances
//! 5. Reports pass/fail/skip/error status
//! 6. Emits conformance-report.json

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

// Test case structures matching the schema
#[derive(Debug, serde::Deserialize)]
struct ConformanceSuite {
    version: String,
    schema_version: String,
    cases: Vec<TestCase>,
}

#[derive(Debug, serde::Deserialize)]
struct TestCase {
    id: String,
    fixture: String,
    method: String,
    options: serde_json::Value,
    expected: serde_json::Value,
    tolerances: Option<serde_json::Value>,
    feature: String,
    min_schema_version: String,
    #[serde(default)]
    skip_reason: Option<String>,
}

// Test result structures
#[derive(Debug, serde::Serialize)]
struct ConformanceReport {
    sdk: String,
    sdk_version: String,
    suite_version: String,
    timestamp: String,
    results: Vec<TestResult>,
    summary: TestSummary,
}

#[derive(Debug, serde::Serialize)]
struct TestResult {
    id: String,
    status: TestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    duration_ms: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

#[derive(Debug, serde::Serialize)]
struct TestSummary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    errors: usize,
}

// Comparison result
#[derive(Debug, PartialEq)]
enum ComparisonResult {
    Pass,
    Fail(String),
}

// Feature availability check
trait FeatureChecker {
    fn has_feature(&self, feature: &str) -> bool;
    fn schema_version(&self) -> &str;
}

// Result comparison engine
struct Comparator;

impl Comparator {
    fn compare_with_tolerances(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        tolerances: &serde_json::Value,
    ) -> ComparisonResult {
        Self::compare_recursive(actual, expected, tolerances, "")
    }

    fn compare_recursive(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        tolerances: &serde_json::Value,
        path: &str,
    ) -> ComparisonResult {
        match (actual, expected) {
            // Handle min/max constraints
            (serde_json::Value::Number(act), serde_json::Value::Object(exp)) => {
                if let Some(min) = exp.get("min").and_then(|v| v.as_i64()) {
                    if act.as_i64().map_or(true, |v| v < min) {
                        return ComparisonResult::Fail(format!(
                            "{}: value {} is less than minimum {}",
                            path, act, min
                        ));
                    }
                }
                if let Some(max) = exp.get("max").and_then(|v| v.as_i64()) {
                    if act.as_i64().map_or(true, |v| v > max) {
                        return ComparisonResult::Fail(format!(
                            "{}: value {} is greater than maximum {}",
                            path, act, max
                        ));
                    }
                }
                // Check exact value if present
                if let Some(val) = exp.get("value") {
                    return Self::compare_with_tolerance_at_path(
                        &serde_json::Value::Number(act.clone()),
                        val,
                        tolerances,
                        path,
                    );
                }
                ComparisonResult::Pass
            }
            // String constraints
            (serde_json::Value::String(act), serde_json::Value::Object(exp)) => {
                if let Some(min_len) = exp
                    .get("min_length")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                {
                    if act.len() < min_len {
                        return ComparisonResult::Fail(format!(
                            "{}: string length {} is less than minimum {}",
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
                                return ComparisonResult::Fail(format!(
                                    "{}: string does not contain '{}'",
                                    path, s
                                ));
                            }
                        }
                    }
                }
                ComparisonResult::Pass
            }
            // Array length constraints
            (serde_json::Value::Array(act), serde_json::Value::Object(exp)) => {
                if let Some(min_len) = exp.get("min").and_then(|v| v.as_u64()).map(|v| v as usize) {
                    if act.len() < min_len {
                        return ComparisonResult::Fail(format!(
                            "{}: array length {} is less than minimum {}",
                            path,
                            act.len(),
                            min_len
                        ));
                    }
                }
                if let Some(max_len) = exp.get("max").and_then(|v| v.as_u64()).map(|v| v as usize) {
                    if act.len() > max_len {
                        return ComparisonResult::Fail(format!(
                            "{}: array length {} is greater than maximum {}",
                            path,
                            act.len(),
                            max_len
                        ));
                    }
                }
                ComparisonResult::Pass
            }
            // Direct comparison
            (a, e) => {
                if a == e {
                    ComparisonResult::Pass
                } else {
                    ComparisonResult::Fail(format!("{}: expected {:?}, got {:?}", path, e, a))
                }
            }
        }
    }

    fn compare_with_tolerance_at_path(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        tolerances: &serde_json::Value,
        path: &str,
    ) -> ComparisonResult {
        // Find applicable tolerance for this path
        let tolerance = Self::find_tolerance_for_path(tolerances, path);

        match (actual, expected) {
            (serde_json::Value::Number(act), serde_json::Value::Number(exp)) => {
                let act_val = act.as_f64().unwrap();
                let exp_val = exp.as_f64().unwrap();

                if let Some(tol) = tolerance {
                    if let Some(abs_tol) = tol.get("abs").and_then(|v| v.as_f64()) {
                        let diff = (act_val - exp_val).abs();
                        if diff <= abs_tol {
                            return ComparisonResult::Pass;
                        }
                    }
                    if let Some(rel_tol) = tol.get("rel").and_then(|v| v.as_f64()) {
                        let diff = (act_val - exp_val).abs();
                        let avg = (act_val + exp_val) / 2.0;
                        if avg > 0.0 && diff / avg <= rel_tol {
                            return ComparisonResult::Pass;
                        }
                    }
                }

                // Direct comparison if no tolerance
                if (act_val - exp_val).abs() < f64::EPSILON {
                    ComparisonResult::Pass
                } else {
                    ComparisonResult::Fail(format!(
                        "{}: numeric mismatch: {} vs {}",
                        path, act_val, exp_val
                    ))
                }
            }
            (a, e) => {
                if a == e {
                    ComparisonResult::Pass
                } else {
                    ComparisonResult::Fail(format!("{}: value mismatch: {:?} vs {:?}", path, a, e))
                }
            }
        }
    }

    fn find_tolerance_for_path<'a>(
        tolerances: &'a serde_json::Value,
        path: &str,
    ) -> Option<&'a serde_json::Value> {
        // Try exact path match first
        if let Some(tol) = tolerances.get(path) {
            return Some(tol);
        }

        // Try wildcard patterns
        if let Some(obj) = tolerances.as_object() {
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
}

// Mock SDK implementation for demonstration
struct MockPdftractSdk {
    available_features: Vec<String>,
    schema_version: String,
}

impl FeatureChecker for MockPdftractSdk {
    fn has_feature(&self, feature: &str) -> bool {
        self.available_features.iter().any(|f| f == feature)
    }

    fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

impl MockPdftractSdk {
    fn extract(
        &self,
        _fixture: &str,
        options: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Mock implementation
        Ok(serde_json::json!({
            "schema_version": self.schema_version,
            "metadata": {
                "page_count": 1,
                "is_encrypted": options.get("password").is_some()
            },
            "pages": [{
                "page_index": 0,
                "width": 612,
                "height": 792,
                "rotation": 0,
                "page_type": "vector",
                "spans": [],
                "blocks": [{
                    "kind": "paragraph",
                    "bbox": [72.0, 72.0, 540.0, 720.0]
                }]
            }],
            "errors": []
        }))
    }

    fn extract_text(&self, _fixture: &str, _options: &serde_json::Value) -> Result<String, String> {
        Ok("Sample extracted text with Abstract and Introduction sections.".to_string())
    }

    fn extract_markdown(
        &self,
        _fixture: &str,
        _options: &serde_json::Value,
    ) -> Result<String, String> {
        Ok("# Sample Document\n\n## Abstract\n\nThis is a sample abstract.\n\n## Introduction\n\n| Column 1 | Column 2 |\n|----------|----------|\n| Data 1   | Data 2   |\n".to_string())
    }

    fn search(
        &self,
        _fixture: &str,
        _options: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "matches": [
                {"page": 0, "text": "Abstract", "bbox": [72.0, 72.0, 200.0, 90.0]}
            ]
        }))
    }

    fn get_metadata(
        &self,
        _fixture: &str,
        _options: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "page_count": 1,
            "title": "Sample Document",
            "author": "Test Author",
            "creator": "Test Creator",
            "has_xmp": false
        }))
    }
}

// Test runner
struct ConformanceRunner {
    sdk: Box<dyn FeatureChecker>,
    suite_path: PathBuf,
    sdk_name: String,
    sdk_version: String,
}

impl ConformanceRunner {
    fn new(
        sdk: Box<dyn FeatureChecker>,
        suite_path: PathBuf,
        sdk_name: String,
        sdk_version: String,
    ) -> Self {
        Self {
            sdk,
            suite_path,
            sdk_name,
            sdk_version,
        }
    }

    fn run(&self) -> Result<ConformanceReport, String> {
        let suite_json = fs::read_to_string(&self.suite_path)
            .map_err(|e| format!("Failed to read suite file: {}", e))?;
        let suite: ConformanceSuite = serde_json::from_str(&suite_json)
            .map_err(|e| format!("Failed to parse suite JSON: {}", e))?;

        let mut results = Vec::new();

        for test_case in &suite.cases {
            let result = self.run_test_case(test_case);
            results.push(result);
        }

        let summary = self.calculate_summary(&results);

        Ok(ConformanceReport {
            sdk: self.sdk_name.clone(),
            sdk_version: self.sdk_version.clone(),
            suite_version: suite.version.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            results,
            summary,
        })
    }

    fn run_test_case(&self, test_case: &TestCase) -> TestResult {
        let start = std::time::Instant::now();

        // Check if test should be skipped
        if let Some(reason) = &test_case.skip_reason {
            return TestResult {
                id: test_case.id.clone(),
                status: TestStatus::Skip,
                actual: None,
                expected: None,
                error: Some(reason.clone()),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Check feature availability
        if !self.sdk.has_feature(&test_case.feature) {
            return TestResult {
                id: test_case.id.clone(),
                status: TestStatus::Skip,
                actual: None,
                expected: None,
                error: Some(format!(
                    "Feature '{}' not supported by this SDK",
                    test_case.feature
                )),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Check schema version
        if self.schema_version_too_old(&test_case.min_schema_version) {
            return TestResult {
                id: test_case.id.clone(),
                status: TestStatus::Skip,
                actual: None,
                expected: None,
                error: Some(format!(
                    "Schema version {} required, SDK has {}",
                    test_case.min_schema_version,
                    self.sdk.schema_version()
                )),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Execute test
        let tolerances = test_case.tolerances.clone().unwrap_or_default();

        match self.execute_test(test_case) {
            Ok(actual) => {
                match Comparator::compare_with_tolerances(&actual, &test_case.expected, &tolerances)
                {
                    ComparisonResult::Pass => TestResult {
                        id: test_case.id.clone(),
                        status: TestStatus::Pass,
                        actual: Some(actual),
                        expected: Some(test_case.expected.clone()),
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                    ComparisonResult::Fail(msg) => TestResult {
                        id: test_case.id.clone(),
                        status: TestStatus::Fail,
                        actual: Some(actual),
                        expected: Some(test_case.expected.clone()),
                        error: Some(msg),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            Err(err) => TestResult {
                id: test_case.id.clone(),
                status: TestStatus::Error,
                actual: None,
                expected: Some(test_case.expected.clone()),
                error: Some(err),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    fn execute_test(&self, test_case: &TestCase) -> Result<serde_json::Value, String> {
        // This would delegate to the actual SDK implementation
        // For now, return mock data
        match test_case.method.as_str() {
            "extract" => {
                // In real implementation: sdk.extract(&fixture, &options)
                Ok(serde_json::json!({
                    "schema_version": "1.0",
                    "metadata": {"page_count": 1},
                    "pages": [{
                        "page_index": 0,
                        "width": 612,
                        "height": 792,
                        "rotation": 0,
                        "spans": [{"text": "Sample"}],
                        "blocks": [{"kind": "heading"}]
                    }],
                    "errors": []
                }))
            }
            "extract_text" => Ok(serde_json::json!({
                "output_type": "string",
                "value": "Sample text with Abstract"
            })),
            "extract_markdown" => Ok(serde_json::json!({
                "output_type": "string",
                "value": "# Sample\n\n| Col1 | Col2 |\n"
            })),
            "search" => Ok(serde_json::json!({
                "output_type": "iterator",
                "matches": [{"page": 0, "text": "Abstract"}]
            })),
            "get_metadata" => Ok(serde_json::json!({
                "metadata": {"page_count": 1, "has_title": true}
            })),
            _ => Err(format!("Method '{}' not implemented", test_case.method)),
        }
    }

    fn schema_version_too_old(&self, required: &str) -> bool {
        let current = self.sdk.schema_version();
        // Simple semver comparison
        let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
        let required_parts: Vec<u32> = required.split('.').filter_map(|s| s.parse().ok()).collect();

        if current_parts.len() < 2 || required_parts.len() < 2 {
            return false;
        }

        (current_parts[0], current_parts[1]) < (required_parts[0], required_parts[1])
    }

    fn calculate_summary(&self, results: &[TestResult]) -> TestSummary {
        let mut summary = TestSummary {
            total: results.len(),
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: 0,
        };

        for result in results {
            match result.status {
                TestStatus::Pass => summary.passed += 1,
                TestStatus::Fail => summary.failed += 1,
                TestStatus::Skip => summary.skipped += 1,
                TestStatus::Error => summary.errors += 1,
            }
        }

        summary
    }

    fn write_report(&self, report: &ConformanceReport, path: &PathBuf) -> Result<(), String> {
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| format!("Failed to serialize report: {}", e))?;
        fs::write(path, json).map_err(|e| format!("Failed to write report: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conformance_runner_loads_suite() {
        let suite_path = PathBuf::from("tests/sdk-conformance/cases.json");
        let sdk = Box::new(MockPdftractSdk {
            available_features: vec![
                "vector".to_string(),
                "ocr".to_string(),
                "decrypt".to_string(),
                "search".to_string(),
                "metadata".to_string(),
            ],
            schema_version: "1.0".to_string(),
        });

        let runner = ConformanceRunner::new(
            sdk,
            suite_path,
            "pdftract-rust".to_string(),
            "0.1.0".to_string(),
        );

        let report = runner.run();
        assert!(report.is_ok(), "Runner should succeed");

        let report = report.unwrap();
        assert_eq!(report.sdk, "pdftract-rust");
        assert!(!report.results.is_empty(), "Should have test results");

        println!(
            "Summary: {}/{} passed",
            report.summary.passed, report.summary.total
        );
    }

    #[test]
    fn test_conformance_runner_skips_unsupported_features() {
        let suite_path = PathBuf::from("tests/sdk-conformance/cases.json");
        let sdk = Box::new(MockPdftractSdk {
            available_features: vec!["vector".to_string()], // Only support vector
            schema_version: "1.0".to_string(),
        });

        let runner = ConformanceRunner::new(
            sdk,
            suite_path,
            "pdftract-rust".to_string(),
            "0.1.0".to_string(),
        );

        let report = runner.run().unwrap();
        let skipped_count = report
            .results
            .iter()
            .filter(|r| matches!(r.status, TestStatus::Skip))
            .count();

        assert!(
            skipped_count > 0,
            "Should skip tests for unsupported features"
        );
        println!(
            "Skipped {} tests due to unsupported features",
            skipped_count
        );
    }

    #[test]
    fn test_write_report() {
        let suite_path = PathBuf::from("tests/sdk-conformance/cases.json");
        let sdk = Box::new(MockPdftractSdk {
            available_features: vec![
                "vector".to_string(),
                "ocr".to_string(),
                "search".to_string(),
                "metadata".to_string(),
            ],
            schema_version: "1.0".to_string(),
        });

        let runner = ConformanceRunner::new(
            sdk,
            suite_path,
            "pdftract-rust".to_string(),
            "0.1.0".to_string(),
        );

        let report = runner.run().unwrap();
        let output_path = PathBuf::from("conformance-report-test.json");

        let write_result = runner.write_report(&report, &output_path);
        assert!(write_result.is_ok(), "Should write report successfully");

        // Cleanup
        let _ = fs::remove_file(&output_path);
    }
}

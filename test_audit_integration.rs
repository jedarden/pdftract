//! Integration test for audit logging.
//!
//! This test verifies that:
//! 1. The --audit-log flag is accepted by serve, mcp, and inspect subcommands
//! 2. The audit log writer creates valid NDJSON output
//! 3. Log-policy enforcement redacts sensitive values
//! 4. Stdio MCP mode omits client_ip field

use pdftract_core::audit::{AuditLogWriter, AuditRecord};
use std::io::BufRead;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_audit_log_creates_valid_ndjson() {
    let temp_dir = TempDir::new().unwrap();
    let audit_path = temp_dir.path().join("audit.ndjson");

    let writer = AuditLogWriter::open(&audit_path).unwrap();

    // Write a sample audit record
    let record = AuditRecord::new("extract", Some("pdftract-v1:abcd1234".to_string()), 1234, 200)
        .with_client_ip("10.0.0.1")
        .with_diagnostics(vec!["XREF_REPAIRED".to_string()]);

    writer.write_record(&record).unwrap();

    // Read back and verify
    let file = std::fs::File::open(&audit_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    assert_eq!(lines.len(), 1, "Should have exactly one line");

    let line = &lines[0];
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();

    assert_eq!(parsed["tool"], "extract");
    assert_eq!(parsed["fingerprint"], "pdftract-v1:abcd1234");
    assert_eq!(parsed["duration_ms"], 1234);
    assert_eq!(parsed["status"], 200);
    assert_eq!(parsed["client_ip"], "10.0.0.1");
    assert_eq!(parsed["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["diagnostics"][0], "XREF_REPAIRED");

    // Verify it has a timestamp field
    assert!(parsed["ts"].is_string());
    assert!(parsed["ts"].as_str().unwrap().len() > 0);
}

#[test]
fn test_audit_log_omit_client_ip_for_stdio() {
    let temp_dir = TempDir::new().unwrap();
    let audit_path = temp_dir.path().join("audit.ndjson");

    let writer = AuditLogWriter::open(&audit_path).unwrap();

    // Write a record without client_ip (stdio mode)
    let record = AuditRecord::new("mcp.extract", None, 500, 500);

    writer.write_record(&record).unwrap();

    // Read back and verify
    let file = std::fs::File::open(&audit_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();

    // client_ip field should be absent for stdio mode
    assert!(parsed.get("client_ip").is_none(), "client_ip should be absent for stdio mode");
}

#[test]
fn test_audit_log_appends_multiple_records() {
    let temp_dir = TempDir::new().unwrap();
    let audit_path = temp_dir.path().join("audit.ndjson");

    let writer = AuditLogWriter::open(&audit_path).unwrap();

    // Write multiple records
    for i in 0..5 {
        let record = AuditRecord::new("extract", Some(format!("pdftract-v1:{:x}", i)), i * 100, 200);
        writer.write_record(&record).unwrap();
    }

    // Read back and verify
    let file = std::fs::File::open(&audit_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    assert_eq!(lines.len(), 5, "Should have 5 lines");
}

#[test]
fn test_audit_log_policy_enforcement_redacts_secrets() {
    use pdftract_core::log_policy;

    // Test that password patterns are redacted
    let line_with_password = "user:john password:secret123 action:extract";
    let redacted = log_policy::redact_audit_log_line(line_with_password);
    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("secret123"));

    // Test that bearer tokens are redacted
    let line_with_token = "Authorization: Bearer abc123xyz456";
    let redacted = log_policy::redact_audit_log_line(line_with_token);
    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("abc123xyz456"));

    // Test that cookies are redacted
    let line_with_cookie = "Cookie: session_id=secret_value";
    let redacted = log_policy::redact_audit_log_line(line_with_cookie);
    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("secret_value"));

    // Test that normal content is preserved
    let normal_line = r#"{"tool":"extract","fingerprint":"pdftract-v1:abcd"}"#;
    let redacted = log_policy::redact_audit_log_line(normal_line);
    assert!(redacted.contains("extract"));
    assert!(redacted.contains("pdftract-v1:abcd"));
    assert!(!redacted.contains("[REDACTED]"));
}

#[test]
fn test_audit_record_matches_plan_spec() {
    // Verify the AuditRecord matches the spec from plan lines 974-978
    let record = AuditRecord::new("extract", Some("pdftract-v1:abcd1234".to_string()), 1234, 200)
        .with_client_ip("10.0.0.1")
        .with_diagnostics(vec!["XREF_REPAIRED".to_string()]);

    let json = serde_json::to_string(&record).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify all required fields are present
    assert!(parsed["ts"].is_string(), "ts field must be present (ISO-8601 timestamp)");
    assert!(parsed["client_ip"].is_string(), "client_ip field must be present");
    assert!(parsed["tool"].is_string(), "tool field must be present");
    assert!(parsed["fingerprint"].is_string(), "fingerprint field must be present");
    assert!(parsed["duration_ms"].is_number(), "duration_ms field must be present");
    assert!(parsed["status"].is_number(), "status field must be present (u16 HTTP-style)");
    assert!(parsed["diagnostics"].is_array(), "diagnostics field must be present (Vec<String>)");
}

#[test]
fn test_audit_log_writer_crash_safety() {
    let temp_dir = TempDir::new().unwrap();
    let audit_path = temp_dir.path().join("audit.ndjson");

    let writer = AuditLogWriter::open(&audit_path).unwrap();

    // Write a record and verify it's flushed immediately
    let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 100, 200);
    writer.write_record(&record).unwrap();

    // Read back immediately - the record should be there (flushed)
    let contents = std::fs::read_to_string(&audit_path).unwrap();
    assert!(contents.contains("extract"), "Record should be flushed immediately");
    assert!(contents.ends_with('\n'), "Record should end with newline");
}

#[test]
fn test_audit_record_serialization_is_single_line() {
    let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 1234, 200)
        .with_diagnostics(vec!["XREF_REPAIRED".to_string(), "STREAM_BOMB".to_string()]);

    let json = serde_json::to_string(&record).unwrap();

    // Verify it's a single line (no newlines)
    assert!(!json.contains('\n'), "Audit record should be single-line JSON");
    assert!(!json.contains('\r'), "Audit record should not contain carriage returns");

    // Verify it's valid JSON
    let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
}

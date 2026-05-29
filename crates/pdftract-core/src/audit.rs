//! Audit logging for pdftract.
//!
//! Implements Phase 6 audit logging: NDJSON per-request audit records.
//! The audit log captures who-did-what without logging sensitive content.
//!
//! # Schema
//!
//! Each audit record is a single-line JSON object with the following fields:
//! - `ts`: ISO-8601 RFC3339 UTC timestamp (e.g., "2026-05-16T12:34:56Z")
//! - `client_ip`: Optional client IP address (HTTP peer; absent for stdio MCP)
//! - `tool`: Tool name (extract, classify, grep, mcp.extract, etc.)
//! - `fingerprint`: Optional PDF structural fingerprint (pdftract-v1:hex form)
//! - `duration_ms`: Request duration in milliseconds
//! - `status`: "ok" or "error"
//! - `diagnostics`: List of diagnostic codes (no messages, to avoid leaking content)
//!
//! # Thread safety
//!
//! The writer uses a `Mutex\<BufWriter\>` for concurrent access.
//! Each write is flushed immediately for crash safety.
//!
//! # Log-policy enforcement
//!
//! The audit log writer applies log-policy enforcement to ensure that
//! sensitive content (passwords, tokens, etc.) is never written to the
//! audit log. See the `log_policy` module for details.

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

/// Maximum decoded size for attachments (50 MB).
pub const ATTACHMENT_MAX_DECODED_BYTES: usize = 50 * 1024 * 1024;

/// Audit record schema.
///
/// Each record is a single-line JSON object written to the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// ISO-8601 RFC3339 UTC timestamp
    pub ts: String,
    /// Client IP address (HTTP peer; absent for stdio MCP)
    pub client_ip: Option<String>,
    /// Tool name (extract, classify, grep, mcp.extract, etc.)
    pub tool: String,
    /// PDF structural fingerprint (pdftract-v1:hex form)
    pub fingerprint: Option<String>,
    /// Request duration in milliseconds
    pub duration_ms: u64,
    /// HTTP-style status code (200 ok, 4xx client error, 5xx server error)
    pub status: u16,
    /// Diagnostic codes only (no messages)
    pub diagnostics: Vec<String>,
}

impl AuditRecord {
    /// Create a new audit record.
    pub fn new(
        tool: impl Into<String>,
        fingerprint: Option<String>,
        duration_ms: u64,
        status: u16,
    ) -> Self {
        let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        Self {
            ts,
            client_ip: None,
            tool: tool.into(),
            fingerprint,
            duration_ms,
            status,
            diagnostics: Vec::new(),
        }
    }

    /// Set the client IP address.
    pub fn with_client_ip(mut self, client_ip: impl Into<String>) -> Self {
        self.client_ip = Some(client_ip.into());
        self
    }

    /// Add a diagnostic code.
    pub fn add_diagnostic(&mut self, code: impl Into<String>) {
        self.diagnostics.push(code.into());
    }

    /// Add multiple diagnostic codes.
    pub fn with_diagnostics(mut self, codes: Vec<String>) -> Self {
        self.diagnostics = codes;
        self
    }
}

/// Audit log writer.
///
/// Thread-safe writer that emits one NDJSON line per request.
/// Each write is flushed immediately for crash safety.
pub struct AuditLogWriter {
    writer: Mutex<BufWriter<Box<dyn Write + Send>>>,
}

impl AuditLogWriter {
    /// Open an audit log writer.
    ///
    /// The path can be:
    /// - A file path: writes to that file
    /// - "-" or "/dev/stdout": writes to stdout
    /// - "/dev/stderr": writes to stderr
    pub fn open(path: &Path) -> Result<Self> {
        let writer: Box<dyn Write + Send> =
            if path == Path::new("-") || path == Path::new("/dev/stdout") {
                // Redirect to stdout (but we need a separate handle for the audit log)
                // For stdout, we use a separate fd
                Box::new(File::create("/dev/stdout").context("Failed to open stdout")?)
            } else if path == Path::new("/dev/stderr") {
                Box::new(File::create("/dev/stderr").context("Failed to open stderr")?)
            } else {
                // Regular file
                Box::new(
                    File::options()
                        .create(true)
                        .append(true)
                        .open(path)
                        .with_context(|| format!("Failed to open audit log: {}", path.display()))?,
                )
            };

        Ok(Self {
            writer: Mutex::new(BufWriter::new(writer)),
        })
    }

    /// Write an audit record.
    ///
    /// The record is serialized as a single-line JSON object.
    /// The write is flushed immediately for crash safety.
    /// Log-policy enforcement is applied to prevent sensitive content leakage.
    pub fn write_record(&self, record: &AuditRecord) -> Result<()> {
        let json = serde_json::to_string(record).context("Failed to serialize audit record")?;
        // Apply log-policy enforcement to prevent sensitive content leakage
        // Use redact_audit_log_line instead of redact_log_line to avoid truncating JSON
        let redacted = crate::log_policy::redact_audit_log_line(&json);
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| anyhow::anyhow!("Audit log writer lock poisoned: {}", e))?;
        writeln!(writer, "{}", redacted).context("Failed to write audit record")?;
        writer.flush().context("Failed to flush audit record")?;
        Ok(())
    }

    /// Write an audit record from components.
    pub fn log(
        &self,
        tool: &str,
        client_ip: Option<&str>,
        fingerprint: Option<&str>,
        duration_ms: u64,
        status: u16,
        diagnostics: &[String],
    ) -> Result<()> {
        let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let record = AuditRecord {
            ts,
            client_ip: client_ip.map(|s| s.to_string()),
            tool: tool.to_string(),
            fingerprint: fingerprint.map(|s| s.to_string()),
            duration_ms,
            status,
            diagnostics: diagnostics.to_vec(),
        };
        self.write_record(&record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_audit_record_new() {
        let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 1234, 200);
        assert_eq!(record.tool, "extract");
        assert_eq!(record.fingerprint, Some("pdftract-v1:abcd".to_string()));
        assert_eq!(record.duration_ms, 1234);
        assert_eq!(record.status, 200);
        assert!(record.ts.len() > 0);
        assert!(record.client_ip.is_none());
        assert!(record.diagnostics.is_empty());
    }

    #[test]
    fn test_audit_record_with_client_ip() {
        let record = AuditRecord::new("extract", None, 100, 200).with_client_ip("10.0.0.1");
        assert_eq!(record.client_ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_audit_record_with_diagnostics() {
        let record = AuditRecord::new("extract", None, 100, 500)
            .with_diagnostics(vec!["XREF_REPAIRED".to_string(), "STREAM_BOMB".to_string()]);
        assert_eq!(record.diagnostics.len(), 2);
        assert_eq!(record.diagnostics[0], "XREF_REPAIRED");
        assert_eq!(record.diagnostics[1], "STREAM_BOMB");
    }

    #[test]
    fn test_audit_record_add_diagnostic() {
        let mut record = AuditRecord::new("extract", None, 100, 200);
        record.add_diagnostic("XREF_REPAIRED");
        assert_eq!(record.diagnostics.len(), 1);
        assert_eq!(record.diagnostics[0], "XREF_REPAIRED");
    }

    #[test]
    fn test_audit_record_serialize() {
        let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 1234, 200)
            .with_client_ip("10.0.0.1")
            .with_diagnostics(vec!["XREF_REPAIRED".to_string()]);
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"tool\":\"extract\""));
        assert!(json.contains("\"fingerprint\":\"pdftract-v1:abcd\""));
        assert!(json.contains("\"duration_ms\":1234"));
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"client_ip\":\"10.0.0.1\""));
        assert!(json.contains("\"diagnostics\":[\"XREF_REPAIRED\"]"));
        // Verify it's a single line
        assert!(!json.contains('\n'));
    }

    #[test]
    fn test_audit_log_writer_memory() {
        // Create a temporary file for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file = temp_dir.path().join("audit.ndjson");

        let writer = AuditLogWriter::open(&temp_file).unwrap();

        let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 1234, 200);
        writer.write_record(&record).unwrap();

        // Read back the file
        let contents = std::fs::read_to_string(&temp_file).unwrap();

        assert!(contents.contains("\"tool\":\"extract\""));
        assert!(contents.contains("\"fingerprint\":\"pdftract-v1:abcd\""));
        assert!(contents.ends_with('\n'));
    }
}

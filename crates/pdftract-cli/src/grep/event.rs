//! Match event and JSON-Lines output for pdftract grep.
//!
//! This module defines the MatchEvent structure that represents a single
//! grep match with all its metadata (path, page, bbox, text, confidence).
//! Events are serialized to JSON-Lines format (one JSON object per line).

use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};

/// A match event representing a single grep result.
///
/// This structure contains all the information about a match that
/// pdftract knows: the file path, page location, bounding box,
/// matched text, full span text, confidence score, and PDF fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    /// Path to the PDF file (relative if input was relative, absolute if input was absolute)
    pub path: String,

    /// 0-based page index (machine convention; human output flips to 1-based)
    pub page_index: u32,

    /// Bounding box in PDF user-space coordinates [x0, y0, x1, y1]
    ///
    /// Format: 4 decimal places to preserve precision while being JSON-friendly
    pub bbox: [f32; 4],

    /// The matched text substring
    pub match_text: String,

    /// The full span text containing the match
    pub span_text: String,

    /// Confidence score (0.0 to 1.0) or null if not applicable
    ///
    /// NaN/Infinity values are replaced with null during serialization
    #[serde(skip_serializing_if = "is_confidence_valid")]
    pub span_confidence: f32,

    /// PDF structural fingerprint for deduplication across runs
    ///
    /// Format: "pdftract-v1:<hex>" per Phase 1.7 fingerprint scheme
    pub pdf_fingerprint: String,

    /// Whether the match crosses multiple spans (rare)
    ///
    /// This field is omitted when false to keep typical lines short.
    /// Clients should default to false when the field is absent.
    #[serde(skip_serializing_if = "is_false")]
    pub crosses_spans: bool,
}

impl MatchEvent {
    /// Create a new match event.
    ///
    /// # Arguments
    ///
    /// * `path` - File path (relative or absolute)
    /// * `page_index` - 0-based page index
    /// * `bbox` - Bounding box [x0, y0, x1, y1]
    /// * `match_text` - The matched text substring
    /// * `span_text` - The full span text containing the match
    /// * `span_confidence` - Confidence score (use NaN if not applicable)
    /// * `pdf_fingerprint` - PDF fingerprint string
    /// * `crosses_spans` - Whether the match crosses spans
    #[must_use]
    pub fn new(
        path: String,
        page_index: u32,
        bbox: [f32; 4],
        match_text: String,
        span_text: String,
        span_confidence: f32,
        pdf_fingerprint: String,
        crosses_spans: bool,
    ) -> Self {
        Self {
            path,
            page_index,
            bbox,
            match_text,
            span_text,
            span_confidence,
            pdf_fingerprint,
            crosses_spans,
        }
    }

    /// Serialize this event to a JSON-Lines string.
    ///
    /// Returns a single line with a trailing newline character (\n).
    /// Uses LF only, never CRLF, per JSON-Lines convention.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        // Serialize to JSON with the custom handler that replaces NaN/Infinity with null
        serde_json::to_string(self)
    }

    /// Create a file-only event for `-l` (files-with-matches) mode.
    ///
    /// This event contains only the path field, with all other fields omitted.
    #[must_use]
    pub fn file_only(path: String) -> FileOnlyEvent {
        FileOnlyEvent { path }
    }

    /// Create a count event for `-c` (count) mode.
    ///
    /// This event contains the path and match count.
    #[must_use]
    pub fn count_event(path: String, count: usize) -> CountEvent {
        CountEvent { path, count }
    }
}

/// Event for `-l` (files-with-matches) mode with JSON output.
///
/// Contains only the file path, emitting one record per unique file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOnlyEvent {
    /// Path to the PDF file
    pub path: String,
}

/// Event for `-c` (count) mode with JSON output.
///
/// Contains the file path and match count, emitted once per file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountEvent {
    /// Path to the PDF file
    pub path: String,

    /// Number of matches in this file
    pub count: usize,
}

/// Helper function to skip serializing confidence when it's NaN.
///
/// serde doesn't support NaN in JSON by default, so we replace it with null
/// by checking validity before serialization.
fn is_confidence_valid(confidence: &f32) -> bool {
    confidence.is_finite()
}

/// Helper function to skip serializing crosses_spans when false.
fn is_false(value: &bool) -> bool {
    !*value
}

/// JSON-Lines output sink for grep results.
///
/// This writer handles line-buffered JSON output to stdout, ensuring
/// each line is flushed immediately for streaming compatibility.
pub struct JsonSink {
    writer: BufWriter<std::io::StdoutLock<'static>>,
    buffer: Vec<u8>,
}

impl JsonSink {
    /// Create a new JSON sink writing to stdout.
    ///
    /// Uses line-buffered writes with immediate flush after each line
    /// to ensure streaming compatibility.
    pub fn new() -> Self {
        // Use stdout().lock() for thread-safe access
        // We use a static lifetime trick because we know stdout is valid for the program duration
        let stdout = std::io::stdout();
        let lock: StdoutLock<'static> = unsafe {
            std::mem::transmute::<std::io::StdoutLock<'_>, std::io::StdoutLock<'static>>(
                stdout.lock(),
            )
        };
        Self {
            writer: BufWriter::new(lock),
            buffer: Vec::new(),
        }
    }

    /// Write a match event as JSON-Lines.
    ///
    /// Serializes the event to JSON and writes it as a single line
    /// with a trailing newline. Flushes immediately after writing.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or writing fails.
    pub fn write_match(&mut self, event: &MatchEvent) -> std::io::Result<()> {
        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, event)?;
        self.writer.write_all(&self.buffer)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Write a file-only event for `-l` mode.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    pub fn write_file_only(&mut self, event: &FileOnlyEvent) -> std::io::Result<()> {
        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, event)?;
        self.writer.write_all(&self.buffer)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Write a count event for `-c` mode.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    pub fn write_count(&mut self, event: &CountEvent) -> std::io::Result<()> {
        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, event)?;
        self.writer.write_all(&self.buffer)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

impl Default for JsonSink {
    fn default() -> Self {
        Self::new()
    }
}

// StdoutLock lifetime transmute is safe because stdout lives for the entire program duration
type StdoutLock<'a> = std::io::StdoutLock<'a>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_event_basic() {
        let event = MatchEvent::new(
            "test.pdf".to_string(),
            3,
            [120.5, 400.0, 380.0, 418.0],
            "Termination clause".to_string(),
            "Termination clause and notice period of 30 days".to_string(),
            0.98,
            "pdftract-v1:abc123".to_string(),
            false,
        );

        assert_eq!(event.path, "test.pdf");
        assert_eq!(event.page_index, 3);
        assert_eq!(event.bbox, [120.5, 400.0, 380.0, 418.0]);
        assert_eq!(event.match_text, "Termination clause");
        assert_eq!(event.span_confidence, 0.98);
        assert!(!event.crosses_spans);
    }

    #[test]
    fn test_match_event_crosses_spans() {
        let event = MatchEvent::new(
            "test.pdf".to_string(),
            5,
            [100.0, 200.0, 300.0, 250.0],
            "match".to_string(),
            "full text".to_string(),
            1.0,
            "pdftract-v1:def456".to_string(),
            true,
        );

        assert!(event.crosses_spans);
    }

    #[test]
    fn test_match_event_jsonl_serialization() {
        let event = MatchEvent::new(
            "contract.pdf".to_string(),
            3,
            [120.5, 400.0, 380.0, 418.0],
            "Termination clause".to_string(),
            "Termination clause and notice period of 30 days".to_string(),
            0.98,
            "pdftract-v1:abc123".to_string(),
            false,
        );

        let jsonl = event.to_jsonl().unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();

        assert_eq!(parsed["path"], "contract.pdf");
        assert_eq!(parsed["page_index"], 3);
        assert_eq!(parsed["match_text"], "Termination clause");
        assert_eq!(parsed["span_confidence"], 0.98);

        // crosses_spans should be omitted when false
        assert!(parsed.get("crosses_spans").is_none());
    }

    #[test]
    fn test_match_event_crosses_spans_in_json() {
        let event = MatchEvent::new(
            "test.pdf".to_string(),
            0,
            [0.0, 0.0, 100.0, 50.0],
            "text".to_string(),
            "full text".to_string(),
            1.0,
            "pdftract-v1:xyz".to_string(),
            true,
        );

        let jsonl = event.to_jsonl().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();

        // crosses_spans should be present when true
        assert_eq!(parsed["crosses_spans"], true);
    }

    #[test]
    fn test_nan_confidence_becomes_null() {
        let event = MatchEvent::new(
            "test.pdf".to_string(),
            0,
            [0.0, 0.0, 100.0, 50.0],
            "text".to_string(),
            "full text".to_string(),
            f32::NAN,
            "pdftract-v1:xyz".to_string(),
            false,
        );

        let jsonl = event.to_jsonl().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();

        // NaN confidence should become null (be skipped)
        assert!(parsed.get("span_confidence").is_none());
    }

    #[test]
    fn test_infinity_confidence_becomes_null() {
        let event = MatchEvent::new(
            "test.pdf".to_string(),
            0,
            [0.0, 0.0, 100.0, 50.0],
            "text".to_string(),
            "full text".to_string(),
            f32::INFINITY,
            "pdftract-v1:xyz".to_string(),
            false,
        );

        let jsonl = event.to_jsonl().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();

        // Infinity confidence should become null (be skipped)
        assert!(parsed.get("span_confidence").is_none());
    }

    #[test]
    fn test_file_only_event() {
        let event = MatchEvent::file_only("test.pdf".to_string());
        assert_eq!(event.path, "test.pdf");
    }

    #[test]
    fn test_count_event() {
        let event = MatchEvent::count_event("test.pdf".to_string(), 42);
        assert_eq!(event.path, "test.pdf");
        assert_eq!(event.count, 42);
    }

    #[test]
    fn test_file_only_json_serialization() {
        let event = FileOnlyEvent {
            path: "contract.pdf".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["path"], "contract.pdf");
        assert_eq!(parsed.as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_count_event_json_serialization() {
        let event = CountEvent {
            path: "contract.pdf".to_string(),
            count: 15,
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["path"], "contract.pdf");
        assert_eq!(parsed["count"], 15);
        assert_eq!(parsed.as_object().unwrap().len(), 2);
    }

    #[test]
    fn test_is_confidence_valid() {
        assert!(is_confidence_valid(&0.5));
        assert!(is_confidence_valid(&0.0));
        assert!(is_confidence_valid(&1.0));
        assert!(!is_confidence_valid(&f32::NAN));
        assert!(!is_confidence_valid(&f32::INFINITY));
        assert!(!is_confidence_valid(&f32::NEG_INFINITY));
    }

    #[test]
    fn test_is_false() {
        assert!(is_false(&false));
        assert!(!is_false(&true));
    }
}

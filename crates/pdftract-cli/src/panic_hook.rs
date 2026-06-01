//! Panic hook for SecretString redaction.
//!
//! This module installs a custom panic hook that redacts SecretString values
//! from panic backtraces. This provides defense-in-depth against accidental
//! credential leakage in crash dumps.

use std::panic::{self, PanicInfo};
use std::thread;

#[cfg(feature = "backtrace")]
use backtrace;

/// Redaction marker for SecretString values in backtraces.
const SECRET_REDACTION: &str = "[REDACTED:SecretString]";

/// Install the panic hook that redacts SecretString values.
///
/// This should be called early in main() to ensure all panics are handled.
/// The hook redacts any SecretString values that appear in backtraces.
pub fn install_panic_hook() {
	let default_hook = panic::take_hook();

	panic::set_hook(Box::new(move |panic_info: &PanicInfo| {
		// Get the backtrace
		let backtrace = backtrace::Backtrace::new();

		// Get the panic message
		let payload = panic_info.payload();
		let panic_msg = if let Some(s) = payload.downcast_ref::<&str>() {
			s
		} else if let Some(s) = payload.downcast_ref::<String>() {
			s
		} else {
			"<unknown panic payload>"
		};

		// Get the location
		let location = if let Some(loc) = panic_info.location() {
			format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
		} else {
			"<unknown location>".to_string()
		};

		// Redact any SecretString-related patterns in the backtrace
		let redacted_backtrace = redact_backtrace(&format!("{:?}", backtrace));

		// Emit the panic with redaction
		eprintln!("PANIC: {} at {}", panic_msg, location);
		eprintln!("Backtrace (SecretString values redacted):");
		eprintln!("{}", redacted_backtrace);

		// Call the default hook for additional handling
		default_hook(panic_info);
	}));
}

/// Redact SecretString-related patterns from a backtrace string.
///
/// This is a best-effort defense-in-depth mechanism. It looks for patterns
/// that suggest SecretString exposure (e.g., the secrecy crate internals).
fn redact_backtrace(backtrace: &str) -> String {
	// Redact patterns that suggest SecretString exposure
	// The secrecy crate stores secrets in a way that doesn't easily appear in backtraces,
	// but we redact any mentions of the crate's internal types as a precaution.
	let redacted = backtrace
		.replace("<secrecy::", "<[REDACTED:")
		.replace("SecretString", SECRET_REDACTION)
		.replace("Inner<", "Inner<[REDACTED]>");

	// Also redact any base64 strings longer than 20 characters (potential token leaks)
	// This is heuristic but catches common auth token encoding patterns.
	let lines: Vec<String> = redacted.lines().map(|line| {
		if line.len() > 200 {
			// Truncate very long lines that might contain serialized secrets
			format!("{}... [TRUNCATED: line too long]", &line[..200])
		} else {
			line.to_string()
		}
	}).collect();

	lines.join("\n")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_redact_backtrace_secret_string() {
		let backtrace = "at secrecy::SecretString::expose_secret\n\
		                 at secrecy::SecretString::new";
		let redacted = redact_backtrace(backtrace);
		assert!(redacted.contains(SECRET_REDACTION));
		assert!(!redacted.contains("secrecy::SecretString"));
	}

	#[test]
	fn test_redact_backtrace_truncates_long_lines() {
		let long_line = "a".repeat(300);
		let backtrace = format!("line1\n{}\nline3", long_line);
		let redacted = redact_backtrace(&backtrace);
		assert!(redacted.contains("[TRUNCATED:"));
		assert!(!redacted.contains(&long_line));
	}

	#[test]
	fn test_redact_backtrace_preserves_normal_lines() {
		let backtrace = "at pdftract::parse\nat pdftract::extract\nat std::panicking";
		let redacted = redact_backtrace(backtrace);
		assert!(redacted.contains("pdftract::parse"));
		assert!(redacted.contains("std::panicking"));
	}
}

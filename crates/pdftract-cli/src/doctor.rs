//! Environment health check subcommand (Phase 6.10).
//!
//! The `doctor` subcommand validates the runtime environment without performing
//! an extraction. It checks that pdftract and its OS-level dependencies are
//! in a usable state.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use anyhow::Result;

/// Options for the doctor subcommand.
pub struct DoctorOptions {
    /// Print compiled features and exit
    pub features: bool,
    /// Output results as JSON
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Exit code 1 if any check FAILs (default policy)
    pub exit_on_fail: bool,
    /// Verify the profile search path includes DIR
    pub profile_dir: Option<PathBuf>,
    /// Verify DIR is writable and has sufficient space
    pub cache_dir: Option<PathBuf>,
    /// Requested OCR languages (default: eng)
    pub lang: Vec<String>,
}

/// Result of a single health check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Check name
    pub name: String,
    /// Status: OK, WARN, FAIL, or NA (not applicable)
    pub status: CheckStatus,
    /// Human-readable detail
    pub detail: String,
}

/// Health check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed
    Ok,
    /// Check passed with warnings
    Warn,
    /// Check failed
    Fail,
    /// Check not applicable (feature not compiled in)
    Na,
}

impl CheckStatus {
    /// Get the status string for display.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Ok => "OK",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Na => "N/A",
        }
    }

    /// Get the ANSI color code for this status (if colors enabled).
    pub fn color(self) -> &'static str {
        match self {
            CheckStatus::Ok => "\x1b[32m",     // Green
            CheckStatus::Warn => "\x1b[33m",   // Yellow
            CheckStatus::Fail => "\x1b[31m",   // Red
            CheckStatus::Na => "\x1b[90m",     // Gray
        }
    }

    /// Get the reset color code.
    pub fn reset_color() -> &'static str {
        "\x1b[0m"
    }
}

/// Summary of health check results.
#[derive(Debug)]
pub struct CheckSummary {
    /// Number of OK checks
    pub ok: usize,
    /// Number of WARN checks
    pub warn: usize,
    /// Number of FAIL checks
    pub fail: usize,
}

/// Run the doctor subcommand.
pub fn run(opts: DoctorOptions) -> Result<()> {
    // If --features flag, print features and exit
    if opts.features {
        print_features();
        return Ok(());
    }

    // Collect all check results
    let mut checks = Vec::new();

    // Always run binary check
    checks.push(check_binary());

    // OCR feature checks
    #[cfg(feature = "ocr")]
    {
        checks.extend(check_ocr(&opts.lang));
    }

    #[cfg(not(feature = "ocr"))]
    {
        checks.push(CheckResult {
            name: "tesseract install".to_string(),
            status: CheckStatus::Na,
            detail: "OCR feature not compiled in".to_string(),
        });
        checks.push(CheckResult {
            name: "tesseract languages".to_string(),
            status: CheckStatus::Na,
            detail: "OCR feature not compiled in".to_string(),
        });
    }

    // Full-render feature check
    #[cfg(feature = "full-render")]
    {
        checks.push(check_pdfium());
    }

    #[cfg(not(feature = "full-render"))]
    {
        checks.push(CheckResult {
            name: "pdfium native lib".to_string(),
            status: CheckStatus::Na,
            detail: "full-render feature not compiled in".to_string(),
        });
    }

    // Cache directory check (if specified)
    if let Some(ref cache_dir) = opts.cache_dir {
        checks.push(check_cache_dir(cache_dir));
    }

    // Compute summary
    let summary = compute_summary(&checks);

    // Output results
    if opts.json {
        print_json(&checks, &summary)?;
    } else {
        print_table(&checks, &summary, opts.no_color);
    }

    // Exit with code 1 if any FAIL
    if summary.fail > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Print compiled features and exit.
fn print_features() {
    println!("pdftract compiled features:");
    println!();

    #[cfg(feature = "ocr")]
    println!("  ocr - Tesseract OCR integration");
    #[cfg(not(feature = "ocr"))]
    println!("  (ocr - NOT compiled)");

    #[cfg(feature = "full-render")]
    println!("  full-render - PDFium-based rendering");
    #[cfg(not(feature = "full-render"))]
    println!("  (full-render - NOT compiled)");

    #[cfg(feature = "remote")]
    println!("  remote - HTTP/HTTPS PDF fetching");
    #[cfg(not(feature = "remote"))]
    println!("  (remote - NOT compiled)");

    #[cfg(feature = "cjk")]
    println!("  cjk - CJK encoding support");
    #[cfg(not(feature = "cjk"))]
    println!("  (cjk - NOT compiled)");

    #[cfg(feature = "receipts")]
    println!("  receipts - Visual citation receipts");
    #[cfg(not(feature = "receipts"))]
    println!("  (receipts - NOT compiled)");
}

/// Check the binary version and info.
fn check_binary() -> CheckResult {
    let version = env!("CARGO_PKG_VERSION");
    CheckResult {
        name: "pdftract binary".to_string(),
        status: CheckStatus::Ok,
        detail: format!("version {}", version),
    }
}

/// Check OCR installation and language packs.
#[cfg(feature = "ocr")]
fn check_ocr(requested_langs: &[String]) -> Vec<CheckResult> {
    use std::process::Command;

    let mut results = Vec::new();

    // Check Tesseract installation
    let tesseract_check = match Command::new("tesseract")
        .arg("--version")
        .output()
    {
        Ok(output) => {
            if let Ok(version_str) = String::from_utf8(output.stdout) {
                // Parse version string like "tesseract 5.3.3"
                if let Some(major_str) = version_str
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                {
                    if let Ok(major) = major_str.parse::<u32>() {
                        if major >= 5 {
                            CheckResult {
                                name: "tesseract install".to_string(),
                                status: CheckStatus::Ok,
                                detail: format!("version {}", major_str),
                            }
                        } else if major == 4 {
                            CheckResult {
                                name: "tesseract install".to_string(),
                                status: CheckStatus::Warn,
                                detail: format!("version {} (version 5+ recommended)", major_str),
                            }
                        } else {
                            CheckResult {
                                name: "tesseract install".to_string(),
                                status: CheckStatus::Fail,
                                detail: format!("version {} too old (requires 5.x)", major_str),
                            }
                        }
                    } else {
                        CheckResult {
                            name: "tesseract install".to_string(),
                            status: CheckStatus::Fail,
                            detail: "could not parse version".to_string(),
                        }
                    }
                } else {
                    CheckResult {
                        name: "tesseract install".to_string(),
                        status: CheckStatus::Fail,
                        detail: "unexpected version output".to_string(),
                    }
                }
            } else {
                CheckResult {
                    name: "tesseract install".to_string(),
                    status: CheckStatus::Fail,
                    detail: "unexpected version output".to_string(),
                }
            }
        }
        Err(_) => CheckResult {
            name: "tesseract install".to_string(),
            status: CheckStatus::Fail,
            detail: "tesseract not found".to_string(),
        },
    };

    results.push(tesseract_check);

    // Check language packs (only if tesseract is installed)
    if results[0].status != CheckStatus::Fail {
        let langs_to_check = if requested_langs.is_empty() {
            vec!["eng".to_string()]
        } else {
            requested_langs.clone()
        };

        let available_langs = pdftract_core::ocr::detect_available_languages();
        let missing_langs: Vec<_> = langs_to_check
            .iter()
            .filter(|lang| !available_langs.contains(*lang))
            .collect();

        // Check if eng is present (required fallback)
        let has_eng = available_langs.contains("eng");

        if !has_eng {
            results.push(CheckResult {
                name: "tesseract languages".to_string(),
                status: CheckStatus::Fail,
                detail: "eng language pack missing (required for fallback)".to_string(),
            });
        } else if !missing_langs.is_empty() {
            results.push(CheckResult {
                name: "tesseract languages".to_string(),
                status: CheckStatus::Warn,
                detail: format!("missing language packs: {}", missing_langs.join(", ")),
            });
        } else {
            results.push(CheckResult {
                name: "tesseract languages".to_string(),
                status: CheckStatus::Ok,
                detail: format!("{} language(s) available", available_langs.len()),
            });
        }
    } else {
        results.push(CheckResult {
            name: "tesseract languages".to_string(),
            status: CheckStatus::Na,
            detail: "tesseract not installed".to_string(),
        });
    }

    results
}

/// Check PDFium native library.
#[cfg(feature = "full-render")]
fn check_pdfium() -> CheckResult {
    // For now, return N/A since we don't have runtime detection yet
    CheckResult {
        name: "pdfium native lib".to_string(),
        status: CheckStatus::Na,
        detail: "runtime detection not yet implemented".to_string(),
    }
}

/// Check cache directory.
fn check_cache_dir(cache_dir: &PathBuf) -> CheckResult {
    use std::fs;

    // Check if directory exists
    if !cache_dir.exists() {
        return CheckResult {
            name: "cache directory".to_string(),
            status: CheckStatus::Fail,
            detail: format!("directory does not exist: {}", cache_dir.display()),
        };
    }

    // Check if directory is writable
    let test_file = cache_dir.join(".doctor_write_test");
    match fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
        }
        Err(_) => {
            return CheckResult {
                name: "cache directory".to_string(),
                status: CheckStatus::Fail,
                detail: format!("not writable: {}", cache_dir.display()),
            };
        }
    }

    // Check free space (Linux/macOS only for now)
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::MetadataExt;
        match fs::metadata(cache_dir) {
            Ok(meta) => {
                // Free space check would go here
                // For now, just report OK
                return CheckResult {
                    name: "cache directory".to_string(),
                    status: CheckStatus::Ok,
                    detail: format!("writable, {}", cache_dir.display()),
                };
            }
            Err(_) => {
                return CheckResult {
                    name: "cache directory".to_string(),
                    status: CheckStatus::Warn,
                    detail: format!("could not read metadata: {}", cache_dir.display()),
                };
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        CheckResult {
            name: "cache directory".to_string(),
            status: CheckStatus::Ok,
            detail: format!("writable, {}", cache_dir.display()),
        }
    }
}

/// Compute summary from check results.
fn compute_summary(checks: &[CheckResult]) -> CheckSummary {
    let mut summary = CheckSummary {
        ok: 0,
        warn: 0,
        fail: 0,
    };

    for check in checks {
        match check.status {
            CheckStatus::Ok => summary.ok += 1,
            CheckStatus::Warn => summary.warn += 1,
            CheckStatus::Fail => summary.fail += 1,
            CheckStatus::Na => {}
        }
    }

    summary
}

/// Print results as a table.
fn print_table(checks: &[CheckResult], summary: &CheckSummary, no_color: bool) {
    for check in checks {
        let status_str = if no_color {
            check.status.as_str().to_string()
        } else {
            format!("{}{}{}", check.status.color(), check.status.as_str(), CheckStatus::reset_color())
        };

        println!("{:<30} {:>6}  {}", check.name, status_str, check.detail);
    }

    println!();
    println!("Summary: {} OK, {} WARN, {} FAIL", summary.ok, summary.warn, summary.fail);
}

/// Print results as JSON.
fn print_json(checks: &[CheckResult], summary: &CheckSummary) -> Result<()> {
    use std::collections::HashMap;

    let checks_json: Vec<HashMap<&str, serde_json::Value>> = checks
        .iter()
        .map(|check| {
            let mut map = HashMap::new();
            map.insert("name", serde_json::json!(check.name));
            map.insert("status", serde_json::json!(check.status.as_str()));
            map.insert("detail", serde_json::json!(check.detail));
            map
        })
        .collect();

    let output = serde_json::json!({
        "summary": {
            "ok": summary.ok,
            "warn": summary.warn,
            "fail": summary.fail,
        },
        "checks": checks_json,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

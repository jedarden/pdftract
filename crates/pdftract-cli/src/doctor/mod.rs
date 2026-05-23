//! Doctor subcommand - environment health checks

use anyhow::Result;
use std::path::PathBuf;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::fmt::Write;
use std::io::Write as IoWrite;

// Private checks module
mod checks;

pub use checks::registry::all_checks;

/// Result of a single doctor check
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Human-readable check name
    pub name: &'static str,
    /// Check status
    pub status: CheckStatus,
    /// Human-readable detail message
    pub detail: String,
}

/// Status of a doctor check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed
    Ok,
    /// Check passed with warnings
    Warn,
    /// Check failed
    Fail,
    /// Check not applicable (feature not compiled)
    NotApplicable,
}

/// Context passed to each check
#[derive(Debug, Clone)]
pub struct DoctorCtx {
    /// Requested OCR languages (from --lang flag)
    pub requested_langs: Vec<String>,
    /// Cache directory path (from --cache-dir flag)
    pub cache_dir: Option<PathBuf>,
    /// Profile search path (from --profile-dir flag)
    pub profile_dir: Option<PathBuf>,
    /// Feature flags compiled in
    pub features: DoctorFeatures,
}

/// Feature flags compiled into the binary
#[derive(Debug, Clone, Default)]
pub struct DoctorFeatures {
    pub ocr: bool,
    pub full_render: bool,
    pub remote: bool,
    pub profiles: bool,
    pub serve: bool,
    pub mcp: bool,
    pub inspect: bool,
    pub grep: bool,
    pub cache: bool,
    pub receipts: bool,
    pub markdown: bool,
}

impl DoctorFeatures {
    /// Detect compiled features from build-time environment variables
    pub fn from_build() -> Self {
        let compiled_features = env!("COMPILED_FEATURES");

        Self {
            ocr: compiled_features.contains("OCR"),
            full_render: compiled_features.contains("FULL_RENDER"),
            remote: compiled_features.contains("REMOTE"),
            profiles: compiled_features.contains("PROFILES"),
            serve: compiled_features.contains("SERVE"),
            mcp: compiled_features.contains("MCP"),
            inspect: compiled_features.contains("INSPECT"),
            grep: compiled_features.contains("GREP"),
            cache: compiled_features.contains("CACHE"),
            receipts: compiled_features.contains("RECEIPTS"),
            markdown: compiled_features.contains("MARKDOWN"),
        }
    }
}

/// Trait for environment checks
pub trait Check: Send + Sync {
    /// Human-readable check name
    fn name(&self) -> &'static str;

    /// Run the check, returning a result
    fn run(&self, ctx: &DoctorCtx) -> CheckResult;
}

/// Wrapper that catches panics in Check::run
pub fn run_check_safe<C: Check + ?Sized>(check: &C, ctx: &DoctorCtx) -> CheckResult {
    let name = check.name();

    match catch_unwind(AssertUnwindSafe(|| check.run(ctx))) {
        Ok(result) => result,
        Err(panic) => {
            let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };

            CheckResult {
                name,
                status: CheckStatus::Fail,
                detail: format!("Panic during check: {}", panic_msg),
            }
        }
    }
}

/// Get version information for the binary
pub fn version_info() -> String {
    format!(
        "{} (git: {})\nFeatures: {}",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_SHA"),
        env!("COMPILED_FEATURES")
    )
}

/// Options for the doctor subcommand
pub struct DoctorOptions {
    /// Print compiled features and exit
    pub features: bool,
    /// Output results as JSON
    pub json: bool,
    /// Exit with code 1 if any check reports FAIL
    pub exit_on_fail: bool,
    /// Verify the profile search path includes DIR
    pub profile_dir: Option<PathBuf>,
    /// Verify DIR is writable and has sufficient space
    pub cache_dir: Option<PathBuf>,
    /// Requested OCR languages (default: eng)
    pub lang: Vec<String>,
}

/// Run the doctor subcommand
pub fn run(opts: DoctorOptions) -> Result<()> {
    // If --features is set, print features and exit
    if opts.features {
        println!("{}", version_info());
        return Ok(());
    }

    // Build the doctor context
    let ctx = DoctorCtx {
        requested_langs: if opts.lang.is_empty() {
            vec!["eng".to_string()]
        } else {
            opts.lang
        },
        cache_dir: opts.cache_dir,
        profile_dir: opts.profile_dir,
        features: DoctorFeatures::from_build(),
    };

    // Run all checks
    let checks = all_checks();
    let mut results: Vec<CheckResult> = Vec::new();

    for check in &checks {
        let result = run_check_safe(&**check, &ctx);
        results.push(result);
    }

    // Output results
    if opts.json {
        output_json(&results);
    } else {
        output_text(&results)?;
    }

    // Determine exit code
    let has_fail = results.iter().any(|r| r.status == CheckStatus::Fail);
    if has_fail {
        std::process::exit(1);
    }

    Ok(())
}

/// Output results as JSON
fn output_json(results: &[CheckResult]) {
    let mut ok = 0;
    let mut warn = 0;
    let mut fail = 0;

    let checks_json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let status_str = match r.status {
                CheckStatus::Ok => {
                    ok += 1;
                    "OK"
                }
                CheckStatus::Warn => {
                    warn += 1;
                    "WARN"
                }
                CheckStatus::Fail => {
                    fail += 1;
                    "FAIL"
                }
                CheckStatus::NotApplicable => "N/A",
            };

            serde_json::json!({
                "name": r.name,
                "status": status_str,
                "detail": r.detail,
            })
        })
        .collect();

    let output = serde_json::json!({
        "summary": {
            "ok": ok,
            "warn": warn,
            "fail": fail,
        },
        "checks": checks_json,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Output results as human-readable text
fn output_text(results: &[CheckResult]) -> Result<()> {
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    let mut ok = 0;
    let mut warn = 0;
    let mut fail = 0;

    for result in results {
        let (color, status_str) = match result.status {
            CheckStatus::Ok => {
                ok += 1;
                (Color::Green, "OK")
            }
            CheckStatus::Warn => {
                warn += 1;
                (Color::Yellow, "WARN")
            }
            CheckStatus::Fail => {
                fail += 1;
                (Color::Red, "FAIL")
            }
            CheckStatus::NotApplicable => (Color::Cyan, "N/A"),
        };

        // Print check name
        stdout.set_color(ColorSpec::new().set_bold(true))?;
        write!(&mut stdout, "{:30}", result.name)?;
        stdout.reset()?;

        // Print status badge
        stdout.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true))?;
        write!(&mut stdout, "[{:4}] ", status_str)?;
        stdout.reset()?;

        // Print detail
        writeln!(&mut stdout, "{}", result.detail)?;
    }

    // Print summary
    writeln!(&mut stdout)?;
    stdout.set_color(ColorSpec::new().set_bold(true))?;
    write!(&mut stdout, "Summary: ")?;
    stdout.reset()?;

    writeln!(
        &mut stdout,
        "{} OK, {} WARN, {} FAIL",
        ok, warn, fail
    )?;

    Ok(())
}

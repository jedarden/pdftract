use std::path::PathBuf;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub mod checks;

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
    /// Cache directory path (from --cache-dir flag or default)
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

/// Get all registered checks
pub fn all_checks() -> Vec<Box<dyn Check>> {
    checks::registry::all_checks()
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

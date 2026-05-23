use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: pdftract binary version and compiled features
///
/// This check always returns OK and reports:
/// - Version from CARGO_PKG_VERSION
/// - Git SHA from build-time env var
/// - Compiled features from build-time env var
pub struct BinaryCheck;

impl Check for BinaryCheck {
    fn name(&self) -> &'static str {
        "pdftract binary"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        let version = env!("CARGO_PKG_VERSION");
        let git_sha = env!("GIT_SHA");
        let features = env!("COMPILED_FEATURES");

        CheckResult {
            name: self.name(),
            status: CheckStatus::Ok,
            detail: format!("{} (git: {})\nFeatures: {}", version, git_sha, features),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_check_always_ok() {
        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: None,
            profile_dir: None,
            features: Default::default(),
        };

        let result = BinaryCheck.run(&ctx);
        assert_eq!(result.status, CheckStatus::Ok);
        assert!(result.detail.contains(env!("CARGO_PKG_VERSION")));
        assert!(result.detail.contains(env!("GIT_SHA")));
    }
}

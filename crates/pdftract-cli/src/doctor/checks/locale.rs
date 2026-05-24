use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};
use std::env;

/// Check: system locale
///
/// OK: UTF-8 locale active
/// WARN: non-UTF-8 with C fallback
/// FAIL: unset
pub struct LocaleCheck;

impl LocaleCheck {
    fn is_utf8_locale(locale: &str) -> bool {
        let locale_lower = locale.to_lowercase();
        locale_lower.contains("utf-8") || locale_lower.contains("utf8")
    }

    fn get_locale() -> Option<String> {
        // Check LC_ALL first (highest priority), then LANG
        // Note: env::var returns Err if not set, Ok(value) if set
        let lc_all = env::var("LC_ALL");
        let lang = env::var("LANG");

        // Prefer LC_ALL, fall back to LANG
        lc_all.ok().or_else(|| lang.ok())
    }
}

impl Check for LocaleCheck {
    fn name(&self) -> &'static str {
        "system locale"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        match Self::get_locale() {
            None => CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: "Locale not set (LANG/LC_ALL environment variables unset)".to_string(),
            },
            Some(locale) if locale.is_empty() => CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail:
                    "Locale is empty (LANG/LC_ALL set to empty string, may cause encoding issues)"
                        .to_string(),
            },
            Some(locale) => {
                if locale == "C" || locale == "POSIX" {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!(
                            "Locale is '{}' (non-UTF-8, may cause encoding issues)",
                            locale
                        ),
                    }
                } else if Self::is_utf8_locale(&locale) {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: format!("Locale '{}' (UTF-8)", locale),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!(
                            "Locale '{}' (non-UTF-8, may cause encoding issues)",
                            locale
                        ),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_check_name() {
        assert_eq!(LocaleCheck.name(), "system locale");
    }

    #[test]
    fn test_is_utf8_locale() {
        assert!(LocaleCheck::is_utf8_locale("en_US.UTF-8"));
        assert!(LocaleCheck::is_utf8_locale("en_US.utf8"));
        assert!(LocaleCheck::is_utf8_locale("C.UTF-8"));
        assert!(!LocaleCheck::is_utf8_locale("en_US.ISO-8859-1"));
        assert!(!LocaleCheck::is_utf8_locale("C"));
    }
}

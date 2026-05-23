use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: pdfium native library (full-render feature)
///
/// OK: runtime detection succeeds, version >= 6555
/// WARN: older version
/// FAIL: not found
///
/// Note: This check requires the pdfium-render crate's runtime detection.
/// For now, we implement a basic check that attempts to load the library.
pub struct PdfiumCheck;

impl PdfiumCheck {
    #[cfg(target_os = "linux")]
    fn load_and_check() -> Result<(u32, String), String> {
        use libloading::{Library, Symbol};

        // Try common library names
        let lib_names = ["libpdfium.so", "pdfium", "libpdfium.so.1"];

        for lib_name in &lib_names {
            if let Ok(lib) = unsafe { Library::new(lib_name) } {
                // Try to get FPDF_GetVersion
                if let Ok(get_version) = unsafe { lib.get::<fn() -> i32>(b"FPDF_GetVersion\0") } {
                    let version = get_version() as u32;
                    return Ok((version, format!("loaded from {}", lib_name)));
                }
            }
        }

        // Try system library paths
        let system_paths = [
            "/usr/lib/x86_64-linux-gnu/libpdfium.so",
            "/usr/lib64/libpdfium.so",
            "/usr/local/lib/libpdfium.so",
        ];

        for path in &system_paths {
            if let Ok(lib) = unsafe { Library::new(path) } {
                if let Ok(get_version) = unsafe { lib.get::<fn() -> i32>(b"FPDF_GetVersion\0") } {
                    let version = get_version() as u32;
                    return Ok((version, format!("loaded from {}", path)));
                }
            }
        }

        Err("pdfium library not found in common paths".to_string())
    }

    #[cfg(not(target_os = "linux"))]
    fn load_and_check() -> Result<(u32, String), String> {
        Err("pdfium detection not implemented on this platform".to_string())
    }
}

impl Check for PdfiumCheck {
    fn name(&self) -> &'static str {
        "pdfium native lib"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        match Self::load_and_check() {
            Ok((version, source)) => {
                // Version >= 6555 means "reasonably modern"
                // (6555 is approximately PDFium 100+)
                if version >= 6555 {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: format!("pdfium {} found ({})", version, source),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("pdfium {} found (< 6555: may have compatibility issues), {}", version, source),
                    }
                }
            }
            Err(e) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("pdfium not found: {}", e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfium_check_name() {
        assert_eq!(PdfiumCheck.name(), "pdfium native lib");
    }
}

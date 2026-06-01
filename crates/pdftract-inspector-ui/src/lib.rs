//! Inspector UI frontend bundle for pdftract.
//!
//! This crate provides the HTML/CSS/JS frontend assets for the inspector mode
//! (Phase 7.9). The assets are bundled at compile time via `include_bytes!`.
//!
//! # Bundle Size Limit
//!
//! The gzipped bundle size must stay under 80 KB (enforced by build.rs).
//! This is a hard limit to keep the pdftract binary size manageable.
//!
//! # Usage
//!
//! The inspector mode serves these assets via HTTP when a user runs
//! `pdftract inspect`. The assets are bundled into the binary, so no
//! external files are required at runtime.

/// HTML index page for the inspector UI.
pub const INDEX_HTML: &[u8] = include_bytes!("../static/index.html");

/// CSS styles for the inspector UI.
pub const STYLE_CSS: &[u8] = include_bytes!("../static/style.css");

/// JavaScript application code for the inspector UI.
pub const APP_JS: &[u8] = include_bytes!("../static/app.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_files_exist() {
        // Verify that the frontend files are non-empty
        assert!(!INDEX_HTML.is_empty(), "INDEX_HTML should not be empty");
        assert!(!STYLE_CSS.is_empty(), "STYLE_CSS should not be empty");
        assert!(!APP_JS.is_empty(), "APP_JS should not be empty");
    }
}

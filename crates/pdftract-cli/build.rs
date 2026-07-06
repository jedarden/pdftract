//! Build script for pdftract-cli.
//!
//! This build script enforces the <80 KB bundle size limit for the inspector
//! frontend (Phase 7.9.3). It computes the gzipped size of the frontend bundle
//! and fails the build if it exceeds the limit.
//!
//! The bundle consists of:
//! - crates/pdftract-cli/src/inspect/frontend/index.html
//! - crates/pdftract-cli/src/inspect/frontend/style.css
//! - crates/pdftract-cli/src/inspect/frontend/app.js

use std::env;
use std::fs;
use std::io::Write;

/// Maximum allowed gzipped bundle size in bytes (80 KB)
const MAX_BUNDLE_SIZE_BYTES: usize = 80 * 1024;

fn main() {
    // Set compile-time environment variables for doctor checks
    // These must be set for all builds, not just pdftract binary
    // GIT_SHA: current git commit SHA (or "unknown" if not in git repo)
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_SHA={}", git_sha);

    // COMPILED_FEATURES: comma-separated list of enabled features
    // Read from CARGO_FEATURE_<FEATURE_NAME> variables set by cargo
    let features = vec![
        ("OCR", cfg!(feature = "ocr")),
        ("FULL_RENDER", cfg!(feature = "full_render")),
        ("REMOTE", cfg!(feature = "remote")),
        ("PROFILES", cfg!(feature = "profiles")),
        ("SERVE", cfg!(feature = "serve")),
        ("MCP", cfg!(feature = "mcp")),
        ("INSPECT", cfg!(feature = "inspect")),
        ("GREP", cfg!(feature = "grep")),
        ("CACHE", cfg!(feature = "cache")),
        ("RECEIPTS", cfg!(feature = "receipts")),
        ("MARKDOWN", cfg!(feature = "markdown")),
    ];
    let enabled_features: Vec<&str> = features
        .iter()
        .filter_map(|(name, enabled)| if *enabled { Some(*name) } else { None })
        .collect();
    println!(
        "cargo:rustc-env=COMPILED_FEATURES={}",
        enabled_features.join(",")
    );

    // Only run the bundle size check when building the pdftract binary
    // Skip for test builds, other binaries, and docs
    let is_pdftract_build = env::var("CARGO_BIN_NAME")
        .map(|name| name == "pdftract")
        .unwrap_or(false);

    if !is_pdftract_build {
        return;
    }

    // Paths to frontend files
    let frontend_dir = [
        env::var("CARGO_MANIFEST_DIR").unwrap_or_default(),
        "src".to_string(),
        "inspect".to_string(),
        "frontend".to_string(),
    ]
    .iter()
    .collect::<std::path::PathBuf>();

    let html_path = frontend_dir.join("index.html");
    let css_path = frontend_dir.join("style.css");
    let js_path = frontend_dir.join("app.js");

    // Read all frontend files
    let html = fs::read_to_string(&html_path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", html_path.display(), e);
    });

    let css = fs::read_to_string(&css_path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", css_path.display(), e);
    });

    let js = fs::read_to_string(&js_path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", js_path.display(), e);
    });

    // Concatenate into a single bundle
    let bundle = format!("{}\n{}\n{}", html, css, js);

    // Compute gzipped size
    let gzipped_bytes = gzip_compress(&bundle);

    let gzipped_size_kb = gzipped_bytes.len() as f64 / 1024.0;
    let raw_size_kb = bundle.len() as f64 / 1024.0;

    // Emit the size information to build logs
    println!("cargo:warning=Inspector frontend bundle size:");
    println!("cargo:warning=  Raw: {:.2} KB", raw_size_kb);
    println!(
        "cargo:warning=  Gzipped: {:.2} KB / {} KB limit",
        gzipped_size_kb,
        MAX_BUNDLE_SIZE_BYTES / 1024
    );

    // Fail the build if the bundle exceeds the size limit
    if gzipped_bytes.len() > MAX_BUNDLE_SIZE_BYTES {
        let _ = writeln!(
            &mut std::io::stderr(),
            "\n\
             ================================================\n\
             ERROR: Inspector frontend bundle exceeds size limit\n\
             ================================================\n\
             \n\
             Bundle size: {:.2} KB\n\
             Limit: {} KB\n\
             \n\
             The inspector frontend bundle must be kept under {} KB gzipped.\n\
             This is a hard limit to keep the pdftract binary size manageable.\n\
             \n\
             To fix this:\n\
             1. Minify the HTML/CSS/JS files further\n\
             2. Remove unnecessary features or assets\n\
             3. Consider splitting the bundle into smaller chunks\n\
             \n\
             Files checked:\n\
             - {}\n\
             - {}\n\
             - {}\n\
             ================================================\n",
            gzipped_size_kb,
            MAX_BUNDLE_SIZE_BYTES / 1024,
            MAX_BUNDLE_SIZE_BYTES / 1024,
            html_path.display(),
            css_path.display(),
            js_path.display()
        );
        std::process::exit(1);
    }

    // Set a cargo cfg flag for conditional compilation
    println!("cargo:rustc-cfg=inspector_bundle_valid");
}

/// Compress data using gzip and libflate.
fn gzip_compress(data: &str) -> Vec<u8> {
    use libflate::gzip::Encoder;

    let mut encoder = Encoder::new(Vec::new()).unwrap();
    encoder.write_all(data.as_bytes()).unwrap();
    encoder.finish().into_result().unwrap()
}

//! Build script for pdftract-inspector-ui.
//!
//! This build script bundles the HTML/CSS/JS frontend for the inspector mode
//! and validates that the gzipped bundle size stays within acceptable limits
//! (Phase 7.9.3).

use std::fs;
use std::io::Write;

/// Maximum allowed gzipped bundle size in bytes (80 KB)
const MAX_BUNDLE_SIZE_BYTES: usize = 80 * 1024;

fn main() {
    // Paths to frontend files
    let frontend_dir = [
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default(),
        "static".to_string(),
    ].iter()
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
    println!("cargo:warning=  Gzipped: {:.2} KB / {} KB limit",
             gzipped_size_kb,
             MAX_BUNDLE_SIZE_BYTES / 1024);

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

/// Compress data using gzip and flate2.
fn gzip_compress(data: &str) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

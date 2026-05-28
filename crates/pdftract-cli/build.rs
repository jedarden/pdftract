use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Maximum gzipped bundle size in bytes (80 KB per Phase 7.9.3)
const MAX_BUNDLE_SIZE_BYTES: usize = 80 * 1024;

fn main() {
    // Phase 7.9.3: Check frontend bundle size (only when inspect feature is enabled)
    if cfg!(feature = "inspect") {
        check_bundle_size();
    }

    // Capture git SHA for version reporting
    let git_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_SHA={}", git_sha);

    // Emit compile-time feature list
    // These are the cargo features that affect doctor output
    let features = [
        ("OCR", cfg!(feature = "ocr")),
        ("FULL_RENDER", cfg!(feature = "full-render")),
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

    let enabled: Vec<&str> = features
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| *name)
        .collect();

    let feature_list = if enabled.is_empty() {
        "default".to_string()
    } else {
        enabled.join(",")
    };

    println!("cargo:rustc-env=COMPILED_FEATURES={}", feature_list);

    // Rebuild if git HEAD changes (for accurate GIT_SHA in dev builds)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_OCR");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_FULL_RENDER");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_REMOTE");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_PROFILES");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERVE");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_MCP");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_INSPECT");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_GREP");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_CACHE");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_RECEIPTS");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_MARKDOWN");
    // Rebuild when frontend files change (for bundle size check)
    println!("cargo:rerun-if-changed=src/inspect/frontend/index.html");
    println!("cargo:rerun-if-changed=src/inspect/frontend/style.css");
    println!("cargo:rerun-if-changed=src/inspect/frontend/app.js");
}

/// Check that the frontend bundle is under the size limit.
///
/// Computes the gzipped size of all frontend files (index.html, style.css, app.js)
/// and fails the build if the total exceeds 80 KB. This is the CI gate for Phase 7.9.3.
fn check_bundle_size() {
    let frontend_dir = Path::new("src/inspect/frontend");

    let files = [
        frontend_dir.join("index.html"),
        frontend_dir.join("style.css"),
        frontend_dir.join("app.js"),
    ];

    let mut total_raw = 0;
    let mut total_gzipped = 0;

    for file_path in &files {
        let content = match fs::read(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read frontend file {}: {}",
                    file_path.display(),
                    e
                );
                continue;
            }
        };

        let raw_len = content.len();
        total_raw += raw_len;

        // Compress with gzip
        let gzipped = gzip_compress(&content);
        let gzipped_len = gzipped.len();
        total_gzipped += gzipped_len;

        eprintln!(
            "frontend/{}: {} bytes raw, {} bytes gzipped",
            file_path.file_name().unwrap().to_string_lossy(),
            raw_len,
            gzipped_len
        );
    }

    eprintln!(
        "Frontend bundle total: {} bytes raw, {} bytes gzipped (limit: {} bytes)",
        total_raw, total_gzipped, MAX_BUNDLE_SIZE_BYTES
    );

    if total_gzipped > MAX_BUNDLE_SIZE_BYTES {
        eprintln!(
            "ERROR: Frontend bundle exceeds {} bytes gzipped. Please optimize the frontend files.",
            MAX_BUNDLE_SIZE_BYTES
        );
        std::process::exit(1);
    }

    println!(
        "cargo:warning=Frontend bundle size: {} bytes gzipped ({} bytes raw)",
        total_gzipped, total_raw
    );
}

/// Compress data with gzip (level 9 for maximum compression).
fn gzip_compress(data: &[u8]) -> Vec<u8> {
    use libflate::gzip::Encoder;
    let mut encoder = Encoder::new(Vec::new()).unwrap();
    encoder.write_all(data).unwrap();
    encoder.finish().into_result().unwrap()
}
use std::process::Command;

fn main() {
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
}

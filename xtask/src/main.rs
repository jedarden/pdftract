use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use lopdf;

/// Helper macro for creating dictionaries
macro_rules! dictionary {
    ($( $key:literal => $value:expr ),* $(,)?) => {{
        let mut dict = lopdf::Dictionary::new();
        $(
            dict.set($key, $value);
        )*
        dict
    }};
}

/// Find the workspace root directory by searching for Cargo.toml
fn find_workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap();

    // If we're in the xtask directory, go to parent
    if current.ends_with("xtask") {
        current = current.parent().unwrap().to_path_buf();
    }

    // Search upward for Cargo.toml with workspace members
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
            if content.contains("[workspace]") {
                return current;
            }
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    // Fallback: use current directory if not found
    std::env::current_dir().unwrap()
}

#[derive(Debug, Deserialize)]
struct Profile {
    description: String,
    #[serde(default)]
    profile_fields: BTreeMap<String, ProfileField>,
    #[serde(default)]
    r#match: MatchConfig,
}

#[derive(Debug, Deserialize)]
struct ProfileField {
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    extraction: ExtractionConfig,
}

#[derive(Debug, Deserialize, Default)]
struct ExtractionConfig {
    #[serde(default)]
    patterns: Vec<String>,
    #[serde(default)]
    region_hint: Option<String>,
    #[serde(default)]
    table_region: Option<String>,
    #[serde(default)]
    columnar_regions: Option<String>,
    #[serde(default)]
    per_page: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    fallback: serde_yaml::Value,
}

#[derive(Debug, Deserialize, Default)]
struct MatchConfig {
    #[serde(default)]
    any: Vec<MatchClause>,
}

#[derive(Debug, Deserialize, Default)]
struct MatchClause {
    #[serde(default)]
    text_patterns: Vec<String>,
    #[serde(default)]
    structural: Vec<serde_yaml::Value>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: xtask <command>");
        eprintln!("Commands:");
        eprintln!("  doc-profile <profile-name>      Generate README skeleton for a profile");
        eprintln!("  doc-profiles                     Generate README skeletons for all profiles");
        eprintln!("  generate-stress-pdfs            Generate stress-test PDFs for memory ceiling testing");
        eprintln!("  generate-page-class-fixtures    Generate page classification test fixtures");
        eprintln!("  memory-ceiling                  Run memory ceiling tests against perf/malformed corpora");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "doc-profile" => {
            if args.len() < 3 {
                eprintln!("Usage: xtask doc-profile <profile-name>");
                std::process::exit(1);
            }
            generate_profile_readme(&args[2])?;
        }
        "doc-profiles" => {
            let profiles_dir = find_workspace_root().join("profiles/builtin");
            for entry in fs::read_dir(&profiles_dir)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    let profile_name = entry.file_name().to_string_lossy().to_string();
                    if let Err(e) = generate_profile_readme(&profile_name) {
                        eprintln!("Error generating README for {}: {}", profile_name, e);
                    }
                }
            }
        }
        "generate-stress-pdfs" => {
            generate_stress_pdfs()?;
        }
        "generate-page-class-fixtures" => {
            generate_page_class_fixtures()?;
        }
        "memory-ceiling" => {
            run_memory_ceiling_tests()?;
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn generate_profile_readme(profile_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Find the workspace root by looking for the parent directory's Cargo.toml
    let workspace_root = find_workspace_root();
    let profile_path = workspace_root.join("profiles/builtin").join(profile_name).join("profile.yaml");
    let readme_path = workspace_root.join("profiles/builtin").join(profile_name).join("README.md");

    if !profile_path.exists() {
        return Err(format!("Profile YAML not found: {}", profile_path.display()).into());
    }

    let yaml_content = fs::read_to_string(&profile_path)?;
    let profile: Profile = serde_yaml::from_str(&yaml_content)?;

    let mut readme = String::new();

    // Title and description
    readme.push_str(&format!("# {} Profile\n\n", profile_name.to_uppercase()));
    readme.push_str(&format!("{}\n\n", profile.description));

    // Match Criteria Summary (placeholder for human to fill)
    readme.push_str("## Match Criteria Summary\n\n");
    readme.push_str("*This section describes the characteristics that cause a document to match this profile. The following signals are considered:*\n\n");

    // Collect all text patterns and structural signals from any clause
    let mut all_patterns: Vec<&String> = Vec::new();
    let mut all_structural: Vec<String> = Vec::new();

    for clause in &profile.r#match.any {
        for pattern in &clause.text_patterns {
            if !all_patterns.contains(&pattern) {
                all_patterns.push(pattern);
            }
        }
        for signal in &clause.structural {
            let signal_str = format!("{:?}", signal);
            if !all_structural.iter().any(|s| s == &signal_str) {
                all_structural.push(signal_str);
            }
        }
    }

    // Show first few patterns as examples
    if !all_patterns.is_empty() {
        let show_count = all_patterns.len().min(3);
        readme.push_str("- **Text patterns**: ");
        for (i, pattern) in all_patterns.iter().take(show_count).enumerate() {
            if i > 0 {
                readme.push_str(", ");
            }
            readme.push_str(&format!("`{}`", pattern));
        }
        if all_patterns.len() > show_count {
            readme.push_str(&format!(" ({} more)", all_patterns.len() - show_count));
        }
        readme.push('\n');
    }

    if !all_structural.is_empty() {
        let show_count = all_structural.len().min(3);
        readme.push_str("- **Structural signals**: ");
        for (i, signal) in all_structural.iter().take(show_count).enumerate() {
            if i > 0 {
                readme.push_str(", ");
            }
            readme.push_str(&format!("`{}`", signal));
        }
        if all_structural.len() > show_count {
            readme.push_str(&format!(" ({} more)", all_structural.len() - show_count));
        }
        readme.push('\n');
    }

    readme.push_str("\n*Additional heuristics and confidence scoring are applied during classification.*\n\n");

    // Extracted Fields
    readme.push_str("## Extracted Fields\n\n");
    readme.push_str("| Field | Type | Description | Example Value | Source Hint |\n");
    readme.push_str("|-------|------|-------------|----------------|-------------|\n");

    for (field_name, field) in &profile.profile_fields {
        let description = format!("Extracted from page text using pattern matching");
        let example = match field.field_type.as_str() {
            "string" => "\"example value\"",
            "decimal" => "123.45",
            "date" => "2024-01-15",
            "int" => "42",
            "array" => "[...]",
            _ => "N/A",
        };
        let mut source_parts = Vec::new();
        if !field.extraction.patterns.is_empty() {
            source_parts.push("regex patterns".to_string());
        }
        if let Some(ref hint) = field.extraction.region_hint {
            source_parts.push(format!("region: {}", hint));
        }
        if let Some(ref table) = field.extraction.table_region {
            source_parts.push(format!("table: {}", table));
        }
        if let Some(ref cols) = field.extraction.columnar_regions {
            source_parts.push(format!("columns: {}", cols));
        }
        if field.extraction.per_page.unwrap_or(false) {
            source_parts.push("per-page".to_string());
        }
        let source = if source_parts.is_empty() {
            "profile YAML".to_string()
        } else {
            source_parts.join(", ")
        };
        readme.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            field_name, field.field_type, description, example, source
        ));
    }

    if profile.profile_fields.is_empty() {
        readme.push_str("| *(none)* | - | *This profile has no field extractors* | - | - |\n");
    }

    readme.push('\n');

    // Known Limitations
    readme.push_str("## Known Limitations\n\n");
    readme.push_str("*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*\n\n");
    readme.push_str("- *Document limitations and edge cases to be added by profile author*\n\n");

    // Sample Input Pointer
    readme.push_str("## Sample Input\n\n");
    readme.push_str(&format!("Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/{}/`.\n\n", profile_name));
    readme.push_str("*See the classifier corpus for representative documents.*\n\n");

    // Configuration Tips
    readme.push_str("## Configuration Tips\n\n");
    readme.push_str("To override this profile:\n\n");
    readme.push_str("```bash\n");
    readme.push_str(&format!("pdftract profiles export {} > my-profile.yaml\n", profile_name));
    readme.push_str("# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns\n");
    readme.push_str("pdftract extract --profile my-profile.yaml document.pdf\n");
    readme.push_str("```\n\n");

    // Footer
    readme.push_str(&format!("---\n\n*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*\n"));

    fs::write(&readme_path, readme)?;
    println!("Generated README for {} at {}", profile_name, readme_path.display());

    Ok(())
}

/// Generate stress-test PDFs for memory ceiling testing
///
/// Creates large-page-count PDFs to validate memory targets:
/// - 100-page vector PDF for buffered mode testing (target: < 512 MB)
/// - 10,000-page stress test for streaming mode validation (target: < 256 MB)
fn generate_stress_pdfs() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================");
    println!("Generating Stress-Test PDFs");
    println!("==========================================");

    let workspace_root = find_workspace_root();
    let perf_dir = workspace_root.join("tests/fixtures/perf");
    fs::create_dir_all(&perf_dir)?;

    let configs = vec![
        (100, "100-page-vector.pdf", "Buffered mode stress test (512 MB budget)"),
        (10000, "10k-page.pdf", "Streaming mode stress test (256 MB budget)"),
    ];

    for (num_pages, filename, description) in &configs {
        println!("\nGenerating: {} ({} pages)", filename, num_pages);
        println!("  Purpose: {}", description);

        let output_path = perf_dir.join(filename);
        generate_stress_pdf(&output_path, *num_pages)?;
    }

    println!("\n==========================================");
    println!("Stress-Test PDF Generation Complete");
    println!("==========================================");
    println!("\nGenerated files:");
    for (_, filename, _) in &configs {
        let path = perf_dir.join(filename);
        if path.exists() {
            let metadata = fs::metadata(&path)?;
            let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
            println!("  - {} ({:.2} MB)", filename, size_mb);
        }
    }

    Ok(())
}

/// Generate a multi-page stress-test PDF
///
/// Creates a PDF with the specified number of pages for memory ceiling testing.
/// Uses a minimal approach with lopdf 0.34.
fn generate_stress_pdf(output_path: &Path, num_pages: usize) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, Dictionary};

    let mut doc = Document::with_version("1.5");

    // Pre-create fonts and resources that will be reused
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", "Font");
    font_dict.set("Subtype", "Type1");
    font_dict.set("BaseFont", "Helvetica");
    let font_id = doc.add_object(font_dict);

    let mut resources = Dictionary::new();
    let mut font_resources = Dictionary::new();
    font_resources.set("F1", font_id);
    resources.set("Font", font_resources);

    // Create all page objects first
    let mut page_ids = Vec::new();
    let mediabox = Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0),
    ]);

    for page_num in 1..=num_pages {
        // Create content stream for this page
        let content_bytes = format!(
            "BT /F1 12 Tf 72 720 Td (Page {} of {}) Tj ET",
            page_num, num_pages
        ).into_bytes();

        let mut content_dict = Dictionary::new();
        content_dict.set("Length", content_bytes.len() as i32);
        let content_stream = Stream::new(content_dict, content_bytes);
        let content_id = doc.add_object(content_stream);

        // Create page dictionary
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", "Page");
        page_dict.set("MediaBox", mediabox.clone());
        page_dict.set("Contents", content_id);
        page_dict.set("Resources", resources.clone());

        let page_id = doc.add_object(page_dict);
        page_ids.push(page_id);
    }

    // Create the Pages root dictionary (Pages tree)
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(num_pages as i64));
    pages_dict.set("Kids", Object::Array(page_ids.iter().map(|&id| Object::Reference(id)).collect()));

    let pages_id = doc.add_object(pages_dict);

    // Set Parent reference for each page
    for &page_id in &page_ids {
        let page_obj = doc.get_object(page_id)?;
        if let Ok(dict) = page_obj.as_dict() {
            let mut updated_dict = dict.clone();
            updated_dict.set("Parent", pages_id);
            // Need to replace the object
            let _ = doc.objects.insert(page_id, Object::Dictionary(updated_dict));
        }
    }

    // Create the Catalog dictionary
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", pages_id);
    let catalog_id = doc.add_object(catalog_dict);

    // Set the document's catalog ID directly
    doc.trailer.set("Root", catalog_id);

    // Save the document
    doc.save(output_path)?;

    let metadata = fs::metadata(output_path)?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
    println!("  Generated: {} ({:.2} MB)", output_path.file_name().unwrap().to_string_lossy(), size_mb);

    Ok(())
}

/// Memory budgets for different document categories (in MB)
#[derive(Debug, Clone)]
struct MemoryBudget {
    pub buffered_100_page: usize,  // 512 MB
    pub streaming_any: usize,       // 256 MB
    pub adversarial_hard_cap: usize, // 1 GB
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self {
            buffered_100_page: 512,
            streaming_any: 256,
            adversarial_hard_cap: 1024,
        }
    }
}

#[derive(Debug, Serialize)]
struct MemoryMeasurement {
    pub peak_rss_mb: usize,
    pub duration_ms: u128,
    pub succeeded: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MemoryTestResult {
    pub file_name: String,
    pub category: String,  // "buffered", "streaming", "adversarial"
    pub peak_rss_mb: usize,
    pub duration_ms: u128,
    pub budget_mb: usize,
    pub passed: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemoryReport {
    pub timestamp: String,
    pub commit_sha: Option<String>,
    pub budgets: MemoryBudgetJson,
    pub results: Vec<MemoryTestResult>,
    pub summary: MemorySummary,
}

#[derive(Debug, Serialize)]
struct MemoryBudgetJson {
    pub buffered_100_page_mb: usize,
    pub streaming_any_mb: usize,
    pub adversarial_hard_cap_mb: usize,
}

#[derive(Debug, Serialize)]
struct MemorySummary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub all_passed: bool,
}

/// Run memory ceiling tests against perf and malformed corpora
///
/// This enforces the Tier-1 Memory targets from the plan:
/// - Peak RSS, 100-page vector PDF (buffered mode) < 512 MB
/// - Peak RSS, streaming/NDJSON mode < 256 MB
/// - Peak RSS, adversarial fixtures < 1 GB hard ceiling
///
/// Analogous to cargo-bloat for memory usage: fails the build if any
/// document exceeds its budget.
///
/// Generates memory-report.json artifact for CI historical tracking.
fn run_memory_ceiling_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================");
    println!("Memory Ceiling Tests");
    println!("==========================================");

    let budgets = MemoryBudget::default();
    let workspace_root = find_workspace_root();
    let perf_dir = workspace_root.join("tests/fixtures/perf");
    let malformed_dir = workspace_root.join("tests/fixtures/malformed");

    println!("\nMemory budgets:");
    println!("  - Buffered 100-page: {} MB", budgets.buffered_100_page);
    println!("  - Streaming mode: {} MB", budgets.streaming_any);
    println!("  - Adversarial hard cap: {} MB", budgets.adversarial_hard_cap);

    // Build pdftract binary first
    println!("\n=== Building pdftract for testing ===");
    let build_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "pdftract", "--locked"])
        .current_dir(&workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !build_status.success() {
        return Err("Failed to build pdftract binary".into());
    }

    let binary_path = workspace_root.join("target/release/pdftract");
    if !binary_path.exists() {
        return Err(format!("pdftract binary not found at {}", binary_path.display()).into());
    }

    println!("Binary: {}", binary_path.display());

    let mut all_results = Vec::new();
    let mut all_passed = true;

    // Test 1: Perf corpus - buffered mode (512 MB budget)
    println!("\n=== Testing perf corpus (buffered mode, budget: {} MB) ===", budgets.buffered_100_page);

    if perf_dir.exists() {
        for entry in fs::read_dir(&perf_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("pdf") {
                continue;
            }

            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            print!("  [buffered] {} ... ", file_name);

            match measure_extraction(&binary_path, &path, &budgets, false) {
                Ok(measurement) => {
                    let passed = measurement.peak_rss_mb <= budgets.buffered_100_page;
                    if passed {
                        println!("PASS ({} MB, {} ms)", measurement.peak_rss_mb, measurement.duration_ms);
                    } else {
                        println!("FAIL ({} MB > {} MB)", measurement.peak_rss_mb, budgets.buffered_100_page);
                        all_passed = false;
                    }
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "buffered".to_string(),
                        peak_rss_mb: measurement.peak_rss_mb,
                        duration_ms: measurement.duration_ms,
                        budget_mb: budgets.buffered_100_page,
                        passed,
                        error_message: measurement.error_message,
                    });
                }
                Err(e) => {
                    println!("ERROR ({})", e);
                    all_passed = false;
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "buffered".to_string(),
                        peak_rss_mb: 0,
                        duration_ms: 0,
                        budget_mb: budgets.buffered_100_page,
                        passed: false,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
    } else {
        println!("  (no perf directory)");
    }

    // Test 2: Perf corpus - streaming mode (256 MB budget)
    println!("\n=== Testing perf corpus (streaming mode, budget: {} MB) ===", budgets.streaming_any);

    if perf_dir.exists() {
        for entry in fs::read_dir(&perf_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("pdf") {
                continue;
            }

            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            print!("  [streaming] {} ... ", file_name);

            match measure_extraction(&binary_path, &path, &budgets, true) {
                Ok(measurement) => {
                    let passed = measurement.peak_rss_mb <= budgets.streaming_any;
                    if passed {
                        println!("PASS ({} MB, {} ms)", measurement.peak_rss_mb, measurement.duration_ms);
                    } else {
                        println!("FAIL ({} MB > {} MB)", measurement.peak_rss_mb, budgets.streaming_any);
                        all_passed = false;
                    }
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "streaming".to_string(),
                        peak_rss_mb: measurement.peak_rss_mb,
                        duration_ms: measurement.duration_ms,
                        budget_mb: budgets.streaming_any,
                        passed,
                        error_message: measurement.error_message,
                    });
                }
                Err(e) => {
                    println!("ERROR ({})", e);
                    all_passed = false;
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "streaming".to_string(),
                        peak_rss_mb: 0,
                        duration_ms: 0,
                        budget_mb: budgets.streaming_any,
                        passed: false,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // Test 3: Malformed corpus - adversarial hard cap (1 GB budget)
    println!("\n=== Testing malformed corpus (adversarial hard cap: {} MB) ===", budgets.adversarial_hard_cap);

    if malformed_dir.exists() {
        for entry in fs::read_dir(&malformed_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("pdf") &&
               path.extension().and_then(|s| s.to_str()) != Some("bin") {
                continue;
            }

            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            print!("  [adversarial] {} ... ", file_name);

            match measure_extraction(&binary_path, &path, &budgets, false) {
                Ok(measurement) => {
                    let passed = measurement.peak_rss_mb <= budgets.adversarial_hard_cap;
                    if passed {
                        println!("PASS ({} MB, {} ms)", measurement.peak_rss_mb, measurement.duration_ms);
                    } else {
                        println!("FAIL ({} MB > {} MB)", measurement.peak_rss_mb, budgets.adversarial_hard_cap);
                        all_passed = false;
                    }
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "adversarial".to_string(),
                        peak_rss_mb: measurement.peak_rss_mb,
                        duration_ms: measurement.duration_ms,
                        budget_mb: budgets.adversarial_hard_cap,
                        passed,
                        error_message: measurement.error_message,
                    });
                }
                Err(e) => {
                    println!("ERROR ({})", e);
                    all_passed = false;
                    all_results.push(MemoryTestResult {
                        file_name: file_name.clone(),
                        category: "adversarial".to_string(),
                        peak_rss_mb: 0,
                        duration_ms: 0,
                        budget_mb: budgets.adversarial_hard_cap,
                        passed: false,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
    } else {
        println!("  (no malformed directory)");
    }

    // Print summary
    println!("\n==========================================");
    println!("Memory Ceiling Summary");
    println!("==========================================");

    let passed_count = all_results.iter().filter(|r| r.passed).count();
    let total_count = all_results.len();

    println!("Passed: {}/{}", passed_count, total_count);

    if !all_passed {
        println!("\nFailed documents:");
        for result in &all_results {
            if !result.passed {
                if result.peak_rss_mb > 0 {
                    println!("  - [{}] {} ({} MB > {} MB)",
                        result.category, result.file_name, result.peak_rss_mb, result.budget_mb);
                } else {
                    println!("  - [{}] {} (error: {})",
                        result.category, result.file_name,
                        result.error_message.as_deref().unwrap_or("unknown"));
                }
            }
        }
        println!("\nMemory ceiling gate FAILED!");
        return Err("Memory ceiling exceeded".into());
    }

    println!("\nMemory ceiling gate PASSED!");

    // Generate JSON report
    let report = MemoryReport {
        timestamp: format!("{}", humantime::format_rfc3339_seconds(std::time::SystemTime::now())),
        commit_sha: get_commit_sha()?,
        budgets: MemoryBudgetJson {
            buffered_100_page_mb: budgets.buffered_100_page,
            streaming_any_mb: budgets.streaming_any,
            adversarial_hard_cap_mb: budgets.adversarial_hard_cap,
        },
        results: all_results.clone(),
        summary: MemorySummary {
            total_tests: total_count,
            passed: passed_count,
            failed: total_count - passed_count,
            all_passed,
        },
    };

    let report_path = workspace_root.join("memory-report.json");
    fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    println!("\nReport written to: {}", report_path.display());

    Ok(())
}

/// Get the current git commit SHA
fn get_commit_sha() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let workspace_root = find_workspace_root();
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&workspace_root)
        .output()?;

    if output.status.success() {
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(sha))
    } else {
        Ok(None)
    }
}

/// Measure memory usage during extraction of a PDF file
///
/// Uses Linux-specific /proc/[pid]/status to sample peak RSS.
/// Falls back to time measurement if RSS sampling is unavailable.
///
/// # Arguments
/// * `binary_path` - Path to the pdftract binary
/// * `pdf_path` - Path to the PDF file to extract
/// * `budgets` - Memory budgets (unused but kept for compatibility)
/// * `streaming` - If true, use streaming/text mode for lower memory; otherwise buffered JSON mode
fn measure_extraction(
    binary_path: &Path,
    pdf_path: &Path,
    _budgets: &MemoryBudget,
    streaming: bool,
) -> Result<MemoryMeasurement, Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Spawn the extraction process and measure its peak RSS
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;

        let mut cmd = Command::new(binary_path);

        if streaming {
            // Streaming mode: use --format text for lower memory footprint
            // Note: --format ndjson is not yet exposed in CLI (Phase 6.2)
            // Using text format as a reasonable proxy for streaming memory behavior
            cmd.arg("extract")
               .arg("--format")
               .arg("text");
        } else {
            // Buffered mode: use --format json for full document buffering
            cmd.arg("extract")
               .arg("--format")
               .arg("json");
        }

        cmd.arg(pdf_path)
           .stdout(Stdio::null())
           .stderr(Stdio::piped())
           .process_group(0);

        let mut child = cmd.spawn()?;

        let pid = child.id();
        let mut peak_rss_kb = 0usize;

        // Sample RSS every 10ms while process runs
        let sample_interval = Duration::from_millis(10);
        loop {
            // Try to wait for the process (non-blocking)
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited
                    let duration = start.elapsed();

                    // Capture stderr for error messages
                    let stderr_output = if let Some(mut stderr) = child.stderr {
                        let mut error_text = String::new();
                        use std::io::Read;
                        let _ = stderr.read_to_string(&mut error_text);
                        error_text
                    } else {
                        String::new()
                    };

                    // Trim error text and use it if non-empty
                    let error_message = if !status.success() {
                        if !stderr_output.is_empty() {
                            Some(stderr_output.trim().to_string())
                        } else {
                            Some(format!("exit code: {:?}", status.code()))
                        }
                    } else {
                        None
                    };

                    return Ok(MemoryMeasurement {
                        peak_rss_mb: peak_rss_kb / 1024,
                        duration_ms: duration.as_millis(),
                        succeeded: status.success(),
                        error_message,
                    });
                }
                Ok(None) => {
                    // Process still running, sample RSS
                    if let Ok(rss_kb) = sample_rss(pid) {
                        peak_rss_kb = peak_rss_kb.max(rss_kb);
                    }
                    std::thread::sleep(sample_interval);
                }
                Err(e) => {
                    return Err(format!("Failed to wait for process: {}", e).into());
                }
            }
        }
    }

    // Fallback for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    {
        let mut cmd = Command::new(binary_path);

        if streaming {
            cmd.arg("extract")
               .arg("--format")
               .arg("text");
        } else {
            cmd.arg("extract")
               .arg("--format")
               .arg("json");
        }

        cmd.arg(pdf_path)
           .stdout(Stdio::null())
           .stderr(Stdio::piped());

        let output = cmd.output()?;

        let duration = start.elapsed();

        Ok(MemoryMeasurement {
            peak_rss_mb: 0, // Cannot measure on this platform
            duration_ms: duration.as_millis(),
            succeeded: output.status.success(),
            error_message: if !output.status.success() {
                Some(format!("exit code: {:?}", output.status.code()))
            } else {
                None
            },
        })
    }
}

/// Sample the current RSS (Resident Set Size) of a process in KB
#[cfg(target_os = "linux")]
fn sample_rss(pid: u32) -> Result<usize, Box<dyn std::error::Error>> {
    let status_path = format!("/proc/{}/status", pid);
    let status = fs::read_to_string(&status_path)?;

    // Parse VmRSS from /proc/[pid]/status
    // Format: VmRSS:    12345 kB
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let rss_kb = parts[1].parse::<usize>()?;
                return Ok(rss_kb);
            }
        }
    }

    Err("VmRSS not found in /proc status".into())
}

/// Generate page classification test fixtures
///
/// Creates 4 fixture types for testing page classification:
/// - vector_pure: Pure text PDF (born-digital)
/// - scanned_single: Image-only PDF (scanned page)
/// - brokenvector_pdfa: Invisible text layer over scanned image
/// - hybrid_header_body: Text header + scanned body
fn generate_page_class_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, Dictionary};

    println!("==========================================");
    println!("Generating Page Classification Fixtures");
    println!("==========================================");

    let workspace_root = find_workspace_root();
    let fixtures_dir = workspace_root.join("tests/fixtures/page_class");
    fs::create_dir_all(&fixtures_dir)?;

    // 1. Vector pure: Born-digital text PDF
    println!("\n1. Generating vector_pure fixture...");
    let vector_dir = fixtures_dir.join("vector_pure");
    fs::create_dir_all(&vector_dir)?;
    generate_vector_pure_pdf(&vector_dir)?;

    // 2. Scanned single: Image-only PDF
    println!("2. Generating scanned_single fixture...");
    let scanned_dir = fixtures_dir.join("scanned_single");
    fs::create_dir_all(&scanned_dir)?;
    generate_scanned_single_pdf(&scanned_dir)?;

    // 3. BrokenVector: Invisible text + image
    println!("3. Generating brokenvector_pdfa fixture...");
    let broken_dir = fixtures_dir.join("brokenvector_pdfa");
    fs::create_dir_all(&broken_dir)?;
    generate_brokenvector_pdf(&broken_dir)?;

    // 4. Hybrid: Text header + scanned body
    println!("4. Generating hybrid_header_body fixture...");
    let hybrid_dir = fixtures_dir.join("hybrid_header_body");
    fs::create_dir_all(&hybrid_dir)?;
    generate_hybrid_pdf(&hybrid_dir)?;

    println!("\n==========================================");
    println!("Page Classification Fixtures Generated");
    println!("==========================================");

    // Print sizes
    for fixture_name in &["vector_pure", "scanned_single", "brokenvector_pdfa", "hybrid_header_body"] {
        let fixture_dir = fixtures_dir.join(fixture_name);
        let pdf_path = fixture_dir.join("source.pdf");
        if let Ok(metadata) = fs::metadata(&pdf_path) {
            let size_kb = metadata.len() as f64 / 1024.0;
            println!("  - {}/source.pdf: {:.2} KB", fixture_name, size_kb);
        }
    }

    Ok(())
}

/// Generate a pure vector PDF (born-digital text)
fn generate_vector_pure_pdf(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, Dictionary};

    let mut doc = Document::with_version("1.5");

    // Create font
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", "Font");
    font_dict.set("Subtype", "Type1");
    font_dict.set("BaseFont", "Helvetica");
    let font_id = doc.add_object(font_dict);

    // Resources
    let mut resources = Dictionary::new();
    let mut font_resources = Dictionary::new();
    font_resources.set("F1", font_id);
    resources.set("Font", font_resources);

    // Content stream: Multiple lines of text with high character count
    let content_text = r#"
        BT /F1 12 Tf 50 750 Td
        (This is a born-digital PDF with pure vector text.) Tj
        0 -15 Td (It contains multiple text operators and high character validity.) Tj
        0 -15 Td (The classification should detect this as a Vector page.) Tj
        0 -15 Td (Lorem ipsum dolor sit amet, consectetur adipiscing elit.) Tj
        0 -15 Td (Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.) Tj
        0 -15 Td (Ut enim ad minim veniam, quis nostrud exercitation ullamco.) Tj
        0 -15 Td (Duis aute irure dolor in reprehenderit in voluptate velit esse.) Tj
        0 -15 Td (Excepteur sint occaecat cupidatat non proident sunt in culpa.) Tj
        ET
    "#;

    let content_bytes = content_text.as_bytes();
    let mut content_dict = Dictionary::new();
    content_dict.set("Length", content_bytes.len() as i32);
    let content_stream = Stream::new(content_dict, content_bytes.to_vec());
    let content_id = doc.add_object(content_stream);

    // Page dictionary
    let page_dict = dictionary! {
        "Type" => "Page",
        "MediaBox" => vec![0.0.into(), 0.0.into(), 612.0.into(), 792.0.into()],
        "Contents" => content_id,
        "Resources" => resources,
        "CropBox" => vec![0.0.into(), 0.0.into(), 612.0.into(), 792.0.into()],
    };
    let page_id = doc.add_object(page_dict);

    // Pages tree
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => 1,
        "Kids" => vec![page_id.into()],
    });

    // Update page with parent reference
    let mut page_obj = doc.get_object(page_id)?.as_dict().cloned()?;
    page_obj.set("Parent", pages_id);
    doc.objects.insert(page_id, Object::Dictionary(page_obj));

    // Catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Save PDF
    let pdf_path = dir.join("source.pdf");
    doc.save(&pdf_path)?;

    // Generate expected.json
    let expected = PageClassExpected {
        class: "Vector".to_string(),
        confidence_min: 0.90,
        hybrid_cells: None,
    };
    let json_path = dir.join("expected.json");
    fs::write(&json_path, serde_json::to_string_pretty(&expected)?)?;

    println!("  Created: {}/source.pdf ({:.2} KB)",
        dir.file_name().unwrap().to_string_lossy(),
        fs::metadata(&pdf_path)?.len() as f64 / 1024.0
    );

    Ok(())
}

/// Generate an image-only scanned PDF
fn generate_scanned_single_pdf(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Dictionary, Stream};

    let mut doc = Document::with_version("1.5");

    // Create a simple 1x1 pixel white image (minimal image object)
    let image_data = vec![0u8; 4]; // 1x1 white pixel in RGB
    let mut image_stream = Stream::new(dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => 1,
        "Height" => 1,
        "BitsPerComponent" => 8,
        "ColorSpace" => "DeviceRGB",
        "Length" => image_data.len() as i32,
    }, image_data);
    let image_id = doc.add_object(image_stream);

    // Resources with image
    let mut resources = Dictionary::new();
    let mut xobject = Dictionary::new();
    xobject.set("Im1", image_id);
    resources.set("XObject", xobject);

    // Content stream: Draw image covering most of the page
    let content_text = r#"
        q 612 792 scale
        /Im1 Do
        Q
    "#;

    let content_bytes = content_text.as_bytes();
    let mut content_dict = Dictionary::new();
    content_dict.set("Length", content_bytes.len() as i32);
    let content_stream = Stream::new(content_dict, content_bytes.to_vec());
    let content_id = doc.add_object(content_stream);

    // Page dictionary
    let page_dict = dictionary! {
        "Type" => "Page",
        "MediaBox" => vec![0.0.into(), 0.0.into(), 612.0.into(), 792.0.into()],
        "Contents" => content_id,
        "Resources" => resources,
    };
    let page_id = doc.add_object(page_dict);

    // Pages tree
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => 1,
        "Kids" => vec![page_id.into()],
    });

    // Update page with parent reference
    let mut page_obj = doc.get_object(page_id)?.as_dict().cloned()?;
    page_obj.set("Parent", pages_id);
    doc.objects.insert(page_id, Object::Dictionary(page_obj));

    // Catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Save PDF
    let pdf_path = dir.join("source.pdf");
    doc.save(&pdf_path)?;

    // Generate expected.json
    let expected = PageClassExpected {
        class: "Scanned".to_string(),
        confidence_min: 0.90,
        hybrid_cells: None,
    };
    let json_path = dir.join("expected.json");
    fs::write(&json_path, serde_json::to_string_pretty(&expected)?)?;

    println!("  Created: {}/source.pdf ({:.2} KB)",
        dir.file_name().unwrap().to_string_lossy(),
        fs::metadata(&pdf_path)?.len() as f64 / 1024.0
    );

    Ok(())
}

/// Generate a BrokenVector PDF (invisible text + image)
fn generate_brokenvector_pdf(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Dictionary, Stream};

    let mut doc = Document::with_version("1.5");

    // Create font
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", "Font");
    font_dict.set("Subtype", "Type1");
    font_dict.set("BaseFont", "Helvetica");
    let font_id = doc.add_object(font_dict);

    // Create a 1x1 white pixel image
    let image_data = vec![255u8; 4];
    let mut image_stream = Stream::new(dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => 1,
        "Height" => 1,
        "BitsPerComponent" => 8,
        "ColorSpace" => "DeviceRGB",
        "Length" => image_data.len() as i32,
    }, image_data);
    let image_id = doc.add_object(image_stream);

    // Resources
    let mut resources = Dictionary::new();
    let mut font_resources = Dictionary::new();
    font_resources.set("F1", font_id);
    resources.set("Font", font_resources);
    let mut xobject = Dictionary::new();
    xobject.set("Im1", image_id);
    resources.set("XObject", xobject);

    // Content stream: Invisible text (Tr=3) + full-page image
    // The text is there but invisible, simulating a bad OCR overlay
    let content_text = r#"
        BT /F1 12 Tf 50 750 Td 3 Tr
        (This text is invisible Tr=3 overlay over scanned image.) Tj
        0 -15 Td (It represents a broken vector PDF with bad OCR layer.) Tj
        0 -15 Td (Classification should detect this as BrokenVector.) Tj
        ET
        q 612 792 scale
        /Im1 Do
        Q
    "#;

    let content_bytes = content_text.as_bytes();
    let mut content_dict = Dictionary::new();
    content_dict.set("Length", content_bytes.len() as i32);
    let content_stream = Stream::new(content_dict, content_bytes.to_vec());
    let content_id = doc.add_object(content_stream);

    // Page dictionary
    let page_dict = dictionary! {
        "Type" => "Page",
        "MediaBox" => vec![0.0.into(), 0.0.into(), 612.0.into(), 792.0.into()],
        "Contents" => content_id,
        "Resources" => resources,
    };
    let page_id = doc.add_object(page_dict);

    // Pages tree
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => 1,
        "Kids" => vec![page_id.into()],
    });

    // Update page with parent reference
    let mut page_obj = doc.get_object(page_id)?.as_dict().cloned()?;
    page_obj.set("Parent", pages_id);
    doc.objects.insert(page_id, Object::Dictionary(page_obj));

    // Catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Save PDF
    let pdf_path = dir.join("source.pdf");
    doc.save(&pdf_path)?;

    // Generate expected.json
    let expected = PageClassExpected {
        class: "BrokenVector".to_string(),
        confidence_min: 0.90,
        hybrid_cells: None,
    };
    let json_path = dir.join("expected.json");
    fs::write(&json_path, serde_json::to_string_pretty(&expected)?)?;

    println!("  Created: {}/source.pdf ({:.2} KB)",
        dir.file_name().unwrap().to_string_lossy(),
        fs::metadata(&pdf_path)?.len() as f64 / 1024.0
    );

    Ok(())
}

/// Generate a Hybrid PDF (text header + scanned body)
fn generate_hybrid_pdf(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Dictionary, Stream};

    let mut doc = Document::with_version("1.5");

    // Create font
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", "Font");
    font_dict.set("Subtype", "Type1");
    font_dict.set("BaseFont", "Helvetica");
    let font_id = doc.add_object(font_dict);

    // Create a 1x1 white pixel image for the body
    let image_data = vec![255u8; 4];
    let mut image_stream = Stream::new(dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => 1,
        "Height" => 1,
        "BitsPerComponent" => 8,
        "ColorSpace" => "DeviceRGB",
        "Length" => image_data.len() as i32,
    }, image_data);
    let image_id = doc.add_object(image_stream);

    // Resources
    let mut resources = Dictionary::new();
    let mut font_resources = Dictionary::new();
    font_resources.set("F1", font_id);
    resources.set("Font", font_resources);
    let mut xobject = Dictionary::new();
    xobject.set("Im1", image_id);
    resources.set("XObject", xobject);

    // Content stream: Text header (top 25%) + image body (bottom 75%)
    // Header: visible text in the top portion
    // Body: image covering the bottom portion
    let content_text = r#"
        BT /F1 14 Tf 50 750 Td
        (This is a HYBRID document with vector text header) Tj
        0 -20 Td (The header contains selectable text) Tj
        0 -20 Td (Below this header is a scanned image body) Tj
        ET
        q
        0 0 612 560 re  W n
        612 792 scale
        /Im1 Do
        Q
    "#;

    let content_bytes = content_text.as_bytes();
    let mut content_dict = Dictionary::new();
    content_dict.set("Length", content_bytes.len() as i32);
    let content_stream = Stream::new(content_dict, content_bytes.to_vec());
    let content_id = doc.add_object(content_stream);

    // Page dictionary
    let page_dict = dictionary! {
        "Type" => "Page",
        "MediaBox" => vec![0.0.into(), 0.0.into(), 612.0.into(), 792.0.into()],
        "Contents" => content_id,
        "Resources" => resources,
    };
    let page_id = doc.add_object(page_dict);

    // Pages tree
    let pages_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Count" => 1,
        "Kids" => vec![page_id.into()],
    });

    // Update page with parent reference
    let mut page_obj = doc.get_object(page_id)?.as_dict().cloned()?;
    page_obj.set("Parent", pages_id);
    doc.objects.insert(page_id, Object::Dictionary(page_obj));

    // Catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Save PDF
    let pdf_path = dir.join("source.pdf");
    doc.save(&pdf_path)?;

    // Generate expected.json
    // For hybrid, we expect specific hybrid_cells (bottom rows of the 8x8 grid)
    // The image covers bottom 75% of page, which corresponds to rows 2-7 (6 rows = 48 cells)
    let hybrid_cells: Vec<usize> = (16..64).collect(); // rows 2-7

    let expected = PageClassExpected {
        class: "Hybrid".to_string(),
        confidence_min: 0.15,
        hybrid_cells: Some(hybrid_cells),
    };
    let json_path = dir.join("expected.json");
    fs::write(&json_path, serde_json::to_string_pretty(&expected)?)?;

    println!("  Created: {}/source.pdf ({:.2} KB)",
        dir.file_name().unwrap().to_string_lossy(),
        fs::metadata(&pdf_path)?.len() as f64 / 1024.0
    );

    Ok(())
}

/// Expected page classification for a fixture
#[derive(Debug, Serialize)]
struct PageClassExpected {
    /// Expected class name (Vector, Scanned, Hybrid, BrokenVector)
    class: String,
    /// Minimum confidence threshold (actual confidence may vary slightly)
    confidence_min: f32,
    /// For Hybrid pages: expected scanned cell indexes
    hybrid_cells: Option<Vec<usize>>,
}

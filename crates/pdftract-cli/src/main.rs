use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod codegen;
mod password;
use codegen::Language;

#[derive(Parser)]
#[command(name = "pdftract")]
#[command(about = "pdftract CLI - PDF extraction and conformance testing", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare actual results against expected values with tolerances (for conformance testing)
    Compare {
        /// Path to the actual results JSON
        actual: PathBuf,
        /// Path to the expected results JSON
        expected: PathBuf,
        /// Path to the tolerances JSON (optional)
        #[arg(short, long)]
        tolerances: Option<PathBuf>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Run SDK conformance test suite
    Conformance {
        /// Path to the conformance suite JSON
        #[arg(short, long, default_value = "tests/sdk-conformance/cases.json")]
        suite: PathBuf,
        /// SDK name
        #[arg(short, long, default_value = "pdftract")]
        sdk: String,
        /// SDK version
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
        /// Output report path
        #[arg(short, long, default_value = "conformance-report.json")]
        output: PathBuf,
    },
    /// SDK code generation commands
    Sdk {
        #[command(subcommand)]
        sdk_command: SdkCommands,
    },
    /// Extract text and structure from a PDF file
    Extract {
        /// Path to the PDF file (use '-' for stdin)
        input: PathBuf,

        /// Read password from stdin (one line, terminated by newline)
        #[arg(long, conflicts_with = "password")]
        password_stdin: bool,

        /// PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
        #[arg(long, conflicts_with = "password_stdin")]
        password: Option<String>,

        /// Output format (json, text, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
enum SdkCommands {
    /// Generate SDK skeleton from templates
    Codegen {
        /// Target language
        #[arg(short, long)]
        lang: Language,
        /// Output directory
        #[arg(short, long)]
        out: PathBuf,
        /// Version string (defaults to current pdftract version)
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
    },
    /// Validate existing SDK against current generator output
    Validate {
        /// Target language
        #[arg(short, long)]
        lang: Language,
        /// Path to existing SDK directory
        #[arg(short, long)]
        sdk_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compare {
            actual,
            expected,
            tolerances,
            format,
        } => {
            cmd_compare(actual, expected, tolerances, &format)?;
        }
        Commands::Conformance {
            suite,
            sdk,
            version,
            output,
        } => {
            cmd_conformance(suite, &sdk, &version, output)?;
        }
        Commands::Sdk { sdk_command } => {
            cmd_sdk(sdk_command)?;
        }
        Commands::Extract {
            input,
            password_stdin,
            password,
            format,
        } => {
            if let Err(e) = cmd_extract(input, password_stdin, password, &format) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn cmd_extract(
    input: PathBuf,
    password_stdin: bool,
    password: Option<String>,
    format: &str,
) -> Result<()> {
    // Resolve password using the priority order defined in TH-07
    let resolved_password = match password::resolve_password(password_stdin, password) {
        Ok(pwd) => pwd,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(password::EXIT_USAGE_ERROR as i32);
        }
    };

    // Report password status (never the value itself)
    if resolved_password.is_some() {
        eprintln!("Password provided via secure channel");
    }

    // Stub: For now, just report what would be extracted
    // Full extraction implementation is in separate beads
    eprintln!("Extract command invoked");
    eprintln!("  Input: {:?}", input);
    eprintln!("  Format: {}", format);
    eprintln!("  Password: {}", if resolved_password.is_some() { "yes" } else { "no" });

    // TODO: Implement actual PDF extraction
    // This will be done in the extraction implementation beads
    eprintln!("NOTE: Full extraction implementation is pending (see plan for extraction beads)");

    Ok(())
}

fn cmd_compare(actual: PathBuf, expected: PathBuf, tolerances: Option<PathBuf>, format: &str) -> Result<()> {
    let actual_json = fs::read_to_string(&actual)
        .context(format!("Failed to read actual results from {:?}", actual))?;
    let actual_val: serde_json::Value = serde_json::from_str(&actual_json)
        .context("Failed to parse actual results as JSON")?;

    let expected_json = fs::read_to_string(&expected)
        .context(format!("Failed to read expected results from {:?}", expected))?;
    let expected_val: serde_json::Value = serde_json::from_str(&expected_json)
        .context("Failed to parse expected results as JSON")?;

    let tolerances_val = if let Some(tol_path) = tolerances {
        let tol_json = fs::read_to_string(&tol_path)
            .context(format!("Failed to read tolerances from {:?}", tol_path))?;
        Some(serde_json::from_str::<serde_json::Value>(&tol_json)
            .context("Failed to parse tolerances as JSON")?)
    } else {
        None
    };

    let result = compare_values(&actual_val, &expected_val, tolerances_val.as_ref())?;

    match format {
        "json" => {
            let output = serde_json::to_string_pretty(&result)?;
            println!("{}", output);
        }
        _ => {
            print_compare_result(&result);
        }
    }

    Ok(())
}

fn cmd_sdk(command: SdkCommands) -> Result<()> {
    match command {
        SdkCommands::Codegen { lang, out, version } => {
            let template_dir = PathBuf::from("templates/sdk-skeleton");
            let mut generator = codegen::CodeGenerator::new(&template_dir, version)?;
            generator.generate(lang, &out)?;
            println!("\nSDK generated successfully to: {}", out.display());
        }
        SdkCommands::Validate { lang, sdk_dir } => {
            let template_dir = PathBuf::from("templates/sdk-skeleton");
            let mut generator = codegen::CodeGenerator::new(&template_dir, "0.1.0".to_string())?;
            let result = generator.validate(lang, &sdk_dir)?;

            if result.differences.is_empty() {
                println!("SDK is up to date with current generator output.");
            } else {
                println!("Found {} differences:", result.differences.len());
                for diff in &result.differences {
                    match diff.kind {
                        codegen::DifferenceKind::MissingInSdk => {
                            println!("  MISSING: {}", diff.path);
                        }
                        codegen::DifferenceKind::ExtraInSdk => {
                            println!("  EXTRA: {}", diff.path);
                        }
                        codegen::DifferenceKind::ContentDiff => {
                            println!("  MODIFIED: {}", diff.path);
                        }
                    }
                }
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn cmd_conformance(suite: PathBuf, sdk: &str, version: &str, output: PathBuf) -> Result<()> {
    println!("Running conformance suite: {:?}", suite);
    println!("SDK: {} v{}", sdk, version);
    println!("Output: {:?}", output);

    let suite_json = fs::read_to_string(&suite)
        .context(format!("Failed to read suite from {:?}", suite))?;
    let suite_val: serde_json::Value = serde_json::from_str(&suite_json)
        .context("Failed to parse suite as JSON")?;

    let cases = suite_val
        .get("cases")
        .and_then(|v| v.as_array())
        .context("Suite missing 'cases' array")?;

    println!("\nFound {} test cases", cases.len());

    // This is a stub - actual implementation would invoke the SDK
    let results: Vec<serde_json::Value> = cases
        .iter()
        .map(|case| {
            serde_json::json!({
                "id": case.get("id").unwrap_or(&serde_json::json!("unknown")),
                "status": "skip",
                "error": "SDK conformance runner not yet implemented - use language-specific runner"
            })
        })
        .collect();

    let report = serde_json::json!({
        "sdk": sdk,
        "sdk_version": version,
        "suite_version": suite_val.get("version").unwrap_or(&serde_json::json!("unknown")),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "results": results,
        "summary": {
            "total": results.len(),
            "passed": 0,
            "failed": 0,
            "skipped": results.len(),
            "errors": 0
        }
    });

    fs::write(&output, serde_json::to_string_pretty(&report)?)
        .context(format!("Failed to write report to {:?}", output))?;

    println!("\nReport written to {:?}", output);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
enum CompareResult {
    Pass,
    Fail { reason: String },
    Missing,
}

fn compare_values(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    tolerances: Option<&serde_json::Value>,
) -> Result<std::collections::HashMap<String, CompareResult>> {
    let mut results = std::collections::HashMap::new();

    compare_recursive(actual, expected, tolerances, "", &mut results);

    Ok(results)
}

fn compare_recursive(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    tolerances: Option<&serde_json::Value>,
    path: &str,
    results: &mut std::collections::HashMap<String, CompareResult>,
) {
    match (actual, expected) {
        // Handle min/max constraints
        (serde_json::Value::Number(act), serde_json::Value::Object(exp)) => {
            if let Some(min) = exp.get("min").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v < min) {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!("value {} is less than minimum {}", act, min),
                        },
                    );
                    return;
                }
            }
            if let Some(max) = exp.get("max").and_then(|v| v.as_i64()) {
                if act.as_i64().map_or(true, |v| v > max) {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!("value {} is greater than maximum {}", act, max),
                        },
                    );
                    return;
                }
            }
            if let Some(val) = exp.get("value") {
                let tol = find_tolerance(tolerances, path);
                let result = compare_with_tolerance(act, val, tol);
                results.insert(path.to_string(), result);
            } else {
                results.insert(path.to_string(), CompareResult::Pass);
            }
        }
        // String constraints
        (serde_json::Value::String(act), serde_json::Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min_length").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() < min_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "string length {} is less than minimum {}",
                                act.len(),
                                min_len
                            ),
                        },
                    );
                    return;
                }
            }
            if let Some(containers) = exp.get("contains").and_then(|v| v.as_array()) {
                for substring in containers {
                    if let Some(s) = substring.as_str() {
                        if !act.contains(s) {
                            results.insert(
                                path.to_string(),
                                CompareResult::Fail {
                                    reason: format!("string does not contain '{}'", s),
                                },
                            );
                            return;
                        }
                    }
                }
            }
            results.insert(path.to_string(), CompareResult::Pass);
        }
        // Array length constraints
        (serde_json::Value::Array(act), serde_json::Value::Object(exp)) => {
            if let Some(min_len) = exp.get("min").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() < min_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "array length {} is less than minimum {}",
                                act.len(),
                                min_len
                            ),
                        },
                    );
                    return;
                }
            }
            if let Some(max_len) = exp.get("max").and_then(|v| v.as_u64()).map(|v| v as usize) {
                if act.len() > max_len {
                    results.insert(
                        path.to_string(),
                        CompareResult::Fail {
                            reason: format!(
                                "array length {} is greater than maximum {}",
                                act.len(),
                                max_len
                            ),
                        },
                    );
                    return;
                }
            }
            results.insert(path.to_string(), CompareResult::Pass);
        }
        // Direct comparison
        (a, e) => {
            if a == e {
                results.insert(path.to_string(), CompareResult::Pass);
            } else {
                results.insert(
                    path.to_string(),
                    CompareResult::Fail {
                        reason: format!("expected {:?}, got {:?}", e, a),
                    },
                );
            }
        }
    }
}

fn compare_with_tolerance(
    actual: &serde_json::Number,
    expected: &serde_json::Value,
    tolerance: Option<&serde_json::Value>,
) -> CompareResult {
    let act_val = actual.as_f64().unwrap();
    let exp_val = match expected {
        serde_json::Value::Number(n) => n.as_f64().unwrap(),
        _ => return CompareResult::Fail { reason: "expected value is not a number".to_string() },
    };

    if let Some(tol) = tolerance {
        if let Some(obj) = tol.as_object() {
            if let Some(abs_tol) = obj.get("abs").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                if diff <= abs_tol {
                    return CompareResult::Pass;
                }
            }
            if let Some(rel_tol) = obj.get("rel").and_then(|v| v.as_f64()) {
                let diff = (act_val - exp_val).abs();
                let avg = (act_val + exp_val) / 2.0;
                if avg > 0.0 && diff / avg <= rel_tol {
                    return CompareResult::Pass;
                }
            }
        }
    }

    // Direct comparison
    if (act_val - exp_val).abs() < f64::EPSILON {
        CompareResult::Pass
    } else {
        CompareResult::Fail {
            reason: format!("numeric mismatch: {} vs {}", act_val, exp_val),
        }
    }
}

fn find_tolerance<'a>(
    tolerances: Option<&'a serde_json::Value>,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let tol = tolerances?;
    if let Some(obj) = tol.as_object() {
        // Try exact path match
        if let Some(val) = obj.get(path) {
            return Some(val);
        }
        // Try wildcard patterns
        for (key, val) in obj {
            if key.contains('*') {
                let pattern = key.replace('*', ".*");
                if let Ok(re) = regex::Regex::new(&pattern) {
                    if re.is_match(path) {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

fn print_compare_result(results: &std::collections::HashMap<String, CompareResult>) {
    let mut passed = 0;
    let mut failed = 0;

    for (path, result) in results {
        match result {
            CompareResult::Pass => {
                passed += 1;
            }
            CompareResult::Fail { reason } => {
                failed += 1;
                eprintln!("FAIL [{}]: {}", path, reason);
            }
            CompareResult::Missing => {
                failed += 1;
                eprintln!("MISSING [{}]: value not found in actual", path);
            }
        }
    }

    println!("\nComparison complete:");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    if failed > 0 {
        std::process::exit(1);
    }
}

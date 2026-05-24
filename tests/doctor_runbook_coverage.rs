//! CI gate: Verify runbook troubleshooting table covers all doctor checks
//!
//! This test ensures that every check in the doctor registry has a corresponding
//! row in the troubleshooting table in docs/operations/manual-platform-smoke.md.
//!
//! Bead: pdftract-653ah (runbook integration)

use std::collections::HashSet;

fn main() {
    // Load the runbook
    let runbook_path = std::path::Path::new("docs/operations/manual-platform-smoke.md");
    let runbook_content = std::fs::read_to_string(runbook_path)
        .expect("Runbook file not found. Has docs/operations/manual-platform-smoke.md been created?");

    // Extract check names from the troubleshooting table
    // The table has rows like: "| tesseract install (FAIL) | ... |"
    let mut table_checks = HashSet::new();
    for line in runbook_content.lines() {
        if let Some(start) = line.find("| ") {
            if let Some(end) = line.find(" (FAIL)") {
                let check_name = line[start + 2..end].trim();
                table_checks.insert(check_name.to_string());
            }
            if let Some(end) = line.find(" (WARN)") {
                let check_name = line[start + 2..end].trim();
                table_checks.insert(check_name.to_string());
            }
        }
    }

    // Get all check names from the doctor registry
    // We use the known check list instead of runtime registry
    let expected_checks = vec![
        "pdftract binary",
        "tesseract install",
        "tesseract languages",
        "leptonica install",
        "libtiff",
        "libopenjp2",
        "pdfium native lib",
        "network reachability",
        "cache directory",
        "profile search path",
        "ulimit -n",
        "available RAM",
        "system locale",
        "temp dir writable",
    ];

    // Verify each expected check is in the table
    let mut missing_checks = Vec::new();
    for check in &expected_checks {
        if !table_checks.contains(*check) {
            missing_checks.push(*check);
        }
    }

    if !missing_checks.is_empty() {
        eprintln!(
            "ERROR: Runbook troubleshooting table is missing checks: {:?}",
            missing_checks
        );
        eprintln!("Please add rows to docs/operations/manual-platform-smoke.md for each missing check.");
        std::process::exit(1);
    }

    // Verify table doesn't have orphaned checks (checks in table but not in registry)
    let expected_set: HashSet<String> = expected_checks.iter().map(|s| s.to_string()).collect();
    let orphaned: Vec<_> = table_checks
        .difference(&expected_set)
        .collect();

    if !orphaned.is_empty() {
        eprintln!(
            "ERROR: Runbook troubleshooting table has orphaned checks (in table but not in registry): {:?}",
            orphaned
        );
        eprintln!("Please remove these rows or add the checks to the doctor registry.");
        std::process::exit(1);
    }

    println!("✓ Runbook troubleshooting table covers all doctor checks");
    println!("✓ No orphaned checks in table");
}

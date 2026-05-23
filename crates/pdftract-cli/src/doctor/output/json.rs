//! JSON output for doctor subcommand

use crate::doctor::{CheckResult, CheckStatus};

/// Output results as JSON (single line by default)
pub fn output_json(results: &[CheckResult]) {
    let mut ok = 0;
    let mut warn = 0;
    let mut fail = 0;
    let mut not_applicable = 0;

    let checks_json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let status_str = match r.status {
                CheckStatus::Ok => {
                    ok += 1;
                    "OK"
                }
                CheckStatus::Warn => {
                    warn += 1;
                    "WARN"
                }
                CheckStatus::Fail => {
                    fail += 1;
                    "FAIL"
                }
                CheckStatus::NotApplicable => {
                    not_applicable += 1;
                    "N/A"
                }
            };

            serde_json::json!({
                "name": r.name,
                "status": status_str,
                "detail": r.detail,
            })
        })
        .collect();

    let output = serde_json::json!({
        "summary": {
            "ok": ok,
            "warn": warn,
            "fail": fail,
            "not_applicable": not_applicable,
        },
        "checks": checks_json,
    });

    // Single line JSON (not pretty-printed)
    println!("{}", serde_json::to_string(&output).unwrap());
}

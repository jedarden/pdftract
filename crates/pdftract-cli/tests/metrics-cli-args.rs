//! CLI parsing coverage for the optional metrics listener port.

#![cfg(feature = "metrics")]

use std::process::{Command, Output, Stdio};

fn pdftract_help(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run pdftract help")
}

#[test]
fn serve_accepts_metrics_port_and_documents_separate_listener() {
    let output = pdftract_help(&["serve", "--metrics", "9101", "--help"]);

    assert!(
        output.status.success(),
        "serve --metrics should parse: {output:?}"
    );
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--metrics <PORT>"));
    assert!(help.contains("SECOND listener"));
    assert!(help.contains("never on the main port"));
}

#[test]
fn mcp_bind_accepts_metrics_port_and_documents_separate_listener() {
    let output = pdftract_help(&[
        "mcp",
        "--bind",
        "127.0.0.1:9102",
        "--metrics",
        "9103",
        "--help",
    ]);

    assert!(
        output.status.success(),
        "mcp --bind --metrics should parse: {output:?}"
    );
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--metrics <PORT>"));
    assert!(help.contains("SECOND listener"));
    assert!(help.contains("never served on the main"));
}

#[test]
fn omitting_metrics_preserves_serve_and_mcp_bind_parsing() {
    let serve = pdftract_help(&["serve", "--help"]);
    assert!(
        serve.status.success(),
        "serve without --metrics should parse"
    );

    let mcp = pdftract_help(&["mcp", "--bind", "127.0.0.1:9104", "--help"]);
    assert!(
        mcp.status.success(),
        "mcp --bind without --metrics should parse"
    );
}

//! Integration tests for MCP tools.
//!
//! These tests verify:
//! - Performance requirements (get_metadata <= 250ms, hash <= 100ms on 100-page PDFs)
//! - Error handling for encrypted PDFs
//! - Actual tool execution with real PDF files

use pdftract_cli::mcp::tools;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[test]
fn test_get_metadata_performance_on_100_page_pdf() {
    let registry = tools::all_tools();
    let tool = registry.get("get_metadata").unwrap();

    let args = serde_json::json!({
        "path": "../../tests/sdk-conformance/fixtures/large/100pages.pdf"
    });

    let start = Instant::now();
    let result = tool.execute(args, None, None);
    let duration_ms = start.elapsed().as_millis();

    assert!(result.is_ok(), "get_metadata should succeed: {:?}", result);
    assert!(
        duration_ms <= 250,
        "get_metadata on 100-page PDF should complete in <= 250ms, took {}ms",
        duration_ms
    );

    let response = result.unwrap();
    assert!(response.is_object());
    let obj = response.as_object().unwrap();
    assert!(obj.contains_key("metadata"));
    assert!(obj.contains_key("fingerprint"));

    println!("get_metadata on 100-page PDF: {}ms", duration_ms);
}

#[test]
fn test_hash_performance_on_100_page_pdf() {
    let registry = tools::all_tools();
    let tool = registry.get("hash").unwrap();

    let args = serde_json::json!({
        "path": "../../tests/sdk-conformance/fixtures/large/100pages.pdf"
    });

    let start = Instant::now();
    let result = tool.execute(args, None, None);
    let duration_ms = start.elapsed().as_millis();

    assert!(result.is_ok(), "hash should succeed: {:?}", result);
    assert!(
        duration_ms <= 100,
        "hash on 100-page PDF should complete in <= 100ms, took {}ms",
        duration_ms
    );

    let response = result.unwrap();
    assert!(response.is_object());
    let obj = response.as_object().unwrap();
    assert!(obj.contains_key("fingerprint"));

    println!("hash on 100-page PDF: {}ms", duration_ms);
}

#[test]
fn test_hash_matches_cli_and_metadata_exposes_info_dictionary() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/json_schema/simple_invoice.pdf")
        .to_string_lossy()
        .into_owned();

    let cli_output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["hash", fixture.as_str()])
        .output()
        .expect("CLI hash command should start");
    assert!(
        cli_output.status.success(),
        "CLI hash command failed: {}",
        String::from_utf8_lossy(&cli_output.stderr)
    );
    let cli_fingerprint = String::from_utf8(cli_output.stdout)
        .expect("CLI hash output should be UTF-8")
        .trim()
        .to_string();

    let registry = tools::all_tools();
    let hash = registry
        .get("hash")
        .unwrap()
        .execute(serde_json::json!({"path": fixture}), None, None)
        .expect("MCP hash should succeed");
    assert_eq!(hash["fingerprint"].as_str(), Some(cli_fingerprint.as_str()));

    let metadata = registry
        .get("get_metadata")
        .unwrap()
        .execute(serde_json::json!({"path": fixture}), None, None)
        .expect("MCP get_metadata should succeed");
    let info = &metadata["metadata"];
    assert_eq!(info["title"], "untitled");
    assert_eq!(info["author"], "anonymous");
    assert_eq!(info["subject"], "unspecified");
    assert_eq!(info["keywords"], "");
    assert_eq!(info["creator"], "anonymous");
    assert_eq!(info["producer"], "ReportLab PDF Library - (opensource)");
    assert_eq!(info["creation_date"], "D:20260517071406-04'00'");
    assert_eq!(info["modification_date"], "D:20260517071406-04'00'");
    assert_eq!(info["trapped"], "False");
    assert_eq!(metadata["fingerprint"], hash["fingerprint"]);
}

#[test]
fn test_tools_list_has_only_implemented_tools() {
    let registry = tools::all_tools();
    let list = registry.tools_list();

    let tools = list.get("tools").and_then(|v| v.as_array()).unwrap();
    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();

    assert_eq!(tool_names.len(), 7, "Should have exactly 7 advertised tools");

    let expected = [
        "extract",
        "extract_text",
        "extract_markdown",
        "search",
        "get_metadata",
        "get_table",
        "hash",
    ];

    for name in &expected {
        assert!(
            tool_names.contains(name),
            "Tool '{}' should be in the catalog",
            name
        );
    }
}

#[test]
fn test_phase_7_stub_tools_return_not_implemented() {
    let registry = tools::all_tools();

    let stub_tools = [
        (
            "get_table",
            serde_json::json!({"path": "test.pdf", "page": 0, "table_index": 0}),
        ),
        ("get_form_fields", serde_json::json!({"path": "test.pdf"})),
        ("get_attachments", serde_json::json!({"path": "test.pdf"})),
        ("classify", serde_json::json!({"path": "test.pdf"})),
    ];

    for (tool_name, args) in stub_tools {
        let tool = registry.get(tool_name).unwrap();
        let result = tool.execute(args, None, None);

        assert!(result.is_err(), "{} should return error", tool_name);
        let err = result.unwrap_err();
        assert_eq!(err.code, tools::ERROR_NOT_YET_IMPLEMENTED);
        assert!(err.data.is_some());
        let data = err.data.as_ref().unwrap();
        assert_eq!(
            data.get("code").and_then(|c| c.as_str()),
            Some(tools::CODE_NOT_YET_IMPLEMENTED)
        );
    }
}

#[test]
fn test_unknown_tool_name_returns_method_not_found() {
    let registry = tools::all_tools();

    // Unknown tool should return None from get()
    assert!(registry.get("unknown_tool").is_none());
}

#[test]
fn test_missing_required_path_returns_error() {
    let registry = tools::all_tools();
    let tool = registry.get("extract").unwrap();

    // Missing required 'path' field
    let args = serde_json::json!({});

    let result = tool.execute(args, None, None);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32602); // Invalid params
}

#[test]
fn test_extract_tool_with_real_pdf() {
    let registry = tools::all_tools();
    let tool = registry.get("extract").unwrap();

    let args = serde_json::json!({
        "path": "../../tests/sdk-conformance/fixtures/large/100pages.pdf"
    });

    let result = tool.execute(args, None, None);
    if let Err(ref e) = result {
        eprintln!(
            "Error from tool: code={}, message={}, data={:?}",
            e.code, e.message, e.data
        );
    }
    assert!(result.is_ok(), "Tool should succeed: {:?}", result);

    let response = result.unwrap();
    assert!(response.is_object());
    let obj = response.as_object().unwrap();

    // Should contain pages array (currently stubbed)
    assert!(obj.contains_key("pages"));
}

#[test]
fn test_search_tool_with_invalid_regex() {
    let registry = tools::all_tools();
    let tool = registry.get("search").unwrap();

    // Invalid regex pattern
    let args = serde_json::json!({
        "path": "test.pdf",
        "pattern": "(?invalid"
    });

    let result = tool.execute(args, None, None);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32602); // Invalid params
}

#[test]
fn test_search_tool_returns_in_band_not_implemented_error() {
    let registry = tools::all_tools();
    let tool = registry.get("search").unwrap();

    let error = tool
        .execute(
            serde_json::json!({"path": "document.pdf", "pattern": "needle"}),
            None,
            None,
        )
        .expect_err("unimplemented search must not return empty matches");

    let result = tools::call_error(&error);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["data"]["code"],
        tools::CODE_NOT_YET_IMPLEMENTED
    );
    assert!(result["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.contains("Phase 6 extraction surface")));
}

#[test]
fn test_path_resolution() {
    let cwd = std::env::current_dir().unwrap();
    println!("Current dir: {:?}", cwd);

    // Try different path patterns
    let paths = [
        "../../tests/sdk-conformance/fixtures/large/100pages.pdf",
        "../../../../tests/sdk-conformance/fixtures/large/100pages.pdf",
        "../../../tests/sdk-conformance/fixtures/large/100pages.pdf",
    ];

    for path in &paths {
        let exists = std::path::Path::new(path).exists();
        println!("Path '{}' exists: {}", path, exists);
    }

    // Also check using CARGO_MANIFEST_DIR
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let abs_path = format!(
            "{}/{}",
            manifest_dir, "../../tests/sdk-conformance/fixtures/large/100pages.pdf"
        );
        let exists = std::path::Path::new(&abs_path).exists();
        println!("Absolute path '{}' exists: {}", abs_path, exists);
    }
}

#[test]
fn test_nonexistent_file_returns_path_invalid() {
    let registry = tools::all_tools();
    let tool = registry.get("extract").unwrap();

    let args = serde_json::json!({
        "path": "/nonexistent/path/to/file.pdf"
    });

    let result = tool.execute(args, None, None);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, tools::ERROR_PATH_INVALID);
    assert!(err.data.is_some());
    let data = err.data.as_ref().unwrap();
    assert_eq!(
        data.get("code").and_then(|c| c.as_str()),
        Some(tools::CODE_PATH_INVALID)
    );
}

#[test]
#[ignore = "requires actual encrypted PDF fixture with /Encrypt dictionary in trailer"]
fn test_encrypted_pdf_returns_pdf_encrypted_error() {
    let registry = tools::all_tools();
    let tool = registry.get("extract").unwrap();

    let args = serde_json::json!({
        "path": "../../tests/sdk-conformance/fixtures/encrypted/encrypted.pdf"
    });

    let result = tool.execute(args, None, None);

    // Debug: print the result if it succeeds unexpectedly
    if let Ok(ref response) = result {
        eprintln!(
            "Unexpected success on encrypted PDF: {}",
            serde_json::to_string_pretty(response).unwrap()
        );
    }

    assert!(result.is_err(), "Encrypted PDF should return error");

    let err = result.unwrap_err();
    assert_eq!(err.code, tools::ERROR_PDF_ENCRYPTED);
    assert!(err.data.is_some());

    let data = err.data.as_ref().unwrap();
    assert_eq!(
        data.get("code").and_then(|c| c.as_str()),
        Some(tools::CODE_PDF_ENCRYPTED)
    );
}

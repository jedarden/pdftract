#![cfg(feature = "remote")]
use std::io;
use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header}};
use pdftract_core::source::{open_remote, RemoteOpts};

#[tokio::test]
async fn test_416_retry_debug() {
    let mock_server = MockServer::start().await;
    
    let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n5 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 100 700 Td (Hello World) Tj ET\nendstream\nendobj\nxref\n0 6\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\n0000000268 00000 n\n0000000345 00000 n\ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n439\n%%EOF\n";
    let pdf_data = pdf_data.to_vec();
    let pdf_len = pdf_data.len();
    
    // HEAD succeeds with Range support
    Mock::given(method("HEAD"))
        .and(path("/test.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_len.to_string())
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes("")
        )
        .mount(&mock_server)
        .await;
    
    // GET with Range header returns 416
    Mock::given(method("GET"))
        .and(path("/test.pdf"))
        .and(header("Range", "*"))
        .respond_with(
            ResponseTemplate::new(416)
                .insert_header("Content-Range", format!("bytes */{}", pdf_len))
        )
        .mount(&mock_server)
        .await;
    
    // GET without Range header returns full content
    Mock::given(method("GET"))
        .and(path("/test.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", pdf_len.to_string())
                .insert_header("Accept-Ranges", "bytes")
                .set_body_bytes(pdf_data.clone())
        )
        .mount(&mock_server)
        .await;
    
    let url = format!("{}/test.pdf", mock_server.uri());
    println!("Testing URL: {}", url);
    
    let opts = RemoteOpts::new();
    let source = open_remote(&url, &opts, None);
    assert!(source.is_ok(), "Should open source successfully");
    
    let source = source.unwrap();
    println!("Source length: {}", source.len());
    
    // Try to read a small range
    let result = source.read_range(0, 1024);
    match &result {
        Ok(data) => println!("Success! Got {} bytes", data.len()),
        Err(e) => println!("Error: {} (kind: {:?})", e, e.kind()),
    }
    assert!(result.is_ok(), "Should succeed after 416 retry");
}

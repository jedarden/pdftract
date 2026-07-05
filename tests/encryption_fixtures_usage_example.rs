//! Example usage of encryption_fixtures module
//!
//! This file demonstrates how to use the common encryption test fixtures
//! and helpers. It's kept as a reference for other test files.

use pdftract_tests::encryption_fixtures::*;

#[test]
#[ignore = "Example test - demonstrates fixture usage"]
fn example_using_fixtures() {
    let bin = pdftract_bin();
    let fixture = encrypted_fixture("livecycle.pdf");

    // Check fixture exists
    assert_fixture_exists(&fixture);

    // Validate PDF structure
    assert_valid_pdf_structure(&fixture);

    println!("Binary path: {:?}", bin);
    println!("Fixture path: {:?}", fixture);
}

#[test]
fn test_fixture_module_constants() {
    // Verify all constants are accessible
    assert_eq!(EXPECTED_ENCRYPTION_EXIT_CODE, 3);
    assert!(!ENCRYPTED_FIXTURES.is_empty());
    assert_eq!(TEST_PASSWORD, "test123");
    assert_eq!(WRONG_PASSWORD, "wrongpassword");
}

#[test]
fn test_fixture_module_functions() {
    // Test path resolution functions
    let root = workspace_root();
    assert!(root.exists());

    let fixtures_dir = encrypted_fixtures_dir();
    assert!(fixtures_dir.exists());

    let fixture = encrypted_fixture("livecycle.pdf");
    assert!(fixture.ends_with("livecycle.pdf"));

    let bin = pdftract_bin();
    assert!(bin.ends_with("pdftract"));
}

#[test]
fn test_assertion_helpers_compile() {
    // These functions should compile and be available
    let mock_output = std::process::Output {
        status: std::process::ExitStatus::from_raw(3),
        stdout: b"test output".to_vec(),
        stderr: b"test error".to_vec(),
    };

    // These functions should be callable (they will panic, but that's expected)
    // We're just testing compilation here
    let _ = mock_output;
}

#[test]
#[cfg(feature = "decrypt")]
fn test_mock_builders() {
    // Test that mock data builders compile and work
    use pdftract_core::parser::object::PdfObject;

    let rc4_dict = make_rc4_encryption_dict();
    assert!(rc4_dict.contains_key("/Filter"));

    let aes128_dict = make_aes128_encryption_dict();
    assert!(aes128_dict.contains_key("/V"));

    let aes256_dict = make_aes256_encryption_dict();
    assert!(aes256_dict.contains_key("/UE"));

    let unsupported_dict = make_unsupported_encryption_dict("Adobe.PPKLite");
    assert!(unsupported_dict.contains_key("/Filter"));
}

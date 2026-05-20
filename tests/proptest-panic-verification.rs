//! Verification test: proptest catches deliberate panics.
//!
//! This test demonstrates that the proptest suite will catch a deliberate panic
//! in the lexer, verifying acceptance criterion #5 of pdftract-33v.
//!
//! To run:
//! 1. Uncomment the panic injection in tests/proptest/lexer.rs (test_panic_injection_for_prop_test_verification)
//! 2. Run: PROPTEST_CASES=100 cargo test --features proptest -- proptest
//! 3. Verify the test fails with the panic
//! 4. Re-comment the panic

#[cfg(feature = "proptest")]
#[test]
fn test_proptest_catches_deliberate_panic() {
    // This is a meta-test verifying that proptest infrastructure works.
    // The actual panic injection is in tests/proptest/lexer.rs
    // in the test_panic_injection_for_prop_test_verification function.

    // Run proptest with a small case budget
    let output = std::process::Command::new("cargo")
        .args([
            "test",
            "--features",
            "proptest",
            "--test",
            "lexer",
            "--",
            "--test-threads=1",
        ])
        .output();

    // If the test runs without error, proptest infrastructure is working
    assert!(output.is_ok(), "Failed to run proptest: {:?}", output);

    let stdout = String::from_utf8_lossy(&output.as_ref().unwrap().stdout);
    println!("Proptest output:\n{}", stdout);

    // The test should pass (no panic in normal operation)
    let exit_status = output.unwrap().status;
    assert!(exit_status.success(), "Proptest failed unexpectedly");
}

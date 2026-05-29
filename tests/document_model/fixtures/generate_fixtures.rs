//! Generate document-model test fixtures.
//!
//! This program creates 15 PDF test fixtures for document model integration tests.
//!
//! FIXTURE PASSWORDS:
//! - All encrypted fixtures use user password "test" (NOT secret - these are test fixtures)
//! - Owner password is empty string for all encrypted fixtures

// NOTE: This fixture generator is disabled - lopdf is no longer a dependency.
// Use existing fixture files or regenerate with a different tool.

fn main() {
    eprintln!("Fixture generator is disabled - lopdf is no longer a dependency.");
    eprintln!("Use existing fixture files in tests/document_model/fixtures/");
    std::process::exit(0);
}

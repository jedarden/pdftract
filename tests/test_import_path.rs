// Test file to verify correct import path for pdftract-py
//
// NOTE: This test is disabled because PyPdfProcessor is a PyO3-based type
// that cannot be linked in standalone test binaries. Python binding types
// must be tested within a Python context, not via cargo test.
//
// The library name in Cargo.toml is "pdftract", not "pdftract_py"
// use pdftract::PyPdfProcessor;

fn main() {
    println!("Python binding types must be tested via Python test harness");
    println!("Core functionality should be tested via pdftract_core imports");
}

//! Generate scanned fixture PDFs from ground truth text files.
//!
//! This is a Rust-native alternative to the Python generator.
//! Run with: cargo run --bin generate_scanned_fixtures

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating scanned fixture metadata...");

    // Ensure directories exist
    create_directories()?;

    // Generate fixture metadata
    generate_fixture_metadata()?;

    println!("\nScanned fixtures corpus structure created.");
    println!("\nNOTE: Actual PDF generation requires external tools.");
    println!("Options:");
    println!("  1. Use Python script: generate_scanned_fixtures.py");
    println!("  2. Manual generation (see GEN_MANIFEST.md)");
    println!("  3. Use printpdf or similar crate for native Rust generation");

    Ok(())
}

fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = [
        "tests/fixtures/scanned/receipt",
        "tests/fixtures/scanned/documents",
        "tests/fixtures/scanned/multi-page",
    ];

    for dir in &dirs {
        fs::create_dir_all(dir)?;
        println!("Created directory: {}", dir);
    }

    Ok(())
}

fn generate_fixture_metadata() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple fixture list for reference
    let fixtures = vec![
        FixtureSpec {
            name: "receipt-300dpi",
            dir: "receipt",
            font: "Helvetica",
            font_size: 10,
            pages: 1,
            wer_target: 3.0,
        },
        FixtureSpec {
            name: "invoice-300dpi",
            dir: "documents",
            font: "Helvetica",
            font_size: 11,
            pages: 1,
            wer_target: 3.0,
        },
        FixtureSpec {
            name: "form-300dpi",
            dir: "documents",
            font: "Helvetica",
            font_size: 11,
            pages: 1,
            wer_target: 3.0,
        },
        FixtureSpec {
            name: "doc-10page-300dpi",
            dir: "multi-page",
            font: "Times-Roman",
            font_size: 12,
            pages: 10,
            wer_target: 3.0,
        },
    ];

    let manifest_path = "tests/fixtures/scanned/.fixtures.json";
    let file = File::create(manifest_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "{{")?;
    writeln!(writer, "  \"fixtures\": [")?;

    for (i, fixture) in fixtures.iter().enumerate() {
        writeln!(
            writer,
            "    {}{{",
            if i == 0 { "" } else { ",\n" }
        )?;
        writeln!(writer, r#"      "name": "{}","#, fixture.name)?;
        writeln!(writer, r#"      "dir": "{}","#, fixture.dir)?;
        writeln!(writer, r#"      "font": "{}","#, fixture.font)?;
        writeln!(writer, r#"      "font_size": {},"#, fixture.font_size)?;
        writeln!(writer, r#"      "pages": {},"#, fixture.pages)?;
        writeln!(writer, r#"      "wer_target": {}"#, fixture.wer_target)?;
        write!(writer, "    }}")?;
    }

    writeln!(writer, "\n  ]")?;
    writeln!(writer, "}}")?;

    println!("Created fixture manifest: {}", manifest_path);

    Ok(())
}

struct FixtureSpec<'a> {
    name: &'a str,
    dir: &'a str,
    font: &'a str,
    font_size: u32,
    pages: u32,
    wer_target: f64,
}

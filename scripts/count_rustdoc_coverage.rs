#!/usr/bin/env rust-script
//! Measure rustdoc coverage for pdftract-core public API.

use std::fs;
use std::path::Path;

#[derive(Default)]
struct DocStats {
    total_items: usize,
    with_docs: usize,
    with_examples: usize,
    modules: usize,
    structs: usize,
    enums: usize,
    traits: usize,
    functions: usize,
    types: usize,
}

impl DocStats {
    fn coverage(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            (self.with_examples as f64 / self.total_items as f64) * 100.0
        }
    }
}

fn scan_file(path: &Path, stats: &mut DocStats) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();

        // Look for doc comments before public items
        let mut has_doc = false;
        let mut has_example = false;

        // Scan backward for doc comments
        if i > 0 {
            for j in (0..i).rev() {
                let prev_line = lines[j].trim();
                if prev_line.starts_with("///") || prev_line.starts_with("//!") {
                    has_doc = true;
                    if prev_line.contains("```") && (prev_line.contains("rust") || prev_line.contains("no_run")) {
                        has_example = true;
                    }
                } else if !prev_line.is_empty() && !prev_line.starts_with("//") && !prev_line.starts_with("#[") {
                    break;
                }
            }
        }

        // Count public items
        if line.starts_with("pub ") && !line.starts_with("pub(crate)") {
            if line.contains("fn ") {
                stats.functions += 1;
            } else if line.contains("struct ") {
                stats.structs += 1;
            } else if line.contains("enum ") {
                stats.enums += 1;
            } else if line.contains("trait ") {
                stats.traits += 1;
            } else if line.contains("type ") {
                stats.types += 1;
            } else if line.contains("mod ") {
                stats.modules += 1;
            }

            stats.total_items += 1;
            if has_doc {
                stats.with_docs += 1;
            }
            if has_example {
                stats.with_examples += 1;
            }
        }
    }
}

fn scan_directory(dir: &Path, stats: &mut DocStats) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_directory(&path, stats);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                scan_file(&path, stats);
            }
        }
    }
}

fn main() {
    let mut stats = DocStats::default();
    scan_directory(Path::new("crates/pdftract-core/src"), &mut stats);

    println!("\n=== Rustdoc Coverage Report ===\n");
    println!("Total public items: {}", stats.total_items);
    println!("With docs: {} ({:.1}%)", stats.with_docs,
        (stats.with_docs as f64 / stats.total_items as f64) * 100.0);
    println!("With examples: {} ({:.1}%)", stats.with_examples,
        (stats.with_examples as f64 / stats.total_items as f64) * 100.0);
    println!("\nBy type:");
    println!("  Modules: {}", stats.modules);
    println!("  Structs: {}", stats.structs);
    println!("  Enums: {}", stats.enums);
    println!("  Traits: {}", stats.traits);
    println!("  Functions: {}", stats.functions);
    println!("  Types: {}", stats.types);
    println!("\nTarget: 80%+ coverage");
    println!("Status: {}", if stats.coverage() >= 80.0 { "✓ PASS" } else { "✗ FAIL" });
    println!("Current: {:.1}%", stats.coverage());
}

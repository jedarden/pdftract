#!/usr/bin/env rust-script
//! Measure rustdoc coverage: count public items with worked examples.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let src_dir = Path::new("crates/pdftract-core/src");

    println!("=== Rustdoc Coverage Measurement ===\n");
    println!("Scanning: {}", src_dir.display());

    let mut total_items = 0;
    let mut items_with_doc = 0;
    let mut items_with_examples = 0;

    // Track files that need examples
    let mut files_needing_examples: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path = entry.path();
        let content = fs::read_to_string(path)?;

        // Count public items in this file
        let pub_items = count_public_items(&content);
        total_items += pub_items.total;

        // Check for doc comments
        let has_doc = content.contains("///") || content.contains("//!");
        if has_doc && pub_items.total > 0 {
            items_with_doc += pub_items.doc;
        }

        // Check for examples
        let has_example = content.contains("```rust") || content.contains("```no_run");
        if has_example && pub_items.total > 0 {
            items_with_examples += pub_items.examples;
        } else if pub_items.total > 0 {
            files_needing_examples.push(path.to_path_buf());
        }
    }

    println!("\n=== Summary ===");
    println!("Public items: {}", total_items);
    println!("Items with doc comments: {}", items_with_doc);
    println!("Items with worked examples: {}", items_with_examples);

    if total_items > 0 {
        let doc_coverage = items_with_doc * 100 / total_items;
        let example_coverage = items_with_examples * 100 / total_items;
        println!("\nDoc comment coverage: {}%", doc_coverage);
        println!("Example coverage: {}%", example_coverage);

        if example_coverage < 80 {
            println!("\n⚠️  Target: 80% example coverage");
            println!("Missing: {} items", (total_items * 80 / 100) - items_with_examples);

            println!("\n=== Files needing examples ===");
            for file in files_needing_examples.iter().take(20) {
                println!("  {}", file.strip_prefix(src_dir).unwrap_or(file).display());
            }
            if files_needing_examples.len() > 20 {
                println!("  ... and {} more", files_needing_examples.len() - 20);
            }
        }
    }

    Ok(())
}

#[derive(Default)]
struct ItemCount {
    total: usize,
    doc: usize,
    examples: usize,
}

fn count_public_items(content: &str) -> ItemCount {
    let mut count = ItemCount::default();

    for line in content.lines() {
        // Look for public items
        if line.contains("pub fn ") || line.contains("pub struct ") || line.contains("pub enum ")
            || line.contains("pub trait ") || line.contains("pub type ")
            || line.contains("pub const ") || line.contains("pub static ")
        {
            count.total += 1;
        }
    }

    // Approximate: if file has doc comments, count half of items as having docs
    // (not perfect, but gives a rough estimate)
    if content.contains("///") {
        count.doc = count.total.min(content.matches("///").count());
    }

    // Approximate: if file has examples, count as having examples
    if content.contains("```rust") || content.contains("```no_run") {
        count.examples = count.total;
    }

    count
}

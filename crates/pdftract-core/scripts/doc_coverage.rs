#!/usr/bin/env rust-script
//! Analyze pdftract-core public API documentation coverage.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
enum PublicItem {
    Struct { name: String, has_doc: bool },
    Enum { name: String, has_doc: bool },
    Fn { name: String, has_doc: bool },
    Trait { name: String, has_doc: bool },
    Type { name: String, has_doc: bool },
    Const { name: String, has_doc: bool },
    Mod { name: String, has_doc: bool },
    Impl { name: String, has_doc: bool },
}

impl PublicItem {
    fn name(&self) -> &str {
        match self {
            PublicItem::Struct { name, .. } => name,
            PublicItem::Enum { name, .. } => name,
            PublicItem::Fn { name, .. } => name,
            PublicItem::Trait { name, .. } => name,
            PublicItem::Type { name, .. } => name,
            PublicItem::Const { name, .. } => name,
            PublicItem::Mod { name, .. } => name,
            PublicItem::Impl { name, .. } => name,
        }
    }

    fn has_doc(&self) -> bool {
        match self {
            PublicItem::Struct { has_doc, .. } => *has_doc,
            PublicItem::Enum { has_doc, .. } => *has_doc,
            PublicItem::Fn { has_doc, .. } => *has_doc,
            PublicItem::Trait { has_doc, .. } => *has_doc,
            PublicItem::Type { has_doc, .. } => *has_doc,
            PublicItem::Const { has_doc, .. } => *has_doc,
            PublicItem::Mod { has_doc, .. } => *has_doc,
            PublicItem::Impl { has_doc, .. } => *has_doc,
        }
    }

    fn item_type(&self) -> &str {
        match self {
            PublicItem::Struct { .. } => "struct",
            PublicItem::Enum { .. } => "enum",
            PublicItem::Fn { .. } => "fn",
            PublicItem::Trait { .. } => "trait",
            PublicItem::Type { .. } => "type",
            PublicItem::Const { .. } => "const",
            PublicItem::Mod { .. } => "mod",
            PublicItem::Impl { .. } => "impl",
        }
    }
}

fn has_doc_comment_before(lines: &[&str], pos: usize) -> bool {
    // Look backwards from pos for doc comments
    let mut i = pos;
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.starts_with("///") || line.starts_with("//!") {
            return true;
        }
        // Stop at non-empty, non-comment line
        if !line.is_empty() && !line.starts_with("//") && line != "{" && line != "}" {
            break;
        }
    }
    false
}

fn parse_public_items(file_content: &str) -> Vec<PublicItem> {
    let lines: Vec<&str> = file_content.lines().collect();
    let mut items = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and non-pub items
        if !trimmed.starts_with("pub ") {
            continue;
        }

        // Check for doc comment before
        let has_doc = has_doc_comment_before(&lines, i);

        // Parse different item types
        if trimmed.starts_with("pub struct ") {
            let name = trimmed
                .strip_prefix("pub struct ")
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('{')
                .trim_end_matches('(');
            if !name.is_empty() && !name.contains("Generic") {
                items.push(PublicItem::Struct {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub enum ") {
            let name = trimmed
                .strip_prefix("pub enum ")
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('{');
            if !name.is_empty() {
                items.push(PublicItem::Enum {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub fn ") {
            let name = trimmed
                .strip_prefix("pub fn ")
                .unwrap()
                .split('(')
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                items.push(PublicItem::Fn {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub trait ") {
            let name = trimmed
                .strip_prefix("pub trait ")
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('{');
            if !name.is_empty() {
                items.push(PublicItem::Trait {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub type ") {
            let name = trimmed
                .strip_prefix("pub type ")
                .unwrap()
                .split('=')
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                items.push(PublicItem::Type {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub const ") {
            let name = trimmed
                .strip_prefix("pub const ")
                .unwrap()
                .split(':')
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                items.push(PublicItem::Const {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.starts_with("pub mod ") {
            let name = trimmed
                .strip_prefix("pub mod ")
                .unwrap()
                .split(';')
                .next()
                .unwrap_or("")
                .trim_end_matches('{')
                .trim();
            if !name.is_empty() && name != "self" {
                items.push(PublicItem::Mod {
                    name: name.to_string(),
                    has_doc,
                });
            }
        } else if trimmed.contains("pub impl ") {
            // Extract the type being implemented
            if let Some(rest) = trimmed.strip_prefix("pub ") {
                if let Some(rest) = rest.strip_prefix("impl ") {
                    let name = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_end_matches('{');
                    if !name.is_empty() && name != "Test" {
                        items.push(PublicItem::Impl {
                            name: name.to_string(),
                            has_doc,
                        });
                    }
                }
            }
        }
    }

    items
}

fn main() {
    let src_path = Path::new("src");
    let mut all_items: Vec<(String, PublicItem)> = Vec::new();

    // Process lib.rs first
    if let Ok(content) = fs::read_to_string(src_path.join("lib.rs")) {
        let items = parse_public_items(&content);
        for item in items {
            all_items.push(("lib.rs".to_string(), item));
        }
    }

    // Recursively process all .rs files in src/
    if let Ok(entries) = fs::read_dir(&src_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let items = parse_public_items(&content);
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    for item in items {
                        all_items.push((filename.clone(), item));
                    }
                }
            }
        }
    }

    // Process subdirectories
    if let Ok(entries) = fs::read_dir(&src_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|s| s.to_str()) == Some("rs") {
                            if let Ok(content) = fs::read_to_string(&sub_path) {
                                let items = parse_public_items(&content);
                                let filename = format!(
                                    "{}/{}",
                                    path.file_name().unwrap().to_string_lossy(),
                                    sub_path.file_name().unwrap().to_string_lossy()
                                );
                                for item in items {
                                    all_items.push((filename.clone(), item));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Count by type and documentation status
    let mut by_type: HashMap<&str, (usize, usize)> = HashMap::new(); // (total, with_doc)

    for (_file, item) in &all_items {
        let entry = by_type.entry(item.item_type()).or_insert((0, 0));
        entry.0 += 1;
        if item.has_doc() {
            entry.1 += 1;
        }
    }

    // Print summary
    println!("=== pdftract-core Public API Documentation Coverage ===\n");

    let total: usize = all_items.len();
    let with_doc: usize = all_items.iter().filter(|(_, i)| i.has_doc()).count();
    let coverage = if total > 0 {
        (with_doc as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("Total public items: {}", total);
    println!("With documentation: {}", with_doc);
    println!("Coverage: {:.1}%\n", coverage);

    println!("=== By Type ===");
    for (item_type, (total_items, with_doc_items)) in by_type.iter().sorted_by_key(|&(k, _)| std::cmp::Reverse(k)) {
        let type_coverage = if *total_items > 0 {
            (*with_doc_items as f64 / *total_items as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:>8}: {} / {} ({:.1}%)",
            item_type,
            with_doc_items,
            total_items,
            type_coverage
        );
    }

    // List items without documentation
    println!("\n=== Items Without Documentation ===");
    let mut missing: Vec<_> = all_items
        .iter()
        .filter(|(_, i)| !i.has_doc())
        .collect();
    missing.sort_by(|a, b| {
            a.1.item_type().cmp(&b.1.item_type())
        });

    for (file, item) in missing.iter().take(50) {
        println!("{} ({} - {})", item.name(), item.item_type(), file);
    }

    if missing.len() > 50 {
        println!("... and {} more", missing.len() - 50);
    }

    println!("\n=== Coverage Status ===");
    if coverage >= 80.0 {
        println!("✓ PASS: {:.1}% coverage meets 80% threshold", coverage);
    } else {
        println!("✗ FAIL: {:.1}% coverage below 80% threshold (need {} more items)", coverage, ((total as f64 * 0.8) - with_doc as f64).ceil() as usize);
    }
}

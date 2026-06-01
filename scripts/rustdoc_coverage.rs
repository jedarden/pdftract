#!/usr/bin/env rust-script
//! Scan pdftract-core source for public API items with/without worked examples.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Attribute, Item, ItemEnum, ItemFn, ItemStruct, ItemTrait, ItemMod, ItemType, Visibility};

#[derive(Debug, Default)]
struct ModuleStats {
    total_items: usize,
    with_examples: usize,
    missing_docs: usize,
    items: Vec<ItemInfo>,
}

#[derive(Debug)]
struct ItemInfo {
    name: String,
    kind: &'static str,
    has_example: bool,
    file: String,
    line: usize,
}

fn extract_examples_from_doc(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if meta.path.is_ident("doc") {
                if let Ok(syn::Expr::Lit(expr_lit)) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let doc = lit_str.value();
                        // Check for ```rust code blocks (worked examples)
                        if doc.contains("```rust") || doc.contains("```no_run") || doc.contains("```ignore") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn count_public_items_in_file(content: &str, file: &Path) -> Vec<ItemInfo> {
    let mut items = Vec::new();

    let file = file.to_path_buf();
    let syntax = match syn::parse_file(content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", file.display(), e);
            return items;
        }
    };

    for item in syntax.items {
        match item {
            Item::Fn(ItemFn { attrs, vis, sig, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = sig.ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "fn",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            Item::Struct(ItemStruct { attrs, vis, ident, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "struct",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            Item::Enum(ItemEnum { attrs, vis, ident, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "enum",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            Item::Trait(ItemTrait { attrs, vis, ident, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "trait",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            Item::Type(ItemType { attrs, vis, ident, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "type",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            Item::Mod(ItemMod { attrs, vis, ident, .. }) => {
                if matches!(vis, Visibility::Public(_)) {
                    let name = ident.to_string();
                    let has_example = extract_examples_from_doc(&attrs);
                    items.push(ItemInfo {
                        name,
                        kind: "mod",
                        has_example,
                        file: file.display().to_string(),
                        line: attrs.first().map(|a| a.span().start().line).unwrap_or(0),
                    });
                }
            }
            _ => {}
        }
    }

    items
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core_src = Path::new("crates/pdftract-core/src");
    let mut module_stats: HashMap<String, ModuleStats> = HashMap::new();

    for entry in walkdir::WalkDir::new(core_src) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let content = fs::read_to_string(path)?;
        let module_name = path
            .strip_prefix(core_src)
            .ok()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("lib")
            .to_string();

        let items = count_public_items_in_file(&content, path);

        for item in items {
            let stats = module_stats
                .entry(module_name.clone())
                .or_insert_with(ModuleStats::default);
            stats.total_items += 1;
            if item.has_example {
                stats.with_examples += 1;
            }
            stats.items.push(item);
        }
    }

    let mut total_items = 0;
    let mut total_with_examples = 0;

    println!("\n=== Rustdoc Coverage Report for pdftract-core ===\n");

    for (module, stats) in module_stats.iter() {
        let coverage = if stats.total_items > 0 {
            (stats.with_examples as f64 / stats.total_items as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{}: {}/{} items with examples ({:.1}%)",
            module, stats.with_examples, stats.total_items, coverage
        );
        total_items += stats.total_items;
        total_with_examples += stats.with_examples;
    }

    let overall_coverage = if total_items > 0 {
        (total_with_examples as f64 / total_items as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "\nOverall: {}/{} items with examples ({:.1}%)",
        total_with_examples, total_items, overall_coverage
    );

    if overall_coverage < 80.0 {
        println!("\n⚠️  Coverage is below 80% target");
    } else {
        println!("\n✅ Coverage meets 80%+ target");
    }

    // List items without examples (limited output)
    println!("\n=== Items without examples (first 20 per module) ===\n");
    for (module, stats) in module_stats.iter() {
        let without_examples: Vec<_> = stats
            .items
            .iter()
            .filter(|i| !i.has_example)
            .take(20)
            .collect();
        if !without_examples.is_empty() {
            println!("{}:", module);
            for item in without_examples {
                println!("  - {} ({}) at {}:{}", item.name, item.kind, item.file, item.line);
            }
            println!();
        }
    }

    Ok(())
}

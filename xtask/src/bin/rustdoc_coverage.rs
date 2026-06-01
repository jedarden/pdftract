//! Calculate rustdoc coverage for pdftract-core.
//!
//! Counts public items and those with worked examples (```rust blocks).

use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
struct CoverageStats {
    total_modules: usize,
    documented_modules: usize,
    total_functions: usize,
    documented_functions: usize,
    total_structs: usize,
    documented_structs: usize,
    total_enums: usize,
    documented_enums: usize,
    total_traits: usize,
    documented_traits: usize,
    total_type_aliases: usize,
    documented_type_aliases: usize,
    total_consts: usize,
    documented_consts: usize,
}

impl CoverageStats {
    fn total_items(&self) -> usize {
        self.total_modules
            + self.total_functions
            + self.total_structs
            + self.total_enums
            + self.total_traits
            + self.total_type_aliases
            + self.total_consts
    }

    fn documented_items(&self) -> usize {
        self.documented_modules
            + self.documented_functions
            + self.documented_structs
            + self.documented_enums
            + self.documented_traits
            + self.documented_type_aliases
            + self.documented_consts
    }

    fn coverage_pct(&self) -> f64 {
        if self.total_items() == 0 {
            0.0
        } else {
            (self.documented_items() as f64 / self.total_items() as f64) * 100.0
        }
    }
}

fn has_worked_example(doc: &str) -> bool {
    doc.contains("```rust")
}

fn analyze_file(path: &Path, stats: &mut CoverageStats) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip private items and doc comments
        if line.trim().starts_with("///") || line.trim().starts_with("//!") {
            i += 1;
            continue;
        }

        // Collect preceding doc comments
        let mut doc = String::new();
        let mut j = i;
        while j > 0 && (lines[j - 1].trim().starts_with("///") || lines[j - 1].trim().starts_with("//!")) {
            doc.push_str(lines[j - 1].trim());
            doc.push('\n');
            j -= 1;
        }

        let has_example = has_worked_example(&doc);

        // Public modules (excluding re-exports)
        if line.contains("pub mod") && !line.contains("pub use") {
            stats.total_modules += 1;
            if has_example {
                stats.documented_modules += 1;
            }
        }
        // Public functions
        else if line.contains("pub fn") || line.contains("pub async fn") {
            stats.total_functions += 1;
            if has_example {
                stats.documented_functions += 1;
            }
        }
        // Public structs
        else if line.contains("pub struct") {
            stats.total_structs += 1;
            if has_example {
                stats.documented_structs += 1;
            }
        }
        // Public enums
        else if line.contains("pub enum") {
            stats.total_enums += 1;
            if has_example {
                stats.documented_enums += 1;
            }
        }
        // Public traits
        else if line.contains("pub trait") {
            stats.total_traits += 1;
            if has_example {
                stats.documented_traits += 1;
            }
        }
        // Public type aliases
        else if line.contains("pub type") {
            stats.total_type_aliases += 1;
            if has_example {
                stats.documented_type_aliases += 1;
            }
        }
        // Public constants
        else if line.contains("pub const") || line.contains("pub static") {
            stats.total_consts += 1;
            if has_example {
                stats.documented_consts += 1;
            }
        }

        i += 1;
    }
}

fn main() {
    let src_dir = Path::new("crates/pdftract-core/src");
    let mut stats = CoverageStats::default();

    // Analyze all .rs files
    for entry in walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "rs") {
            analyze_file(path, &mut stats);
        }
    }

    println!("=== Rustdoc Coverage Report for pdftract-core ===\n");
    println!("{:<25} {:>10} {:>10} {:>10}", "Category", "Total", "Documented", "Coverage");
    println!("{}", "-" * 59);
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Modules",
        stats.total_modules,
        stats.documented_modules,
        if stats.total_modules > 0 {
            (stats.documented_modules as f64 / stats.total_modules as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Functions",
        stats.total_functions,
        stats.documented_functions,
        if stats.total_functions > 0 {
            (stats.documented_functions as f64 / stats.total_functions as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Structs",
        stats.total_structs,
        stats.documented_structs,
        if stats.total_structs > 0 {
            (stats.documented_structs as f64 / stats.total_structs as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Enums",
        stats.total_enums,
        stats.documented_enums,
        if stats.total_enums > 0 {
            (stats.documented_enums as f64 / stats.total_enums as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Traits",
        stats.total_traits,
        stats.documented_traits,
        if stats.total_traits > 0 {
            (stats.documented_traits as f64 / stats.total_traits as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Type Aliases",
        stats.total_type_aliases,
        stats.documented_type_aliases,
        if stats.total_type_aliases > 0 {
            (stats.documented_type_aliases as f64 / stats.total_type_aliases as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "Constants",
        stats.total_consts,
        stats.documented_consts,
        if stats.total_consts > 0 {
            (stats.documented_consts as f64 / stats.total_consts as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("{}", "-" * 59);
    println!(
        "{:<25} {:>10} {:>10} {:>9.1}%",
        "TOTAL",
        stats.total_items(),
        stats.documented_items(),
        stats.coverage_pct()
    );

    println!("\nTarget: 80.0%");
    if stats.coverage_pct() >= 80.0 {
        println!("Status: PASS ✓");
    } else {
        println!("Status: FAIL - Need {:.1}% more", 80.0 - stats.coverage_pct());
    }
}

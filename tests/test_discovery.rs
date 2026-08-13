// Simple test to verify fixture discovery works
use std::path::{Path, PathBuf};
use std::collections::HashMap;

fn discover_pdf_fixtures_simple<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let mut pdf_files = Vec::new();
    
    // Simple recursive walk
    fn walk_dir(dir: &Path, pdf_files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, pdf_files);
                } else if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
                    pdf_files.push(path);
                }
            }
        }
    }
    
    walk_dir(fixtures_path, &mut pdf_files);
    pdf_files.sort();
    pdf_files
}

fn main() {
    let fixtures_dir = Path::new("tests/fixtures");
    let pdf_files = discover_pdf_fixtures_simple(fixtures_dir);
    
    println!("=== PDF Fixture Discovery Test ===");
    println!("Fixtures directory: {}", fixtures_dir.display());
    println!("Total PDF files discovered: {}", pdf_files.len());
    
    if !pdf_files.is_empty() {
        println!("\nFirst 10 PDF files:");
        for (i, path) in pdf_files.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, path.display());
        }
        
        if pdf_files.len() > 10 {
            println!("  ... and {} more", pdf_files.len() - 10);
        }
        
        // Show breakdown by directory
        println!("\n=== Files by subdirectory ===");
        let mut by_dir: HashMap<String, usize> = HashMap::new();
        for path in &pdf_files {
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name() {
                    let dir_str = dir_name.to_string_lossy().to_string();
                    *by_dir.entry(dir_str).or_insert(0) += 1;
                }
            }
        }
        
        let mut dirs: Vec<_> = by_dir.iter().collect();
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (dir, count) in dirs {
            println!("  {}: {} files", dir, count);
        }
    }
    
    println!("=====================================\n");
    println!("SUCCESS: Discovered {} PDF fixtures", pdf_files.len());
}

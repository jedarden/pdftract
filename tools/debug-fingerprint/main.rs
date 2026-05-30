// Debug tool for fingerprint computation
use std::path::Path;
use std::time::Instant;
use pdftract_core::document::compute_pdf_fingerprint;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: debug-fingerprint <pdf-path>");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    if !path.exists() {
        eprintln!("File not found: {}", args[1]);
        std::process::exit(1);
    }

    println!("Computing fingerprint for: {}", args[1]);
    let start = Instant::now();

    match compute_pdf_fingerprint(path) {
        Ok(fp) => {
            let elapsed = start.elapsed();
            println!("Fingerprint: {}", fp);
            println!("Time: {:?}", elapsed);
        }
        Err(e) => {
            let elapsed = start.elapsed();
            eprintln!("Error after {:?}: {}", elapsed, e);
            std::process::exit(1);
        }
    }
}

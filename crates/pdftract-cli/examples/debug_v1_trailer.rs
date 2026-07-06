use pdftract_core::parser::stream::{FileSource, PdfSource};
use std::path::Path;

fn main() {
    let path = Path::new("tests/fingerprint/fixtures/byte_identical/v1.pdf");
    let source = FileSource::open(path).unwrap();

    let len = source.len().unwrap();
    println!("File length: {}", len);

    // Read last 500 bytes
    let scan_size = 500.min(len) as usize;
    let scan_start = len - scan_size as u64;
    let tail_data = source.read_at(scan_start, scan_size).unwrap();

    println!("Tail data (last {} bytes):", tail_data.len());
    println!("{}", String::from_utf8_lossy(&tail_data));
}

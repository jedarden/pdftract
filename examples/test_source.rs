// Test to verify source module is complete
use pdftract_core::source::{FileSource, MemorySource, MmapSource, PdfSource};
use std::io::Write;
use tempfile::NamedTempFile;

fn main() {
    // Test MemorySource
    let data = b"Hello, World!".to_vec();
    let mem_source = MemorySource::new(data);
    assert_eq!(mem_source.len(), 13);
    let bytes = mem_source.read_range(0, 5).unwrap();
    assert_eq!(&bytes[..], b"Hello");
    println!("MemorySource: OK");

    // Test MmapSource
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"Hello from mmap!").unwrap();
    let mmap_source = MmapSource::open(temp_file.path()).unwrap();
    assert_eq!(mmap_source.len(), 16);
    let bytes = mmap_source.read_range(0, 5).unwrap();
    assert_eq!(&bytes[..], b"Hello");
    println!("MmapSource: OK");

    // Test FileSource
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"Hello from file!").unwrap();
    let file_source = FileSource::open(temp_file.path()).unwrap();
    assert_eq!(file_source.len(), 16);
    let bytes = file_source.read_range(0, 5).unwrap();
    assert_eq!(&bytes[..], b"Hello");
    println!("FileSource: OK");

    // Test prefetch is no-op for local sources
    mem_source.prefetch(0, 100);
    mmap_source.prefetch(0, 100);
    file_source.prefetch(0, 100);
    println!("prefetch: OK");

    println!("\nAll source implementations working!");
}

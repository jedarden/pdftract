/// Test atomic file writer functionality
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pdftract_core::atomic_file_writer::AtomicFileWriter;

    let temp_dir = tempfile::TempDir::new()?;
    let test_file = temp_dir.path().join("test-output.txt");

    // Test 1: Successful commit
    {
        let mut writer = AtomicFileWriter::create(&test_file)?;
        writeln!(writer, "Hello, world!")?;
        writer.commit()?;
    }

    assert!(test_file.exists());
    let content = std::fs::read_to_string(&test_file)?;
    assert_eq!(content, "Hello, world!\n");
    println!("✓ Test 1 passed: Successful commit");

    // Test 2: Drop without commit removes temp file
    let test_file2 = temp_dir.path().join("test-output-2.txt");
    {
        let mut writer = AtomicFileWriter::create(&test_file2)?;
        writeln!(writer, "Partial data")?;
        // Drop without commit
    }

    assert!(!test_file2.exists());
    println!("✓ Test 2 passed: Drop without commit removes temp file");

    // Test 3: Overwrite existing file
    std::fs::write(&test_file, "old content")?;
    {
        let mut writer = AtomicFileWriter::create(&test_file)?;
        writeln!(writer, "new content")?;
        writer.commit()?;
    }

    let content = std::fs::read_to_string(&test_file)?;
    assert_eq!(content, "new content\n");
    println!("✓ Test 3 passed: Overwrite existing file");

    // Test 4: Stdout passthrough
    let stdout_writer = AtomicFileWriter::stdout();
    assert_eq!(stdout_writer.target_path(), PathBuf::from("-"));
    assert!(!stdout_writer.is_committed());
    stdout_writer.commit()?;
    println!("✓ Test 4 passed: Stdout passthrough");

    println!("\nAll atomic file writer tests passed!");
    Ok(())
}

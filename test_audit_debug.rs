use pdftract_core::audit::{AuditLogWriter, AuditRecord};
use tempfile::tempdir;

fn main() {
    let temp_dir = tempdir().unwrap();
    let temp_file = temp_dir.path().join("audit.ndjson");

    let writer = AuditLogWriter::open(&temp_file).unwrap();
    let record = AuditRecord::new("extract", Some("pdftract-v1:abcd".to_string()), 1234, 200);
    writer.write_record(&record).unwrap();

    let contents = std::fs::read_to_string(&temp_file).unwrap();
    println!("Output: {:?}", contents);
}

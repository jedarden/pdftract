use std::fs::File;
use std::io::Write;

fn main() {
    let mut output = File::create("tests/object_parser/fixtures/deep_nesting.pdf.in").unwrap();

    // Create 300 levels of nested dicts
    for _ in 0..300 {
        write!(output, "<< /A ").unwrap();
    }
    write!(output, "1").unwrap();
    for _ in 0..300 {
        write!(output, " >>").unwrap();
    }
    writeln!(output).unwrap();

    println!("Generated deep_nesting.pdf.in with 300 levels of nesting");
}

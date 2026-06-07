//! ObjStm fixture builder.
//!
//! Generates compressed Object Stream fixtures for testing.
//!
//! Usage:
//!   build-objstm-fixture <output.bin> <N> <obj1_index> <obj1_bytes...> <obj2_index> <obj2_bytes...> ...
//!
//! where:
//!   - output.bin: path to write the compressed ObjStm data
//!   - N: number of objects in the stream (must match the count of index+bytes pairs)
//!   - objN_index: the object number for each object
//!   - objN_bytes: the raw bytes for each object (as hex string)
//!
//! Example:
//!   build-objstm-fixture objstm_basic.bin 5 1 3130 2 3131 3 3132 4 3133 5 3134
//!
//! This creates a minimal ObjStm with 5 objects (indices 1-5, each containing "00", "01", etc.)

use std::env;
use std::fs::File;
use std::io::{self, Write, BufWriter};
use std::process;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Build an Object Stream.
///
/// An Object Stream has the structure:
///   N [obj0_index obj1_offset obj1_index obj2_offset ...] stream_data endstream endobj
///
/// where:
///   - N is the number of objects
///   - The array contains alternating index and offset pairs
///   - stream_data contains the concatenated object bytes
fn build_objstm(n: u16, objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
    // Build the offset table
    let mut offsets = Vec::new();
    let mut current_offset = 0u32;

    for (index, bytes) in objects {
        offsets.push(*index);
        offsets.push(current_offset);
        current_offset += bytes.len() as u32;
    }

    // Build the header: N [ offsets ]
    let mut header = format!("{} [", n).into_bytes();
    for (i, val) in offsets.iter().enumerate() {
        if i > 0 {
            header.push(b' ');
        }
        header.extend_from_slice(val.to_string().as_bytes());
    }
    header.push(b']');
    header.push(b'\n');

    // Build the stream data
    let mut stream_data = Vec::new();
    for (_, bytes) in objects {
        stream_data.extend_from_slice(bytes);
    }

    // Build the trailer: endstream endobj
    let trailer = b"endstream\nendobj\n".to_vec();

    // Concatenate: header + stream_data + trailer
    let mut objstm = header;
    objstm.extend_from_slice(&stream_data);
    objstm.extend_from_slice(&trailer);

    objstm
}

/// Compress data using deflate (FlateDecode).
fn compress(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <output.bin> <N> <obj1_index> <obj1_hex> ...", args[0]);
        eprintln!("Example: {} objstm_basic.bin 5 1 3130 2 3131 3 3132 4 3133 5 3134", args[0]);
        process::exit(1);
    }

    let output_path = &args[1];
    let n: u16 = args[2].parse().expect("N must be a number");

    // Parse object pairs: (index, hex_bytes)
    let mut objects = Vec::new();
    let mut i = 3;
    while i < args.len() {
        let index: u32 = args[i].parse().expect("object index must be a number");
        i += 1;

        if i >= args.len() {
            eprintln!("Missing hex bytes for object {}", index);
            process::exit(1);
        }

        let hex = &args[i];
        let bytes = hex::decode(hex).expect("invalid hex string");
        i += 1;

        objects.push((index, bytes));
    }

    if objects.len() != n as usize {
        eprintln!("Expected {} objects, got {}", n, objects.len());
        process::exit(1);
    }

    // Build the ObjStm
    let objstm = build_objstm(n, &objects);

    // Compress it
    let compressed = match compress(&objstm) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Compression failed: {}", e);
            process::exit(1);
        }
    };

    // Write to output
    let file = File::create(output_path).expect("failed to create output file");
    let mut writer = BufWriter::new(file);
    writer.write_all(&compressed).expect("failed to write output");
    writer.flush().expect("failed to flush output");

    println!("Created ObjStm fixture: {}", output_path);
    println!("  N: {}", n);
    println!("  Objects: {} (compressed: {} bytes)", objstm.len(), compressed.len());
}

// Minimal hex decode
mod hex {
    pub fn decode(hex: &str) -> Result<Vec<u8>, String> {
        let hex = hex.trim();
        if hex.len() % 2 != 0 {
            return Err("hex string must have even length".to_string());
        }

        (0..hex.len() / 2)
            .map(|i| {
                let byte_str = &hex[i * 2..i * 2 + 2];
                u8::from_str_radix(byte_str, 16)
                    .map_err(|e| format!("invalid hex byte: {}", e))
            })
            .collect()
    }
}

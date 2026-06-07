//! Generate font fingerprint entry from a TTF/OTF file.
//!
//! Usage: cargo run --example gen_font_fingerprint -- /path/to/font.ttf
//!
//! Outputs JSON in the format required by build/font-fingerprints.json.

use std::env;
use std::fs;
use std::io::Read;

use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <font.ttf>", args[0]);
        std::process::exit(1);
    }

    let font_path = &args[1];

    // Read font file
    let mut font_data = Vec::new();
    fs::File::open(font_path)?.read_to_end(&mut font_data)?;

    // Compute SHA-256
    let mut hasher = Sha256::new();
    hasher.update(&font_data);
    let sha256_hex = format!("{:x}", hasher.finalize());

    // Parse font using ttf_parser (index 0 for the first face in the font)
    let face = ttf_parser::Face::parse(&font_data, 0)
        .map_err(|e| format!("Failed to parse font: {:?}", e))?;

    // Build GID->codepoint mappings
    let mut gid_to_cp: Vec<(u16, u32)> = Vec::new();

    // Scan Unicode ranges that the font likely supports
    // We test each codepoint and record the mapping
    for cp in 0x20..0x7F {  // Printable ASCII
        let c = char::from_u32(cp).unwrap();
        if let Some(gid) = face.glyph_index(c) {
            gid_to_cp.push((gid.0, cp));
        }
    }

    // Add Latin-1 Supplement (0xA0-0xFF)
    for cp in 0xA0..0x100 {
        let c = char::from_u32(cp).unwrap();
        if let Some(gid) = face.glyph_index(c) {
            gid_to_cp.push((gid.0, cp));
        }
    }

    // Common punctuation and symbols (0x2000-0x206F, 0x20A0-0x20CF)
    for cp in 0x2000..0x20D0 {
        let c = char::from_u32(cp).unwrap();
        if let Some(gid) = face.glyph_index(c) {
            gid_to_cp.push((gid.0, cp));
        }
    }

    // Sort by GID for output
    gid_to_cp.sort_by_key(|(gid, _)| *gid);
    // Remove duplicates (same GID may map to multiple codepoints)
    gid_to_cp.dedup_by_key(|(gid, _)| *gid);

    // Get font name from path
    let font_name = font_path
        .rsplit('/')
        .next()
        .or_else(|| font_path.rsplit('\\').next())
        .unwrap_or("Unknown");

    // Output JSON entry
    let json = serde_json::json!([{
        "sha256_hex": sha256_hex,
        "font_name": font_name,
        "entries": gid_to_cp
    }]);

    println!("{}", serde_json::to_string_pretty(&json)?);

    Ok(())
}

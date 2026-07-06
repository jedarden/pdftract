use pdftract_core::parser::stream::{
    ASCII85Decoder, FlateDecoder, StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES,
};

#[test]
fn test_filter_array_debug() {
    let encoded = [
        0x3c, 0x7e, 0x6f, 0x31, 0x37, 0x2d, 0x4a, 0x61, 0x6b, 0x27, 0x41, 0x71, 0x63, 0x53, 0x2a,
        0x46, 0x34, 0x3b, 0x24, 0x36, 0x6b, 0x7e, 0x3e,
    ];

    println!("Input: {:02x?}", encoded);

    // Step 1: Decode ASCII85
    let mut counter = 0u64;
    let result1 = ASCII85Decoder.decode(&encoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    match &result1 {
        Ok(bytes) => println!("After ASCII85 ({:?} bytes): {:02x?}", bytes.len(), bytes),
        Err(e) => println!("ASCII85 error: {:?}", e),
    }

    // Step 2: Decode Flate
    if let Ok(a85_decoded) = result1 {
        let mut counter2 = 0u64;
        let result2 = FlateDecoder.decode(
            &a85_decoded,
            None,
            &mut counter2,
            DEFAULT_MAX_DECOMPRESS_BYTES,
        );
        match &result2 {
            Ok(bytes) => println!(
                "After Flate ({:?} bytes): {:?}",
                bytes.len(),
                String::from_utf8_lossy(bytes)
            ),
            Err(e) => println!("Flate error: {:?}", e),
        }
    }
}

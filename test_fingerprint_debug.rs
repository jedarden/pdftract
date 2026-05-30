use pdftract_core::fingerprint::canonicalize::normalize_content_bytes;

fn main() {
    let v1 = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello World) Tj\n    ET\n    ";
    let v2 = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello Worl) Tj\n    ET\n    ";
    
    let norm1 = normalize_content_bytes(v1);
    let norm2 = normalize_content_bytes(v2);
    
    println!("v1 normalized ({} bytes): {:?}", norm1.len(), String::from_utf8_lossy(&norm1));
    println!("v2 normalized ({} bytes): {:?}", norm2.len(), String::from_utf8_lossy(&norm2));
    println!("Equal: {}", norm1 == norm2);
}

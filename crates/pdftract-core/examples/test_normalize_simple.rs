use pdftract_core::fingerprint::canonicalize::normalize_content_stream;

fn main() {
    let v1 = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello World) Tj\n    ET\n    ";
    let v2 = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello Worl) Tj\n    ET\n    ";

    let v1_norm = normalize_content_stream(v1);
    let v2_norm = normalize_content_stream(v2);

    println!("v1 normalized: {}", String::from_utf8_lossy(&v1_norm));
    println!("v2 normalized: {}", String::from_utf8_lossy(&v2_norm));
    println!("Equal? {}", v1_norm == v2_norm);
}

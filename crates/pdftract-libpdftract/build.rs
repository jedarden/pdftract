fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Try to generate bindings with cbindgen, but don't fail if it can't parse
    let config = match cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml")) {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("Warning: cbindgen config not found, skipping header generation");
            return;
        }
    };

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(format!("{crate_dir}/include/pdftract.h"));
        }
        Err(e) => {
            eprintln!("Warning: cbindgen failed to generate bindings: {}", e);
            eprintln!("Using manually maintained header instead");
        }
    }
}

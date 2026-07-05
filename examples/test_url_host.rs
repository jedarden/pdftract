use url::Url;

fn main() {
    let urls = vec![
        "https://[::1]/",
        "https://[fe80::1]/",
        "https://example.com/",
        "https://127.0.0.1/",
    ];

    for url_str in urls {
        match Url::parse(url_str) {
            Ok(url) => {
                println!("URL: {}", url_str);
                println!("  host_str(): {:?}", url.host_str());
                println!("  host(): {:?}", url.host());
                println!();
            }
            Err(e) => {
                println!("Failed to parse {}: {}", url_str, e);
            }
        }
    }
}

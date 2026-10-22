use std::path::{Path, PathBuf};

fn main() {
    let url_decoded = urlencoding::decode("%2e%2e/%2e%2e/etc/passwd").unwrap().into_owned();
    println!("url decoded: {}", url_decoded);
}

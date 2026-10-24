use std::path::{Path, PathBuf};

fn main() {
    let raw_url = "/%2e%2e/%2e%2e/etc/passwd";
    let url_path = raw_url.split('?').next().unwrap_or("/");
    let trimmed_path = url_path.trim_start_matches('/');
    let target_path = Path::new("/var/www").join(trimmed_path);

    println!("Target path: {:?}", target_path);

    let is_parent = target_path.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Is parent: {}", is_parent);

    // I can simulate url decoding myself.
    let decoded = trimmed_path.replace("%2e", ".").replace("%2E", ".");
    let decoded_path = Path::new("/var/www").join(&decoded);
    println!("Decoded path: {:?}", decoded_path);
    let decoded_is_parent = decoded_path.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Decoded is parent: {}", decoded_is_parent);
}

use std::path::{Path, PathBuf};

fn main() {
    let root = Path::new("/var/www");
    let url = "/%2E%2E/%2E%2E/etc/passwd";

    // tiny_http does not decode url? Let's check.
    // Usually paths come in un-decoded, or maybe decoded. If un-decoded:
    let decoded = url.replace("%2E", ".");
    let trimmed = decoded.trim_start_matches('/');

    let target = root.join(trimmed);
    println!("Target: {:?}", target);

    let has_parent = target.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Has parent: {}", has_parent);

    // Check if canonicalize fails
    let canonical = target.canonicalize();
    println!("Canonicalize: {:?}", canonical);
}

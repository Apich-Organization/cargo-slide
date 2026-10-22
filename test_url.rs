use std::path::Path;

fn main() {
    let root = Path::new("/var/www");
    let relative = "/etc/passwd"; // absolute path due to leading slash! Wait, trim_start_matches('/') prevents this?
    let trimmed = relative.trim_start_matches('/');

    // trimmed is "etc/passwd"
    let target = root.join(trimmed);

    println!("Target: {:?}", target);
}

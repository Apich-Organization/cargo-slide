use std::path::Path;

fn main() {
    let p = Path::new("C:\\www");
    let rel = "D:/secrets.txt";
    let joined = p.join(rel);
    println!("Joined: {:?}", joined);

    let has_parent = joined.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Has parent: {}", has_parent);
}

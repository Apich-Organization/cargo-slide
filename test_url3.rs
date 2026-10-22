use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from("/var/www").canonicalize().unwrap_or_else(|_| PathBuf::from("/var/www"));

    // what if I pass `C:\Windows\System32\cmd.exe` as path on Windows? (in target_path logic)

    let path_str = r#"..\..\..\..\..\..\..\..\..\Windows\System32\cmd.exe"#;
    let target_path = root.join(path_str);

    let is_parent = target_path.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Target path: {:?}", target_path);
    println!("Is parent: {}", is_parent);

    // Using clean / canonicalize
}

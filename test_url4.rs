use std::path::{Path, PathBuf};

fn main() {
    let root = Path::new("/var/www");
    let relative_path = "some_dir/%2E%2E/secrets";
    let url_decoded = urlencoding::decode(relative_path).unwrap().into_owned();
    let target = root.join(&url_decoded);

    println!("Target decoded: {:?}", target);
    let is_parent = target.components().any(|c| matches!(c, std::path::Component::ParentDir));
    println!("Is parent: {}", is_parent);

    // what about root directory breakout on windows? (e.g. C:\)
    // if I pass `C:\Windows`, root.join will just become C:\Windows!
    let win_path = "C:/Windows/System32";
    let target_win = Path::new("/var/www").join(win_path);
    println!("Target win: {:?}", target_win);
}

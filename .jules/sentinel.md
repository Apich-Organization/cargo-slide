## 2024-05-24 - [Unzeroized Memory Leak in OS GUI Dialogs]
**Vulnerability:** System GUI commands like `osascript`, `zenity`, and `kdialog` captured PIN/passphrase secrets into a `std::process::Output` buffer whose raw `stdout` bytes were never zeroized, leading to an info leak.
**Learning:** Even if the Rust object `Zeroizing<String>` is used correctly for storing the final password, intermediate buffers (like OS process outputs) must also be manually zeroized, e.g., `out.stdout.zeroize()`, because `Vec<u8>` buffers from process standard output hold the unmasked data.
**Prevention:** Always verify the lifecycle of the actual bytes returned by any `std::process::Command::output()` when dealing with secrets, explicitly triggering zeroization on intermediate copies.

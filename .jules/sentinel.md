## 2026-10-31 - [Sentinel] Fix OS Command Injection in OS-native dialog shells

**Vulnerability:** OS command injection in GUI secret prompts in `crates/slide-core/src/security/piv.rs`. The code was interpolating user inputs (`title` and `prompt`) directly into AppleScript and PowerShell scripts using `format!` and executing them via `Command::new("osascript")` and `Command::new("powershell")`.

**Learning:** When invoking external scripting environments (like `osascript`, `powershell`, or even `/bin/sh`) from Rust using `std::process::Command::new`, passing unescaped or poorly-escaped strings into the script body creates a critical command injection vulnerability. A malicious string can break out of string context (e.g., using quotes or shell metacharacters) and execute arbitrary code.

**Prevention:** Never interpolate dynamic variables into scripts invoked by `Command::new`. Instead, pass dynamic data via environment variables using `.env("VAR_NAME", &value)` and retrieve them within the script natively (e.g., `system attribute "VAR_NAME"` in AppleScript, `[Environment]::GetEnvironmentVariable("VAR_NAME")` in PowerShell).

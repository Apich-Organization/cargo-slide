## 2024-05-24 - [CRITICAL] Command Injection in URL Handlers
**Vulnerability:** User-provided URLs were directly passed into unescaped system commands via `std::process::Command::new` (e.g., `cmd /C start <url>` on Windows and `xdg-open <url>` on Linux) in both `slide-editor` and `slide-player`. This allows an attacker to execute arbitrary commands by embedding shell metacharacters in the URL.
**Learning:** Manual OS-specific command execution for opening URLs is highly prone to escaping issues. Passing untrusted strings to a shell command runner like `cmd.exe` or `sh` is a critical command injection vector.
**Prevention:** Always use established libraries designed to safely handle OS-specific escaping for file/URL opening, such as the `open` crate in Rust (`open::that_detached(url)`).

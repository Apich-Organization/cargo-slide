## YYYY-MM-DD - Fix Command Injection Vulnerability in OS Open Commands
**Vulnerability:** Found uses of `std::process::Command::new` with shell invocations (e.g., `cmd /C start`) to open arbitrary, user-supplied URLs and files, leading to a severe command injection risk.
**Learning:** External URL resolution can easily cross trust boundaries when relying on OS command parsing. The `open` crate manages platform differences securely by using underlying APIs (like `ShellExecuteW` on Windows) instead of passing raw strings to `cmd`.
**Prevention:** Strictly prohibit custom `std::process::Command::new("cmd")` or similar system shells for launching files/urls. Always enforce the use of `open::that` or `open::that_detached`.

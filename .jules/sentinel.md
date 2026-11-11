## 2024-05-15 - Command Injection in OS-specific Shell Dialogs
**Vulnerability:** Command injection vulnerability in PowerShell and AppleScript dialogs when displaying PIV PIN prompts using unescaped user input interpolated into strings.
**Learning:** Using `format!` to interpolate variables directly into shell commands (like PowerShell script bodies or AppleScript) is a dangerous pattern, as the user input can break out of the string boundary and execute arbitrary commands.
**Prevention:** Pass variables to the script via environment variables instead, using `.env()` on the Rust `std::process::Command` builder, and retrieving them inside the script natively (e.g. `$env:VAR_NAME` or `system attribute "VAR_NAME"`).

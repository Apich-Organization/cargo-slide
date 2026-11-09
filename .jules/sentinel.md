## 2025-01-30 - Prevent Command Injection in OS-specific shell scripts
**Vulnerability:** Command injection in `osascript` and `powershell` by directly interpolating untrusted input into scripts in `prompt_secret_gui`.
**Learning:** Shell scripts executed via `std::process::Command` are vulnerable to injection if user input is interpolated into strings, even if naively escaped (e.g. `replace('"', "\\\"")`).
**Prevention:** Pass untrusted user strings via environment variables (`.env()`) when executing shell scripts, and read them natively (e.g. `$env:VAR_NAME` or `system attribute`).

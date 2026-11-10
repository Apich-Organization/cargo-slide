## 2026-11-10 - Avoid cloning large structs in Leptos signals
**Learning:** Calling `.get()` on a signal holding a large struct (like `Slide`) clones the entire struct on every access, which can be a huge performance bottleneck.
**Action:** Use `.with(|s| ...)` to access the inner fields by reference without cloning.

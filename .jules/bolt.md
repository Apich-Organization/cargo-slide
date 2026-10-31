## 2024-05-18 - Leptos Signal `.get()` on Large Structs
**Learning:** In Leptos, calling `.get()` on a Signal holding a large struct (like `Option<Slide>` which contains multi-megabyte `svg_data`) clones the entire struct every time it's read. When multiple memos read from this signal (e.g. `current_svg`, `current_hotspots`), this causes massive redundant memory allocations per slide transition.
**Action:** Use `.with(|s| ...)` to read large structs by reference and extract/clone only the specific fields needed (e.g. `s.svg_data.clone()`) to prevent unnecessary allocations.

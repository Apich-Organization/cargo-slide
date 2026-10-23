## 2024-05-18 - Avoid cloning large structures on Leptos signal reads
**Learning:** In Leptos 0.8, `.get()` on signals cloning the entire structure is computationally expensive and memory-intensive, especially for large reactive structures like `SlideDeck`.
**Action:** Use `.with(|data| ...)` to read signals holding large structures by reference rather than `.get()` to avoid unnecessary whole-structure cloning and boost performance.

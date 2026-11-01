
## 2024-05-19 - Avoid cloning large structures inside reactive contexts
**Learning:** In Leptos 0.8, `.get()` on signals containing large structures (like `SlideDeck` or `Slide`) clones the entire structure. Inside reactive derivations or views, this causes significant performance overhead and unnecessary allocations.
**Action:** Use `.with(|d| ...)` instead of `.get()` to read by reference when accessing fields of large reactive structures. This prevents full cloning and improves frontend performance.

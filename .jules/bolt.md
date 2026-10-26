## 2024-05-14 - Optimize ChartData access in Leptos
**Learning:** In Leptos 0.8, avoid calling `.get()` on signals holding large structures inside reactive contexts, as it clones the entire structure; use `.with(|d| ...)` to read by reference.
**Action:** Always prefer `.with()` over `.get()` when working with complex types like `ChartData` inside `Memo` or views to avoid expensive heap allocations on every read.

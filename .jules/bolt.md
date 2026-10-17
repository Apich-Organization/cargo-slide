## 2023-10-27 - [Leptos 0.8 `Signal::derive` Re-evaluates on Read]
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on every read. For computationally expensive reactive derivations, use `Memo::new(move |_| { ... })` instead to cache results. Note that the closure for `Memo::new` requires one argument (e.g., `move |_|`), unlike `Signal::derive` which takes zero arguments (`move ||`).
**Action:** Replace `Signal::derive(move || ...)` with `Memo::new(move |_| ...)` for expensive operations, especially when they clone large structs.

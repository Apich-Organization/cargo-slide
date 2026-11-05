## 2024-05-18 - Avoid cloning large structures in Leptos signals
**Learning:** In Leptos, calling `.get()` on signals containing large structures (like `Option<Slide>`) clones the entire structure on every read, which is a significant performance anti-pattern inside reactive derivations.
**Action:** Use `.with(|opt| opt.as_ref().map(...))` instead of `.get()` to access specific fields by reference, avoiding full structure clones and reducing memory pressure during re-evaluations.

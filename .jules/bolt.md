## 2025-02-18 - Leptos Signal `.get()` Cloning large Structs
**Learning:** In Leptos, reading a signal using `.get()` that holds a large structure (like `Slide` containing large SVG strings) causes the entire structure to be cloned on every access inside reactive contexts.
**Action:** Always prefer `.with(|d| d...)` for reading signals containing large structures, mapping with `as_ref().map(...)` if the outer structure is an `Option`, to avoid allocating expensive copies of memory on every reactivity tick.

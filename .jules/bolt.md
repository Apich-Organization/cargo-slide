## 2024-11-04 - [Leptos Signal Cloning Avoidance]
**Learning:** In Leptos, calling `.get()` on a signal that contains a large structure (like `Option<Slide>`) clones the entire structure. This can be a performance bottleneck if the structure is large and frequently accessed.
**Action:** Use `.with(|opt| opt.as_ref().map(|inner| ...))` to access the inner value by reference without unnecessarily cloning the `Option` or the structure it contains.

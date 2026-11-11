## 2024-05-24 - Avoiding unnecessary cloning of Option<Slide> in Leptos signals
**Learning:** When using `.with()` on a Leptos signal that contains an `Option<T>` (e.g., `Memo<Option<Slide>>`), the closure receives a reference `&Option<T>`. Calling `.get()` on signals holding large structures clones the entire structure.
**Action:** Use `.with(|opt| opt.as_ref().map(|inner| ...))` to correctly extract data without unnecessarily cloning the `Option`.


## 2024-05-14 - [Leptos Signal::derive performance bottleneck]
**Learning:** In Leptos, `Signal::derive` re-evaluates its closure on every read, leading to significant performance overhead when used for complex reactive derivations, especially those holding large structures. Additionally, using `.get()` on signals holding large structures clones the entire structure.
**Action:** Use `Memo::new(move |_| { ... })` instead of `Signal::derive` to cache results for computationally expensive derivations. Furthermore, use `.with(|d| ...)` instead of `.get()` to read values from signals holding large structures by reference to prevent unnecessary cloning. Also note that the cached type in `Memo::new` must implement `PartialEq`.

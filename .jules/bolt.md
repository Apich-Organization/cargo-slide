
## 2024-05-14 - Prevent Massive Cloning on Leptos Signals
**Learning:** Leptos `Memo::new()` and view bindings natively invoke `signal.get()`, which aggressively clones the underlying struct. For massive nested collections (like `Option<Slide>` or `ChartData`), `.get()` triggers heavy heap allocations that kill UI reactivity on hover/typing events.
**Action:** Always substitute `.get()` with `.with(|s| ...)` inside high-frequency reactive derivations (or view bindings) whenever the container type holds large elements like vectors or deeply nested structures.

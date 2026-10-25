
## 2024-10-25 - Reactive Signal cloning bottleneck in Leptos
**Learning:** Calling `.get()` on signals holding large structures (like `SlideDeck` containing full SVG strings for all slides) inside `Signal::derive` in Leptos 0.8 causes the entire structure to be cloned on every single read. `Signal::derive` re-evaluates its closure frequently.
**Action:** Always use `Memo::new` combined with `.with(|d| ...)` instead of `Signal::derive` and `.get()` when working with large data structures in reactive contexts to cache results and prevent massive memory allocations/cloning on every derivation read.

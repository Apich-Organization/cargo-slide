## 2024-05-24 - [Avoid Signal::derive for expensive Leptos 0.8 calculations]
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on every read, leading to major performance bottlenecks if the calculation is expensive (e.g., calling `.get()` which clones large structures like `SlideDeck`).
**Action:** Use `Memo::new(move |_| { ... })` instead for computationally expensive reactive derivations to cache results. Ensure the cached type implements `PartialEq`. For large signals, use `.with(|d| ...)` instead of `.get()` to avoid full cloning on every read.

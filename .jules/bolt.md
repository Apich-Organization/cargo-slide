## 2024-05-24 - [Leptos signal derived optimization]
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on every read. Computationally expensive reactive derivations can lead to severe performance bottleneck if their results are read frequently.
**Action:** Use `Memo::new(move |_| { ... })` instead to cache computationally expensive reactive results to improve performance. Note that the closure for `Memo::new` requires one argument, unlike `Signal::derive` which takes zero arguments.

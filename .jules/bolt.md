## 2024-05-30 - Signal::derive closure re-evaluation in Leptos 0.8
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on every read, whereas `Memo::new` caches the result and only re-evaluates when dependencies change. When applying this to computationally expensive reactive derivations like chart transformations, `Memo::new(move |_| { ... })` should be used instead of `Signal::derive`.
**Action:** Use `Memo::new(move |_| { ... })` for computationally expensive derivations like chart transformations to avoid re-evaluation on every read.

## 2024-10-17 - Leptos 0.8 Reactive System Performance
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on *every read*, which can cause significant performance degradation when used for computationally expensive operations like filtering data or rendering SVG charts within loops or multiple reads per render cycle.
**Action:** Always use `Memo::new(move |_| { ... })` instead of `Signal::derive` for computationally expensive reactive derivations to cache the result and prevent unnecessary recalculations.

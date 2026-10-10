## 2024-10-10 - Leptos Memoization Over Derivations
**Learning:** In Leptos, `Signal::derive` re-evaluates its closure on *every* read. When dealing with expensive UI derivations like SVG chart generation, O(N) filtering, and statistical math, this causes major frame drops during unrelated reactive state changes.
**Action:** Default to using `Memo::new(move |_| { ... })` instead of `Signal::derive(move || { ... })` for any computationally heavy derived state to cache results and only recompute when upstream signals actually change.

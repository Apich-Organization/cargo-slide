## 2024-05-18 - Leptos Reactivity: Signal::derive vs Memo
**Learning:** In Leptos 0.8, `Signal::derive` re-evaluates its closure on every read. This caused massive performance issues in `slide-web` because expensive chart data transformations (`apply_transform`), statistical calculations (`calculate_stats`), and SVG string rendering (`render_svg_chart`) were being re-computed every time they were accessed by the view, rather than only when their upstream dependencies changed.

**Action:** For computationally expensive reactive derivations in Leptos, ALWAYS use `Memo::new(move |_| { ... })` instead of `Signal::derive`. This ensures the result is cached and only recomputed when dependencies change, significantly reducing CPU overhead during view rendering.

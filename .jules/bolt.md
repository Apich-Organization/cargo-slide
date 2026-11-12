## 2024-05-24 - Optimize signal reads in SlideWeb Chart Inspector
**Learning:** In Leptos 0.8, calling `.get()` on signals holding large structures (like `SlideDeck` or `ChartData` / `transformed_data`) inside reactive contexts clones the entire structure.
**Action:** Use `.with(|d| ...)` to read by reference when only accessing properties of a signal's value to avoid unnecessary allocations and large struct cloning on re-renders, especially inside the `ChartInspectorModal` for data visualization.

## 2024-05-15 - Dynamic ARIA Labels in Lists
**Learning:** Using static strings for ARIA labels in dynamically generated lists (like `series-chip` items) creates an accessibility nightmare where screen readers announce identical labels for distinct interactive elements.
**Action:** When adding `aria-label`s to elements generated inside `view!` iterators, always use dynamic formatting (e.g., `format!("Toggle {}", item.name)`) to ensure unique and descriptive readouts for each list item. Also include `aria-pressed` or `aria-checked` to communicate state clearly.

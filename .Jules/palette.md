## 2024-10-17 - Dynamic ARIA Labels in Leptos Loops
**Learning:** When adding `aria-label`s to dynamically generated lists of UI elements (e.g., inside Leptos `view!` loops), static `aria-label`s cause major accessibility regressions by making all buttons announce identically to screen readers.
**Action:** Use dynamic formatting (e.g., `aria-label=format!("Toggle {}", item.name)`) to ensure each item is distinct. For items with reactive states, use reactive closures (e.g., `aria-label=move || if state.get() { "State A" } else { "State B" }`) to accurately reflect changes.

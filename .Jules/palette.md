## 2024-05-24 - Accessibility Labels for Leptos HUD
**Learning:** This Rust web application uses the Leptos framework for its UI. Many interactive HUD and tool elements were icon-only (using emoji or unicode characters) and relied solely on standard HTML `title` attributes for context, which is suboptimal for screen readers.
**Action:** Always add descriptive `aria-label`s to interactive `<button>` elements in Leptos templates when they only contain symbols or emojis.

## 2023-10-24 - Missing accessible names on close buttons
**Learning:** The application uses a consistent `.close-btn` pattern across various modals (video, overview, help, notes, inspector), but relies solely on the visual `"✕"` character without providing an accessible name for screen readers.
**Action:** Always ensure icon-only buttons, especially standard ones like close buttons, include an `aria-label` to provide context for assistive technologies.

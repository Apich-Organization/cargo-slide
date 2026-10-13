## 2024-10-14 - Keyboard Shortcut Tooltips for Primary Actions
**Learning:** Found that primary toolbar actions like undo and redo lacked keyboard shortcut hints. In an editor, these shortcuts are extremely common, and surfacing them via simple tooltips drastically improves discoverability.
**Action:** Created `tooltip_container_style` in `theme.rs` and added `tooltip` wrappers around toolbar action buttons to show hotkeys. Keep surfacing hotkeys for core actions where space permits.

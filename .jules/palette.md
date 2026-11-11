## 2024-05-24 - Wrapped Decorative Emojis in HUD Buttons
**Learning:** Found a specific component pattern with decorative emojis in toggle buttons causing screen reader clutter.
**Action:** Wrapped decorative emojis in the HUD toolbar with `<span aria-hidden="true">` to prevent screen readers from redundantly reading out literal emoji descriptions.

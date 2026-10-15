## 2024-05-30 - Added ARIA labels to HUD icon buttons
**Learning:** The slide web player's HUD (`crates/slide-web/src/lib.rs`) relied exclusively on Unicode emojis and symbols for presentation controls (play, pause, mute, pointer). While it had `title` attributes (which help mouse users via tooltips), icon-only buttons lack screen reader text, making the presentation interface inaccessible.
**Action:** Always ensure icon-only control buttons, even those with tooltips or standard emojis, have descriptive `aria-label` attributes for screen readers.

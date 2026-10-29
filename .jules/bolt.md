## 2024-05-18 - Optimized current_slide usage
**Learning:** Using `current_slide.get()` in a Memo or effect clones the whole `Slide` object, including potentially large fields like svg_data and steps. By using `.with` and mapping over references, we can avoid full clones when just projecting specific fields.
**Action:** Always prefer `.with` over `.get()` when dealing with potentially large signal data.

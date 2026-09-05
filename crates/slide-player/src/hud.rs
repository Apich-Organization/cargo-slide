use slide_core::animation::blend_pixel_fast;
use slide_core::chart::ChartData;
use slide_core::chart::ChartTransform;
use slide_core::chart::ChartType;
use slide_core::chart::DEFAULT_CHART_COLORS;
use slide_core::chart::SeriesData;
use slide_core::model::Rect;
use slide_core::model::RenderMetrics;


pub const PALETTE_COLORS: [(&str, u32); 7] = [
    ("Cyan", 0xFF00E5FF),   // Neon Cyan (default for pen)
    ("Red", 0xFFFF3366),    // Crimson Neon Red (default for laser)
    ("Green", 0xFF00E676),  // Bright Emerald Green
    ("Yellow", 0xFFFFD600), // Solar Yellow
    ("Purple", 0xFFD500F9), // Electric Purple
    ("White", 0xFFFFFFFF),  // Crisp White
    ("Orange", 0xFFFF9100), // Vivid Coral Orange
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenterMode {
    Normal,
    Laser,
    Pen,
}

#[derive(Debug, Clone)]
pub struct InkStroke {
    pub points: Vec<(usize, usize)>,
    pub color: u32,
    pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockAction {
    Prev,
    Next,
    ModeNormal,
    ModeLaser,
    ModePen,
    TogglePalette,
    ClearInk,
    ToggleMute,
    ToggleFullscreen,
    ToggleHelp,
}

/// Draw a smoothly animated highlight rectangle around active/hovered hotspot
pub fn draw_hotspot_highlight(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    screen_rect: Rect,
    color: u32,
    alpha: f32,
) {
    if alpha <= 0.005 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);

    let x1 = (screen_rect.x as usize).min(width.saturating_sub(1));
    let y1 = (screen_rect.y as usize).min(height.saturating_sub(1));
    let x2 = ((screen_rect.x + screen_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((screen_rect.y + screen_rect.height) as usize).min(height.saturating_sub(1));

    if x2 <= x1 || y2 <= y1 {
        return;
    }

    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    #[inline]
    fn blend_px(
        bg: u32,
        r: u32,
        g: u32,
        b: u32,
        alpha: f32,
    ) -> u32 {
        let bg_r = (bg >> 16) & 0xFF;
        let bg_g = (bg >> 8) & 0xFF;
        let bg_b = bg & 0xFF;
        let out_r = ((r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u32).min(255);
        let out_g = ((g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u32).min(255);
        let out_b = ((b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u32).min(255);
        (out_r << 16) | (out_g << 8) | out_b
    }

    // 1. Subtle interior card tint fill (opacity = a * 0.08)
    let fill_alpha = a * 0.08;
    for y in (y1 + 2)..(y2.saturating_sub(1)) {
        let row = y * width;
        for x in (x1 + 2)..(x2.saturating_sub(1)) {
            let idx = row + x;
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, fill_alpha);
        }
    }

    // 2. 1px soft outer glow fringe (opacity = a * 0.35)
    let glow_alpha = a * 0.35;
    if y1 > 0 {
        let row = (y1 - 1) * width;
        for x in x1..=x2 {
            buffer[row + x] = blend_px(buffer[row + x], cr, cg, cb, glow_alpha);
        }
    }
    if y2 + 1 < height {
        let row = (y2 + 1) * width;
        for x in x1..=x2 {
            buffer[row + x] = blend_px(buffer[row + x], cr, cg, cb, glow_alpha);
        }
    }
    if x1 > 0 {
        for y in y1..=y2 {
            let idx = y * width + (x1 - 1);
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, glow_alpha);
        }
    }
    if x2 + 1 < width {
        for y in y1..=y2 {
            let idx = y * width + (x2 + 1);
            buffer[idx] = blend_px(buffer[idx], cr, cg, cb, glow_alpha);
        }
    }

    // 3. 2px crisp border (opacity = a * 0.90)
    let border_alpha = a * 0.90;
    for x in x1..=x2 {
        let idx1 = y1 * width + x;
        let idx2 = y2 * width + x;
        buffer[idx1] = blend_px(buffer[idx1], cr, cg, cb, border_alpha);
        buffer[idx2] = blend_px(buffer[idx2], cr, cg, cb, border_alpha);
        if y1 + 1 < height {
            let idx3 = (y1 + 1) * width + x;
            buffer[idx3] = blend_px(buffer[idx3], cr, cg, cb, border_alpha);
        }
        if y2 > 0 {
            let idx4 = (y2 - 1) * width + x;
            buffer[idx4] = blend_px(buffer[idx4], cr, cg, cb, border_alpha);
        }
    }
    for y in y1..=y2 {
        let idx1 = y * width + x1;
        let idx2 = y * width + x2;
        buffer[idx1] = blend_px(buffer[idx1], cr, cg, cb, border_alpha);
        buffer[idx2] = blend_px(buffer[idx2], cr, cg, cb, border_alpha);
        if x1 + 1 < width {
            let idx3 = y * width + (x1 + 1);
            buffer[idx3] = blend_px(buffer[idx3], cr, cg, cb, border_alpha);
        }
        if x2 > 0 {
            let idx4 = y * width + (x2 - 1);
            buffer[idx4] = blend_px(buffer[idx4], cr, cg, cb, border_alpha);
        }
    }
}

/// Draw the simulated glowing laser pointer with active color
pub fn render_laser_pointer(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    color: u32,
) {
    let radius: isize = 13;
    let r_sq = radius * radius;
    let cx_i = cx as isize;
    let cy_i = cy as isize;

    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    for dy in -radius..=radius {
        let py = cy_i + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let py_u = py as usize;
        let row_offset = py_u * width;

        for dx in -radius..=radius {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > r_sq {
                continue;
            }
            let px = cx_i + dx;
            if px < 0 || px >= width as isize {
                continue;
            }
            let px_u = px as usize;
            let dist = (dist_sq as f32).sqrt();

            let (lr, lg, lb, alpha) = if dist <= 2.5 {
                (255, 255, 255, 1.0f32) // Glowing brilliant core
            } else if dist <= 5.5 {
                (cr, cg, cb, 0.90f32) // Vibrant colored core
            } else {
                let falloff = (1.0 - (dist - 5.5) / (radius as f32 - 5.5)).max(0.0);
                (cr, cg, cb, falloff * falloff * 0.70) // Smooth outer aura
            };

            let bg = buffer[row_offset + px_u];
            let bg_r = (bg >> 16) & 0xFF;
            let bg_g = (bg >> 8) & 0xFF;
            let bg_b = bg & 0xFF;

            let out_r = ((1.0 - alpha) * bg_r as f32 + alpha * lr as f32).min(255.0) as u32;
            let out_g = ((1.0 - alpha) * bg_g as f32 + alpha * lg as f32).min(255.0) as u32;
            let out_b = ((1.0 - alpha) * bg_b as f32 + alpha * lb as f32).min(255.0) as u32;

            buffer[row_offset + px_u] = (0xFF << 24) | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

/// Draw fading laser motion blur / phosphorescent trailing tail
pub fn render_laser_trail(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    trail: &[(usize, usize, f32)], // (x, y, freshness factor 0.0..1.0)
    color: u32,
) {
    if trail.len() < 2 {
        return;
    }

    for window in trail.windows(2) {
        let (x0, y0, f0) = window[0];
        let (x1, y1, f1) = window[1];
        let avg_f = (f0 + f1) * 0.5;
        let radius = (1.5 + avg_f * 3.5).round() as isize;
        let alpha = avg_f * avg_f * 0.75;

        draw_thick_line_alpha(
            buffer,
            width,
            height,
            x0 as isize,
            y0 as isize,
            x1 as isize,
            y1 as isize,
            radius,
            color,
            alpha,
        );
    }
}

/// Draw a continuous thick line segment with alpha blending
fn draw_thick_line_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    radius: isize,
    color: u32,
    alpha: f32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_disk_alpha(buffer, width, height, x0, y0, radius, color, alpha);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a disk stamp with alpha blending
fn draw_disk_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    r: isize,
    color: u32,
    alpha: f32,
) {
    let r_sq = r * r;
    let cr = (color >> 16) & 0xFF;
    let cg = (color >> 8) & 0xFF;
    let cb = color & 0xFF;

    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -r..=r {
            if dx * dx + dy * dy <= r_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    let idx = row_offset + px as usize;
                    let bg = buffer[idx];
                    let bg_r = (bg >> 16) & 0xFF;
                    let bg_g = (bg >> 8) & 0xFF;
                    let bg_b = bg & 0xFF;

                    let out_r = ((1.0 - alpha) * bg_r as f32 + alpha * cr as f32) as u32;
                    let out_g = ((1.0 - alpha) * bg_g as f32 + alpha * cg as f32) as u32;
                    let out_b = ((1.0 - alpha) * bg_b as f32 + alpha * cb as f32) as u32;

                    buffer[idx] = (0xFF << 24) | (out_r << 16) | (out_g << 8) | out_b;
                }
            }
        }
    }
}

/// Draw a disk / circle stamp for ink strokes
fn draw_disk(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    r: isize,
    color: u32,
) {
    let r_sq = r * r;
    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -r..=r {
            if dx * dx + dy * dy <= r_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    buffer[row_offset + px as usize] = color;
                }
            }
        }
    }
}

/// Draw a continuous thick line segment between two points
fn draw_thick_line(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    radius: isize,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_disk(buffer, width, height, x0, y0, radius, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Render whiteboard pen strokes onto the screen
pub fn render_ink_strokes(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    strokes: &[InkStroke],
) {
    for stroke in strokes {
        let r = (stroke.width / 2).max(1) as isize;
        if stroke.points.is_empty() {
            continue;
        }
        if stroke.points.len() == 1 {
            let (x, y) = stroke.points[0];
            draw_disk(
                buffer,
                width,
                height,
                x as isize,
                y as isize,
                r,
                stroke.color,
            );
            continue;
        }
        for window in stroke.points.windows(2) {
            let (x0, y0) = window[0];
            let (x1, y1) = window[1];
            draw_thick_line(
                buffer,
                width,
                height,
                x0 as isize,
                y0 as isize,
                x1 as isize,
                y1 as isize,
                r,
                stroke.color,
            );
        }
    }
}

/// Calculate dock bounds and individual button bounds
pub fn get_dock_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Vec<(DockAction, Rect, &'static str)>) {
    let dock_w = 460.0f32;
    let dock_h = 36.0f32;
    let dock_x = ((screen_w as f32) - dock_w) / 2.0;
    let dock_y = (screen_h as f32) - 48.0;

    let dock_rect = Rect::new(dock_x, dock_y, dock_w, dock_h);

    let full_label = if is_fullscreen {
        "WIN"
    } else {
        "FULL"
    };

    let items_def = [
        (DockAction::Prev, "<"),
        (DockAction::Next, ">"),
        (DockAction::ModeNormal, "PTR"),
        (DockAction::ModeLaser, "LSR"),
        (DockAction::ModePen, "PEN"),
        (DockAction::TogglePalette, "COL"),
        (DockAction::ClearInk, "CLR"),
        (DockAction::ToggleMute, "VOL"),
        (DockAction::ToggleFullscreen, full_label),
        (DockAction::ToggleHelp, "?"),
    ];

    let pad_x = 8.0f32;
    let pad_y = 5.0f32;
    let gap = 4.0f32;
    let total_gaps = (items_def.len() - 1) as f32 * gap;
    let btn_w = (dock_w - pad_x * 2.0 - total_gaps) / (items_def.len() as f32);
    let btn_h = dock_h - pad_y * 2.0;

    let mut item_rects = Vec::new();
    let mut cur_x = dock_x + pad_x;

    for (action, label) in items_def {
        let btn_rect = Rect::new(cur_x, dock_y + pad_y, btn_w, btn_h);
        item_rects.push((action, btn_rect, label));
        cur_x += btn_w + gap;
    }

    (dock_rect, item_rects)
}

/// Check if a mouse coordinate hits any dock action
pub fn hit_test_dock(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<DockAction> {
    let (dock_rect, items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    if !dock_rect.contains(mx, my) {
        return None;
    }
    for (action, rect, _) in items {
        if rect.contains(mx, my) {
            return Some(action);
        }
    }
    None
}

/// Draw the floating interactive dock
pub fn render_dock(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mode: PresenterMode,
    is_muted: bool,
    volume: u32,
    hovered: Option<DockAction>,
    mouse_near: bool,
    is_fullscreen: bool,
    active_color: u32,
    palette_open: bool,
) {
    let (dock_rect, items) = get_dock_rects(width, height, is_fullscreen);

    let alpha = if mouse_near {
        0.94f32
    } else {
        0.45f32
    };

    let start_x = dock_rect.x as usize;
    let start_y = dock_rect.y as usize;
    let end_x = ((dock_rect.x + dock_rect.width) as usize).min(width.saturating_sub(1));
    let end_y = ((dock_rect.y + dock_rect.height) as usize).min(height.saturating_sub(1));

    // Draw dock background
    for y in start_y..=end_y {
        let row_offset = y * width;
        for x in start_x..=end_x {
            let bg = buffer[row_offset + x];
            let r = (((bg >> 16) & 0xFF) as f32 * (1.0 - alpha) + 0x16 as f32 * alpha) as u32;
            let g = (((bg >> 8) & 0xFF) as f32 * (1.0 - alpha) + 0x1b as f32 * alpha) as u32;
            let b = ((bg & 0xFF) as f32 * (1.0 - alpha) + 0x22 as f32 * alpha) as u32;
            buffer[row_offset + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Dock outline border
    let border_color = if mouse_near {
        0xFF388bfd
    } else {
        0xFF30363d
    };
    for x in start_x..=end_x {
        buffer[start_y * width + x] = border_color;
        buffer[end_y * width + x] = border_color;
    }
    for y in start_y..=end_y {
        buffer[y * width + start_x] = border_color;
        buffer[y * width + end_x] = border_color;
    }

    // Render each dock button
    for (action, rect, default_label) in items {
        let is_active = match action {
            | DockAction::ModeNormal => mode == PresenterMode::Normal,
            | DockAction::ModeLaser => mode == PresenterMode::Laser,
            | DockAction::ModePen => mode == PresenterMode::Pen,
            | DockAction::TogglePalette => palette_open,
            | _ => false,
        };
        let is_hovered = hovered == Some(action);

        let bx1 = rect.x as usize;
        let by1 = rect.y as usize;
        let bx2 = ((rect.x + rect.width) as usize).min(width.saturating_sub(1));
        let by2 = ((rect.y + rect.height) as usize).min(height.saturating_sub(1));

        // Background color for button
        if is_active {
            for y in by1..=by2 {
                let row = y * width;
                for x in bx1..=bx2 {
                    buffer[row + x] = 0xFF1f6feb; // Vibrant blue for active
                }
            }
        } else if is_hovered {
            for y in by1..=by2 {
                let row = y * width;
                for x in bx1..=bx2 {
                    buffer[row + x] = 0xFF30363d; // Gray hover
                }
            }
        }

        // Custom label / badge for volume
        let label_owned;
        let label = if action == DockAction::ToggleMute {
            if is_muted {
                "MUT"
            } else {
                label_owned = format!("{}%", volume.min(100));
                &label_owned
            }
        } else {
            default_label
        };

        let text_color = if is_active {
            0xFFffffff
        } else if action == DockAction::TogglePalette {
            active_color // Show selected active palette color!
        } else if is_hovered {
            0xFF58a6ff
        } else if action == DockAction::ToggleMute && is_muted {
            0xFFf85149
        } else {
            0xFFc9d1d9
        };

        draw_text_centered(buffer, width, height, rect, label, text_color);
    }
}

/// Calculate bounds for the floating color palette popup
pub fn get_palette_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Vec<(usize, Rect)>) {
    let (_, dock_items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    let col_btn = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::TogglePalette)
        .map(|(_, r, _)| *r)
        .unwrap_or(Rect::new(
            screen_w as f32 / 2.0,
            screen_h as f32 - 48.0,
            36.0,
            26.0,
        ));

    let popup_w = 200.0f32;
    let popup_h = 36.0f32;
    let popup_x = (col_btn.x + col_btn.width / 2.0 - popup_w / 2.0)
        .clamp(10.0, screen_w as f32 - popup_w - 10.0);
    let popup_y = col_btn.y - popup_h - 8.0;

    let popup_rect = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let count = PALETTE_COLORS.len();
    let pad = 8.0f32;
    let gap = 4.0f32;
    let chip_w = (popup_w - pad * 2.0 - (count - 1) as f32 * gap) / count as f32;
    let chip_h = popup_h - pad * 2.0;

    let mut chip_rects = Vec::new();
    let mut cur_x = popup_x + pad;
    for i in 0..count {
        chip_rects.push((i, Rect::new(cur_x, popup_y + pad, chip_w, chip_h)));
        cur_x += chip_w + gap;
    }

    (popup_rect, chip_rects)
}

/// Check if mouse clicked on any color swatch chip
pub fn hit_test_palette(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<usize> {
    let (popup_rect, chips) = get_palette_rects(screen_w, screen_h, is_fullscreen);
    if !popup_rect.contains(mx, my) {
        return None;
    }
    for (idx, rect) in chips {
        if rect.contains(mx, my) {
            return Some(idx);
        }
    }
    None
}

/// Render the elegant floating color palette popup
pub fn render_palette_popup(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    is_fullscreen: bool,
    active_idx: usize,
    hovered_idx: Option<usize>,
) {
    let (popup_rect, chips) = get_palette_rects(width, height, is_fullscreen);

    let x1 = popup_rect.x as usize;
    let y1 = popup_rect.y as usize;
    let x2 = ((popup_rect.x + popup_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((popup_rect.y + popup_rect.height) as usize).min(height.saturating_sub(1));

    // Dark glassmorphic background
    for y in y1..=y2 {
        let row = y * width;
        for x in x1..=x2 {
            buffer[row + x] = 0xF0161b22;
        }
    }

    // Border
    for x in x1..=x2 {
        buffer[y1 * width + x] = 0xFF58a6ff;
        buffer[y2 * width + x] = 0xFF58a6ff;
    }
    for y in y1..=y2 {
        buffer[y * width + x1] = 0xFF58a6ff;
        buffer[y * width + x2] = 0xFF58a6ff;
    }

    // Draw color chips
    for (i, rect) in chips {
        let cx = (rect.x + rect.width / 2.0) as isize;
        let cy = (rect.y + rect.height / 2.0) as isize;
        let color = PALETTE_COLORS[i].1;

        let r: isize = if hovered_idx == Some(i) {
            8
        } else {
            7
        };
        draw_disk(buffer, width, height, cx, cy, r, color);

        // Highlight ring around active selection
        if i == active_idx {
            draw_circle_ring(buffer, width, height, cx, cy, 10, 0xFFFFFFFF);
        }
    }
}

/// Draw an unfilled circle ring
fn draw_circle_ring(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: isize,
    cy: isize,
    radius: isize,
    color: u32,
) {
    let r_min_sq = (radius - 1) * (radius - 1);
    let r_max_sq = radius * radius;

    for dy in -radius..=radius {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        let row_offset = (py as usize) * width;
        for dx in -radius..=radius {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= r_min_sq && dist_sq <= r_max_sq {
                let px = cx + dx;
                if px >= 0 && px < width as isize {
                    buffer[row_offset + px as usize] = color;
                }
            }
        }
    }
}

/// Calculate bounds for floating volume slider popup
pub fn get_volume_slider_rects(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
) -> (Rect, Rect) {
    let (_, dock_items) = get_dock_rects(screen_w, screen_h, is_fullscreen);
    let vol_btn = dock_items
        .iter()
        .find(|(a, _, _)| *a == DockAction::ToggleMute)
        .map(|(_, r, _)| *r)
        .unwrap_or(Rect::new(
            screen_w as f32 / 2.0,
            screen_h as f32 - 48.0,
            36.0,
            26.0,
        ));

    let popup_w = 38.0f32;
    let popup_h = 130.0f32;
    let popup_x = vol_btn.x + (vol_btn.width - popup_w) / 2.0;
    let popup_y = vol_btn.y - popup_h - 8.0;

    let popup_rect = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let track_w = 8.0f32;
    let track_h = 100.0f32;
    let track_x = popup_x + (popup_w - track_w) / 2.0;
    let track_y = popup_y + 15.0;
    let track_rect = Rect::new(track_x, track_y, track_w, track_h);

    (popup_rect, track_rect)
}

/// Check if mouse interacted with vertical volume slider track
pub fn hit_test_volume_slider(
    screen_w: usize,
    screen_h: usize,
    is_fullscreen: bool,
    mx: f32,
    my: f32,
) -> Option<f32> {
    let (popup_rect, track_rect) = get_volume_slider_rects(screen_w, screen_h, is_fullscreen);
    if !popup_rect.contains(mx, my) {
        return None;
    }
    // Fraction: bottom is 0.0, top is 1.0
    let norm = 1.0 - ((my - track_rect.y) / track_rect.height).clamp(0.0, 1.0);
    Some(norm)
}

/// Render the interactive volume slider popup
pub fn render_volume_slider_popup(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    is_fullscreen: bool,
    volume: f32,
    is_muted: bool,
) {
    let (popup_rect, track_rect) = get_volume_slider_rects(width, height, is_fullscreen);

    let x1 = popup_rect.x as usize;
    let y1 = popup_rect.y as usize;
    let x2 = ((popup_rect.x + popup_rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((popup_rect.y + popup_rect.height) as usize).min(height.saturating_sub(1));

    // Background
    for y in y1..=y2 {
        let row = y * width;
        for x in x1..=x2 {
            buffer[row + x] = 0xF0161b22;
        }
    }
    // Border
    for x in x1..=x2 {
        buffer[y1 * width + x] = 0xFF58a6ff;
        buffer[y2 * width + x] = 0xFF58a6ff;
    }
    for y in y1..=y2 {
        buffer[y * width + x1] = 0xFF58a6ff;
        buffer[y * width + x2] = 0xFF58a6ff;
    }

    // Draw track
    let tx1 = track_rect.x as usize;
    let ty1 = track_rect.y as usize;
    let tx2 = ((track_rect.x + track_rect.width) as usize).min(width.saturating_sub(1));
    let ty2 = ((track_rect.y + track_rect.height) as usize).min(height.saturating_sub(1));

    let fill_h = if is_muted {
        0
    } else {
        (track_rect.height * volume.clamp(0.0, 1.0)).round() as usize
    };
    let fill_start_y = ty2.saturating_sub(fill_h);

    for y in ty1..=ty2 {
        let row = y * width;
        for x in tx1..=tx2 {
            if y >= fill_start_y && !is_muted {
                buffer[row + x] = 0xFF39d353; // Green fill
            } else {
                buffer[row + x] = 0xFF30363d; // Inactive track
            }
        }
    }

    // Draw thumb knob
    let thumb_y = if is_muted {
        ty2
    } else {
        fill_start_y
    };
    let thumb_x = (track_rect.x + track_rect.width / 2.0) as isize;
    draw_disk(
        buffer,
        width,
        height,
        thumb_x,
        thumb_y as isize,
        6,
        0xFFFFFFFF,
    );
}

/// Draw volume toast HUD overlay when volume changes
pub fn draw_volume_toast(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    volume_pct: u32,
    is_muted: bool,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }

    let card_w = 180usize;
    let card_h = 42usize;
    let card_x = width.saturating_sub(card_w + 20);
    let card_y = 20usize;

    let x2 = (card_x + card_w).min(width.saturating_sub(1));
    let y2 = (card_y + card_h).min(height.saturating_sub(1));

    // Draw dark card background with alpha
    for y in card_y..=y2 {
        let row = y * width;
        for x in card_x..=x2 {
            let bg = buffer[row + x];
            let r = (((bg >> 16) & 0xFF) as f32 * (1.0 - alpha) + 0x16 as f32 * alpha) as u32;
            let g = (((bg >> 8) & 0xFF) as f32 * (1.0 - alpha) + 0x1b as f32 * alpha) as u32;
            let b = ((bg & 0xFF) as f32 * (1.0 - alpha) + 0x22 as f32 * alpha) as u32;
            buffer[row + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Border
    let border_col = if is_muted {
        0xFFf85149
    } else {
        0xFF58a6ff
    };
    for x in card_x..=x2 {
        buffer[card_y * width + x] = border_col;
        buffer[y2 * width + x] = border_col;
    }
    for y in card_y..=y2 {
        buffer[y * width + card_x] = border_col;
        buffer[y * width + x2] = border_col;
    }

    // Text & meter
    let text = if is_muted {
        "VOLUME: MUTED".to_string()
    } else {
        format!("VOLUME: {}%", volume_pct)
    };
    draw_text(
        buffer,
        width,
        height,
        card_x + 12,
        card_y + 8,
        &text,
        0xFFFFFFFF,
    );

    // Progress bar inside toast
    let bar_x1 = card_x + 12;
    let bar_y1 = card_y + 24;
    let bar_w = card_w - 24;
    let bar_h = 8;
    let fill_w = if is_muted {
        0
    } else {
        (bar_w as f32 * (volume_pct as f32 / 100.0)).round() as usize
    };

    for dy in 0..bar_h {
        let y = bar_y1 + dy;
        if y >= height {
            break;
        }
        let row = y * width;
        for dx in 0..bar_w {
            let x = bar_x1 + dx;
            if x >= width {
                break;
            }
            if dx <= fill_w && !is_muted {
                buffer[row + x] = 0xFF39d353;
            } else {
                buffer[row + x] = 0xFF30363d;
            }
        }
    }
}

/// Simple pixel-based badge for slide index and substep counter
pub fn draw_page_badge(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    current_slide: usize,
    total_slides: usize,
    current_step: usize,
    total_steps: usize,
) {
    let badge_str = if total_steps > 0 {
        format!(
            "{} / {}  [STEP {}/{}]",
            current_slide, total_slides, current_step, total_steps
        )
    } else {
        format!("{} / {}", current_slide, total_slides)
    };

    let char_w = 6;
    let char_h = 8;
    let padding = 8;

    let text_w = badge_str.len() * (char_w + 1);
    let box_w = text_w + padding * 2;
    let box_h = char_h + padding * 2;

    let start_x = width.saturating_sub(box_w + 16);
    let start_y = height.saturating_sub(box_h + 16);

    // Draw semi-transparent background box
    for y in start_y..(start_y + box_h).min(height) {
        let row_start = y * width;
        for x in start_x..(start_x + box_w).min(width) {
            let bg = buffer[row_start + x];
            let r = (((bg >> 16) & 0xFF) * 20 + 0x16 * 80) / 100;
            let g = (((bg >> 8) & 0xFF) * 20 + 0x1b * 80) / 100;
            let b = ((bg & 0xFF) * 20 + 0x22 * 80) / 100;
            buffer[row_start + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    let mut cur_x = start_x + padding;
    let cur_y = start_y + padding;

    for ch in badge_str.chars() {
        draw_char(buffer, width, height, cur_x, cur_y, ch, 0xFF58a6ff);
        cur_x += char_w + 1;
    }
}

/// Draw a presenter shortcut help overlay in the center of the screen
pub fn draw_help_overlay(
    buffer: &mut [u32],
    width: usize,
    height: usize,
) {
    let lines = [
        "CARGO SLIDE PRESENTER SHORTCUTS",
        "-----------------------------------------",
        "Space / Right / Left-Click: Next step / slide",
        "Backspace / Left / Right-Click: Prev step / slide",
        "F11 / F              : Toggle Fullscreen / Windowed",
        "1 .. 9               : Jump to slide",
        "Home / End           : First / Last slide",
        "L                    : Toggle Laser pointer (with trail)",
        "P                    : Toggle Whiteboard Pen",
        "K / Dock [COL]       : Open Color Palette (1..7 keys)",
        "C / X                : Clear ink strokes",
        "Mouse Wheel          : Adjust Volume (+/- 5%)",
        "+ / = / Up           : Volume +5%",
        "- / _ / Down         : Volume -5%",
        "M                    : Mute / Unmute audio",
        "Bottom Dock          : Touch / Click presentation controls",
        "R                    : Live reload slides",
        "H / ?                : Toggle this help",
        "Esc / Q              : Exit presentation",
    ];

    let char_w = 6;
    let line_height = 14;
    let padding = 18;

    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(20);
    let box_w = max_line_len * (char_w + 1) + padding * 2;
    let box_h = lines.len() * line_height + padding * 2;

    let start_x = (width.saturating_sub(box_w)) / 2;
    let start_y = (height.saturating_sub(box_h)) / 2;

    // Dark modal background
    for y in start_y..(start_y + box_h).min(height) {
        let row_start = y * width;
        for x in start_x..(start_x + box_w).min(width) {
            let bg = buffer[row_start + x];
            let r = (((bg >> 16) & 0xFF) * 8 + 0x10 * 92) / 100;
            let g = (((bg >> 8) & 0xFF) * 8 + 0x14 * 92) / 100;
            let b = ((bg & 0xFF) * 8 + 0x1d * 92) / 100;
            buffer[row_start + x] = (0xFF << 24) | (r << 16) | (g << 8) | b;
        }
    }

    // Modal border
    let end_x = (start_x + box_w).min(width.saturating_sub(1));
    let end_y = (start_y + box_h).min(height.saturating_sub(1));
    let border_color = 0xFF58a6ff;
    for x in start_x..=end_x {
        buffer[start_y * width + x] = border_color;
        buffer[end_y * width + x] = border_color;
    }
    for y in start_y..=end_y {
        buffer[y * width + start_x] = border_color;
        buffer[y * width + end_x] = border_color;
    }

    // Draw text lines
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + padding + i * line_height;
        let color = if i == 0 {
            0xFF58a6ff // Accent title
        } else if i == 1 {
            0xFF8b949e // Divider
        } else {
            0xFFe6edf3 // Body text
        };

        draw_text(buffer, width, height, start_x + padding, y, line, color);
    }
}

/// Draw a line of text at (x, y)
pub fn draw_text(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mut x: usize,
    y: usize,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    for ch in text.chars() {
        draw_char(buffer, width, height, x, y, ch, color);
        x += char_w + 1;
    }
}

/// Draw text centered within a given bounding box
pub fn draw_text_centered(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    let char_h = 7;
    let text_w = text.len() * (char_w + 1);
    let start_x = rect.x as usize + (rect.width as usize).saturating_sub(text_w) / 2;
    let start_y = rect.y as usize + (rect.height as usize).saturating_sub(char_h) / 2;
    draw_text(buffer, width, height, start_x, start_y, text, color);
}

/// Draw text centered around a coordinate point (cx, cy)
pub fn draw_text_at_center(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    text: &str,
    color: u32,
) {
    let char_w = 6;
    let char_h = 7;
    let text_w = text.len() * (char_w + 1);
    let start_x = cx.saturating_sub(text_w / 2);
    let start_y = cy.saturating_sub(char_h / 2);
    draw_text(buffer, width, height, start_x, start_y, text, color);
}

/// Minimal 5x7 bitmap font for HUD numerals & basic symbols
fn draw_char(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    ch: char,
    color: u32,
) {
    let upper = ch.to_ascii_uppercase();
    let bitmap: [u8; 7] = match upper {
        | '0' => {
            [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ]
        },
        | '1' => {
            [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ]
        },
        | '2' => {
            [
                0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
            ]
        },
        | '3' => {
            [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ]
        },
        | '4' => {
            [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ]
        },
        | '5' => {
            [
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ]
        },
        | '6' => {
            [
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ]
        },
        | '7' => {
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ]
        },
        | '8' => {
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ]
        },
        | '9' => {
            [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
            ]
        },
        | 'A' => {
            [
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'B' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ]
        },
        | 'C' => {
            [
                0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
            ]
        },
        | 'D' => {
            [
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ]
        },
        | 'E' => {
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ]
        },
        | 'F' => {
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ]
        },
        | 'G' => {
            [
                0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
            ]
        },
        | 'H' => {
            [
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'I' => {
            [
                0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ]
        },
        | 'J' => {
            [
                0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
            ]
        },
        | 'K' => {
            [
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ]
        },
        | 'L' => {
            [
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ]
        },
        | 'M' => {
            [
                0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'N' => {
            [
                0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
            ]
        },
        | 'O' => {
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ]
        },
        | 'P' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ]
        },
        | 'Q' => {
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10001, 0b01101,
            ]
        },
        | 'R' => {
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ]
        },
        | 'S' => {
            [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ]
        },
        | 'T' => {
            [
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | 'U' => {
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ]
        },
        | 'V' => {
            [
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ]
        },
        | 'W' => {
            [
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
            ]
        },
        | 'X' => {
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
            ]
        },
        | 'Y' => {
            [
                0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | 'Z' => {
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
            ]
        },
        | '<' => {
            [
                0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
            ]
        },
        | '>' => {
            [
                0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
            ]
        },
        | '[' => {
            [
                0b11110, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11110,
            ]
        },
        | ']' => {
            [
                0b01111, 0b00001, 0b00001, 0b00001, 0b00001, 0b00001, 0b01111,
            ]
        },
        | '%' => {
            [
                0b11001, 0b11010, 0b00100, 0b01000, 0b01011, 0b10011, 0b00000,
            ]
        },
        | '/' => {
            [
                0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
            ]
        },
        | '\\' => {
            [
                0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
            ]
        },
        | '-' => {
            [
                0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '+' => {
            [
                0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
            ]
        },
        | '=' => {
            [
                0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
            ]
        },
        | '.' => {
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
            ]
        },
        | ',' => {
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100,
            ]
        },
        | ':' => {
            [
                0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
            ]
        },
        | '?' => {
            [
                0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
            ]
        },
        | '(' => {
            [
                0b00100, 0b01000, 0b10000, 0b10000, 0b10000, 0b01000, 0b00100,
            ]
        },
        | ')' => {
            [
                0b00100, 0b00010, 0b00001, 0b00001, 0b00001, 0b00010, 0b00100,
            ]
        },
        | '$' => {
            [
                0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100,
            ]
        },
        | '#' => {
            [
                0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
            ]
        },
        | '|' => {
            [
                0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ]
        },
        | '*' => {
            [
                0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000,
            ]
        },
        | ' ' => [0; 7],
        | _ => {
            [
                0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
            ]
        },
    };

    for (row, bits) in bitmap.iter().enumerate() {
        let py = y + row;
        if py >= height {
            break;
        }
        for col in 0..5 {
            let px = x + col;
            if px >= width {
                break;
            }
            if (bits & (1 << (4 - col))) != 0 {
                buffer[py * width + px] = color;
            }
        }
    }
}

/// Quick on-slide action for charts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartQuickAction {
    CycleType,
    OpenInspector,
}

/// Action produced by clicking within the Chart Data Inspector modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorAction {
    Close,
    SetType(ChartType),
    SetTransform(ChartTransform),
    ToggleSeries(usize),
    ExportCsv,
    SelectCategory(usize),
    ScrollTable(isize),
}

pub const TRANSFORM_PRESETS: [(&str, ChartTransform); 7] = [
    ("ORIG", ChartTransform::None),
    ("TOP 5", ChartTransform::TopK(5)),
    ("SORT ▼", ChartTransform::SortDesc),
    ("SORT ▲", ChartTransform::SortAsc),
    ("CUM", ChartTransform::Cumulative),
    ("% SHARE", ChartTransform::Percent100),
    ("MA3", ChartTransform::MovingAvg(3)),
];

pub fn get_transform_preset_rects(series_strip: Rect) -> Vec<(Rect, &'static str, ChartTransform)> {
    let mut result = Vec::new();
    let mut right_x = series_strip.x + series_strip.width;
    let gap = 5.0;
    let chip_h = 22.0;

    for &(label, t) in TRANSFORM_PRESETS.iter().rev() {
        let chip_w = (label.len() * 6 + 16) as f32;
        right_x -= chip_w;
        let r = Rect::new(right_x, series_strip.y + 2.0, chip_w, chip_h);
        result.push((r, label, t));
        right_x -= gap;
    }
    result.reverse();
    result
}

/// State of the active Chart Data Inspector modal
#[derive(Debug, Clone)]
pub struct ChartInspectorState {
    pub original_chart_data: ChartData,
    pub chart_data: ChartData,
    pub active_type: ChartType,
    pub active_transform: ChartTransform,
    pub hidden_series: std::collections::HashSet<usize>,
    pub hovered_category: Option<usize>,
    pub hovered_series: Option<usize>,
    pub table_scroll: usize,
    pub toast_message: Option<(String, std::time::Instant)>,
}

impl ChartInspectorState {
    pub fn new(
        chart_data: ChartData,
        initial_type: ChartType,
    ) -> Self {
        Self {
            original_chart_data: chart_data.clone(),
            chart_data,
            active_type: initial_type,
            active_transform: ChartTransform::None,
            hidden_series: std::collections::HashSet::new(),
            hovered_category: None,
            hovered_series: None,
            table_scroll: 0,
            toast_message: None,
        }
    }

    pub fn set_transform(
        &mut self,
        transform: ChartTransform,
    ) {
        if self.active_transform == transform {
            self.active_transform = ChartTransform::None;
            self.chart_data = self.original_chart_data.clone();
            self.show_toast("Reset to Original Data");
        } else {
            self.active_transform = transform;
            self.chart_data = self.original_chart_data.apply_transform(transform);
            let msg = match transform {
                | ChartTransform::None => "Original Data".to_string(),
                | ChartTransform::TopK(k) => format!("Top {} by Total", k),
                | ChartTransform::SortDesc => "Sorted Descending (Max First)".to_string(),
                | ChartTransform::SortAsc => "Sorted Ascending (Min First)".to_string(),
                | ChartTransform::Cumulative => "Cumulative Running Sum".to_string(),
                | ChartTransform::Percent100 => "100% Share Normalized".to_string(),
                | ChartTransform::MovingAvg(w) => format!("Moving Average (MA{})", w),
            };
            self.show_toast(&msg);
        }
    }

    pub fn cycle_type(&mut self) {
        self.active_type = match self.active_type {
            | ChartType::Bar => ChartType::Line,
            | ChartType::Line => ChartType::Area,
            | ChartType::Area => ChartType::Pie,
            | ChartType::Pie => ChartType::Donut,
            | ChartType::Donut => ChartType::Bar,
            | ChartType::Scatter => ChartType::Bar,
        };
    }

    pub fn toggle_series(
        &mut self,
        s_idx: usize,
    ) {
        if s_idx < self.chart_data.series.len() {
            if self.hidden_series.contains(&s_idx) {
                self.hidden_series.remove(&s_idx);
            } else {
                // Ensure at least one series remains visible
                if self.hidden_series.len() + 1 < self.chart_data.series.len() {
                    self.hidden_series.insert(s_idx);
                }
            }
        }
    }

    pub fn show_toast(
        &mut self,
        msg: &str,
    ) {
        self.toast_message = Some((msg.to_string(), std::time::Instant::now()));
    }
}

/// Get rectangles for on-slide quick action pills `[TYPE]` and `[DATA]`
pub fn get_chart_quick_action_rects(
    metrics: &RenderMetrics,
    chart_rect: Rect,
) -> (Rect, Rect) {
    let (sx, sy) = metrics.svg_to_screen(chart_rect.x, chart_rect.y);
    let sw = chart_rect.width * metrics.scale;
    let btn_w = 48.0;
    let btn_h = 18.0;
    let pad = 6.0;

    let inspect_btn = Rect::new(sx + sw - pad - btn_w, sy + pad, btn_w, btn_h);
    let type_btn = Rect::new(sx + sw - pad - btn_w - 4.0 - btn_w, sy + pad, btn_w, btn_h);
    (type_btn, inspect_btn)
}

/// Hit-test on-slide quick action buttons
pub fn hit_test_chart_quick_actions(
    metrics: &RenderMetrics,
    chart_rect: Rect,
    mouse_x: f32,
    mouse_y: f32,
) -> Option<ChartQuickAction> {
    let (type_btn, inspect_btn) = get_chart_quick_action_rects(metrics, chart_rect);
    if type_btn.contains(mouse_x, mouse_y) {
        Some(ChartQuickAction::CycleType)
    } else if inspect_btn.contains(mouse_x, mouse_y) {
        Some(ChartQuickAction::OpenInspector)
    } else {
        None
    }
}

/// Draw a sleek pill-shaped quick action button
pub fn draw_chart_quick_button(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    label: &str,
    is_hovered: bool,
    accent_color: u32,
) {
    let x1 = (rect.x as usize).min(width.saturating_sub(1));
    let y1 = (rect.y as usize).min(height.saturating_sub(1));
    let x2 = ((rect.x + rect.width) as usize).min(width.saturating_sub(1));
    let y2 = ((rect.y + rect.height) as usize).min(height.saturating_sub(1));

    if x2 <= x1 || y2 <= y1 {
        return;
    }

    let bg_color = if is_hovered {
        0xEE21262d
    } else {
        0xD0161b22
    };
    let border_color = if is_hovered {
        accent_color
    } else {
        0xFF30363d
    };

    for y in y1..=y2 {
        let row = y * width;
        for x in x1..=x2 {
            buffer[row + x] = bg_color;
        }
    }

    // Border
    for x in x1..=x2 {
        buffer[y1 * width + x] = border_color;
        buffer[y2 * width + x] = border_color;
    }
    for y in y1..=y2 {
        buffer[y * width + x1] = border_color;
        buffer[y * width + x2] = border_color;
    }

    // Centered text
    let text_w = label.len() * 6;
    let tx = (rect.x + (rect.width - text_w as f32) * 0.5).max(rect.x + 2.0) as usize;
    let ty = (rect.y + (rect.height - 7.0) * 0.5) as usize;
    draw_text(
        buffer,
        width,
        height,
        tx,
        ty,
        label,
        if is_hovered {
            0xFFFFFFFF
        } else {
            0xFFc9d1d9
        },
    );
}

/// Fast rectangular fill with solid color
pub fn fill_rect(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    let x2 = (x + w).min(width);
    let y2 = (y + h).min(height);
    for py in y.min(height)..y2 {
        let row = py * width;
        for px in x.min(width)..x2 {
            buffer[row + px] = color;
        }
    }
}

/// Fast rectangular fill with alpha blending
pub fn fill_rect_alpha(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
    alpha_256: u32,
) {
    let x2 = (x + w).min(width);
    let y2 = (y + h).min(height);
    for py in y.min(height)..y2 {
        let row = py * width;
        for px in x.min(width)..x2 {
            buffer[row + px] = blend_pixel_fast(buffer[row + px], color, alpha_256);
        }
    }
}

/// Draw 1px rectangle outline
pub fn draw_rect_outline(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if w == 0 || h == 0 || x >= width || y >= height {
        return;
    }
    let x2 = (x + w - 1).min(width.saturating_sub(1));
    let y2 = (y + h - 1).min(height.saturating_sub(1));
    for px in x..=x2 {
        buffer[y * width + px] = color;
        buffer[y2 * width + px] = color;
    }
    for py in y..=y2 {
        buffer[py * width + x] = color;
        buffer[py * width + x2] = color;
    }
}

/// Draw horizontal dashed line
pub fn draw_dashed_hline(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    x1: usize,
    x2: usize,
    y: usize,
    color: u32,
    dash_len: usize,
    gap_len: usize,
) {
    if y >= height {
        return;
    }
    let row = y * width;
    let mut px = x1;
    while px < x2 && px < width {
        let seg_end = (px + dash_len).min(x2).min(width);
        for x in px..seg_end {
            buffer[row + x] = color;
        }
        px += dash_len + gap_len;
    }
}

/// Render interactive hovering indicators and glassmorphic tooltip card over a chart
pub fn render_chart_hover(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    metrics: &RenderMetrics,
    chart_rect: Rect,
    chart_data: &slide_core::chart::ChartData,
    active_type: ChartType,
    mouse_x: f32,
    mouse_y: f32,
) {
    let (type_btn, inspect_btn) = get_chart_quick_action_rects(metrics, chart_rect);
    let hovered_action = hit_test_chart_quick_actions(metrics, chart_rect, mouse_x, mouse_y);

    // Draw top-right quick action pills
    let type_label = match active_type {
        | ChartType::Bar => "BAR",
        | ChartType::Line => "LINE",
        | ChartType::Area => "AREA",
        | ChartType::Pie => "PIE",
        | ChartType::Donut => "DONUT",
        | ChartType::Scatter => "SCAT",
    };
    draw_chart_quick_button(
        buffer,
        width,
        height,
        type_btn,
        type_label,
        hovered_action == Some(ChartQuickAction::CycleType),
        0xFF38bdf8,
    );
    draw_chart_quick_button(
        buffer,
        width,
        height,
        inspect_btn,
        "DATA",
        hovered_action == Some(ChartQuickAction::OpenInspector),
        0xFF34d399,
    );

    // If mouse is hovering a button pill, don't show category tooltip overlapping the buttons
    if hovered_action.is_some() {
        return;
    }

    if let Some((svg_x, svg_y)) = metrics.screen_to_svg(mouse_x, mouse_y) {
        if !chart_rect.contains(svg_x, svg_y) {
            return;
        }

        match active_type {
            | slide_core::chart::ChartType::Bar
            | slide_core::chart::ChartType::Line
            | slide_core::chart::ChartType::Area
            | slide_core::chart::ChartType::Scatter => {
                if let Some(cat_idx) = chart_data.hit_test_category(chart_rect, svg_x, svg_y) {
                    let cat_name = chart_data
                        .categories
                        .get(cat_idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Item {}", cat_idx + 1));
                    let plot = chart_data.plot_area(chart_rect);
                    let cat_count = chart_data.categories.len().max(1);
                    let col_w = plot.width / cat_count as f32;

                    if active_type == slide_core::chart::ChartType::Bar {
                        // Highlight vertical column with fast bit-shift alpha blending
                        let col_svg_x = plot.x + cat_idx as f32 * col_w;
                        let (screen_col_x, screen_plot_y) =
                            metrics.svg_to_screen(col_svg_x, plot.y);
                        let screen_col_w = col_w * metrics.scale;
                        let screen_plot_h = plot.height * metrics.scale;

                        let x1 = (screen_col_x.max(0.0) as usize).min(width);
                        let y1 = (screen_plot_y.max(0.0) as usize).min(height);
                        let x2 = ((screen_col_x + screen_col_w).max(0.0) as usize).min(width);
                        let y2 = ((screen_plot_y + screen_plot_h).max(0.0) as usize).min(height);

                        for py in y1..y2 {
                            let row = py * width;
                            for px in x1..x2 {
                                buffer[row + px] =
                                    blend_pixel_fast(buffer[row + px], 0xFF388bfd, 56);
                            }
                        }
                    } else {
                        // Vertical guideline crosshair
                        let center_svg_x = plot.x + (cat_idx as f32 + 0.5) * col_w;
                        let (guide_x, guide_top_y) = metrics.svg_to_screen(center_svg_x, plot.y);
                        let (_, guide_bottom_y) =
                            metrics.svg_to_screen(center_svg_x, plot.y + plot.height);
                        let gx = guide_x.round() as usize;
                        if gx < width {
                            let gy1 = (guide_top_y.max(0.0) as usize).min(height);
                            let gy2 = (guide_bottom_y.max(0.0) as usize).min(height);
                            for gy in gy1..gy2 {
                                buffer[gy * width + gx] = 0xAA58a6ff;
                                if gx + 1 < width {
                                    buffer[gy * width + gx + 1] = 0x4458a6ff;
                                }
                            }
                        }

                        // Glowing dots at points on the series
                        let (y_min, y_max, _) = chart_data.nice_scale(5);
                        for s in &chart_data.series {
                            if let Some(&val) = s.values.get(cat_idx) {
                                let pt_svg_y = plot.y + plot.height
                                    - ((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                                        * plot.height;
                                let (screen_pt_x, screen_pt_y) =
                                    metrics.svg_to_screen(center_svg_x, pt_svg_y);
                                draw_glow_marker(
                                    buffer,
                                    width,
                                    height,
                                    screen_pt_x as usize,
                                    screen_pt_y as usize,
                                    0xFF58a6ff,
                                );
                            }
                        }
                    }

                    // Render floating tooltip HUD card
                    render_tooltip_card(
                        buffer,
                        width,
                        height,
                        mouse_x as usize,
                        mouse_y as usize,
                        &cat_name,
                        &chart_data.series,
                        cat_idx,
                    );
                }
            },
            | slide_core::chart::ChartType::Pie | slide_core::chart::ChartType::Donut => {
                if let Some(slice_idx) = chart_data.hit_test_pie_slice(chart_rect, svg_x, svg_y) {
                    let (slice_name, slice_val, total): (String, f64, f64) =
                        if chart_data.series.len() == 1 {
                            let name = chart_data
                                .categories
                                .get(slice_idx)
                                .cloned()
                                .unwrap_or_else(|| format!("Slice {}", slice_idx + 1));
                            let val = chart_data.series[0]
                                .values
                                .get(slice_idx)
                                .copied()
                                .unwrap_or(0.0);
                            let tot = chart_data.series[0].values.iter().sum();
                            (name, val, tot)
                        } else if slice_idx < chart_data.series.len() {
                            let s = &chart_data.series[slice_idx];
                            let val = s.values.first().copied().unwrap_or(0.0);
                            let tot = chart_data
                                .series
                                .iter()
                                .map(|s| s.values.first().copied().unwrap_or(0.0))
                                .sum();
                            (s.name.clone(), val, tot)
                        } else {
                            return;
                        };

                    let pct = if total > 0.0 {
                        (slice_val / total) * 100.0
                    } else {
                        0.0
                    };
                    let lines = vec![
                        format!(
                            "VALUE: {}",
                            slide_core::chart::ChartData::format_value(slice_val)
                        ),
                        format!("SHARE: {:.1}%", pct),
                    ];
                    render_simple_tooltip(
                        buffer,
                        width,
                        height,
                        mouse_x as usize,
                        mouse_y as usize,
                        &slice_name,
                        &lines,
                    );
                }
            },
        }
    }
}

fn draw_glow_marker(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    color: u32,
) {
    for dy in -4..=4 {
        let py = cy as isize + dy;
        if py < 0 || py as usize >= height {
            continue;
        }
        let uy = py as usize;
        for dx in -4..=4 {
            let px = cx as isize + dx;
            if px < 0 || px as usize >= width {
                continue;
            }
            let ux = px as usize;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= 4 {
                buffer[uy * width + ux] = 0xFFffffff; // White center
            } else if dist_sq <= 16 {
                buffer[uy * width + ux] = color; // Colored halo
            }
        }
    }
}

fn render_tooltip_card(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    title: &str,
    series: &[slide_core::chart::SeriesData],
    cat_idx: usize,
) {
    let card_w = 170usize;
    let card_h = 32 + series.len() * 18;

    let card_x = if mx + 16 + card_w < width {
        mx + 16
    } else {
        mx.saturating_sub(card_w + 10)
    };
    let card_y = if my + 16 + card_h < height {
        my + 16
    } else {
        my.saturating_sub(card_h + 10)
    };

    for y in card_y..(card_y + card_h).min(height) {
        let row = y * width;
        let is_edge_y = y == card_y || y == card_y + card_h - 1;
        for x in card_x..(card_x + card_w).min(width) {
            let is_edge_x = x == card_x || x == card_x + card_w - 1;
            if is_edge_y || is_edge_x {
                buffer[row + x] = 0xFF30363d;
            } else {
                buffer[row + x] = blend_pixel_fast(buffer[row + x], 0xFF161b22, 235);
            }
        }
    }

    draw_text(
        buffer,
        width,
        height,
        card_x + 10,
        card_y + 8,
        title,
        0xFF58a6ff,
    );

    let div_y = card_y + 22;
    if div_y < height {
        let row = div_y * width;
        for x in (card_x + 8)..(card_x + card_w - 8).min(width) {
            buffer[row + x] = 0xFF30363d;
        }
    }

    for (i, s) in series.iter().enumerate() {
        let sy = card_y + 28 + i * 18;
        if sy + 10 >= height {
            break;
        }

        let color = if let Some(ref c) = s.color {
            parse_hex_color(c)
        } else {
            0xFF58a6ff
        };

        for by in 0..6 {
            let brow = (sy + by) * width;
            for bx in 0..6 {
                if card_x + 10 + bx < width {
                    buffer[brow + card_x + 10 + bx] = color;
                }
            }
        }

        let val_str = s
            .values
            .get(cat_idx)
            .map(|&v| slide_core::chart::ChartData::format_value(v))
            .unwrap_or_else(|| "0".into());
        let label = format!("{}: {}", s.name, val_str);
        draw_text(buffer, width, height, card_x + 22, sy, &label, 0xFFe6edf3);
    }
}

fn render_simple_tooltip(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    mx: usize,
    my: usize,
    title: &str,
    lines: &[String],
) {
    let card_w = 150usize;
    let card_h = 28 + lines.len() * 16;

    let card_x = if mx + 16 + card_w < width {
        mx + 16
    } else {
        mx.saturating_sub(card_w + 10)
    };
    let card_y = if my + 16 + card_h < height {
        my + 16
    } else {
        my.saturating_sub(card_h + 10)
    };

    for y in card_y..(card_y + card_h).min(height) {
        let row = y * width;
        let is_edge_y = y == card_y || y == card_y + card_h - 1;
        for x in card_x..(card_x + card_w).min(width) {
            let is_edge_x = x == card_x || x == card_x + card_w - 1;
            if is_edge_y || is_edge_x {
                buffer[row + x] = 0xFF30363d;
            } else {
                buffer[row + x] = blend_pixel_fast(buffer[row + x], 0xFF161b22, 235);
            }
        }
    }

    draw_text(
        buffer,
        width,
        height,
        card_x + 10,
        card_y + 8,
        title,
        0xFF58a6ff,
    );

    for (i, line) in lines.iter().enumerate() {
        let ly = card_y + 24 + i * 16;
        draw_text(buffer, width, height, card_x + 10, ly, line, 0xFFe6edf3);
    }
}

pub fn parse_hex_color(hex: &str) -> u32 {
    let clean = hex.trim_start_matches('#');
    if let Ok(val) = u32::from_str_radix(clean, 16) {
        if clean.len() <= 6 {
            0xFF000000 | val
        } else {
            val
        }
    } else {
        0xFF58a6ff
    }
}

/// Calculate modal layout coordinates for the Chart Data Inspector
pub fn get_inspector_layout(
    width: usize,
    height: usize,
) -> (Rect, Rect, Rect, Rect, Rect, Rect, Rect, Rect) {
    let max_w = (width as f32 - 20.0).max(300.0);
    let max_h = (height as f32 - 20.0).max(250.0);
    let modal_w = ((width as f32) * 0.90).clamp(300.0, 1240.0).min(max_w);
    let modal_h = ((height as f32) * 0.88).clamp(250.0, 840.0).min(max_h);
    let modal_x = ((width as f32 - modal_w) * 0.5).max(10.0);
    let modal_y = ((height as f32 - modal_h) * 0.5).max(10.0);
    let modal_rect = Rect::new(modal_x, modal_y, modal_w, modal_h);

    let header_rect = Rect::new(modal_x, modal_y, modal_w, 44.0);
    let close_btn = Rect::new(modal_x + modal_w - 74.0, modal_y + 9.0, 64.0, 26.0);
    let export_btn = Rect::new(modal_x + modal_w - 162.0, modal_y + 9.0, 80.0, 26.0);

    let series_strip = Rect::new(modal_x + 12.0, modal_y + 48.0, modal_w - 24.0, 28.0);
    let kpi_strip = Rect::new(modal_x + 12.0, modal_y + 80.0, modal_w - 24.0, 52.0);

    let body_y = modal_y + 138.0;
    let body_h = (modal_h - 138.0 - 28.0).max(60.0);
    let left_w = ((modal_w - 32.0) * 0.55).round();
    let left_pane = Rect::new(modal_x + 12.0, body_y, left_w, body_h);
    let right_pane = Rect::new(
        modal_x + 12.0 + left_w + 8.0,
        body_y,
        (modal_w - 24.0 - left_w - 8.0).max(100.0),
        body_h,
    );

    (
        modal_rect,
        header_rect,
        close_btn,
        export_btn,
        series_strip,
        kpi_strip,
        left_pane,
        right_pane,
    )
}

/// Calculate bounds for series filter chips in series_strip
#[must_use]
pub fn get_series_chip_rects(
    series_strip: Rect,
    series: &[slide_core::chart::SeriesData],
) -> Vec<(usize, Rect)> {
    let mut cur_x = series_strip.x + 56.0;
    let mut chips = Vec::with_capacity(series.len());
    for (s_idx, s) in series.iter().enumerate() {
        let chip_w = (s.name.len().saturating_mul(6).saturating_add(26)).max(50) as f32;
        let chip_rect = Rect::new(cur_x, series_strip.y + 2.0, chip_w, 22.0);
        chips.push((s_idx, chip_rect));
        cur_x += chip_w + 6.0;
    }
    chips
}

/// Hit-test within the Chart Data Inspector modal
pub fn hit_test_chart_inspector(
    state: &ChartInspectorState,
    width: usize,
    height: usize,
    mouse_x: f32,
    mouse_y: f32,
) -> Option<InspectorAction> {
    let (modal_rect, _, close_btn, export_btn, series_strip, _, left_pane, right_pane) =
        get_inspector_layout(width, height);

    if !modal_rect.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::Close);
    }

    if close_btn.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::Close);
    }

    if export_btn.contains(mouse_x, mouse_y) {
        return Some(InspectorAction::ExportCsv);
    }

    // Hit test type switcher tabs
    let types = [
        ("BAR", ChartType::Bar),
        ("LINE", ChartType::Line),
        ("AREA", ChartType::Area),
        ("PIE", ChartType::Pie),
        ("DONUT", ChartType::Donut),
    ];
    let tab_w = 48.0;
    let tab_h = 26.0;
    let tab_gap = 6.0;
    let start_tabs_x = export_btn.x - 10.0 - (types.len() as f32 * (tab_w + tab_gap));
    for (i, &(_, t)) in types.iter().enumerate() {
        let tx = start_tabs_x + i as f32 * (tab_w + tab_gap);
        let tab_rect = Rect::new(tx, close_btn.y, tab_w, tab_h);
        if tab_rect.contains(mouse_x, mouse_y) {
            return Some(InspectorAction::SetType(t));
        }
    }

    // Hit test series filter chips and transform presets
    if series_strip.contains(mouse_x, mouse_y) {
        for (rect, _, t) in get_transform_preset_rects(series_strip) {
            if rect.contains(mouse_x, mouse_y) {
                return Some(InspectorAction::SetTransform(t));
            }
        }

        for (s_idx, chip_rect) in get_series_chip_rects(series_strip, &state.chart_data.series) {
            if chip_rect.contains(mouse_x, mouse_y) {
                return Some(InspectorAction::ToggleSeries(s_idx));
            }
        }
    }

    // Hit test data table rows in right pane
    if right_pane.contains(mouse_x, mouse_y) {
        let header_h = 26.0;
        let row_h = 24.0;
        let table_y = right_pane.y + header_h;
        let visible_rows = ((right_pane.height - header_h - 2.0).max(0.0) / row_h) as usize;
        if mouse_y >= table_y && mouse_y < (right_pane.y + right_pane.height) {
            let row_slot = ((mouse_y - table_y) / row_h) as usize;
            if row_slot < visible_rows {
                let row_idx = row_slot + state.table_scroll;
                if row_idx < state.chart_data.categories.len() {
                    return Some(InspectorAction::SelectCategory(row_idx));
                }
            }
        }
    }

    // Hit test category in left pane visualizer
    if left_pane.contains(mouse_x, mouse_y) {
        let px = left_pane.x + 44.0;
        let pw = left_pane.width - 60.0;
        let py = left_pane.y + 36.0;
        let ph = left_pane.height - 70.0;
        let cat_count = state.chart_data.categories.len().max(1);
        let col_w = pw / cat_count as f32;
        if mouse_x >= px && mouse_x <= (px + pw) && mouse_y >= py && mouse_y <= (py + ph + 20.0) {
            let c_idx = (((mouse_x - px) / col_w) as usize).min(cat_count.saturating_sub(1));
            return Some(InspectorAction::SelectCategory(c_idx));
        }
    }

    None
}

/// Standalone fast raster visualizer for charts (used in Inspector and on-slide override)
pub fn draw_chart_visualizer(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    chart_data: &ChartData,
    chart_type: ChartType,
    hidden_series: &std::collections::HashSet<usize>,
    hovered_cat: Option<usize>,
    _mouse_pos: Option<(f32, f32)>,
) {
    let rx = rect.x as usize;
    let ry = rect.y as usize;
    let rw = rect.width as usize;
    let rh = rect.height as usize;

    if rw < 60 || rh < 60 || rx >= width || ry >= height {
        return;
    }

    // Dark card background
    fill_rect(buffer, width, height, rx, ry, rw, rh, 0xFF161b22);
    draw_rect_outline(buffer, width, height, rx, ry, rw, rh, 0xFF30363d);

    // Title at top-left
    if let Some(ref title) = chart_data.title {
        draw_text(buffer, width, height, rx + 14, ry + 10, title, 0xFF58a6ff);
    }

    // Filter visible series
    let visible_series: Vec<(usize, &SeriesData)> = chart_data
        .series
        .iter()
        .enumerate()
        .filter(|(idx, _)| !hidden_series.contains(idx))
        .collect();

    // Legend at top right
    let mut cur_legend_x = (rx + rw).saturating_sub(16);
    for (idx, s) in chart_data.series.iter().enumerate().rev() {
        if hidden_series.contains(&idx) {
            continue;
        }
        let color = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[idx % DEFAULT_CHART_COLORS.len()]),
        );
        let text_len = s.name.len() * 6 + 18;
        if cur_legend_x < rx + text_len + 40 {
            break;
        }
        cur_legend_x -= text_len;
        fill_rect(buffer, width, height, cur_legend_x, ry + 11, 8, 8, color);
        draw_text(
            buffer,
            width,
            height,
            cur_legend_x + 12,
            ry + 11,
            &s.name,
            0xFF8b949e,
        );
    }

    if visible_series.is_empty() {
        draw_text_at_center(
            buffer,
            width,
            height,
            rx + rw / 2,
            ry + rh / 2,
            "No visible series",
            0xFF8b949e,
        );
        return;
    }

    match chart_type {
        | ChartType::Pie | ChartType::Donut => {
            let cx = (rx + rw / 2) as isize;
            let cy = (ry + rh / 2 + 8) as isize;
            let radius = ((rw.min(rh) as f32) * 0.36).max(25.0) as isize;
            let inner_r = if chart_type == ChartType::Donut {
                (radius as f32 * 0.52).round() as isize
            } else {
                0
            };

            let (names, values): (Vec<String>, Vec<f64>) = if chart_data.series.len() == 1 {
                let s = &chart_data.series[0];
                let vals: Vec<f64> = (0..chart_data.categories.len())
                    .map(|i| s.values.get(i).copied().unwrap_or(0.0))
                    .collect();
                (chart_data.categories.clone(), vals)
            } else {
                let n: Vec<String> = visible_series.iter().map(|(_, s)| s.name.clone()).collect();
                let v: Vec<f64> = visible_series
                    .iter()
                    .map(|(_, s)| s.values.first().copied().unwrap_or(0.0))
                    .collect();
                (n, v)
            };

            let total: f64 = values.iter().sum();
            if total > 0.0 {
                let mut slice_angles = Vec::new();
                let mut current_ang = 0.0f32;
                for (i, &v) in values.iter().enumerate() {
                    let frac = (v / total) as f32;
                    let slice_ang = frac * std::f32::consts::TAU;
                    let col = parse_hex_color(DEFAULT_CHART_COLORS[i % DEFAULT_CHART_COLORS.len()]);
                    slice_angles.push((current_ang, current_ang + slice_ang, col));
                    current_ang += slice_ang;
                }

                let r_sq = radius * radius;
                let in_sq = inner_r * inner_r;
                for dy in -radius..=radius {
                    let py = cy + dy;
                    if py < 0 || py as usize >= height {
                        continue;
                    }
                    let row = (py as usize) * width;
                    for dx in -radius..=radius {
                        let px = cx + dx;
                        if px < 0 || px as usize >= width {
                            continue;
                        }
                        let d_sq = dx * dx + dy * dy;
                        if d_sq >= in_sq && d_sq <= r_sq {
                            let mut ang = (dy as f32).atan2(dx as f32);
                            if ang < 0.0 {
                                ang += std::f32::consts::TAU;
                            }
                            for (s_i, &(a1, a2, col)) in slice_angles.iter().enumerate() {
                                if ang >= a1 && ang <= a2 {
                                    let final_col = if hovered_cat == Some(s_i) {
                                        blend_pixel_fast(col, 0xFFFFFFFF, 60)
                                    } else {
                                        col
                                    };
                                    buffer[row + px as usize] = final_col;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if chart_type == ChartType::Donut {
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    (cy - 7) as usize,
                    "TOTAL",
                    0xFF8b949e,
                );
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    (cy + 4) as usize,
                    &ChartData::format_value(total),
                    0xFFFFFFFF,
                );
            }

            // Hover tooltip for slice
            if let Some(cat_idx) = hovered_cat
                && cat_idx < names.len()
            {
                let val = values.get(cat_idx).copied().unwrap_or(0.0);
                let pct = if total > 0.0 {
                    (val / total) * 100.0
                } else {
                    0.0
                };
                let tip_text = format!(
                    "{}: {} ({:.1}%)",
                    names[cat_idx],
                    ChartData::format_value(val),
                    pct
                );
                let tip_y = (cy as usize) + radius as usize + 12;
                if tip_y < height {
                    draw_text_at_center(
                        buffer,
                        width,
                        height,
                        cx as usize,
                        tip_y,
                        &tip_text,
                        0xFF58a6ff,
                    );
                }
            }
        },
        | ChartType::Bar | ChartType::Line | ChartType::Area | ChartType::Scatter => {
            let px = rx + 44;
            let py = ry + 36;
            let pw = rw.saturating_sub(60);
            let ph = rh.saturating_sub(70);

            if pw < 20 || ph < 20 {
                return;
            }

            // Find min/max across visible series
            let mut y_min = 0.0f64;
            let mut y_max = 0.0f64;
            for (_, s) in &visible_series {
                for &v in &s.values {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                }
            }
            if y_max <= y_min {
                y_max = y_min + 1.0;
            }

            // Draw Y-axis gridlines and labels
            for step in 0..=4 {
                let tick_val = y_min + (step as f64 / 4.0) * (y_max - y_min);
                let tick_y = (py + ph).saturating_sub(((step as f32 / 4.0) * ph as f32) as usize);
                draw_dashed_hline(buffer, width, height, px, px + pw, tick_y, 0xFF21262d, 3, 3);
                let tick_str = ChartData::format_value(tick_val);
                let label_x = px.saturating_sub(tick_str.len() * 6 + 6);
                draw_text(
                    buffer,
                    width,
                    height,
                    label_x,
                    tick_y.saturating_sub(3),
                    &tick_str,
                    0xFF8b949e,
                );
            }

            // Baseline
            fill_rect(buffer, width, height, px, py + ph, pw, 1, 0xFF30363d);

            let cat_count = chart_data.categories.len().max(1);
            let col_w = pw as f32 / cat_count as f32;

            // X-axis category labels
            for (c_idx, cat) in chart_data.categories.iter().enumerate() {
                let cx = px as f32 + (c_idx as f32 + 0.5) * col_w;
                draw_text_at_center(
                    buffer,
                    width,
                    height,
                    cx as usize,
                    py + ph + 8,
                    cat,
                    0xFF8b949e,
                );
            }

            // Highlight hovered column
            if let Some(c_idx) = hovered_cat {
                let hx = (px as f32 + c_idx as f32 * col_w) as usize;
                fill_rect_alpha(
                    buffer,
                    width,
                    height,
                    hx,
                    py,
                    col_w as usize,
                    ph,
                    0xFF388bfd,
                    35,
                );
            }

            if chart_type == ChartType::Bar {
                let group_pad = col_w * 0.15;
                let bar_w = ((col_w - group_pad * 2.0) / visible_series.len() as f32).max(2.0);

                for (s_order, (s_idx, s)) in visible_series.iter().enumerate() {
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    for (c_idx, &val) in s.values.iter().enumerate() {
                        let val_h = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                            * ph as f32)
                            .clamp(0.0, ph as f32);
                        let bx =
                            (px as f32 + c_idx as f32 * col_w + group_pad + s_order as f32 * bar_w)
                                as usize;
                        let by = (py + ph).saturating_sub(val_h as usize);
                        fill_rect(
                            buffer,
                            width,
                            height,
                            bx,
                            by,
                            (bar_w as usize).saturating_sub(1).max(1),
                            val_h as usize,
                            col,
                        );
                        if hovered_cat == Some(c_idx) {
                            // Bright cap on active bar
                            fill_rect(
                                buffer,
                                width,
                                height,
                                bx,
                                by,
                                (bar_w as usize).saturating_sub(1).max(1),
                                2,
                                0xFFFFFFFF,
                            );
                        }
                    }
                }
            } else {
                for (s_idx, s) in &visible_series {
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    let mut pts: Vec<(usize, usize)> = Vec::with_capacity(s.values.len());

                    for (c_idx, &val) in s.values.iter().enumerate() {
                        let val_h = (((val - y_min) / (y_max - y_min).max(1e-6)) as f32
                            * ph as f32)
                            .clamp(0.0, ph as f32);
                        let pt_x = (px as f32 + (c_idx as f32 + 0.5) * col_w) as usize;
                        let pt_y = (py + ph).saturating_sub(val_h as usize);
                        pts.push((pt_x, pt_y));
                    }

                    if chart_type == ChartType::Area {
                        for window in pts.windows(2) {
                            let (x0, y0) = window[0];
                            let (x1, y1) = window[1];
                            let dx = (x1 as isize - x0 as isize).max(1) as f32;
                            for x in x0..x1 {
                                let t = (x - x0) as f32 / dx;
                                let y = (y0 as f32 + t * (y1 as f32 - y0 as f32)) as usize;
                                for py_fill in y..(py + ph) {
                                    if py_fill < height && x < width {
                                        buffer[py_fill * width + x] =
                                            blend_pixel_fast(buffer[py_fill * width + x], col, 65);
                                    }
                                }
                            }
                        }
                    }

                    // Line segments
                    if chart_type != ChartType::Scatter {
                        for window in pts.windows(2) {
                            draw_thick_line(
                                buffer,
                                width,
                                height,
                                window[0].0 as isize,
                                window[0].1 as isize,
                                window[1].0 as isize,
                                window[1].1 as isize,
                                1,
                                col,
                            );
                        }
                    }

                    // Data markers
                    for &(pt_x, pt_y) in &pts {
                        draw_disk(buffer, width, height, pt_x as isize, pt_y as isize, 3, col);
                        draw_disk(
                            buffer,
                            width,
                            height,
                            pt_x as isize,
                            pt_y as isize,
                            1,
                            0xFFFFFFFF,
                        );
                    }
                }
            }

            // If a category is hovered, draw a glassmorphic category card
            if let Some(cat_idx) = hovered_cat
                && cat_idx < chart_data.categories.len()
            {
                let cat_name = &chart_data.categories[cat_idx];
                let card_w = 160usize;
                let card_h = 24 + visible_series.len() * 16;
                let card_x = px + 12;
                let card_y = py + 12;

                fill_rect_alpha(
                    buffer, width, height, card_x, card_y, card_w, card_h, 0xFF0d1117, 230,
                );
                draw_rect_outline(
                    buffer, width, height, card_x, card_y, card_w, card_h, 0xFF30363d,
                );
                draw_text(
                    buffer,
                    width,
                    height,
                    card_x + 8,
                    card_y + 6,
                    cat_name,
                    0xFF58a6ff,
                );

                for (i, (s_idx, s)) in visible_series.iter().enumerate() {
                    let sy = card_y + 22 + i * 16;
                    let col = parse_hex_color(
                        s.color
                            .as_deref()
                            .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
                    );
                    fill_rect(buffer, width, height, card_x + 8, sy + 3, 6, 6, col);
                    let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
                    let line = format!("{}: {}", s.name, ChartData::format_value(val));
                    draw_text(buffer, width, height, card_x + 18, sy, &line, 0xFFe6edf3);
                }
            }
        },
    }
}

/// Render the complete, responsive HUD-grade Chart Data Inspector modal
pub fn draw_chart_inspector(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    state: &ChartInspectorState,
    mouse_x: f32,
    mouse_y: f32,
) {
    let (
        modal_rect,
        header_rect,
        close_btn,
        export_btn,
        series_strip,
        kpi_strip,
        left_pane,
        right_pane,
    ) = get_inspector_layout(width, height);

    // 1. Fullscreen dark glass backdrop
    fill_rect_alpha(buffer, width, height, 0, 0, width, height, 0x00000000, 180);

    // 2. Main modal dialog body
    fill_rect(
        buffer,
        width,
        height,
        modal_rect.x as usize,
        modal_rect.y as usize,
        modal_rect.width as usize,
        modal_rect.height as usize,
        0xFF0d1117,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        modal_rect.x as usize,
        modal_rect.y as usize,
        modal_rect.width as usize,
        modal_rect.height as usize,
        0xFF30363d,
    );

    // 3. Header Bar
    fill_rect(
        buffer,
        width,
        height,
        header_rect.x as usize,
        header_rect.y as usize,
        header_rect.width as usize,
        header_rect.height as usize,
        0xFF161b22,
    );
    fill_rect(
        buffer,
        width,
        height,
        header_rect.x as usize,
        (header_rect.y + header_rect.height - 1.0) as usize,
        header_rect.width as usize,
        1,
        0xFF30363d,
    );

    // Header Title
    draw_text(
        buffer,
        width,
        height,
        (header_rect.x + 14.0) as usize,
        (header_rect.y + 14.0) as usize,
        "DATA INSPECTOR",
        0xFF38bdf8,
    );
    if let Some(ref title) = state.chart_data.title {
        let title_str = format!("| {}", title);
        draw_text(
            buffer,
            width,
            height,
            (header_rect.x + 115.0) as usize,
            (header_rect.y + 14.0) as usize,
            &title_str,
            0xFFc9d1d9,
        );
    }

    // Header Type Switcher Tabs
    let types = [
        ("BAR", ChartType::Bar),
        ("LINE", ChartType::Line),
        ("AREA", ChartType::Area),
        ("PIE", ChartType::Pie),
        ("DONUT", ChartType::Donut),
    ];
    let tab_w = 48.0;
    let tab_h = 26.0;
    let tab_gap = 6.0;
    let start_tabs_x = export_btn.x - 10.0 - (types.len() as f32 * (tab_w + tab_gap));

    for (i, &(label, t)) in types.iter().enumerate() {
        let tx = start_tabs_x + i as f32 * (tab_w + tab_gap);
        let tab_rect = Rect::new(tx, close_btn.y, tab_w, tab_h);
        let is_active = state.active_type == t;
        let is_hovered = tab_rect.contains(mouse_x, mouse_y);

        let bg = if is_active {
            0xFF1f6feb
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF161b22
        };
        let border = if is_active {
            0xFF58a6ff
        } else if is_hovered {
            0xFF8b949e
        } else {
            0xFF30363d
        };
        let text_col = if is_active {
            0xFFFFFFFF
        } else {
            0xFF8b949e
        };

        fill_rect(
            buffer,
            width,
            height,
            tx as usize,
            tab_rect.y as usize,
            tab_w as usize,
            tab_h as usize,
            bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            tx as usize,
            tab_rect.y as usize,
            tab_w as usize,
            tab_h as usize,
            border,
        );
        let text_x = (tx + (tab_w - label.len() as f32 * 6.0) * 0.5) as usize;
        let text_y = (tab_rect.y + 9.0) as usize;
        draw_text(buffer, width, height, text_x, text_y, label, text_col);
    }

    // Export CSV Button
    let is_export_hover = export_btn.contains(mouse_x, mouse_y);
    let export_bg = if is_export_hover {
        0xFF2ea043
    } else {
        0xFF238636
    };
    fill_rect(
        buffer,
        width,
        height,
        export_btn.x as usize,
        export_btn.y as usize,
        export_btn.width as usize,
        export_btn.height as usize,
        export_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        export_btn.x as usize,
        export_btn.y as usize,
        export_btn.width as usize,
        export_btn.height as usize,
        0xFF3fb950,
    );
    draw_text_centered(buffer, width, height, export_btn, "EXPORT CSV", 0xFFFFFFFF);

    // Close Button
    let is_close_hover = close_btn.contains(mouse_x, mouse_y);
    let close_bg = if is_close_hover {
        0xFFf85149
    } else {
        0xFFda3633
    };
    fill_rect(
        buffer,
        width,
        height,
        close_btn.x as usize,
        close_btn.y as usize,
        close_btn.width as usize,
        close_btn.height as usize,
        close_bg,
    );
    draw_rect_outline(
        buffer,
        width,
        height,
        close_btn.x as usize,
        close_btn.y as usize,
        close_btn.width as usize,
        close_btn.height as usize,
        0xFFf85149,
    );
    draw_text_centered(buffer, width, height, close_btn, "CLOSE [X]", 0xFFFFFFFF);

    // 4. Series Filter Strip
    draw_text(
        buffer,
        width,
        height,
        series_strip.x as usize,
        (series_strip.y + 7.0) as usize,
        "SERIES:",
        0xFF8b949e,
    );
    for (s_idx, chip_rect) in get_series_chip_rects(series_strip, &state.chart_data.series) {
        let s = &state.chart_data.series[s_idx];
        let chip_w = chip_rect.width;
        let is_hidden = state.hidden_series.contains(&s_idx);
        let is_hovered = chip_rect.contains(mouse_x, mouse_y);

        let chip_bg = if is_hidden {
            0xFF161b22
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF1f242c
        };
        let chip_border = if is_hidden {
            0xFF21262d
        } else if is_hovered {
            0xFF58a6ff
        } else {
            0xFF30363d
        };
        let col = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
        );

        fill_rect(
            buffer,
            width,
            height,
            chip_rect.x as usize,
            chip_rect.y as usize,
            chip_w as usize,
            chip_rect.height as usize,
            chip_bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            chip_rect.x as usize,
            chip_rect.y as usize,
            chip_w as usize,
            chip_rect.height as usize,
            chip_border,
        );

        // Status circle dot
        let dot_col = if is_hidden { 0xFF484f58 } else { col };
        draw_disk(
            buffer,
            width,
            height,
            (chip_rect.x + 8.0) as isize,
            (chip_rect.y + 11.0) as isize,
            3,
            dot_col,
        );

        // Series label
        let text_col = if is_hidden {
            0xFF6e7681
        } else {
            0xFFe6edf3
        };
        draw_text(
            buffer,
            width,
            height,
            (chip_rect.x + 16.0) as usize,
            (chip_rect.y + 7.0) as usize,
            &s.name,
            text_col,
        );
    }

    // Transform Presets (right-aligned in series_strip)
    for (rect, label, t) in get_transform_preset_rects(series_strip) {
        let is_active = state.active_transform == t;
        let is_hovered = rect.contains(mouse_x, mouse_y);
        let bg = if is_active {
            0xFF1f6feb
        } else if is_hovered {
            0xFF21262d
        } else {
            0xFF161b22
        };
        let border = if is_active {
            0xFF58a6ff
        } else if is_hovered {
            0xFF8b949e
        } else {
            0xFF30363d
        };
        let text_col = if is_active {
            0xFFFFFFFF
        } else {
            0xFF8b949e
        };

        fill_rect(
            buffer,
            width,
            height,
            rect.x as usize,
            rect.y as usize,
            rect.width as usize,
            rect.height as usize,
            bg,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            rect.x as usize,
            rect.y as usize,
            rect.width as usize,
            rect.height as usize,
            border,
        );
        draw_text_centered(buffer, width, height, rect, label, text_col);
    }

    // 5. KPI Stat Cards Strip
    let stats = state.chart_data.summary_stats(&state.hidden_series);
    let kpi_w = (kpi_strip.width - 24.0) / 4.0;
    let kpis = [
        (
            "TOTAL SUM",
            ChartData::format_value(stats.total_sum),
            0xFF38bdf8,
        ),
        ("AVERAGE", format!("{:.1}", stats.avg), 0xFF34d399),
        (
            "PEAK / MAX",
            if stats.max_cat.is_empty() {
                ChartData::format_value(stats.max_val)
            } else {
                format!(
                    "{} ({})",
                    ChartData::format_value(stats.max_val),
                    stats.max_cat
                )
            },
            0xFFf59e0b,
        ),
        (
            "MIN / LOW",
            if stats.min_cat.is_empty() {
                ChartData::format_value(stats.min_val)
            } else {
                format!(
                    "{} ({})",
                    ChartData::format_value(stats.min_val),
                    stats.min_cat
                )
            },
            0xFFa855f7,
        ),
    ];

    for (i, &(kpi_title, ref kpi_val, accent_col)) in kpis.iter().enumerate() {
        let kx = (kpi_strip.x + i as f32 * (kpi_w + 8.0)) as usize;
        let ky = kpi_strip.y as usize;
        let kw = kpi_w as usize;
        let kh = kpi_strip.height as usize;

        fill_rect(buffer, width, height, kx, ky, kw, kh, 0xFF161b22);
        draw_rect_outline(buffer, width, height, kx, ky, kw, kh, 0xFF30363d);
        // Accent bar on top
        fill_rect(buffer, width, height, kx, ky, kw, 2, accent_col);

        draw_text(
            buffer,
            width,
            height,
            kx + 10,
            ky + 8,
            kpi_title,
            0xFF8b949e,
        );
        draw_text(buffer, width, height, kx + 10, ky + 26, kpi_val, 0xFFFFFFFF);
    }

    // 6. Left Pane: Standalone Fast Visualizer
    draw_chart_visualizer(
        buffer,
        width,
        height,
        left_pane,
        &state.chart_data,
        state.active_type,
        &state.hidden_series,
        state.hovered_category,
        Some((mouse_x, mouse_y)),
    );

    // 7. Right Pane: Interactive Synchronized Data Table
    let rx = right_pane.x as usize;
    let ry = right_pane.y as usize;
    let rw = right_pane.width as usize;
    let rh = right_pane.height as usize;

    fill_rect(buffer, width, height, rx, ry, rw, rh, 0xFF161b22);
    draw_rect_outline(buffer, width, height, rx, ry, rw, rh, 0xFF30363d);

    // Table Header
    let cat_col_w = ((rw as f32) * 0.35).max(75.0) as usize;
    let visible_series: Vec<(usize, &SeriesData)> = state
        .chart_data
        .series
        .iter()
        .enumerate()
        .filter(|(idx, _)| !state.hidden_series.contains(idx))
        .collect();
    let num_s = visible_series.len().max(1);
    let s_col_w = (rw.saturating_sub(cat_col_w)) / num_s;

    fill_rect(buffer, width, height, rx, ry, rw, 26, 0xFF21262d);
    fill_rect(buffer, width, height, rx, ry + 25, rw, 1, 0xFF30363d);
    draw_text(
        buffer,
        width,
        height,
        rx + 10,
        ry + 9,
        "CATEGORY",
        0xFF8b949e,
    );

    for (i, (s_idx, s)) in visible_series.iter().enumerate() {
        let col_x = rx + cat_col_w + i * s_col_w;
        let col = parse_hex_color(
            s.color
                .as_deref()
                .unwrap_or(DEFAULT_CHART_COLORS[s_idx % DEFAULT_CHART_COLORS.len()]),
        );
        draw_text(buffer, width, height, col_x + 6, ry + 9, &s.name, col);
    }

    // Table Rows
    let row_h = 24usize;
    let visible_rows = (rh.saturating_sub(28)) / row_h;
    for row_i in 0..visible_rows {
        let cat_idx = state.table_scroll + row_i;
        if cat_idx >= state.chart_data.categories.len() {
            break;
        }
        let row_y = ry + 26 + row_i * row_h;
        let is_hovered = state.hovered_category == Some(cat_idx);

        let row_bg = if is_hovered {
            0xFF1f385c // Electric blue highlight
        } else if cat_idx.is_multiple_of(2) {
            0xFF161b22
        } else {
            0xFF0d1117
        };

        fill_rect(
            buffer,
            width,
            height,
            rx + 1,
            row_y,
            rw.saturating_sub(2),
            row_h - 1,
            row_bg,
        );

        // Category name
        let cat_name = &state.chart_data.categories[cat_idx];
        let text_col = if is_hovered {
            0xFFFFFFFF
        } else {
            0xFFe6edf3
        };
        draw_text(
            buffer,
            width,
            height,
            rx + 10,
            row_y + 8,
            cat_name,
            text_col,
        );

        // Series values
        for (i, (_, s)) in visible_series.iter().enumerate() {
            let col_x = rx + cat_col_w + i * s_col_w;
            let val = s.values.get(cat_idx).copied().unwrap_or(0.0);
            let val_str = ChartData::format_value(val);
            draw_text(
                buffer,
                width,
                height,
                col_x + 6,
                row_y + 8,
                &val_str,
                text_col,
            );
        }
    }

    // 8. Footer Keyboard Shortcuts
    let footer_text = "[Esc] Close   [Tab] Switch Type   [1-9] Toggle Series   [C] Export CSV   [Wheel] Scroll Table";
    draw_text(
        buffer,
        width,
        height,
        (modal_rect.x + 14.0) as usize,
        (modal_rect.y + modal_rect.height - 18.0) as usize,
        footer_text,
        0xFF8b949e,
    );

    // 9. Toast Notification HUD Pill
    if let Some((ref msg, ref t)) = state.toast_message
        && t.elapsed() < std::time::Duration::from_millis(2500)
    {
        let toast_w = (msg.len() * 6 + 32).max(180) as f32;
        let toast_x = modal_rect.x + (modal_rect.width - toast_w) * 0.5;
        let toast_y = modal_rect.y + 10.0;
        let toast_rect = Rect::new(toast_x, toast_y, toast_w, 24.0);

        fill_rect(
            buffer,
            width,
            height,
            toast_x as usize,
            toast_y as usize,
            toast_w as usize,
            24,
            0xFF238636,
        );
        draw_rect_outline(
            buffer,
            width,
            height,
            toast_x as usize,
            toast_y as usize,
            toast_w as usize,
            24,
            0xFF3fb950,
        );
        draw_text_centered(buffer, width, height, toast_rect, msg, 0xFFFFFFFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slide_core::chart::ChartData;
    use slide_core::chart::ChartType;
    use slide_core::chart::SeriesData;

    fn make_sample_chart() -> ChartData {
        let mut chart = ChartData::new(ChartType::Bar);
        chart.title = Some("Quarterly Performance".into());
        chart.categories = vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()];
        chart.series.push(SeriesData {
            name: "Revenue".into(),
            values: vec![120.0, 200.0, 150.0, 310.0],
            color: Some("#38bdf8".into()),
        });
        chart.series.push(SeriesData {
            name: "Profit".into(),
            values: vec![40.0, 80.0, 60.0, 140.0],
            color: Some("#34d399".into()),
        });
        chart
    }

    #[test]
    fn test_chart_inspector_state() {
        let chart = make_sample_chart();
        let mut inspector = ChartInspectorState::new(chart, ChartType::Bar);

        assert_eq!(inspector.active_type, ChartType::Bar);
        inspector.cycle_type();
        assert_eq!(inspector.active_type, ChartType::Line);
        inspector.cycle_type();
        assert_eq!(inspector.active_type, ChartType::Area);

        // Toggle series
        inspector.toggle_series(1);
        assert!(inspector.hidden_series.contains(&1));
        inspector.toggle_series(1);
        assert!(!inspector.hidden_series.contains(&1));

        // Toast
        inspector.show_toast("Testing Toast");
        assert!(inspector.toast_message.is_some());
    }

    #[test]
    fn test_chart_inspector_hit_test() {
        let chart = make_sample_chart();
        let inspector = ChartInspectorState::new(chart, ChartType::Bar);
        let width = 1280;
        let height = 720;

        let (_modal_rect, _, close_btn, export_btn, _, _, _, _) =
            get_inspector_layout(width, height);

        // Click outside modal
        let outside_action = hit_test_chart_inspector(&inspector, width, height, 5.0, 5.0);
        assert_eq!(outside_action, Some(InspectorAction::Close));

        // Click close button
        let close_action = hit_test_chart_inspector(
            &inspector,
            width,
            height,
            close_btn.x + close_btn.width * 0.5,
            close_btn.y + close_btn.height * 0.5,
        );
        assert_eq!(close_action, Some(InspectorAction::Close));

        // Click export button
        let export_action = hit_test_chart_inspector(
            &inspector,
            width,
            height,
            export_btn.x + export_btn.width * 0.5,
            export_btn.y + export_btn.height * 0.5,
        );
        assert_eq!(export_action, Some(InspectorAction::ExportCsv));
    }

    #[test]
    fn test_chart_visualizer_rendering_all_types() {
        let chart = make_sample_chart();
        let width = 800;
        let height = 600;
        let mut buffer = vec![0u32; width * height];
        let rect = Rect::new(50.0, 50.0, 700.0, 500.0);
        let empty_set = std::collections::HashSet::new();

        for &t in &[
            ChartType::Bar,
            ChartType::Line,
            ChartType::Area,
            ChartType::Pie,
            ChartType::Donut,
        ] {
            draw_chart_visualizer(
                &mut buffer,
                width,
                height,
                rect,
                &chart,
                t,
                &empty_set,
                Some(1),
                Some((200.0, 200.0)),
            );
        }
    }

    #[test]
    fn test_draw_chart_inspector_full() {
        let chart = make_sample_chart();
        let mut inspector = ChartInspectorState::new(chart, ChartType::Bar);
        inspector.show_toast("Export Complete");

        let width = 1024;
        let height = 768;
        let mut buffer = vec![0u32; width * height];

        draw_chart_inspector(&mut buffer, width, height, &inspector, 500.0, 400.0);
    }
}

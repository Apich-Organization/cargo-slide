use slide_core::animation::SlideSurface;
use slide_core::error::Result;
use slide_core::error::SlideError;
use usvg::Options;
use usvg::Tree;

pub struct SvgRenderer {
    options: Options<'static>,
}

impl Default for SvgRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgRenderer {
    pub fn new() -> Self {
        let mut opt = Options::default();
        opt.dpi = 72.0; // Typst SVG coordinates are in 72 DPI points (pt)
        opt.fontdb_mut().load_system_fonts();
        Self { options: opt }
    }

    /// Render an SVG string into a `SlideSurface` with specified dimensions, maintaining aspect ratio
    pub fn render_svg(
        &self,
        svg_content: &str,
        target_width: usize,
        target_height: usize,
    ) -> Result<(SlideSurface, RenderMetrics)> {
        let sanitized_svg = slide_core::svg::sanitize_svg(svg_content);
        let tree = Tree::from_str(&sanitized_svg, &self.options)
            .map_err(|e| SlideError::SvgParse(format!("usvg error: {}", e)))?;

        let (vb_x, vb_y, vb_w, vb_h) = match slide_core::svg::parse_svg_slide(svg_content) {
            | Ok(info) => {
                (
                    info.view_box.x,
                    info.view_box.y,
                    info.view_box.width,
                    info.view_box.height,
                )
            },
            | Err(_) => (0.0, 0.0, tree.size().width(), tree.size().height()),
        };

        let svg_w = if vb_w > 0.0 {
            vb_w
        } else {
            tree.size().width()
        };
        let svg_h = if vb_h > 0.0 {
            vb_h
        } else {
            tree.size().height()
        };

        if target_width == 0 || target_height == 0 || svg_w <= 0.0 || svg_h <= 0.0 {
            return Ok((
                SlideSurface::blank(target_width.max(1), target_height.max(1), 0xFF000000),
                RenderMetrics::default(),
            ));
        }

        // Calculate aspect-preserving scale and letterbox offset
        let scale_x = target_width as f32 / svg_w;
        let scale_y = target_height as f32 / svg_h;
        let scale = scale_x.min(scale_y);

        let content_w = (svg_w * scale).round() as usize;
        let content_h = (svg_h * scale).round() as usize;

        let offset_x = (target_width.saturating_sub(content_w)) / 2;
        let offset_y = (target_height.saturating_sub(content_h)) / 2;

        let mut pixmap =
            tiny_skia::Pixmap::new(content_w.max(1) as u32, content_h.max(1) as u32)
                .ok_or_else(|| SlideError::Format("Failed to allocate pixmap".to_string()))?;

        // Render SVG to pixmap with scaling
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // Allocate surface buffer for full window and blit pixmap with letterbox padding
        let mut surface_pixels = vec![0xFF0f111a; target_width * target_height];
        let pixmap_data = pixmap.data();

        for cy in 0..content_h {
            let win_y = offset_y + cy;
            if win_y >= target_height {
                break;
            }
            let win_row_start = win_y * target_width;
            let pix_row_start = cy * content_w * 4;

            for cx in 0..content_w {
                let win_x = offset_x + cx;
                if win_x >= target_width {
                    break;
                }
                let idx = pix_row_start + cx * 4;
                let r = pixmap_data[idx] as u32;
                let g = pixmap_data[idx + 1] as u32;
                let b = pixmap_data[idx + 2] as u32;
                let a = pixmap_data[idx + 3] as u32;

                // Pack ARGB
                surface_pixels[win_row_start + win_x] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }

        let surface = SlideSurface::new(target_width, target_height, surface_pixels);
        let metrics = RenderMetrics {
            scale,
            offset_x: offset_x as f32,
            offset_y: offset_y as f32,
            content_width: content_w as f32,
            content_height: content_h as f32,
            view_box_x: vb_x,
            view_box_y: vb_y,
        };

        Ok((surface, metrics))
    }
}

pub use slide_core::model::RenderMetrics;

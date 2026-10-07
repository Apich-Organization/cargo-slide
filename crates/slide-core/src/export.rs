//! Standalone presentation export utilities (HTML, handouts, offline players).

use crate::error::Result;
use crate::model::SlideDeck;
use std::fs::create_dir_all;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;

/// Escape HTML special characters
#[must_use]
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Generate a complete, standalone, responsive single-file HTML presentation player
#[must_use]
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn generate_standalone_html(
    deck: &SlideDeck,
    selected_pages: Option<&[usize]>,
) -> String {
    use std::fmt::Write as _;
    let mut html = String::new();
    let title = html_escape(&deck.title);

    let default_pages: Vec<usize> = deck.slides.iter().map(|s| s.page_number).collect();
    let pages = selected_pages.unwrap_or(&default_pages);
    let total_selected = pages.len();

    let _ = writeln!(
        html,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<title>{title}</title>"
    );
    let _ = writeln!(
        html,
        "<style>\n* {{ box-sizing: border-box; margin: 0; padding: 0; }}\nbody {{ background: #0b0f19; color: #f0f6fc; font-family: system-ui, -apple-system, sans-serif; overflow: hidden; height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; }}\n#stage {{ position: relative; width: 100vw; height: 100vh; display: flex; align-items: center; justify-content: center; background: #000; }}\n.slide-viewport {{ position: relative; width: 100%; height: 100%; max-width: 177.78vh; max-height: 56.25vw; display: flex; align-items: center; justify-content: center; }}\n.slide {{ display: none; width: 100%; height: 100%; }}\n.slide.active {{ display: flex; align-items: center; justify-content: center; }}\n.slide svg {{ width: 100%; height: 100%; object-fit: contain; display: block; }}\n#dock {{ position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%); background: rgba(18, 24, 38, 0.85); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.1); border-radius: 30px; padding: 6px 14px; display: flex; align-items: center; gap: 8px; z-index: 100; transition: opacity 0.3s; box-shadow: 0 10px 30px rgba(0,0,0,0.5); }}\n#dock:hover, #dock.active {{ opacity: 1; }}\n.dock-btn {{ background: transparent; border: none; color: #c9d1d9; font-size: 15px; padding: 6px 10px; border-radius: 8px; cursor: pointer; transition: all 0.2s; }}\n.dock-btn:hover {{ background: rgba(255,255,255,0.1); color: #fff; }}\n.counter {{ font-size: 13px; font-weight: 600; padding: 0 8px; color: #58a6ff; }}\n#progress-bar {{ position: fixed; bottom: 0; left: 0; height: 3px; background: #58a6ff; transition: width 0.3s ease; z-index: 200; }}\n.modal {{ display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.7); backdrop-filter: blur(8px); z-index: 300; align-items: center; justify-content: center; }}\n.modal.open {{ display: flex; }}\n.modal-card {{ background: #161b22; border: 1px solid rgba(255,255,255,0.15); border-radius: 12px; padding: 24px; max-width: 600px; width: 90%; max-height: 80vh; overflow-y: auto; color: #f0f6fc; box-shadow: 0 20px 50px rgba(0,0,0,0.8); }}\n.modal-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 10px; }}\n.close-btn {{ background: none; border: none; color: #8b949e; font-size: 18px; cursor: pointer; }}\n.close-btn:hover {{ color: #fff; }}\n.notes-text {{ white-space: pre-wrap; line-height: 1.6; font-size: 15px; background: rgba(0,0,0,0.3); padding: 14px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.06); }}\n.grid-container {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 12px; max-height: 60vh; overflow-y: auto; }}\n.grid-item {{ border: 1px solid rgba(255,255,255,0.1); border-radius: 6px; padding: 6px; cursor: pointer; text-align: center; transition: all 0.2s; background: rgba(255,255,255,0.02); }}\n.grid-item:hover, .grid-item.active {{ border-color: #58a6ff; background: rgba(88,166,255,0.1); }}\n.blank-screen {{ display: none; position: fixed; inset: 0; z-index: 250; }}\n.blank-screen.black {{ background: #000; }}\n.blank-screen.white {{ background: #fff; }}\n.blank-screen.active {{ display: block; }}\n</style>\n</head>\n<body>"
    );

    let _ = writeln!(html, "<div id=\"stage\">\n  <div class=\"slide-viewport\">");
    for (i, slide) in deck
        .slides
        .iter()
        .filter(|s| pages.contains(&s.page_number))
        .enumerate()
    {
        let active_cls = if i == 0 { " active" } else { "" };
        let _ = writeln!(
            html,
            "    <section class=\"slide{active_cls}\" data-page=\"{}\" id=\"slide-{}\">",
            slide.page_number,
            i + 1
        );
        let _ = writeln!(html, "{}", slide.svg_data);
        let _ = writeln!(html, "    </section>");
    }
    let _ = writeln!(html, "  </div>\n</div>");

    let _ = writeln!(
        html,
        "<div id=\"dock\">\n  <button class=\"dock-btn\" onclick=\"jump(0)\" title=\"First (Home)\">⏮</button>\n  <button class=\"dock-btn\" onclick=\"prev()\" title=\"Previous (Left/Backspace)\">◀</button>\n  <span class=\"counter\" id=\"counter\">1 / {total_selected}</span>\n  <button class=\"dock-btn\" onclick=\"next()\" title=\"Next (Right/Space)\">▶</button>\n  <button class=\"dock-btn\" onclick=\"jump({})\">⏭</button>\n  <button class=\"dock-btn\" onclick=\"toggleNotes()\" title=\"Notes (N)\">📝</button>\n  <button class=\"dock-btn\" onclick=\"toggleGrid()\" title=\"Grid (G)\">▦</button>\n  <button class=\"dock-btn\" onclick=\"toggleFullscreen()\" title=\"Fullscreen (F)\">⛶</button>\n  <button class=\"dock-btn\" onclick=\"toggleHelp()\" title=\"Help (?)\">?</button>\n</div>",
        total_selected.saturating_sub(1)
    );

    let _ = writeln!(
        html,
        "<div id=\"progress-bar\" style=\"width: {}%\"></div>",
        100.0 / total_selected.max(1) as f32
    );
    let _ = writeln!(html, "<div id=\"blank\" class=\"blank-screen\"></div>");

    // Speaker Notes Modal
    let _ = writeln!(
        html,
        "<div id=\"notes-modal\" class=\"modal\" onclick=\"if(event.target===this)toggleNotes()\">\n  <div class=\"modal-card\">\n    <div class=\"modal-header\">\n      <h3>📝 Speaker Notes</h3>\n      <button class=\"close-btn\" onclick=\"toggleNotes()\">✕</button>\n    </div>\n    <div class=\"notes-text\" id=\"notes-content\">No notes</div>\n  </div>\n</div>"
    );

    // Grid Overview Modal
    let _ = writeln!(
        html,
        "<div id=\"grid-modal\" class=\"modal\" onclick=\"if(event.target===this)toggleGrid()\">\n  <div class=\"modal-card\" style=\"max-width: 800px;\">\n    <div class=\"modal-header\">\n      <h3>▦ Slide Overview</h3>\n      <button class=\"close-btn\" onclick=\"toggleGrid()\">✕</button>\n    </div>\n    <div class=\"grid-container\" id=\"grid-content\"></div>\n  </div>\n</div>"
    );

    // Help Modal
    let _ = writeln!(
        html,
        "<div id=\"help-modal\" class=\"modal\" onclick=\"if(event.target===this)toggleHelp()\">\n  <div class=\"modal-card\">\n    <div class=\"modal-header\">\n      <h3>? Keyboard Shortcuts</h3>\n      <button class=\"close-btn\" onclick=\"toggleHelp()\">✕</button>\n    </div>\n    <table style=\"width: 100%; font-size: 14px; line-height: 2;\">\n      <tr><td><b>Space / → / Enter</b></td><td>Next slide</td></tr>\n      <tr><td><b>← / Backspace</b></td><td>Previous slide</td></tr>\n      <tr><td><b>Home / End</b></td><td>First / Last slide</td></tr>\n      <tr><td><b>F</b></td><td>Toggle Fullscreen</td></tr>\n      <tr><td><b>N</b></td><td>Toggle Speaker Notes</td></tr>\n      <tr><td><b>G / O</b></td><td>Toggle Grid Overview</td></tr>\n      <tr><td><b>B / .</b></td><td>Blank black screen</td></tr>\n      <tr><td><b>W</b></td><td>Blank white screen</td></tr>\n      <tr><td><b>?</b></td><td>Toggle this help</td></tr>\n    </table>\n  </div>\n</div>"
    );

    // Encode speaker notes into JSON array
    let notes_json: Vec<String> = deck
        .slides
        .iter()
        .filter(|s| pages.contains(&s.page_number))
        .map(|s| s.notes.clone().unwrap_or_default())
        .collect();
    let notes_encoded = serde_json::to_string(&notes_json).unwrap_or_else(|_| "[]".to_string());

    // Embedded Script
    let _ = writeln!(
        html,
        "<script>\nconst slides = document.querySelectorAll('.slide');\nconst total = slides.length;\nconst notes = {notes_encoded};\nlet current = 0;\nfunction update() {{\n  slides.forEach((s, i) => s.classList.toggle('active', i === current));\n  document.getElementById('counter').innerText = `${{current + 1}} / ${{total}}`;\n  document.getElementById('progress-bar').style.width = `${{((current + 1) / total) * 100}}%`;\n  const n = notes[current] || 'No speaker notes defined for this slide.';\n  document.getElementById('notes-content').innerText = n;\n  location.hash = `#${{current + 1}}`;\n}}\nfunction next() {{ if (current < total - 1) {{ current++; update(); }} }}\nfunction prev() {{ if (current > 0) {{ current--; update(); }} }}\nfunction jump(idx) {{ if (idx >= 0 && idx < total) {{ current = idx; update(); closeModals(); }} }}\nfunction toggleNotes() {{ document.getElementById('notes-modal').classList.toggle('open'); }}\nfunction toggleGrid() {{\n  const gm = document.getElementById('grid-modal');\n  gm.classList.toggle('open');\n  if (gm.classList.contains('open')) {{\n    const gc = document.getElementById('grid-content');\n    gc.innerHTML = '';\n    slides.forEach((s, idx) => {{\n      const item = document.createElement('div');\n      item.className = 'grid-item' + (idx === current ? ' active' : '');\n      item.innerHTML = `<div><b>Slide ${{idx + 1}}</b></div>`;\n      item.onclick = () => jump(idx);\n      gc.appendChild(item);\n    }});\n  }}\n}}\nfunction toggleHelp() {{ document.getElementById('help-modal').classList.toggle('open'); }}\nfunction closeModals() {{ document.querySelectorAll('.modal').forEach(m => m.classList.remove('open')); }}\nfunction toggleFullscreen() {{ if (!document.fullscreenElement) document.documentElement.requestFullscreen(); else document.exitFullscreen(); }}\nlet blankState = 0;\nfunction toggleBlank(white) {{\n  const b = document.getElementById('blank');\n  if (blankState) {{ b.className = 'blank-screen'; blankState = 0; }}\n  else {{ b.className = 'blank-screen active ' + (white ? 'white' : 'black'); blankState = 1; }}\n}}\nwindow.addEventListener('keydown', (e) => {{\n  if (['ArrowRight', 'Space', 'Enter', 'PageDown'].includes(e.key)) {{ e.preventDefault(); next(); }}\n  else if (['ArrowLeft', 'Backspace', 'PageUp'].includes(e.key)) {{ e.preventDefault(); prev(); }}\n  else if (e.key === 'Home') {{ e.preventDefault(); jump(0); }}\n  else if (e.key === 'End') {{ e.preventDefault(); jump(total - 1); }}\n  else if (e.key === 'f' || e.key === 'F') {{ e.preventDefault(); toggleFullscreen(); }}\n  else if (e.key === 'n' || e.key === 'N') {{ e.preventDefault(); toggleNotes(); }}\n  else if (e.key === 'g' || e.key === 'G' || e.key === 'o' || e.key === 'O') {{ e.preventDefault(); toggleGrid(); }}\n  else if (e.key === 'b' || e.key === 'B' || e.key === '.') {{ e.preventDefault(); toggleBlank(false); }}\n  else if (e.key === 'w' || e.key === 'W') {{ e.preventDefault(); toggleBlank(true); }}\n  else if (e.key === '?') {{ e.preventDefault(); toggleHelp(); }}\n  else if (e.key === 'Escape') {{ closeModals(); if (blankState) toggleBlank(false); }}\n}});\nlet touchStartX = 0;\nwindow.addEventListener('touchstart', e => {{ if (e.touches[0]) touchStartX = e.touches[0].clientX; }});\nwindow.addEventListener('touchend', e => {{\n  if (e.changedTouches[0]) {{\n    const dx = e.changedTouches[0].clientX - touchStartX;\n    if (dx < -50) next(); else if (dx > 50) prev();\n  }}\n}});\nif (location.hash) {{\n  const p = parseInt(location.hash.replace('#', ''));\n  if (!isNaN(p) && p >= 1 && p <= total) current = p - 1;\n}}\nupdate();\n</script>\n</body>\n</html>"
    );

    html
}

/// Export presentation to a standalone single-file HTML presentation player
pub fn export_standalone_html(
    deck: &SlideDeck,
    selected_pages: Option<&[usize]>,
    out_file: &Path,
) -> Result<PathBuf> {
    let html = generate_standalone_html(deck, selected_pages);

    if let Some(parent) = out_file.parent() {
        create_dir_all(parent)?;
    }
    write(out_file, html)?;

    Ok(out_file.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rect;
    use crate::model::Slide;

    #[test]
    fn test_html_escape() {
        assert_eq!(
            html_escape("Rust <Typst> & \"Code\" 'fun'"),
            "Rust &lt;Typst&gt; &amp; &quot;Code&quot; &#39;fun&#39;"
        );
    }

    #[test]
    fn test_generate_standalone_html() {
        let deck = SlideDeck {
            title: "HTML Presentation".to_string(),
            default_animation: "fade".to_string(),
            slides: vec![Slide {
                page_number: 1,
                svg_data: "<svg><text>Title</text></svg>".to_string(),
                view_box: Rect::new(0.0, 0.0, 1920.0, 1080.0),
                hotspots: vec![],
                animation: None,
                steps: vec![],
                notes: Some("Welcome note".to_string()),
            }],
        };

        let html = generate_standalone_html(&deck, None);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("HTML Presentation"));
        assert!(html.contains("<svg><text>Title</text></svg>"));
        assert!(html.contains("Welcome note"));
    }
}

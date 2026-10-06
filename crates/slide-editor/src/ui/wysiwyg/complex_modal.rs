//! Dedicated deep interactive editing modal dialog for complex elements
//! (Tables, Charts, Grids/Columns, Callouts, and Boxes).

use crate::app::Message;
use crate::ui::theme::AppTheme;
use crate::ui::theme::{
    self,
};
use crate::ui::typst_highlighter::TypstHighlightSettings;
use crate::ui::typst_highlighter::TypstHighlighter;
use crate::ui::typst_highlighter::to_typst_format;
use crate::ui::wysiwyg::block_view::extract_bracket_contents;
use crate::ui::wysiwyg::block_view::extract_bracket_spans;
use crate::ui::wysiwyg::block_view::extract_column_items;
use crate::ui::wysiwyg::block_view::extract_table_cells;
use crate::ui::wysiwyg::block_view::parse_callout_data;
use crate::ui::wysiwyg::block_view::parse_chart_data;
use crate::ui::wysiwyg::block_view::parse_color_spec;
use crate::ui::wysiwyg::block_view::parse_column_count;
use crate::ui::wysiwyg::block_view::render_callout_body;
use crate::ui::wysiwyg::block_view::split_args;
use iced::Alignment;
use iced::Background;
use iced::Border;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::border;
use iced::widget::Space;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::scrollable;
use iced::widget::text;
use iced::widget::text_editor;
use iced::widget::text_input;
use std::ops::Range;

/// Active complex element modal state
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexElementModal {
    pub slide_idx: usize,
    pub block_id: String,
    pub range: Range<usize>,
    pub callee: String,
    pub original_raw: String,
    pub state: ComplexModalState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplexModalState {
    Table {
        cols: usize,
        col_spec: String,
        has_header: bool,
        header_cells: Vec<String>,
        rows: Vec<Vec<String>>,
        properties: Vec<String>,
    },
    Chart {
        title: String,
        chart_type: String, // "bar", "line", "area", "pie", "donut", "scatter"
        source: String,     // e.g. "assets/benchmarks.csv", "assets/telemetry.db?query=..."
        sql: String,        // e.g. "SELECT stage, p50, p95 FROM latency_stats"
        dsl: String,        // e.g. "filter throughput > 8000 | sort desc | limit 5"
        format: String,     // e.g. "currency", "percentage", "compact", "standard"
        unit: String,       // e.g. "USD", "ms", "MB"
        prefix: String,     // e.g. "$", "¥"
        height: String,     // e.g. "120pt"
        items: Vec<(String, f32)>,
    },
    Grid {
        cols: usize,
        columns: Vec<String>,
    },
    Callout {
        title: String,
        body: String,
        stroke_color: String,
    },
    BoxBlock {
        content: String,
        fill: String,
        stroke: String,
        radius: String,
        inset: String,
        width: String,
    },
    Link {
        url: String,
        label: String,
    },
    TitleSlide {
        title: String,
        subtitle: String,
        author: String,
        date: String,
        version: String,
        institution: String,
        extra_args: Vec<(String, String)>,
        body: String,
    },
    Badge {
        label: String,
        fill: String,
        text_color: String,
    },
    Video {
        source: String,
        caption: String,
        duration: String,
        quality: String,
        style: String,
        width: String,
    },
    Audio {
        source: String,
        is_player: bool,
        title: String,
        artist: String,
        autoplay: bool,
        loop_playback: bool,
        volume: f32,
        width: String,
    },
}

fn extract_first_string_arg(args: &str) -> Option<String> {
    let trimmed = args.trim().trim_start_matches('(').trim_end_matches(')');
    if let Some(s_idx) = trimmed.find('"') {
        let after = &trimmed[s_idx + 1..];
        if let Some(e_idx) = after.find('"') {
            return Some(after[..e_idx].to_string());
        }
    }
    let first = trimmed
        .split(',')
        .next()?
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

fn extract_arg_str(
    args: &str,
    key: &str,
) -> Option<String> {
    let pattern = format!("{key}:");
    let idx = args.find(&pattern)?;
    let after = args[idx + pattern.len()..].trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end_quote = stripped.find('"')?;
        Some(stripped[..end_quote].to_string())
    } else {
        let end = after.find([',', '\n', ')']).unwrap_or(after.len());
        let val = after[..end].trim().trim_matches('"');
        if val.is_empty() || val == "none" {
            None
        } else {
            Some(val.to_string())
        }
    }
}

type ParsedTableData = (
    usize,
    String,
    bool,
    Vec<String>,
    Vec<Vec<String>>,
    Vec<String>,
);

fn parse_table_data(
    args: &str,
    raw: &str,
) -> ParsedTableData {
    let cols = parse_column_count(args).max(1);
    let mut col_spec = format!("{cols}");
    if let Some(pos) = args.find("columns:") {
        let after = args[pos + 8..].trim_start();
        if after.starts_with('(')
            && let Some(end_paren) = after.find(')')
        {
            col_spec = after[..=end_paren].trim().to_string();
        }
    }

    let mut properties = Vec::new();
    for part in split_args(args) {
        let trimmed = part.trim();
        if let Some((key, _)) = trimmed.split_once(':') {
            let key = key.trim();
            if [
                "stroke",
                "fill",
                "inset",
                "align",
                "gutter",
                "stroke-color",
                "fill-color",
            ]
            .contains(&key)
            {
                properties.push(trimmed.to_string());
            }
        }
    }

    let mut has_header = false;
    let mut header_cells = Vec::new();

    // Check table.header(...)
    if let Some(h_start) = raw.find("table.header(") {
        has_header = true;
        let after_h = &raw[h_start + 13..];
        if let Some(h_end) = after_h.find(')') {
            let h_content = &after_h[..h_end];
            for span in extract_bracket_spans(h_content) {
                let cell_raw = h_content[span]
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim();
                header_cells.push(cell_raw.to_string());
            }
        }
    }

    let cells = extract_table_cells(args);
    let mut rows = Vec::new();

    let body_cells = if has_header && !header_cells.is_empty() && cells.len() >= header_cells.len()
    {
        &cells[header_cells.len()..]
    } else {
        &cells[..]
    };

    if !body_cells.is_empty() {
        for chunk in body_cells.chunks(cols) {
            let mut row_vec: Vec<String> = chunk.iter().map(|s| (*s).to_string()).collect();
            while row_vec.len() < cols {
                row_vec.push(String::new());
            }
            rows.push(row_vec);
        }
    }

    if header_cells.is_empty() && !has_header {
        for i in 0..cols {
            header_cells.push(format!("Header {}", i + 1));
        }
    }

    while header_cells.len() < cols {
        header_cells.push(format!("Header {}", header_cells.len() + 1));
    }

    if rows.is_empty() {
        rows.push(vec![String::new(); cols]);
    }

    (cols, col_spec, has_header, header_cells, rows, properties)
}

impl ComplexElementModal {
    /// Construct modal state by parsing the raw Typst block syntax
    #[must_use]
    pub fn from_raw(
        slide_idx: usize,
        block_id: String,
        range: Range<usize>,
        callee: &str,
        raw: &str,
    ) -> Self {
        let trimmed_callee = callee.trim_start_matches('#');
        let args = if let Some(first_paren) = raw.find('(') {
            let last_paren = raw.rfind(')').unwrap_or(raw.len());
            if first_paren < last_paren {
                &raw[first_paren + 1..last_paren]
            } else {
                raw
            }
        } else {
            raw
        };

        let state = match trimmed_callee {
            | "table" => {
                let (cols, col_spec, has_header, header_cells, rows, properties) =
                    parse_table_data(args, raw);
                ComplexModalState::Table {
                    cols,
                    col_spec,
                    has_header,
                    header_cells,
                    rows,
                    properties,
                }
            },
            | "chart" | "chart-bar" | "chart-pie" | "chart-line" | "plot" => {
                let data = parse_chart_data(args);
                let chart_type = extract_arg_str(args, "type").unwrap_or_else(|| {
                    if trimmed_callee.contains("pie") {
                        "pie".to_string()
                    } else if trimmed_callee.contains("line") || trimmed_callee == "plot" {
                        "line".to_string()
                    } else if trimmed_callee.contains("area") {
                        "area".to_string()
                    } else if trimmed_callee.contains("donut") {
                        "donut".to_string()
                    } else if trimmed_callee.contains("scatter") {
                        "scatter".to_string()
                    } else {
                        "bar".to_string()
                    }
                });

                let title =
                    extract_arg_str(args, "title").unwrap_or_else(|| data.title.to_string());
                let source = extract_arg_str(args, "source").unwrap_or_default();
                let sql = extract_arg_str(args, "sql").unwrap_or_default();
                let dsl = extract_arg_str(args, "dsl").unwrap_or_default();
                let format = extract_arg_str(args, "format").unwrap_or_default();
                let unit = extract_arg_str(args, "unit").unwrap_or_default();
                let prefix = extract_arg_str(args, "prefix").unwrap_or_default();
                let height = extract_arg_str(args, "height").unwrap_or_else(|| "120pt".to_string());

                let items = if data.items.is_empty() {
                    vec![("Item A".to_string(), 40.0), ("Item B".to_string(), 70.0)]
                } else {
                    data.items
                        .into_iter()
                        .map(|(l, v)| (l.to_string(), v))
                        .collect()
                };

                ComplexModalState::Chart {
                    title,
                    chart_type,
                    source,
                    sql,
                    dsl,
                    format,
                    unit,
                    prefix,
                    height,
                    items,
                }
            },
            | "grid" | "cols" | "columns" => {
                let col_items = extract_column_items(raw);
                let columns: Vec<String> = if col_items.is_empty() {
                    let brackets = extract_bracket_contents(args);
                    if brackets.is_empty() {
                        let outer_brackets = extract_bracket_spans(raw);
                        if outer_brackets.is_empty() {
                            vec![
                                "*Column 1*\nContent".to_string(),
                                "*Column 2*\nContent".to_string(),
                            ]
                        } else {
                            outer_brackets
                                .into_iter()
                                .map(|span| {
                                    raw[span]
                                        .trim_start_matches('[')
                                        .trim_end_matches(']')
                                        .trim()
                                        .to_string()
                                })
                                .collect()
                        }
                    } else {
                        brackets.into_iter().map(|s| s.to_string()).collect()
                    }
                } else {
                    col_items
                        .into_iter()
                        .map(|item| item.display_text.to_string())
                        .collect()
                };
                let cols = columns.len().max(1);
                ComplexModalState::Grid { cols, columns }
            },
            | "callout" | "note" | "tip" | "warning" | "info" | "alert" | "quote" => {
                let data = parse_callout_data(trimmed_callee, raw);
                let default_color = match trimmed_callee {
                    | "tip" | "hint" => "slide-colors.accent-cyan",
                    | "warning" | "alert" | "caution" => "slide-colors.accent-orange",
                    | "quote" => "slide-colors.accent-purple",
                    | _ => "slide-colors.accent",
                };
                let stroke_color = data.stroke_color.unwrap_or(default_color).to_string();
                ComplexModalState::Callout {
                    title: data.title.to_string(),
                    body: data.body.to_string(),
                    stroke_color,
                }
            },
            | "link" => {
                let url = extract_first_string_arg(args).unwrap_or_else(|| {
                    let trimmed = args
                        .trim()
                        .trim_matches(|c| c == '(' || c == ')' || c == '"' || c == '\'');
                    if trimmed.is_empty() {
                        "https://cargo-slide.dev".to_string()
                    } else {
                        trimmed.to_string()
                    }
                });
                let label = if let Some(first_bracket) = raw.find('[')
                    && let Some(last_bracket) = raw.rfind(']')
                    && first_bracket < last_bracket
                {
                    raw[first_bracket + 1..last_bracket].trim().to_string()
                } else {
                    url.clone()
                };
                ComplexModalState::Link { url, label }
            },
            | "title-slide" => {
                let title = extract_arg_str(args, "title").unwrap_or_default();
                let subtitle = extract_arg_str(args, "subtitle").unwrap_or_default();
                let author = extract_arg_str(args, "author").unwrap_or_default();
                let date = extract_arg_str(args, "date").unwrap_or_default();
                let version = extract_arg_str(args, "version").unwrap_or_default();
                let institution = extract_arg_str(args, "institution")
                    .or_else(|| extract_arg_str(args, "affiliation"))
                    .or_else(|| extract_arg_str(args, "org"))
                    .unwrap_or_default();

                let mut extra_args = Vec::new();
                for part in args.split(',') {
                    let trimmed = part.trim().trim_start_matches('(').trim_end_matches(')');
                    if let Some((k, v)) = trimmed.split_once(':') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        if !k.is_empty()
                            && !matches!(
                                k,
                                "title"
                                    | "subtitle"
                                    | "author"
                                    | "date"
                                    | "version"
                                    | "institution"
                                    | "affiliation"
                                    | "org"
                            )
                        {
                            extra_args.push((k.to_string(), v.to_string()));
                        }
                    }
                }

                let body = if let Some(first_bracket) = raw.find('[')
                    && let Some(last_bracket) = raw.rfind(']')
                    && first_bracket < last_bracket
                {
                    raw[first_bracket + 1..last_bracket].trim().to_string()
                } else {
                    String::new()
                };

                ComplexModalState::TitleSlide {
                    title,
                    subtitle,
                    author,
                    date,
                    version,
                    institution,
                    extra_args,
                    body,
                }
            },
            | "badge" | "pill" => {
                let label = extract_first_string_arg(args).unwrap_or_else(|| {
                    extract_arg_str(args, "label").unwrap_or_else(|| "Badge".to_string())
                });
                let fill = extract_arg_str(args, "fill").unwrap_or_else(|| {
                    if let Some(pos) = args.find("fill:") {
                        let after = args[pos + 5..].trim_start();
                        let end = after.find([',', ')', '\n']).unwrap_or(after.len());
                        let val = after[..end].trim();
                        if val.is_empty() {
                            "slide-colors.accent".to_string()
                        } else {
                            val.to_string()
                        }
                    } else {
                        "slide-colors.accent".to_string()
                    }
                });
                let text_color = extract_arg_str(args, "text-color").unwrap_or_default();
                ComplexModalState::Badge {
                    label,
                    fill,
                    text_color,
                }
            },
            | "video" => {
                let source =
                    extract_first_string_arg(args).unwrap_or_else(|| "assets/demo.mp4".to_string());
                let caption = extract_arg_str(args, "caption").unwrap_or_default();
                let duration = extract_arg_str(args, "duration").unwrap_or_default();
                let quality =
                    extract_arg_str(args, "quality").unwrap_or_else(|| "4K 60FPS".to_string());
                let style = extract_arg_str(args, "style").unwrap_or_else(|| "glass".to_string());
                let width = extract_arg_str(args, "width").unwrap_or_else(|| "90%".to_string());
                ComplexModalState::Video {
                    source,
                    caption,
                    duration,
                    quality,
                    style,
                    width,
                }
            },
            | "audio" | "audio-player" => {
                let is_player = trimmed_callee == "audio-player";
                let source = extract_first_string_arg(args)
                    .unwrap_or_else(|| "assets/soundtrack.mp3".to_string());
                let title = extract_arg_str(args, "title")
                    .unwrap_or_else(|| "Background Music".to_string());
                let artist = extract_arg_str(args, "artist")
                    .unwrap_or_else(|| "cargo-slide soundtrack".to_string());
                let autoplay = if let Some(pos) = args.find("autoplay:") {
                    let after = args[pos + 9..].trim_start();
                    after.starts_with("true")
                } else {
                    !is_player
                };
                let loop_playback = if let Some(pos) = args.find("loop:") {
                    let after = args[pos + 5..].trim_start();
                    !after.starts_with("false")
                } else {
                    true
                };
                let volume = if let Some(pos) = args.find("volume:") {
                    let after = args[pos + 7..].trim_start();
                    let end = after.find([',', ')', '\n']).unwrap_or(after.len());
                    after[..end].trim().parse::<f32>().unwrap_or(0.8)
                } else {
                    0.8
                };
                let width = extract_arg_str(args, "width").unwrap_or_else(|| "100%".to_string());
                ComplexModalState::Audio {
                    source,
                    is_player,
                    title,
                    artist,
                    autoplay,
                    loop_playback,
                    volume,
                    width,
                }
            },
            | _ => {
                let fill = extract_arg_str(args, "fill").unwrap_or_default();
                let stroke = extract_arg_str(args, "stroke").unwrap_or_default();
                let radius = extract_arg_str(args, "radius").unwrap_or_default();
                let inset = extract_arg_str(args, "inset").unwrap_or_default();
                let width = extract_arg_str(args, "width").unwrap_or_default();
                let body = if let Some(first_bracket) = raw.find('[')
                    && let Some(last_bracket) = raw.rfind(']')
                    && first_bracket < last_bracket
                {
                    raw[first_bracket + 1..last_bracket].trim().to_string()
                } else {
                    let bracketed = extract_bracket_contents(args);
                    if let Some(first) = bracketed.into_iter().next() {
                        first.to_string()
                    } else {
                        raw.trim().to_string()
                    }
                };
                ComplexModalState::BoxBlock {
                    content: body,
                    fill,
                    stroke,
                    radius,
                    inset,
                    width,
                }
            },
        };

        Self {
            slide_idx,
            block_id,
            range,
            callee: trimmed_callee.to_string(),
            original_raw: raw.to_string(),
            state,
        }
    }

    /// Generate updated Typst markup from modal state
    #[must_use]
    pub fn to_typst(&self) -> String {
        match &self.state {
            | ComplexModalState::Table {
                cols,
                col_spec,
                has_header,
                header_cells,
                rows,
                properties,
            } => {
                let safe_cols = (*cols).max(1);
                let spec = if col_spec.trim().is_empty() || col_spec.trim().parse::<usize>().is_ok()
                {
                    format!("{safe_cols}")
                } else {
                    col_spec.trim().to_string()
                };

                let prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let mut out = format!("{prefix}table(\n  columns: {spec},\n");
                for prop in properties {
                    out.push_str(&format!("  {prop},\n"));
                }
                if *has_header && !header_cells.is_empty() {
                    out.push_str("  table.header(");
                    let h_strs = header_cells
                        .iter()
                        .map(|c| format!("[{c}]"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&h_strs);
                    out.push_str("),\n");
                }
                for row in rows {
                    out.push_str("  ");
                    for cell in row {
                        out.push_str(&format!("[{cell}], "));
                    }
                    out.push('\n');
                }
                out.push(')');
                out
            },
            | ComplexModalState::Chart {
                title,
                chart_type,
                source,
                sql,
                dsl,
                format,
                unit,
                prefix,
                height,
                items,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                if self.callee.starts_with("chart-")
                    && source.trim().is_empty()
                    && sql.trim().is_empty()
                    && dsl.trim().is_empty()
                {
                    let macro_name = match chart_type.as_str() {
                        | "pie" => format!("{code_prefix}chart-pie"),
                        | "line" => format!("{code_prefix}chart-line"),
                        | _ => format!("{code_prefix}chart-bar"),
                    };
                    let mut out = format!("{macro_name}(\n");
                    if !title.trim().is_empty() {
                        out.push_str(&format!("  title: \"{}\",\n", title.trim()));
                    }
                    for (lbl, val) in items {
                        out.push_str(&format!("  (\"{lbl}\", {val:.0}),\n"));
                    }
                    out.push(')');
                    out
                } else {
                    let mut out = format!("{code_prefix}chart(\n  type: \"{chart_type}\",\n");
                    if !title.trim().is_empty() {
                        out.push_str(&format!("  title: \"{}\",\n", title.trim()));
                    }
                    if !source.trim().is_empty() {
                        out.push_str(&format!("  source: \"{}\",\n", source.trim()));
                    }
                    if !sql.trim().is_empty() {
                        out.push_str(&format!("  sql: \"{}\",\n", sql.trim()));
                    }
                    if !dsl.trim().is_empty() {
                        out.push_str(&format!("  dsl: \"{}\",\n", dsl.trim()));
                    }
                    if !format.trim().is_empty() {
                        out.push_str(&format!("  format: \"{}\",\n", format.trim()));
                    }
                    if !unit.trim().is_empty() {
                        out.push_str(&format!("  unit: \"{}\",\n", unit.trim()));
                    }
                    if !prefix.trim().is_empty() {
                        out.push_str(&format!("  prefix: \"{}\",\n", prefix.trim()));
                    }
                    let h = if height.trim().is_empty() {
                        "120pt"
                    } else {
                        height.trim()
                    };
                    out.push_str(&format!("  height: {h},\n"));
                    if source.trim().is_empty() && sql.trim().is_empty() && !items.is_empty() {
                        let cat_strs = if items.len() == 1 {
                            format!("\"{}\",", items[0].0)
                        } else {
                            items
                                .iter()
                                .map(|(l, _)| format!("\"{l}\""))
                                .collect::<Vec<_>>()
                                .join(", ")
                        };
                        let val_strs = if items.len() == 1 {
                            format!("{:.0},", items[0].1)
                        } else {
                            items
                                .iter()
                                .map(|(_, v)| format!("{v:.0}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        };
                        out.push_str(&format!(
                            "  data: (\n    categories: ({cat_strs}),\n    series: ((name: \"Data\", values: ({val_strs})),),\n  ),\n"
                        ));
                    }
                    out.push(')');
                    out
                }
            },
            | ComplexModalState::Grid { cols: _, columns } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let safe_cols = columns.len().max(1);
                let is_cols_macro = self.callee == "cols"
                    || self.original_raw.contains("#cols(")
                    || self.original_raw.contains("cols(");

                if is_cols_macro {
                    let mut out = format!("{code_prefix}cols(\n");
                    for col in columns {
                        out.push_str(&format!("  [\n    {col}\n  ],\n"));
                    }
                    out.push(')');
                    out
                } else {
                    let mut out = format!("{code_prefix}grid(\n  columns: {safe_cols},\n");
                    for col in columns {
                        out.push_str(&format!("  [\n    {col}\n  ],\n"));
                    }
                    out.push(')');
                    out
                }
            },
            | ComplexModalState::Callout {
                title,
                body,
                stroke_color,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let mut args = Vec::new();
                let t_trim = title.trim();
                if !t_trim.is_empty() {
                    args.push(format!("title: \"{t_trim}\""));
                }
                let sc = stroke_color.trim();
                if !sc.is_empty() {
                    args.push(format!("stroke-color: {sc}"));
                }
                let args_str = if args.is_empty() {
                    String::new()
                } else {
                    format!("({})", args.join(", "))
                };
                let clean_body = body.trim();
                if clean_body.contains('\n') {
                    format!("{code_prefix}callout{args_str}[\n  {clean_body}\n]")
                } else if clean_body.is_empty() {
                    format!("{code_prefix}callout{args_str}[]")
                } else {
                    format!("{code_prefix}callout{args_str}[{clean_body}]")
                }
            },
            | ComplexModalState::BoxBlock {
                content,
                fill,
                stroke,
                radius,
                inset,
                width,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let mut props = Vec::new();
                let f_trim = fill.trim();
                if !f_trim.is_empty() && f_trim != "none" {
                    props.push(format!("fill: {f_trim}"));
                }
                let s_trim = stroke.trim();
                if !s_trim.is_empty() && s_trim != "none" {
                    props.push(format!("stroke: {s_trim}"));
                }
                let r_trim = radius.trim();
                if !r_trim.is_empty() && r_trim != "0pt" && r_trim != "none" {
                    props.push(format!("radius: {r_trim}"));
                }
                let i_trim = inset.trim();
                if !i_trim.is_empty() && i_trim != "0pt" && i_trim != "none" {
                    props.push(format!("inset: {i_trim}"));
                }
                let w_trim = width.trim();
                if !w_trim.is_empty() && w_trim != "auto" && w_trim != "none" {
                    props.push(format!("width: {w_trim}"));
                }

                let callee = if self.callee.is_empty() {
                    "box"
                } else {
                    &self.callee
                };
                if props.is_empty() {
                    format!("{code_prefix}{callee}[\n  {content}\n]")
                } else {
                    format!(
                        "{code_prefix}{callee}({})[\n  {content}\n]",
                        props.join(", ")
                    )
                }
            },
            | ComplexModalState::Link { url, label } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let u = url.trim();
                let l = label.trim();
                if l.is_empty() || l == u {
                    format!("{code_prefix}link(\"{u}\")")
                } else {
                    format!("{code_prefix}link(\"{u}\")[{l}]")
                }
            },
            | ComplexModalState::TitleSlide {
                title,
                subtitle,
                author,
                date,
                version,
                institution,
                extra_args,
                body,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let mut out = format!("{code_prefix}title-slide(\n");
                out.push_str(&format!("  title: \"{}\",\n", title.trim()));
                if !subtitle.trim().is_empty() {
                    out.push_str(&format!("  subtitle: \"{}\",\n", subtitle.trim()));
                }
                if !author.trim().is_empty() {
                    out.push_str(&format!("  author: \"{}\",\n", author.trim()));
                }
                if !date.trim().is_empty() {
                    out.push_str(&format!("  date: \"{}\",\n", date.trim()));
                }
                if !version.trim().is_empty() {
                    out.push_str(&format!("  version: \"{}\",\n", version.trim()));
                }
                if !institution.trim().is_empty() {
                    out.push_str(&format!("  institution: \"{}\",\n", institution.trim()));
                }
                for (k, v) in extra_args {
                    let kt = k.trim();
                    let vt = v.trim();
                    if !kt.is_empty() {
                        out.push_str(&format!("  {kt}: \"{vt}\",\n"));
                    }
                }
                out.push(')');
                if !body.trim().is_empty() {
                    out.push_str(&format!("[\n  {}\n]", body.trim()));
                }
                out
            },
            | ComplexModalState::Badge {
                label,
                fill,
                text_color,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let callee = if self.callee.is_empty() {
                    "badge"
                } else {
                    &self.callee
                };
                let mut args = vec![format!("\"{}\"", label.trim())];
                let f_trim = fill.trim();
                if !f_trim.is_empty() {
                    args.push(format!("fill: {f_trim}"));
                }
                let tc_trim = text_color.trim();
                if !tc_trim.is_empty() {
                    args.push(format!("text-color: {tc_trim}"));
                }
                format!("{code_prefix}{callee}({})", args.join(", "))
            },
            | ComplexModalState::Video {
                source,
                caption,
                duration,
                quality,
                style,
                width,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                let mut args = vec![format!("\"{}\"", source.trim())];
                let c = caption.trim();
                if !c.is_empty() {
                    args.push(format!("caption: \"{c}\""));
                }
                let d = duration.trim();
                if !d.is_empty() {
                    args.push(format!("duration: \"{d}\""));
                }
                let q = quality.trim();
                if !q.is_empty() && q != "4K 60FPS" {
                    args.push(format!("quality: \"{q}\""));
                }
                let s = style.trim();
                if !s.is_empty() && s != "glass" {
                    args.push(format!("style: \"{s}\""));
                }
                let w = width.trim();
                if !w.is_empty() && w != "90%" {
                    args.push(format!("width: {w}"));
                }
                if args.len() == 1 {
                    format!("{code_prefix}video(\"{}\")", source.trim())
                } else {
                    format!("{code_prefix}video(\n  {},\n)", args.join(",\n  "))
                }
            },
            | ComplexModalState::Audio {
                source,
                is_player,
                title,
                artist,
                autoplay,
                loop_playback,
                volume,
                width,
            } => {
                let code_prefix = if self.original_raw.trim_start().starts_with('#') {
                    "#"
                } else {
                    ""
                };
                if *is_player {
                    let mut args = vec![format!("\"{}\"", source.trim())];
                    let t = title.trim();
                    if !t.is_empty() && t != "Background Music" {
                        args.push(format!("title: \"{t}\""));
                    }
                    let a = artist.trim();
                    if !a.is_empty() && a != "cargo-slide soundtrack" {
                        args.push(format!("artist: \"{a}\""));
                    }
                    if *autoplay {
                        args.push("autoplay: true".to_string());
                    }
                    if !*loop_playback {
                        args.push("loop: false".to_string());
                    }
                    if (*volume - 0.8).abs() > 0.01 {
                        args.push(format!("volume: {:.1}", volume));
                    }
                    let w = width.trim();
                    if !w.is_empty() && w != "100%" {
                        args.push(format!("width: {w}"));
                    }
                    format!("{code_prefix}audio-player(\n  {},\n)", args.join(",\n  "))
                } else {
                    let mut args = vec![format!("\"{}\"", source.trim())];
                    if !*autoplay {
                        args.push("autoplay: false".to_string());
                    }
                    if !*loop_playback {
                        args.push("loop: false".to_string());
                    }
                    if (*volume - 0.8).abs() > 0.01 {
                        args.push(format!("volume: {:.1}", volume));
                    }
                    if args.len() == 1 {
                        format!("{code_prefix}audio(\"{}\")", source.trim())
                    } else {
                        format!("{code_prefix}audio({})", args.join(", "))
                    }
                }
            },
        }
    }
}

/// Helper function to create clean bordered input styling for the modal dialog
fn modal_input_style(theme: AppTheme) -> text_input::Style {
    text_input::Style {
        background: Background::Color(if theme.is_dark() {
            Color::from_rgba(1.0, 1.0, 1.0, 0.05)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.03)
        }),
        border: Border {
            color: theme.border_color(),
            width: 1.0,
            radius: iced::border::Radius::from(4.0),
        },
        icon: theme.text_muted(),
        placeholder: theme.text_muted(),
        value: theme.text_primary(),
        selection: Color::from_rgba(0.2, 0.5, 0.9, 0.25),
    }
}

/// Render the complex element modal dialog
#[must_use]
pub fn view_complex_element_modal<'a>(
    theme: AppTheme,
    modal: &'a ComplexElementModal,
    col_editors: Option<&'a [text_editor::Content]>,
    box_editor: Option<&'a text_editor::Content>,
    title_slide_editor: Option<&'a text_editor::Content>,
    callout_editor: Option<&'a text_editor::Content>,
) -> Element<'a, Message> {
    let (tag, header_title) = match &modal.state {
        | ComplexModalState::Table { .. } => ("[Table]", "Table Editor"),
        | ComplexModalState::Chart { .. } => ("[Chart]", "Chart Data & SQL Inspector"),
        | ComplexModalState::Grid { .. } => ("[Grid]", "Multi-Column Layout Editor"),
        | ComplexModalState::Callout { .. } => ("[Note]", "Callout & Note Editor"),
        | ComplexModalState::BoxBlock { .. } => ("[Box]", "Box Container Editor"),
        | ComplexModalState::Link { .. } => ("[Link]", "Interactive Hyperlink Editor"),
        | ComplexModalState::TitleSlide { .. } => ("[Title]", "Presentation Title Slide Editor"),
        | ComplexModalState::Badge { .. } => ("[Badge]", "Badge & Pill Live Editor"),
        | ComplexModalState::Video { .. } => ("[Video]", "Video Playback & Visual Style Config"),
        | ComplexModalState::Audio { is_player, .. } => {
            (
                "[Audio]",
                if *is_player {
                    "Audio Player & Soundtrack Config"
                } else {
                    "Background Audio Trigger Config"
                },
            )
        },
    };

    let title_widget = row![
        container(text(tag).size(11).color(theme.accent()))
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            }),
        text(header_title)
            .size(16)
            .wrapping(iced::widget::text::Wrapping::Word)
            .color(theme.text_primary()),
        Space::new().width(Length::Fill),
        button(text("×").size(16))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 8])
            .on_press(Message::CloseComplexModal)
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body_content: Element<'a, Message> = match &modal.state {
        | ComplexModalState::Table {
            cols,
            col_spec,
            has_header,
            header_cells,
            rows,
            ..
        } => render_modal_table_editor(theme, *cols, col_spec, *has_header, header_cells, rows),
        | ComplexModalState::Chart {
            title,
            chart_type,
            source,
            sql,
            dsl,
            format,
            unit,
            prefix: _,
            height: _,
            items,
        } => {
            render_modal_chart_editor(
                theme, title, chart_type, source, sql, dsl, format, unit, items,
            )
        },
        | ComplexModalState::Grid { cols, columns } => {
            render_modal_grid_editor(theme, *cols, columns, col_editors)
        },
        | ComplexModalState::Callout {
            title,
            body,
            stroke_color,
        } => render_modal_callout_editor(theme, title, body, stroke_color, callout_editor),
        | ComplexModalState::BoxBlock {
            content,
            fill,
            stroke,
            radius,
            inset,
            width,
        } => {
            render_modal_box_editor(
                theme, content, fill, stroke, radius, inset, width, box_editor,
            )
        },
        | ComplexModalState::Link { url, label } => render_modal_link_editor(theme, url, label),
        | ComplexModalState::TitleSlide {
            title,
            subtitle,
            author,
            date,
            version,
            institution,
            extra_args,
            ..
        } => {
            render_modal_title_slide_editor(
                theme,
                title,
                subtitle,
                author,
                date,
                version,
                institution,
                extra_args,
                title_slide_editor,
            )
        },
        | ComplexModalState::Badge {
            label,
            fill,
            text_color,
        } => render_modal_badge_editor(theme, label, fill, text_color),
        | ComplexModalState::Video {
            source,
            caption,
            duration,
            quality,
            style,
            width,
        } => render_modal_video_editor(theme, source, caption, duration, quality, style, width),
        | ComplexModalState::Audio {
            source,
            is_player,
            title,
            artist,
            autoplay,
            loop_playback,
            volume,
            width: _,
        } => {
            render_modal_audio_editor(
                theme,
                source,
                *is_player,
                title,
                artist,
                *autoplay,
                *loop_playback,
                *volume,
            )
        },
    };

    // Dialog footer with Discard and Apply
    let cancel_btn = button(text("Cancel").size(13))
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .padding([7, 16])
        .on_press(Message::CloseComplexModal);

    let apply_btn = button(text("Apply & Save").size(13))
        .style(move |_t, _s| theme::primary_button_style(theme))
        .padding([7, 20])
        .on_press(Message::ApplyComplexModal);

    let footer = row![Space::new().width(Length::Fill), cancel_btn, apply_btn]
        .spacing(12)
        .align_y(Alignment::Center);

    let dialog_card = container(
        column![
            title_widget,
            Space::new().height(10),
            scrollable(body_content).height(Length::FillPortion(1)),
            Space::new().height(12),
            footer
        ]
        .spacing(8),
    )
    .width(Length::Fixed(820.0))
    .max_height(680.0)
    .padding(22)
    .style(move |_| theme::modal_dialog_style(theme));

    // Semi-transparent backdrop overlay
    container(dialog_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| theme::modal_backdrop_style(theme))
        .into()
}

/// Render table spreadsheet-style editor with column spec and header row support
fn render_modal_table_editor<'a>(
    theme: AppTheme,
    cols: usize,
    col_spec: &'a str,
    has_header: bool,
    header_cells: &'a [String],
    rows: &'a [Vec<String>],
) -> Element<'a, Message> {
    let spec_label = text("Columns Spec:").size(12).color(theme.text_secondary());
    let spec_input = text_input("(e.g. (auto, 1fr) or 3)", col_spec)
        .size(12)
        .padding([5, 8])
        .width(Length::Fixed(180.0))
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalTableUpdateColSpec);

    let header_btn = button(
        text(if has_header {
            "Header: On"
        } else {
            "+ Header"
        })
        .size(11),
    )
    .style(move |_t, _s| theme::subtle_button_style(theme, has_header))
    .padding([4, 10])
    .on_press(Message::ModalTableToggleHeader);

    let top_controls = row![
        spec_label,
        spec_input,
        Space::new().width(8),
        header_btn,
        Space::new().width(Length::Fill),
        button(text("+ Add Row").size(11))
            .style(move |_t, _s| theme::primary_button_style(theme))
            .padding([4, 10])
            .on_press(Message::ModalTableAddRow),
        button(text("+ Add Col").size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([4, 10])
            .on_press(Message::ModalTableAddCol),
        button(text("- Delete Col").size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([4, 8])
            .on_press_maybe(if cols > 1 {
                Some(Message::ModalTableDeleteCol(cols.saturating_sub(1)))
            } else {
                None
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut table_col = column![top_controls].spacing(6).width(Length::Fill);

    // Optional table.header row
    if has_header {
        let mut h_row = row![].spacing(6).align_y(Alignment::Center);
        let h_badge = container(text("Header").size(10).color(theme.accent()))
            .width(Length::Fixed(46.0))
            .padding([2, 4]);
        h_row = h_row.push(h_badge);

        for (c_idx, cell_str) in header_cells.iter().enumerate() {
            let input = text_input("Header...", cell_str)
                .size(12)
                .padding([6, 8])
                .style(move |_t, _s| modal_input_style(theme))
                .on_input(move |new_val| {
                    Message::ModalTableUpdateHeaderCell {
                        col: c_idx,
                        val: new_val,
                    }
                });
            h_row = h_row.push(container(input).width(Length::FillPortion(1)));
        }

        let h_container = container(h_row).padding([4, 6]).style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.2, 0.5, 0.9, 0.12))),
                border: Border {
                    color: theme.accent().scale_alpha(0.4),
                    width: 1.0,
                    radius: border::Radius::from(4.0),
                },
                ..container::Style::default()
            }
        });

        table_col = table_col.push(h_container);
    }

    // Data rows
    for (r_idx, row_data) in rows.iter().enumerate() {
        let mut row_widget = row![].spacing(6).align_y(Alignment::Center);

        let row_badge = text(format!("R{}", r_idx + 1))
            .size(11)
            .color(theme.text_muted());

        let mut row_header = row![row_badge].align_y(Alignment::Center);

        if rows.len() > 1 {
            let btn_del_row = button(text("×").size(10))
                .style(move |_t, _s| theme::danger_button_style(theme))
                .padding([2, 5])
                .on_press(Message::ModalTableDeleteRow(r_idx));
            row_header = row_header.push(btn_del_row);
        }
        row_widget = row_widget.push(container(row_header).width(Length::Fixed(46.0)));

        for (c_idx, cell_str) in row_data.iter().enumerate() {
            let input = text_input("Cell...", cell_str)
                .size(12)
                .padding([6, 8])
                .style(move |_t, _s| modal_input_style(theme))
                .on_input(move |new_val| {
                    Message::ModalTableUpdateCell {
                        row: r_idx,
                        col: c_idx,
                        val: new_val,
                    }
                });

            row_widget = row_widget.push(container(input).width(Length::FillPortion(1)));
        }

        let row_container = container(row_widget).padding([3, 6]).style(move |_| {
            container::Style {
                background: Some(Background::Color(if r_idx % 2 == 1 {
                    Color::from_rgba(0.5, 0.5, 0.5, 0.03)
                } else {
                    Color::TRANSPARENT
                })),
                border: Border {
                    color: Color::from_rgba(0.5, 0.5, 0.5, 0.1),
                    width: 0.5,
                    radius: border::Radius::from(4.0),
                },
                ..container::Style::default()
            }
        });

        table_col = table_col.push(row_container);
    }

    table_col.into()
}

/// Render full cargo-slide interactive chart editor (supporting SQL queries, datasets, pipeline DSL, and inline series)
#[allow(clippy::too_many_arguments)]
fn render_modal_chart_editor<'a>(
    theme: AppTheme,
    title: &'a str,
    chart_type: &'a str,
    source: &'a str,
    sql: &'a str,
    dsl: &'a str,
    format_opt: &'a str,
    unit: &'a str,
    items: &'a [(String, f32)],
) -> Element<'a, Message> {
    // 1. Title input and chart types switchers
    let title_label = text("Chart Title:").size(12).color(theme.text_secondary());
    let title_input = text_input("Enter chart title...", title)
        .size(13)
        .padding([6, 8])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalChartUpdateTitle);

    let type_btn = |target_type: &'static str, label: &'static str| {
        let is_sel = chart_type == target_type;
        button(text(label).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 8])
            .on_press(Message::ModalChartSetType(target_type.to_string()))
    };

    let type_switchers = row![
        text("Type:").size(12).color(theme.text_secondary()),
        type_btn("bar", "Bar"),
        type_btn("line", "Line"),
        type_btn("area", "Area"),
        type_btn("pie", "Pie"),
        type_btn("donut", "Donut"),
        type_btn("scatter", "Scatter"),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let top_bar = row![
        title_label,
        title_input,
        Space::new().width(8),
        type_switchers
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // 2. Data Source & SQL Pipeline section
    let source_label = text("Dataset File (CSV / JSON / SQLite DB):")
        .size(11)
        .color(theme.text_secondary());
    let source_input = text_input(
        "e.g. assets/benchmarks.csv or assets/telemetry.db?query=...",
        source,
    )
    .size(12)
    .padding([5, 8])
    .style(move |_t, _s| modal_input_style(theme))
    .on_input(Message::ModalChartUpdateSource);

    let sql_label = text("SQL Query (Native SQLite or In-Memory):")
        .size(11)
        .color(theme.accent());
    let sql_input = text_input(
        "SELECT stage, p50, p95, p99 FROM latency_stats WHERE ...",
        sql,
    )
    .size(12)
    .padding([5, 8])
    .style(move |_t, _s| modal_input_style(theme))
    .on_input(Message::ModalChartUpdateSql);

    let dsl_label = text("Pipeline DSL Transformation:")
        .size(11)
        .color(theme.text_secondary());
    let dsl_input = text_input("filter throughput > 8000 | sort desc | limit 5", dsl)
        .size(12)
        .padding([5, 8])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalChartUpdateDsl);

    let fmt_label = text("Format:").size(11).color(theme.text_secondary());
    let fmt_input = text_input("currency / percentage / compact", format_opt)
        .size(12)
        .padding([4, 6])
        .width(Length::Fixed(140.0))
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalChartUpdateFormat);

    let unit_label = text("Unit:").size(11).color(theme.text_secondary());
    let unit_input = text_input("USD, ms, MB", unit)
        .size(12)
        .padding([4, 6])
        .width(Length::Fixed(90.0))
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalChartUpdateUnit);

    let query_controls = column![
        source_label,
        source_input,
        sql_label,
        sql_input,
        dsl_label,
        dsl_input,
        row![
            fmt_label,
            fmt_input,
            Space::new().width(12),
            unit_label,
            unit_input
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    let query_box = container(query_controls).padding(10).style(move |_| {
        container::Style {
            background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.04))),
            border: Border {
                color: theme.border_color(),
                width: 0.5,
                radius: border::Radius::from(6.0),
            },
            ..container::Style::default()
        }
    });

    // 3. Inline Data points editor list (fallback or direct preview)
    let colors = [
        Color::from_rgb(0.24, 0.51, 0.96),
        Color::from_rgb(0.06, 0.73, 0.51),
        Color::from_rgb(0.96, 0.62, 0.13),
        Color::from_rgb(0.93, 0.27, 0.27),
        Color::from_rgb(0.64, 0.38, 0.95),
        Color::from_rgb(0.18, 0.78, 0.92),
    ];

    let mut items_col = column![].spacing(5);
    let list_header = row![
        text("Color").size(11).color(theme.text_muted()),
        Space::new().width(16),
        text("Data Category / Label")
            .size(11)
            .color(theme.text_muted()),
        Space::new().width(Length::FillPortion(2)),
        text("Value").size(11).color(theme.text_muted()),
        Space::new().width(Length::FillPortion(1)),
        button(text("+ Add Item").size(11))
            .style(move |_t, _s| theme::primary_button_style(theme))
            .padding([3, 8])
            .on_press(Message::ModalChartAddItem),
    ]
    .align_y(Alignment::Center);

    items_col = items_col.push(list_header);

    for (i, (lbl, val)) in items.iter().enumerate() {
        let color = colors[i % colors.len()];
        let dot = container(Space::new())
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0))
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(color)),
                    border: border::rounded(6.0),
                    ..container::Style::default()
                }
            });

        let lbl_input = text_input("Label", lbl)
            .size(12)
            .padding([4, 6])
            .style(move |_t, _s| modal_input_style(theme))
            .on_input(move |new_lbl| {
                Message::ModalChartUpdateItemLabel {
                    idx: i,
                    label: new_lbl,
                }
            });

        let val_input = text_input("Value", &format!("{val:.0}"))
            .size(12)
            .padding([4, 6])
            .style(move |_t, _s| modal_input_style(theme))
            .on_input(move |new_v| {
                let parsed = new_v.parse::<f32>().unwrap_or(0.0);
                Message::ModalChartUpdateItemValue { idx: i, val: parsed }
            });

        let mut item_row = row![
            dot,
            container(lbl_input).width(Length::FillPortion(2)),
            container(val_input).width(Length::FillPortion(1))
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        if items.len() > 1 {
            let del_btn = button(text("×").size(10))
                .style(move |_t, _s| theme::danger_button_style(theme))
                .padding([2, 5])
                .on_press(Message::ModalChartDeleteItem(i));
            item_row = item_row.push(del_btn);
        }

        items_col = items_col.push(item_row);
    }

    // 4. Live visual preview of chart with formatting
    let max_val = items.iter().map(|(_, v)| *v).fold(1.0f32, f32::max);
    let mut preview_col = column![
        row![
            text("Preview").size(11).color(theme.accent()),
            Space::new().width(Length::Fill),
            text(format!("Type: {chart_type}"))
                .size(10)
                .color(theme.text_muted()),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    if !sql.trim().is_empty() {
        let sql_badge = container(
            text(format!("SQL: {}", sql.trim()))
                .size(9)
                .wrapping(iced::widget::text::Wrapping::Word)
                .color(theme.accent()),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.2, 0.5, 0.9, 0.1))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });
        preview_col = preview_col.push(sql_badge);
    } else if !source.trim().is_empty() {
        let src_badge = container(
            text(format!("Source: {}", source.trim()))
                .size(9)
                .wrapping(iced::widget::text::Wrapping::Word)
                .color(theme.text_secondary()),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.08))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });
        preview_col = preview_col.push(src_badge);
    }

    for (i, (lbl, val)) in items.iter().enumerate() {
        let color = colors[i % colors.len()];
        let pct = (val / max_val).clamp(0.02, 1.0);
        let bar_width = 160.0 * pct;

        let bar_widget = container(Space::new())
            .width(Length::Fixed(bar_width))
            .height(Length::Fixed(12.0))
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(color)),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            });

        let unit_suffix = if !unit.is_empty() {
            format!(" {unit}")
        } else {
            String::new()
        };
        let row_vis = row![
            text(lbl)
                .size(10)
                .color(theme.text_secondary())
                .width(Length::Fixed(70.0)),
            bar_widget,
            text(format!(" {val:.0}{unit_suffix}"))
                .size(10)
                .color(theme.text_muted()),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        preview_col = preview_col.push(row_vis);
    }

    let preview_box = container(preview_col)
        .width(Length::Fixed(300.0))
        .padding(10)
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_color(),
                    width: 0.5,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        });

    let main_row = row![
        container(items_col).width(Length::FillPortion(1)),
        Space::new().width(10),
        preview_box
    ]
    .spacing(8);

    column![top_bar, query_box, Space::new().height(4), main_row]
        .spacing(8)
        .into()
}

/// Render multi-column layout editor with text_editor multiline Enter support
fn render_modal_grid_editor<'a>(
    theme: AppTheme,
    cols: usize,
    columns: &'a [String],
    col_editors: Option<&'a [text_editor::Content]>,
) -> Element<'a, Message> {
    let top_bar = row![
        text(format!("Columns ({cols})"))
            .size(13)
            .color(theme.accent()),
        Space::new().width(Length::Fixed(8.0)),
        text("• Press Enter inside any column to create new lines")
            .size(11)
            .color(theme.text_muted()),
        Space::new().width(Length::Fill),
        button(text("+ Add Column").size(11))
            .style(move |_t, _s| theme::primary_button_style(theme))
            .padding([4, 12])
            .on_press(Message::ModalGridAddCol),
    ]
    .align_y(Alignment::Center);

    let mut cols_row = row![].spacing(16).width(Length::Fill);

    for (i, content) in columns.iter().enumerate() {
        let col_title = format!("Column {}", i + 1);
        let header_row = row![
            text(col_title).size(12).color(theme.accent()),
            Space::new().width(Length::Fill),
            button(text("×").size(10))
                .style(move |_t, _s| theme::danger_button_style(theme))
                .padding([2, 6])
                .on_press_maybe(if columns.len() > 1 {
                    Some(Message::ModalGridDeleteCol(i))
                } else {
                    None
                })
        ]
        .align_y(Alignment::Center);

        let input_widget: Element<'a, Message> = if let Some(editors) = col_editors
            && let Some(editor) = editors.get(i)
        {
            text_editor(editor)
                .placeholder("Column markup content (press Enter for newlines)...")
                .wrapping(iced::widget::text::Wrapping::Word)
                .size(12)
                .height(Length::Fixed(180.0))
                .padding([8, 10])
                .style(move |_t, _s| theme::editor_style(theme))
                .highlight_with::<TypstHighlighter>(
                    TypstHighlightSettings {
                        is_dark: theme.is_dark(),
                    },
                    to_typst_format,
                )
                .on_action(move |action| Message::ModalGridEditorAction { idx: i, action })
                .into()
        } else {
            text_input("Column markup content...", content)
                .size(12)
                .padding([8, 10])
                .style(move |_t, _s| modal_input_style(theme))
                .on_input(move |new_val| Message::ModalGridUpdateCol { idx: i, val: new_val })
                .into()
        };

        let preview = container(
            text(content)
                .size(11)
                .wrapping(iced::widget::text::Wrapping::Word)
                .color(theme.text_secondary()),
        )
        .padding([4, 6])
        .width(Length::Fill);

        let col_card = container(
            column![
                header_row,
                Space::new().height(4),
                input_widget,
                Space::new().height(4),
                text("Preview:").size(10).color(theme.text_muted()),
                preview
            ]
            .spacing(4),
        )
        .width(Length::FillPortion(1))
        .padding(14)
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        });

        cols_row = cols_row.push(col_card);
    }

    column![top_bar, Space::new().height(8), cols_row]
        .spacing(8)
        .into()
}

/// Render callout editor with wrapped preview, multiline editor, and stroke color presets
fn render_modal_callout_editor<'a>(
    theme: AppTheme,
    title: &'a str,
    body: &'a str,
    stroke_color: &'a str,
    callout_editor: Option<&'a text_editor::Content>,
) -> Element<'a, Message> {
    let make_color_btn = |color_val: &'static str, label: &'static str| {
        let is_sel = stroke_color == color_val;
        button(text(label).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 10])
            .on_press(Message::ModalCalloutSetStrokeColor(color_val.to_string()))
    };

    let color_presets_row = row![
        text("Accent Color:").size(12).color(theme.text_secondary()),
        make_color_btn("slide-colors.accent", "Blue"),
        make_color_btn("slide-colors.accent-cyan", "Cyan"),
        make_color_btn("slide-colors.accent-purple", "Purple"),
        make_color_btn("slide-colors.accent-orange", "Orange"),
        make_color_btn("slide-colors.accent-red", "Red"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let title_input = text_input("Title (optional)", title)
        .size(13)
        .padding([7, 10])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalCalloutUpdateTitle);

    let color_input = text_input(
        "Stroke color (e.g. slide-colors.accent, rgb(\"#38bdf8\"))",
        stroke_color,
    )
    .size(12)
    .padding([6, 10])
    .style(move |_t, _s| modal_input_style(theme))
    .on_input(Message::ModalCalloutUpdateStrokeColor);

    let editor_widget: Element<'a, Message> = if let Some(editor) = callout_editor {
        text_editor(editor)
            .id(iced::widget::Id::new("modal_callout_editor"))
            .placeholder(
                "Callout body text, markdown lists (- ...), or code blocks (```lang ... ```)...",
            )
            .wrapping(iced::widget::text::Wrapping::Word)
            .size(12)
            .height(Length::Fixed(140.0))
            .padding([8, 10])
            .style(move |_t, _s| theme::editor_style(theme))
            .highlight_with::<TypstHighlighter>(
                TypstHighlightSettings {
                    is_dark: theme.is_dark(),
                },
                to_typst_format,
            )
            .on_action(Message::ModalCalloutEditorAction)
            .into()
    } else {
        text_input("Callout message body text...", body)
            .size(13)
            .padding([8, 10])
            .style(move |_t, _s| modal_input_style(theme))
            .on_input(Message::ModalCalloutUpdateBody)
            .into()
    };

    let accent_color = parse_color_spec(stroke_color, theme).unwrap_or_else(|| theme.accent());

    let mut preview_col = column![].spacing(6).width(Length::Fill);
    if !title.trim().is_empty() {
        preview_col = preview_col.push(text(title.trim()).size(12).color(accent_color).font(
            iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            },
        ));
    }
    preview_col = preview_col.push(render_callout_body(body, theme, accent_color, 1.0));

    let preview_container = container(preview_col)
        .width(Length::Fill)
        .padding([10, 14])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(if theme.is_dark() {
                    Color::from_rgba(0.12, 0.15, 0.20, 0.6)
                } else {
                    Color::from_rgba(0.96, 0.97, 0.99, 0.9)
                })),
                border: Border {
                    color: accent_color,
                    width: 2.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        });

    column![
        color_presets_row,
        Space::new().height(4),
        text("Stroke Color Expression:")
            .size(11)
            .color(theme.text_secondary()),
        color_input,
        Space::new().height(6),
        text("Header Title:").size(12).color(theme.text_secondary()),
        title_input,
        Space::new().height(6),
        text("Body Content (Enter for multiline, code blocks, lists):")
            .size(12)
            .color(theme.text_secondary()),
        editor_widget,
        Space::new().height(6),
        text("Live Structured Preview:")
            .size(11)
            .color(theme.text_muted()),
        preview_container,
    ]
    .spacing(6)
    .into()
}

/// Render box block editor with styling chips, property controls, text_editor multiline Enter support, and styled preview
#[allow(clippy::too_many_arguments)]
fn render_modal_box_editor<'a>(
    theme: AppTheme,
    content: &'a str,
    fill: &'a str,
    stroke: &'a str,
    radius: &'a str,
    inset: &'a str,
    width: &'a str,
    box_editor: Option<&'a text_editor::Content>,
) -> Element<'a, Message> {
    // Style presets bar
    let make_preset_btn = |name: &'static str, label: &'static str| {
        button(text(label).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([3, 8])
            .on_press(Message::ModalBoxApplyPreset(name))
    };

    let preset_row = row![
        text("Presets:").size(11).color(theme.text_muted()),
        make_preset_btn("subtle", "Subtle"),
        make_preset_btn("blue", "Blue Card"),
        make_preset_btn("green", "Green"),
        make_preset_btn("amber", "Amber"),
        make_preset_btn("red", "Red"),
        make_preset_btn("outline", "Outline"),
        make_preset_btn("none", "Clear"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    // Style properties inputs
    let fill_input = text_input("e.g. rgb(\"eff6ff\")", fill)
        .size(11)
        .padding([4, 6])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalBoxSetFill);

    let stroke_input = text_input("e.g. 1pt + rgb(\"3b82f6\")", stroke)
        .size(11)
        .padding([4, 6])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalBoxSetStroke);

    let radius_input = text_input("e.g. 6pt", radius)
        .size(11)
        .padding([4, 6])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalBoxSetRadius);

    let inset_input = text_input("e.g. 8pt", inset)
        .size(11)
        .padding([4, 6])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalBoxSetInset);

    let width_input = text_input("e.g. 100% or auto", width)
        .size(11)
        .padding([4, 6])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalBoxSetWidth);

    let props_grid = row![
        column![
            text("Fill:").size(10).color(theme.text_secondary()),
            fill_input
        ]
        .spacing(2)
        .width(Length::FillPortion(1)),
        column![
            text("Stroke:").size(10).color(theme.text_secondary()),
            stroke_input
        ]
        .spacing(2)
        .width(Length::FillPortion(1)),
        column![
            text("Radius:").size(10).color(theme.text_secondary()),
            radius_input
        ]
        .spacing(2)
        .width(Length::FillPortion(1)),
        column![
            text("Inset:").size(10).color(theme.text_secondary()),
            inset_input
        ]
        .spacing(2)
        .width(Length::FillPortion(1)),
        column![
            text("Width:").size(10).color(theme.text_secondary()),
            width_input
        ]
        .spacing(2)
        .width(Length::FillPortion(1)),
    ]
    .spacing(8);

    // Multiline editor with text_editor
    let editor_widget: Element<'a, Message> = if let Some(editor) = box_editor {
        text_editor(editor)
            .id(iced::widget::Id::new("modal_box_editor"))
            .placeholder("Box container markup content (press Enter for newlines)...")
            .wrapping(iced::widget::text::Wrapping::Word)
            .size(12)
            .height(Length::Fixed(140.0))
            .padding([8, 10])
            .style(move |_t, _s| theme::editor_style(theme))
            .highlight_with::<TypstHighlighter>(
                TypstHighlightSettings {
                    is_dark: theme.is_dark(),
                },
                to_typst_format,
            )
            .on_action(Message::ModalBoxEditorAction)
            .into()
    } else {
        text_input("Container box content...", content)
            .size(13)
            .padding([8, 10])
            .style(move |_t, _s| modal_input_style(theme))
            .on_input(Message::ModalBoxUpdateContent)
            .into()
    };

    // Live styled preview
    let preview_bg = if fill.contains("eff6ff") || fill.contains("blue") {
        Color::from_rgba(0.2, 0.5, 0.9, 0.10)
    } else if fill.contains("f0fdf4") || fill.contains("green") {
        Color::from_rgba(0.1, 0.7, 0.3, 0.10)
    } else if fill.contains("fffbeb") || fill.contains("amber") {
        Color::from_rgba(0.9, 0.6, 0.1, 0.10)
    } else if fill.contains("fef2f2") || fill.contains("red") {
        Color::from_rgba(0.9, 0.2, 0.2, 0.10)
    } else if fill.contains("none") {
        Color::TRANSPARENT
    } else {
        Color::from_rgba(0.5, 0.5, 0.5, 0.05)
    };

    let preview_border = if stroke.contains("3b82f6") || stroke.contains("blue") {
        Color::from_rgb(0.23, 0.51, 0.96)
    } else if stroke.contains("10b981") || stroke.contains("green") {
        Color::from_rgb(0.06, 0.72, 0.50)
    } else if stroke.contains("f59e0b") || stroke.contains("amber") {
        Color::from_rgb(0.96, 0.62, 0.04)
    } else if stroke.contains("ef4444") || stroke.contains("red") {
        Color::from_rgb(0.94, 0.27, 0.27)
    } else if stroke.contains("none") {
        Color::TRANSPARENT
    } else {
        theme.border_color()
    };

    let preview = container(
        text(content)
            .size(12)
            .wrapping(iced::widget::text::Wrapping::Word)
            .color(theme.text_primary()),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(preview_bg)),
            border: Border {
                color: preview_border,
                width: if stroke.contains("none") {
                    0.0
                } else {
                    1.5
                },
                radius: border::Radius::from(6.0),
            },
            ..container::Style::default()
        }
    });

    column![
        preset_row,
        Space::new().height(4),
        props_grid,
        Space::new().height(6),
        text("Box Body Content:")
            .size(12)
            .color(theme.text_secondary()),
        editor_widget,
        Space::new().height(4),
        text("Live Styled Preview:")
            .size(11)
            .color(theme.text_muted()),
        preview,
    ]
    .spacing(6)
    .into()
}

/// Render link editor with protocol presets, label and URL inputs, and interactive preview
fn render_modal_link_editor<'a>(
    theme: AppTheme,
    url: &'a str,
    label: &'a str,
) -> Element<'a, Message> {
    let make_proto_btn = |proto: &'static str| {
        let u_str = url.to_string();
        button(text(proto).size(10))
            .style(move |_t, _s| theme::subtle_button_style(theme, false))
            .padding([2, 6])
            .on_press(Message::ModalLinkUpdateUrl(
                if u_str.starts_with("http") || u_str.starts_with("mailto") {
                    format!(
                        "{proto}{}",
                        u_str
                            .split_once("://")
                            .map_or(u_str.as_str(), |(_, rest)| rest)
                    )
                } else {
                    format!("{proto}{u_str}")
                },
            ))
    };

    let proto_row = row![
        text("Protocols:").size(10).color(theme.text_muted()),
        make_proto_btn("https://"),
        make_proto_btn("http://"),
        make_proto_btn("mailto:"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let url_input = text_input("Target URL (e.g. https://typst.app)", url)
        .size(13)
        .padding([8, 10])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalLinkUpdateUrl);

    let label_input = text_input("Display text label (e.g. Typst Official Site)", label)
        .size(13)
        .padding([8, 10])
        .style(move |_t, _s| modal_input_style(theme))
        .on_input(Message::ModalLinkUpdateLabel);

    let display_title = if label.trim().is_empty() {
        url
    } else {
        label
    };
    let preview_card = container(
        row![
            text("[Link]").size(12).color(theme.accent()),
            column![
                text(display_title).size(12).color(theme.accent()),
                text(url)
                    .size(10)
                    .font(iced::Font::MONOSPACE)
                    .color(theme.text_muted()),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            button(text("Test URL").size(11))
                .style(move |_t, _s| theme::subtle_button_style(theme, false))
                .padding([4, 8])
                .on_press(Message::OpenUrl(url.to_string())),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([10, 14])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(Color::from_rgba(0.2, 0.5, 0.9, 0.06))),
            border: Border {
                color: Color::from_rgba(0.2, 0.5, 0.9, 0.3),
                width: 1.0,
                radius: border::Radius::from(6.0),
            },
            ..container::Style::default()
        }
    });

    column![
        proto_row,
        Space::new().height(4),
        text("Target URL:").size(12).color(theme.text_secondary()),
        url_input,
        Space::new().height(4),
        text("Display Label:")
            .size(12)
            .color(theme.text_secondary()),
        label_input,
        Space::new().height(6),
        text("Live Link Card Preview:")
            .size(11)
            .color(theme.text_muted()),
        preview_card,
    ]
    .spacing(6)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn render_modal_title_slide_editor<'a>(
    theme: AppTheme,
    title: &'a str,
    subtitle: &'a str,
    author: &'a str,
    date: &'a str,
    version: &'a str,
    institution: &'a str,
    extra_args: &'a [(String, String)],
    title_slide_editor: Option<&'a text_editor::Content>,
) -> Element<'a, Message> {
    // 1. Style & Date Presets
    let presets_header = text("Title Slide Presets:")
        .size(11)
        .color(theme.text_secondary());

    let keynote_btn = button(text("Tech Keynote").size(10))
        .padding([2, 7])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideApplyPreset {
            title: None,
            subtitle: None,
            author: None,
            date: Some("2026".to_string()),
            version: Some("v0.2.0".to_string()),
            institution: None,
        });

    let academic_btn = button(text("Academic / Lab").size(10))
        .padding([2, 7])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideApplyPreset {
            title: None,
            subtitle: None,
            author: None,
            date: Some("2026".to_string()),
            version: Some("v1.0".to_string()),
            institution: Some("University / Lab".to_string()),
        });

    let business_btn = button(text("Business Strategy").size(10))
        .padding([2, 7])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideApplyPreset {
            title: None,
            subtitle: None,
            author: Some("Strategy Team".to_string()),
            date: Some("Q4 2026".to_string()),
            version: None,
            institution: Some("Enterprise Corp".to_string()),
        });

    let minimalist_btn = button(text("Minimalist").size(10))
        .padding([2, 7])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideApplyPreset {
            title: None,
            subtitle: None,
            author: None,
            date: None,
            version: Some(String::new()),
            institution: Some(String::new()),
        });

    let date_2026_btn = button(text("2026").size(10))
        .padding([2, 6])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideUpdateDate("2026".to_string()));

    let date_today_btn = button(text("Today").size(10))
        .padding([2, 6])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideUpdateDate("2026-10-04".to_string()));

    let date_spring_btn = button(text("Spring 2026").size(10))
        .padding([2, 6])
        .style(move |_t, _s| theme::subtle_button_style(theme, false))
        .on_press(Message::ModalTitleSlideUpdateDate(
            "Spring 2026".to_string(),
        ));

    let presets_row = row![
        presets_header,
        keynote_btn,
        academic_btn,
        business_btn,
        minimalist_btn,
        Space::new().width(8),
        date_2026_btn,
        date_today_btn,
        date_spring_btn,
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    // 2. Main Title Field
    let title_label = text("Slide Title *").size(12).color(theme.accent());
    let title_input = text_input("Enter presentation title...", title)
        .size(14)
        .padding([6, 10])
        .on_input(Message::ModalTitleSlideUpdateTitle);

    // 3. Subtitle Field
    let subtitle_label = text("Subtitle / Tagline")
        .size(12)
        .color(theme.text_secondary());
    let subtitle_input = text_input("Enter subtitle or abstract tagline...", subtitle)
        .size(13)
        .padding([5, 8])
        .on_input(Message::ModalTitleSlideUpdateSubtitle);

    // 4. Author & Date Row
    let author_label = text("Speaker / Author")
        .size(12)
        .color(theme.text_secondary());
    let author_input = text_input("Author or presenter name...", author)
        .size(13)
        .padding([5, 8])
        .on_input(Message::ModalTitleSlideUpdateAuthor);
    let author_col = column![author_label, author_input]
        .spacing(4)
        .width(Length::FillPortion(1));

    let date_label = text("Presentation Date")
        .size(12)
        .color(theme.text_secondary());
    let date_input = text_input("e.g. 2026 or October 2026...", date)
        .size(13)
        .padding([5, 8])
        .on_input(Message::ModalTitleSlideUpdateDate);
    let date_col = column![date_label, date_input]
        .spacing(4)
        .width(Length::FillPortion(1));

    let author_date_row = row![author_col, date_col].spacing(12).width(Length::Fill);

    // 5. Version & Institution Row
    let version_label = text("Version / Build")
        .size(12)
        .color(theme.text_secondary());
    let version_input = text_input("e.g. v1.0.0 or 0.2.0...", version)
        .size(13)
        .padding([5, 8])
        .on_input(Message::ModalTitleSlideUpdateVersion);
    let version_col = column![version_label, version_input]
        .spacing(4)
        .width(Length::FillPortion(1));

    let inst_label = text("Institution / Organization")
        .size(12)
        .color(theme.text_secondary());
    let inst_input = text_input("e.g. Cargo Slide Team / Research Lab...", institution)
        .size(13)
        .padding([5, 8])
        .on_input(Message::ModalTitleSlideUpdateInstitution);
    let inst_col = column![inst_label, inst_input]
        .spacing(4)
        .width(Length::FillPortion(1));

    let ver_inst_row = row![version_col, inst_col].spacing(12).width(Length::Fill);

    // 6. Custom Named Arguments
    let extra_header = row![
        text("Custom Named Arguments")
            .size(12)
            .color(theme.text_secondary()),
        Space::new().width(Length::Fill),
        button(text("+ Add Parameter").size(10))
            .padding([2, 8])
            .style(move |_t, _s| theme::primary_button_style(theme))
            .on_press(Message::ModalTitleSlideAddExtraArg),
    ]
    .align_y(Alignment::Center);

    let mut extra_col = column![extra_header].spacing(6).width(Length::Fill);
    for (i, (k, v)) in extra_args.iter().enumerate() {
        let k_input = text_input("Key (e.g. theme)", k)
            .size(12)
            .padding([4, 6])
            .width(Length::FillPortion(1))
            .on_input(move |nk| Message::ModalTitleSlideUpdateExtraKey(i, nk));
        let v_input = text_input("Value (e.g. dark)", v)
            .size(12)
            .padding([4, 6])
            .width(Length::FillPortion(2))
            .on_input(move |nv| Message::ModalTitleSlideUpdateExtraVal(i, nv));
        let del_btn = button(text("×").size(10))
            .padding([4, 6])
            .style(move |_t, _s| theme::danger_button_style(theme))
            .on_press(Message::ModalTitleSlideRemoveExtraArg(i));
        extra_col = extra_col.push(
            row![k_input, v_input, del_btn]
                .spacing(6)
                .align_y(Alignment::Center),
        );
    }

    // 7. Optional Content Block / Body
    let body_label = text("Optional Content Block [ ... ] (Markdown / Typst)")
        .size(12)
        .color(theme.text_secondary());
    let body_widget: Element<'a, Message> = if let Some(ed) = title_slide_editor {
        container(
            text_editor(ed)
                .padding([6, 8])
                .highlight_with::<TypstHighlighter>(
                    TypstHighlightSettings::default(),
                    to_typst_format,
                )
                .on_action(Message::ModalTitleSlideEditorAction),
        )
        .height(Length::Fixed(72.0))
        .width(Length::Fill)
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: border::rounded(4.0),
                ..container::Style::default()
            }
        })
        .into()
    } else {
        Space::new().height(0).into()
    };

    // 8. Live Visual Preview Card
    let prev_title = text(if title.trim().is_empty() {
        "Untitled Presentation"
    } else {
        title
    })
    .size(18)
    .color(theme.accent());

    let mut preview_items = column![prev_title]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::Fill);

    if !subtitle.trim().is_empty() {
        preview_items = preview_items.push(text(subtitle).size(12).color(theme.text_secondary()));
    }

    preview_items = preview_items.push(
        container(Space::new().height(1.0))
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.border_color())),
                    ..container::Style::default()
                }
            }),
    );

    let mut meta_badges = row![].spacing(10).align_y(Alignment::Center);
    if !author.trim().is_empty() {
        meta_badges = meta_badges.push(
            container(
                text(format!("Speaker: {author}"))
                    .size(10)
                    .color(theme.text_primary()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(4.0),
                    ..container::Style::default()
                }
            }),
        );
    }
    if !date.trim().is_empty() {
        meta_badges = meta_badges.push(
            container(
                text(format!("Date: {date}"))
                    .size(10)
                    .color(theme.text_secondary()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(4.0),
                    ..container::Style::default()
                }
            }),
        );
    }
    if !version.trim().is_empty() {
        meta_badges = meta_badges.push(
            container(
                text(format!("Ver: {version}"))
                    .size(10)
                    .color(theme.accent()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(4.0),
                    ..container::Style::default()
                }
            }),
        );
    }
    if !institution.trim().is_empty() {
        meta_badges = meta_badges.push(
            container(
                text(format!("Org: {institution}"))
                    .size(10)
                    .color(theme.text_secondary()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(4.0),
                    ..container::Style::default()
                }
            }),
        );
    }

    preview_items = preview_items.push(meta_badges);

    let preview_card = container(preview_items)
        .padding([12, 16])
        .width(Length::Fill)
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: border::Radius::from(6.0),
                },
                ..container::Style::default()
            }
        });

    let preview_section = column![
        text("Live Title Slide Visual Preview:")
            .size(11)
            .color(theme.text_secondary()),
        preview_card,
    ]
    .spacing(6)
    .width(Length::Fill);

    column![
        presets_row,
        column![title_label, title_input].spacing(4),
        column![subtitle_label, subtitle_input].spacing(4),
        author_date_row,
        ver_inst_row,
        extra_col,
        column![body_label, body_widget].spacing(4),
        preview_section,
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

/// Render dedicated Badge and Pill live editor
fn render_modal_badge_editor<'a>(
    theme: AppTheme,
    label: &'a str,
    fill: &'a str,
    text_color: &'a str,
) -> Element<'a, Message> {
    let preview_fill = if fill.contains("cyan") {
        Color::from_rgb(0.02, 0.71, 0.83)
    } else if fill.contains("purple") {
        Color::from_rgb(0.55, 0.36, 0.96)
    } else if fill.contains("orange") {
        Color::from_rgb(0.98, 0.45, 0.09)
    } else if fill.contains("green") || fill.contains("238636") {
        Color::from_rgb(0.14, 0.53, 0.21)
    } else if fill.contains("red") || fill.contains("ef4444") {
        Color::from_rgb(0.94, 0.27, 0.27)
    } else if fill.contains("accent") {
        theme.accent()
    } else {
        Color::from_rgb(0.12, 0.43, 0.92)
    };

    let preview_fg = if text_color.contains("1e293b") || text_color.contains("black") {
        Color::from_rgb(0.12, 0.16, 0.23)
    } else {
        Color::WHITE
    };

    let preview_chip = container(
        row![
            container(text("•").size(12).color(preview_fg.scale_alpha(0.8))).padding([0, 2]),
            text(if label.is_empty() {
                "Preview Badge"
            } else {
                label
            })
            .size(13)
            .color(preview_fg)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 14])
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(preview_fill)),
            border: Border {
                color: preview_fill.scale_alpha(0.8),
                width: 1.0,
                radius: border::Radius::from(999.0),
            },
            ..container::Style::default()
        }
    });

    let preview_section = container(
        column![
            text("Live Preview:").size(12).color(theme.text_secondary()),
            Space::new().height(4),
            preview_chip,
        ]
        .spacing(4),
    )
    .padding(14)
    .width(Length::Fill)
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(8.0),
            ..container::Style::default()
        }
    });

    let label_label = text("Badge Text / Label:")
        .size(13)
        .color(theme.text_secondary());
    let label_input = text_input("e.g. LaTeX Math, v1.0, Production Ready", label)
        .on_input(Message::ModalBadgeUpdateLabel)
        .padding(8);

    let color_btn = |val: &'static str, name: &'static str, bg_c: Color| {
        let is_sel = fill == val;
        button(
            row![
                container(
                    Space::new()
                        .width(Length::Fixed(10.0))
                        .height(Length::Fixed(10.0))
                )
                .style(move |_| {
                    container::Style {
                        background: Some(Background::Color(bg_c)),
                        border: border::rounded(2.0),
                        ..container::Style::default()
                    }
                }),
                text(name).size(12)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
        .padding([5, 10])
        .on_press(Message::ModalBadgeSetFill(val.to_string()))
    };

    let colors_label = text("Color / Theme Presets:")
        .size(13)
        .color(theme.text_secondary());
    let colors_row = row![
        color_btn(
            "slide-colors.accent",
            "Accent",
            Color::from_rgb(0.2, 0.45, 0.9)
        ),
        color_btn(
            "slide-colors.accent-cyan",
            "Cyan",
            Color::from_rgb(0.02, 0.71, 0.83)
        ),
        color_btn(
            "slide-colors.accent-purple",
            "Purple",
            Color::from_rgb(0.55, 0.36, 0.96)
        ),
        color_btn(
            "slide-colors.accent-orange",
            "Orange",
            Color::from_rgb(0.98, 0.45, 0.09)
        ),
        color_btn(
            "rgb(\"238636\")",
            "Green",
            Color::from_rgb(0.14, 0.53, 0.21)
        ),
        color_btn("rgb(\"ef4444\")", "Red", Color::from_rgb(0.94, 0.27, 0.27)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let fill_label = text("Custom Fill Expression:")
        .size(12)
        .color(theme.text_muted());
    let fill_input = text_input("e.g. rgb(\"1f6feb\") or slide-colors.accent", fill)
        .on_input(Message::ModalBadgeSetFill)
        .padding(6);

    let tc_btn = |tc_val: &'static str, tc_name: &'static str| {
        let is_sel = text_color == tc_val;
        button(text(tc_name).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 8])
            .on_press(Message::ModalBadgeSetTextColor(tc_val.to_string()))
    };

    let tc_row = row![
        text("Text Color:").size(12).color(theme.text_muted()),
        tc_btn("", "White (Default)"),
        tc_btn("rgb(\"1e293b\")", "Dark Navy"),
        tc_btn("rgb(\"000000\")", "Pure Black"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    column![
        preview_section,
        Space::new().height(6),
        label_label,
        label_input,
        Space::new().height(6),
        colors_label,
        colors_row,
        Space::new().height(4),
        fill_label,
        fill_input,
        Space::new().height(4),
        tc_row,
    ]
    .spacing(6)
    .into()
}

/// Render dedicated Video element visual editor and configuration dialog
#[allow(clippy::too_many_arguments)]
fn render_modal_video_editor<'a>(
    theme: AppTheme,
    source: &'a str,
    caption: &'a str,
    duration: &'a str,
    quality: &'a str,
    style: &'a str,
    width: &'a str,
) -> Element<'a, Message> {
    let display_title = if caption.trim().is_empty() {
        if source.trim().is_empty() {
            "Video Playback"
        } else {
            source.trim()
        }
    } else {
        caption.trim()
    };

    // Live preview card matching slide.typ styles
    let is_cinema = style == "cinema";
    let is_minimal = style == "minimal";

    let preview_card: Element<'a, Message> = if is_cinema {
        let rec_badge = container(
            text("● REC")
                .size(10)
                .color(Color::from_rgb(0.95, 0.25, 0.25)),
        )
        .padding([2, 6])
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.95, 0.25, 0.25, 0.2))),
                border: Border {
                    color: Color::from_rgb(0.95, 0.25, 0.25),
                    width: 1.0,
                    radius: border::Radius::from(3.0),
                },
                ..container::Style::default()
            }
        });

        let quality_badge = container(
            text(if quality.is_empty() {
                "4K 60FPS"
            } else {
                quality
            })
            .size(10)
            .color(theme.text_secondary()),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let mut right_meta = row![quality_badge].spacing(6).align_y(Alignment::Center);
        if !duration.trim().is_empty() {
            let dur_badge = container(
                text(format!("Duration: {}", duration.trim()))
                    .size(10)
                    .color(theme.text_secondary()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            });
            right_meta = right_meta.push(dur_badge);
        }

        let header_row = row![rec_badge, Space::new().width(Length::Fill), right_meta,]
            .align_y(Alignment::Center);

        let play_btn = container(text("▶").size(20).color(Color::WHITE))
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(44.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.98, 0.45, 0.09))),
                    border: border::rounded(22.0),
                    ..container::Style::default()
                }
            });

        let center_content = column![
            play_btn,
            Space::new().height(4),
            text(display_title).size(14).color(Color::WHITE),
            text("Click to launch FFmpeg hardware video player")
                .size(10)
                .color(theme.text_muted()),
        ]
        .spacing(4)
        .align_x(Alignment::Center);

        container(
            column![
                header_row,
                Space::new().height(10),
                center_content,
                Space::new().height(4),
            ]
            .spacing(4),
        )
        .padding(14)
        .width(Length::Fill)
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.03, 0.04, 0.06))),
                border: Border {
                    color: Color::from_rgb(0.98, 0.45, 0.09),
                    width: 1.5,
                    radius: border::Radius::from(8.0),
                },
                ..container::Style::default()
            }
        })
        .into()
    } else if is_minimal {
        let play_btn = container(text("▶").size(13).color(Color::WHITE))
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.accent())),
                    border: border::rounded(14.0),
                    ..container::Style::default()
                }
            });

        let play_pill = container(text("PLAY").size(10).color(theme.accent()))
            .padding([3, 8])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: border::rounded(4.0),
                    ..container::Style::default()
                }
            });

        let row_content = row![
            play_btn,
            column![
                text(display_title).size(13).color(theme.text_primary()),
                text("Interactive Video Stream")
                    .size(9)
                    .color(theme.text_muted()),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            play_pill,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(row_content)
            .padding([10, 14])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    } else {
        // Default "glass"
        let play_btn = container(text("▶").size(22).color(Color::WHITE))
            .width(Length::Fixed(46.0))
            .height(Length::Fixed(46.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.accent())),
                    border: border::rounded(23.0),
                    ..container::Style::default()
                }
            });

        let quality_pill = container(
            text(if quality.is_empty() {
                "4K 60FPS"
            } else {
                quality
            })
            .size(10)
            .color(theme.accent()),
        )
        .padding([2, 6])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(theme.bg_subtle())),
                border: Border {
                    color: theme.border_active(),
                    width: 0.8,
                    radius: border::Radius::from(3.0),
                },
                ..container::Style::default()
            }
        });

        let mut badges_row = row![quality_pill].spacing(6).align_y(Alignment::Center);
        if !duration.trim().is_empty() {
            let dur_pill = container(
                text(format!("Duration: {}", duration.trim()))
                    .size(10)
                    .color(theme.text_secondary()),
            )
            .padding([2, 6])
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                    border: border::rounded(3.0),
                    ..container::Style::default()
                }
            });
            badges_row = badges_row.push(dur_pill);
        }

        let center_content = column![
            play_btn,
            Space::new().height(4),
            text(display_title).size(14).color(theme.text_primary()),
            text("Hardware-accelerated presentation playback with FFmpeg")
                .size(10)
                .color(theme.text_muted()),
            Space::new().height(4),
            badges_row,
        ]
        .spacing(4)
        .align_x(Alignment::Center);

        container(center_content)
            .padding(16)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: theme.border_active(),
                        width: 1.5,
                        radius: border::Radius::from(8.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    };

    let preview_section = container(
        column![
            row![
                text("Live Card Preview:")
                    .size(12)
                    .color(theme.text_secondary()),
                Space::new().width(Length::Fill),
                text(format!("Width: {width}"))
                    .size(11)
                    .color(theme.text_muted()),
            ]
            .align_y(Alignment::Center),
            Space::new().height(4),
            preview_card,
        ]
        .spacing(4),
    )
    .padding(14)
    .width(Length::Fill)
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(8.0),
            ..container::Style::default()
        }
    });

    // Form inputs:
    let source_label = text("Video Source Path:")
        .size(13)
        .color(theme.text_secondary());
    let source_input = text_input("e.g. assets/demo.mp4, assets/screencast.webm", source)
        .on_input(Message::ModalVideoUpdateSource)
        .padding(8);

    let preset_btn = |p: &'static str| {
        let is_sel = source == p;
        button(text(p).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([3, 7])
            .on_press(Message::ModalVideoUpdateSource(p.to_string()))
    };

    let source_presets_row = row![
        text("Presets:").size(11).color(theme.text_muted()),
        preset_btn("assets/demo.mp4"),
        preset_btn("assets/screencast.mp4"),
        preset_btn("assets/intro.webm"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let caption_label = text("Caption / Title:")
        .size(12)
        .color(theme.text_secondary());
    let caption_input = text_input("e.g. Product Demo, Benchmark Walkthrough", caption)
        .on_input(Message::ModalVideoUpdateCaption)
        .padding(7);

    let duration_label = text("Duration:").size(12).color(theme.text_secondary());
    let duration_input = text_input("e.g. 01:42, 00:30, 05:00", duration)
        .on_input(Message::ModalVideoUpdateDuration)
        .padding(7);

    let meta_cols = row![
        column![caption_label, caption_input]
            .spacing(4)
            .width(Length::FillPortion(2)),
        column![duration_label, duration_input]
            .spacing(4)
            .width(Length::FillPortion(1)),
    ]
    .spacing(12);

    let style_btn = |st_val: &'static str, st_label: &'static str| {
        let is_sel = style == st_val || (style.is_empty() && st_val == "glass");
        button(text(st_label).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 10])
            .on_press(Message::ModalVideoSetStyle(st_val.to_string()))
    };

    let style_row = row![
        text("Visual Style:").size(12).color(theme.text_secondary()),
        style_btn("glass", "Glassmorphic Tech"),
        style_btn("cinema", "Cinematic Letterbox"),
        style_btn("minimal", "Minimal Pill"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let quality_btn = |q_val: &'static str| {
        let is_sel = quality == q_val || (quality.is_empty() && q_val == "4K 60FPS");
        button(text(q_val).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 8])
            .on_press(Message::ModalVideoSetQuality(q_val.to_string()))
    };

    let quality_row = row![
        text("Quality Badge:")
            .size(12)
            .color(theme.text_secondary()),
        quality_btn("4K 60FPS"),
        quality_btn("1080P 60FPS"),
        quality_btn("720P"),
        quality_btn("Source"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let width_btn = |w_val: &'static str| {
        let is_sel = width == w_val || (width.is_empty() && w_val == "90%");
        button(text(w_val).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 8])
            .on_press(Message::ModalVideoSetWidth(w_val.to_string()))
    };

    let width_row = row![
        text("Card Width:").size(12).color(theme.text_secondary()),
        width_btn("100%"),
        width_btn("90%"),
        width_btn("80%"),
        width_btn("70%"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    column![
        preview_section,
        Space::new().height(6),
        source_label,
        source_input,
        source_presets_row,
        Space::new().height(4),
        meta_cols,
        Space::new().height(4),
        style_row,
        Space::new().height(4),
        quality_row,
        Space::new().height(4),
        width_row,
    ]
    .spacing(6)
    .into()
}

/// Render dedicated Audio element visual editor and configuration dialog
#[allow(clippy::too_many_arguments)]
fn render_modal_audio_editor<'a>(
    theme: AppTheme,
    source: &'a str,
    is_player: bool,
    title: &'a str,
    artist: &'a str,
    autoplay: bool,
    loop_playback: bool,
    volume: f32,
) -> Element<'a, Message> {
    let display_title = if title.trim().is_empty() {
        "Background Music"
    } else {
        title.trim()
    };
    let display_artist = if artist.trim().is_empty() {
        "cargo-slide soundtrack"
    } else {
        artist.trim()
    };
    let vol_pct = (volume * 100.0).round() as usize;

    let preview_card: Element<'a, Message> = if is_player {
        let note_circle = container(
            text("BGM")
                .size(10)
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
        )
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(34.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.02, 0.71, 0.83, 0.2))),
                border: Border {
                    color: Color::from_rgb(0.02, 0.71, 0.83),
                    width: 1.0,
                    radius: border::Radius::from(17.0),
                },
                ..container::Style::default()
            }
        });

        // 5 simulated equalizer bars
        let bar = |h: f32| {
            container(Space::new())
                .width(Length::Fixed(3.0))
                .height(Length::Fixed(h))
                .style(|_| {
                    container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.02, 0.71, 0.83))),
                        border: border::rounded(1.0),
                        ..container::Style::default()
                    }
                })
        };

        let eq_stack = row![bar(7.0), bar(14.0), bar(9.0), bar(16.0), bar(6.0),]
            .spacing(2)
            .align_y(Alignment::End);

        let vol_pill = container(
            text(format!("Vol: {}%", vol_pct))
                .size(10)
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
        )
        .padding([2, 6])
        .style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.02, 0.71, 0.83, 0.15))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let auto_pill = container(
            text(if autoplay { "Auto" } else { "Manual" })
                .size(9)
                .color(theme.text_secondary()),
        )
        .padding([2, 5])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let loop_pill = container(
            text(if loop_playback {
                "Loop"
            } else {
                "Once"
            })
            .size(9)
            .color(theme.text_secondary()),
        )
        .padding([2, 5])
        .style(move |_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: border::rounded(3.0),
                ..container::Style::default()
            }
        });

        let right_cluster = row![eq_stack, vol_pill, auto_pill, loop_pill,]
            .spacing(6)
            .align_y(Alignment::Center);

        let card_body = row![
            note_circle,
            column![
                text(display_title).size(13).color(theme.text_primary()),
                text(display_artist).size(10).color(theme.text_secondary()),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            right_cluster,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        container(card_body)
            .padding([10, 14])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_card())),
                    border: Border {
                        color: Color::from_rgb(0.02, 0.71, 0.83),
                        width: 1.2,
                        radius: border::Radius::from(8.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    } else {
        // Compact background trigger
        let trigger_content = row![
            text("Vol").size(11),
            text(format!(
                "Background Audio Trigger: {}",
                if source.is_empty() {
                    "assets/ambient.mp3"
                } else {
                    source
                }
            ))
            .size(12)
            .color(theme.text_primary()),
            Space::new().width(Length::Fill),
            text(format!("Vol: {}%", vol_pct))
                .size(10)
                .color(Color::from_rgb(0.02, 0.71, 0.83)),
            text(if autoplay {
                "Autoplay"
            } else {
                "Manual"
            })
            .size(10)
            .color(theme.text_secondary()),
            text(if loop_playback {
                "Loop"
            } else {
                "Once"
            })
            .size(10)
            .color(theme.text_secondary()),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        container(trigger_content)
            .padding([8, 12])
            .width(Length::Fill)
            .style(move |_| {
                container::Style {
                    background: Some(Background::Color(theme.bg_subtle())),
                    border: Border {
                        color: theme.border_color(),
                        width: 1.0,
                        radius: border::Radius::from(6.0),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    };

    let preview_section = container(
        column![
            row![
                text("Live Preview:").size(12).color(theme.text_secondary()),
                Space::new().width(Length::Fill),
                text(if is_player {
                    "#audio-player card"
                } else {
                    "#audio hidden trigger"
                })
                .size(11)
                .color(theme.text_muted()),
            ]
            .align_y(Alignment::Center),
            Space::new().height(4),
            preview_card,
        ]
        .spacing(4),
    )
    .padding(14)
    .width(Length::Fill)
    .style(move |_| {
        container::Style {
            background: Some(Background::Color(theme.bg_subtle())),
            border: border::rounded(8.0),
            ..container::Style::default()
        }
    });

    let mode_btn = |player_mode: bool, label: &'static str| {
        let is_sel = is_player == player_mode;
        button(text(label).size(12))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([5, 12])
            .on_press(Message::ModalAudioSetIsPlayer(player_mode))
    };

    let mode_row = row![
        text("Component Type:")
            .size(12)
            .color(theme.text_secondary()),
        mode_btn(true, "Interactive Audio Player Card (#audio-player)"),
        mode_btn(false, "Background Soundtrack (#audio)"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let source_label = text("Audio File Path:")
        .size(13)
        .color(theme.text_secondary());
    let source_input = text_input("e.g. assets/soundtrack.mp3, assets/ambient.wav", source)
        .on_input(Message::ModalAudioUpdateSource)
        .padding(8);

    let preset_btn = |p: &'static str| {
        let is_sel = source == p;
        button(text(p).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([3, 7])
            .on_press(Message::ModalAudioUpdateSource(p.to_string()))
    };

    let source_presets_row = row![
        text("Presets:").size(11).color(theme.text_muted()),
        preset_btn("assets/soundtrack.mp3"),
        preset_btn("assets/ambient.wav"),
        preset_btn("assets/chime.ogg"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let title_input_widget = if is_player {
        let t_label = text("Track Title:").size(12).color(theme.text_secondary());
        let t_input = text_input("e.g. Keynote Soundtrack, Ambient Background", title)
            .on_input(Message::ModalAudioUpdateTitle)
            .padding(7);

        let a_label = text("Artist / Attribution:")
            .size(12)
            .color(theme.text_secondary());
        let a_input = text_input("e.g. cargo-slide soundtrack, Studio Orchestra", artist)
            .on_input(Message::ModalAudioUpdateArtist)
            .padding(7);

        row![
            column![t_label, t_input]
                .spacing(4)
                .width(Length::FillPortion(1)),
            column![a_label, a_input]
                .spacing(4)
                .width(Length::FillPortion(1)),
        ]
        .spacing(12)
    } else {
        row![]
    };

    let auto_btn = button(
        text(if autoplay {
            "Autoplay: ON"
        } else {
            "Autoplay: OFF"
        })
        .size(11),
    )
    .style(move |_t, _s| theme::subtle_button_style(theme, autoplay))
    .padding([5, 10])
    .on_press(Message::ModalAudioToggleAutoplay);

    let loop_btn = button(
        text(if loop_playback {
            "Loop: ON"
        } else {
            "Loop: OFF"
        })
        .size(11),
    )
    .style(move |_t, _s| theme::subtle_button_style(theme, loop_playback))
    .padding([5, 10])
    .on_press(Message::ModalAudioToggleLoop);

    let vol_btn = |val: f32, label: &'static str| {
        let is_sel = (volume - val).abs() < 0.05;
        button(text(label).size(11))
            .style(move |_t, _s| theme::subtle_button_style(theme, is_sel))
            .padding([4, 8])
            .on_press(Message::ModalAudioSetVolume(val))
    };

    let playback_row = row![
        text("Playback Options:")
            .size(12)
            .color(theme.text_secondary()),
        auto_btn,
        loop_btn,
        Space::new().width(Length::Fixed(8.0)),
        text("Volume:").size(12).color(theme.text_secondary()),
        vol_btn(0.2, "20%"),
        vol_btn(0.5, "50%"),
        vol_btn(0.8, "80%"),
        vol_btn(1.0, "100%"),
        vol_btn(0.0, "Mute"),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut col = column![
        preview_section,
        Space::new().height(6),
        mode_row,
        Space::new().height(4),
        source_label,
        source_input,
        source_presets_row,
        Space::new().height(4),
    ]
    .spacing(6);

    if is_player {
        col = col.push(title_input_widget);
        col = col.push(Space::new().height(4));
    }

    col = col.push(playback_row);

    col.into()
}

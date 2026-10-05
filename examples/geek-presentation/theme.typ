// Base theme and styling for cargo-slide presentations
#import "slide.typ": *

#let slide-theme(
  aspect-ratio: "16-9",
  theme: "dark",
  font: none,
  code-font: none,
  header: none,
  header-right: none,
  footer: none,
  footer-right: auto,
  body
) = {
  let page-width = 28cm
  let page-height = 15.75cm
  
  if aspect-ratio == "4-3" {
    page-width = 21cm
    page-height = 15.75cm
  } else if aspect-ratio == "16-10" {
    page-width = 25.2cm
    page-height = 15.75cm
  } else if aspect-ratio == "3-2" {
    page-width = 23.625cm
    page-height = 15.75cm
  }
  
  let bg-color = if theme == "light" { rgb("f6f8fa") } else { rgb("0f111a") }
  let fg-color = if theme == "light" { rgb("1f2328") } else { rgb("e6edf3") }

  let footer-left-content = if footer != none and footer != auto and footer != "" {
    [#text(weight: "bold")[#footer]]
  } else {
    none
  }

  let has-footer = footer-left-content != none or (footer-right != none and footer-right != "")
  let footer-content = if has-footer {
    context [
      #set text(size: 9pt, fill: rgb("8b949e"))
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [
          #if footer-left-content != none { footer-left-content }
        ],
        [
          #if footer-right == auto {
            counter(page).display("1 / 1", both: true)
          } else if footer-right != none and footer-right != "" {
            footer-right
          }
        ]
      )
    ]
  } else {
    none
  }

  let has-header = (header != none and header != "") or (header-right != none and header-right != "")
  let header-content = if has-header {
    context [
      #set text(size: 8.5pt, fill: rgb("8b949e"))
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [
          #if header != none and header != "" { header }
        ],
        [
          #if header-right != none and header-right != "" { header-right }
        ]
      )
      #v(0.15cm)
    ]
  } else {
    none
  }

  set page(
    width: page-width,
    height: page-height,
    margin: (x: 1.6cm, top: if has-header { 1.3cm } else { 0.9cm }, bottom: if has-footer { 0.9cm } else { 0.6cm }),
    fill: bg-color,
    header: header-content,
    footer: footer-content,
  )

  let default-fonts = (
    "Noto Sans",
    "Segoe UI",
    "SF Pro Display",
    "SF Pro Text",
    "Helvetica Neue",
    "Cantarell",
    "Arial",
    "PingFang SC",
    "Microsoft YaHei",
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "Liberation Sans",
    "DejaVu Sans",
  )

  let active-fonts = if font != none {
    if type(font) == array { font }
    else { (font,) }
  } else {
    default-fonts
  }

  set text(
    font: active-fonts,
    size: 11.5pt,
    fill: fg-color,
  )

  let default-code-fonts = (
    "DejaVu Sans Mono",
    "Consolas",
    "SF Mono",
    "Cascadia Code",
    "Liberation Mono",
    "Menlo",
    "Courier New",
  )

  let active-code-fonts = if code-font != none {
    if type(code-font) == array { code-font }
    else { (code-font,) }
  } else {
    default-code-fonts
  }

  // Math formula styling
  show math.equation: set text(weight: "regular")
  
  // Link styling
  show link: it => text(fill: rgb("58a6ff"))[#it]

  // Raw code block styling
  show raw: set text(font: active-code-fonts)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: rgb("161b22"),
    inset: 9pt,
    radius: 6pt,
    stroke: 1pt + rgb("30363d"),
    [
      #set text(size: 10pt)
      #it
    ]
  )

  body
}

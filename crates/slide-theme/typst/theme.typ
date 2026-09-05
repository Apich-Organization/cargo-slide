// Base theme and styling for cargo-slide presentations
#import "slide.typ": *

#let slide-theme(
  aspect-ratio: "16-9",
  theme: "dark",
  body
) = {
  let page-width = 28cm
  let page-height = 15.75cm
  
  if aspect-ratio == "4-3" {
    page-width = 21cm
    page-height = 15.75cm
  }
  
  let bg-color = if theme == "light" { rgb("f6f8fa") } else { rgb("0f111a") }
  let fg-color = if theme == "light" { rgb("1f2328") } else { rgb("e6edf3") }

  set page(
    width: page-width,
    height: page-height,
    margin: (x: 1.6cm, top: 1.1cm, bottom: 1.0cm),
    fill: bg-color,
    footer: context [
      #set text(size: 9pt, fill: rgb("8b949e"))
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [
          #text(weight: "bold")[cargo-slide]
        ],
        [
          #counter(page).display("1 / 1", both: true)
        ]
      )
    ]
  )

  set text(
    font: ("Noto Sans", "Open Sans", "Cantarell", "Adwaita Sans", "DejaVu Sans Mono"),
    size: 13pt,
    fill: fg-color,
  )

  // Math formula styling
  show math.equation: set text(weight: "regular")
  
  // Link styling
  show link: it => text(fill: rgb("58a6ff"))[#it]

  // Raw code block styling
  show raw.where(block: true): it => block(
    width: 100%,
    fill: rgb("161b22"),
    inset: 10pt,
    radius: 6pt,
    stroke: 1pt + rgb("30363d"),
    [
      #set text(size: 11pt)
      #it
    ]
  )

  body
}

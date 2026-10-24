// Source of overview-light.png / overview-dark.png (README illustration).
// Render with `cargo run -p runaround-harness --bin render-docs`.
#import "/src/lib.typ": runaround, wrap
#let dark = sys.inputs.at("theme", default: "light") == "dark"
#set page(width: 15cm, height: 12.5cm, margin: 6mm, fill: if dark { rgb("#0d1117") } else { white })
#set text(size: 8pt, fill: if dark { rgb("#e6edf3") } else { black })
#set par(justify: true)
#show: runaround

#columns(2, gutter: 6mm)[
  #wrap(mode: "after")[
    #figure(rect(width: 2.2cm, height: 1.8cm, fill: gradient.linear(aqua, blue)), caption: [A figure.])
  ]
  #lorem(70)

  #wrap(mode: "tight", align: right)[#circle(radius: 1.1cm, fill: orange)]
  #lorem(60)

  #wrap(mode: "tight", align: center)[#image(path("/docs/star.png"), width: 2.6cm)]
  #lorem(80)

  #wrap(mode: "before")[#rect(width: 2cm, height: 2.2cm, radius: 3pt, fill: green)]
  #lorem(30)
]

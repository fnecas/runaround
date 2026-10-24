// Source of modes-light.png / modes-dark.png (README illustration): one page
// per mode, composed into a grid. Render with
// `cargo run -p runaround-harness --bin render-docs`.
#import "/src/lib.typ": runaround, wrap
#let dark = sys.inputs.at("theme", default: "light") == "dark"
#set page(width: 6.4cm, height: 6.2cm, margin: 4mm, fill: if dark { rgb("#0d1117") } else { white })
#set text(size: 6pt, fill: if dark { rgb("#e6edf3") } else { black })
#set par(justify: true)
#show: runaround.with(gap: 0.8em, min-width: 3em)

#let obj(c) = rect(width: 1.8cm, height: 1.5cm, fill: c, radius: 2pt)
#let title(m) = text(weight: "bold", size: 7pt, raw(m))

#for (mode, shape) in (
  ("none", obj(red)),
  ("before", obj(blue)),
  ("after", obj(blue)),
  ("parallel", obj(purple)),
  ("through", obj(yellow.transparentize(30%))),
  ("tight", circle(radius: 0.85cm, fill: orange)),
) [
  #title(mode) \
  #lorem(14)
  #wrap(mode: mode)[#shape]
  #lorem(70)
  #pagebreak(weak: true)
]

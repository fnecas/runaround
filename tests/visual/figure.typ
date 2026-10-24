// Figures: captions wrap to the body width; references work.
#import "/src/lib.typ": runaround, wrap
#set page(width: 11cm, height: auto, margin: 8mm)
#set text(size: 9pt)
#set par(justify: true, first-line-indent: 1em)
#show: runaround

#lorem(25)

#wrap(mode: "after")[
  #figure(rect(width: 3cm, height: 2.4cm, fill: gradient.linear(red, blue)), caption: [A gradient that has a long caption.]) <grad>
]
As @grad shows, #lorem(55)

#lorem(40)

#wrap(mode: "before", gap: (x: 6mm, y: 3mm))[
  #figure(image("/tests/assets/dot.png", width: 30%), caption: [Relative width.])
]
#lorem(70)

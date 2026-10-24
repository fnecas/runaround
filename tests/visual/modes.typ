// One example per mode on small pages.
#import "/src/lib.typ": runaround, wrap
#set page(width: 10cm, height: auto, margin: 8mm)
#set text(size: 9pt)
#set par(justify: true)
#show: runaround

#let box-obj(w, h, c) = rect(width: w, height: h, fill: c, radius: 2pt)

*none* (default) \
#wrap[#box-obj(2.5cm, 1.2cm, red)]
#lorem(30)

*after* \
#wrap(mode: "after")[#box-obj(2.5cm, 2cm, red)]
#lorem(60)

*before* \
#wrap(mode: "before")[#box-obj(2.5cm, 2cm, blue)]
#lorem(60)

*after, centred object* \
#wrap(mode: "after", align: center)[#box-obj(2.5cm, 1.5cm, green)]
#lorem(50)

*through* \
#wrap(mode: "through")[#box-obj(5cm, 1.5cm, yellow)]
#lorem(40)

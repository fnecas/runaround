// Mode "tight": text follows contours of shapes, images and figures.
#import "/src/lib.typ": runaround, wrap
#set page(width: 11cm, height: auto, margin: 8mm)
#set text(size: 9pt)
#set par(justify: true)
#show: runaround.with(mode: "tight", gap: 4pt)

#wrap[#circle(radius: 1.4cm, fill: red)]
#lorem(80)

#wrap(align: right)[#polygon(fill: blue, (0cm, 0cm), (3cm, 1.2cm), (0.5cm, 3cm))]
#lorem(80)

#wrap(align: center)[#image(path("/tests/assets/star.png"), width: 3.2cm)]
#lorem(90)

#wrap[#figure(ellipse(width: 3cm, height: 2cm, fill: green), caption: [An ellipse.])]
#lorem(70)

#wrap(align: right, contour: (y) => (50% * (1 - y), 100%))[#rect(width: 3cm, height: 2.5cm, fill: gradient.linear(white, purple, angle: 0deg))]
#lorem(60)

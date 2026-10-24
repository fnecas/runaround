// auto-wrap: figures wrap without being marked; a function decides per element;
// wrap(mode: "none") opts out; floating figures stay floats.
#import "/src/lib.typ": runaround, wrap
#set page(width: 10cm, height: auto, margin: 8mm)
#set text(size: 9pt)
#set par(justify: true)
#show: runaround.with(auto-wrap: (
  figure: it => if it.body.func() == rect { (mode: "after") } else if it.body.func() == circle { (mode: "tight", align: right) },
))

#figure(rect(width: 2.5cm, height: 1.8cm, fill: teal), caption: [Automatic.])
#lorem(50)

#figure(circle(radius: 1cm, fill: orange), caption: [Automatic, tight.])
#lorem(50)

#wrap(mode: "none")[#figure(rect(width: 2.5cm, height: 1cm, fill: gray), caption: [Opted out.])]
#lorem(20)

#figure(ellipse(width: 2cm, height: 1cm), caption: [Not matched by the function.])
#lorem(20)

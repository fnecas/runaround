// An object that does not fit at the bottom of a page moves to the next one,
// and text fills the space it leaves.
#import "/src/lib.typ": runaround, wrap
#set page(width: 10cm, height: 9cm, margin: 8mm)
#set text(size: 9pt)
#set par(justify: true)
#show: runaround

#lorem(120)

#wrap(mode: "after")[#rect(width: 3cm, height: 3.5cm, fill: red)]
#lorem(110)

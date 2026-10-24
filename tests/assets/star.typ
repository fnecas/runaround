// Source of star.png (a cut-out with transparency, for contour tests):
// typst compile --ppi 100 tests/assets/star.typ tests/assets/star.png
#set page(width: 4cm, height: 4cm, margin: 0pt, fill: none)
#let pts = range(10).map(i => {
  let r = if calc.rem(i, 2) == 0 { 2cm } else { 0.85cm }
  let a = 90deg + i * 36deg
  (2cm + r * calc.cos(a), 2cm - r * calc.sin(a))
})
#polygon(fill: orange, ..pts)

// The scanner and markers.
#import "/src/lib.typ": runaround, wrap
#import "/src/marker.typ": as-wrap-marker

// Documents without wrap objects are returned unchanged.
#let doc = [= Title
  #set text(red)
  Some text. #figure(rect(), caption: [c])
  - a list
]
#assert(runaround(doc) == doc)
#assert(runaround.with(gap: 2pt)(doc) == doc)

// A marker carries its options and body.
#let m = wrap(mode: "after", gap: 3pt)[#rect(width: 1cm)]
#let info = as-wrap-marker(m)
#assert.eq(info.opts, (mode: "after", gap: 3pt))
#assert.eq(info.body.func(), rect)
#assert.eq(as-wrap-marker([x]), none)

// Without the show rule, wrap renders the object as a block.
#context assert(measure(wrap[#rect(width: 1cm, height: 1cm)]).height >= 1cm)

// With the show rule, the document changes.
#assert(runaround([#wrap(mode: "after")[#rect()] text]) != [#wrap(mode: "after")[#rect()] text])

// Auto-wrap only affects matching, non-floating elements.
#let figs = [#figure(rect()) text #figure(rect(), placement: top)]
#assert(runaround.with(auto-wrap: (table: "after"))(figs) == figs)
#assert(runaround.with(auto-wrap: (figure: it => none))(figs) == figs)
#assert(runaround.with(auto-wrap: (figure: "after"))(figs) != figs)

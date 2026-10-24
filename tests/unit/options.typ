// Option validation and resolution.
#import "/src/options.typ": *

#let all = defaults
#assert.eq(check-named("t", (mode: "after", gap: (x: 2pt), align: right)), (mode: "after", gap: (x: 2pt), align: right))

#context {
  let r = resolve(defaults + (mode: "after"), 300pt)
  assert.eq(r.align, left)
  assert.eq(r.gap, (x: 1em.to-absolute(), y: 1em.to-absolute()))
  assert.eq(r.min-width, 6em.to-absolute())

  assert.eq(resolve(defaults + (mode: "before"), 300pt).align, right)
  assert.eq(resolve(defaults + (mode: "parallel"), 300pt).align, center)
  assert.eq(resolve(defaults + (mode: "none"), 300pt).align, center)
  assert.eq(resolve(defaults + (mode: "after", align: right), 300pt).align, right)
  assert.eq(resolve(defaults + (min-width: 20%), 300pt).min-width, 60pt)
  assert.eq(resolve(defaults + (dx: 10% + 5pt), 300pt).dx, 35pt)
  assert.eq(resolve(defaults + (gap: (y: 3pt)), 300pt).gap, (x: 1em.to-absolute(), y: 3pt))
}

// start/end follow the text direction.
#context assert.eq(physical(start), left)
#context assert.eq(physical(end), right)
#set text(dir: rtl)
#context assert.eq(physical(start), right)
#set text(dir: auto, lang: "ar")
#context assert.eq(physical(end), left)

// auto-wrap specs.
#assert.eq(check-auto-wrap((figure: "after")).len(), 1)
#assert.eq(spec-to-options("before"), (mode: "before"))
#assert.eq(spec-to-options(none), none)
#assert.eq(spec-to-options((mode: "after", gap: 1pt)), (mode: "after", gap: 1pt))

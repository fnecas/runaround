// Content plumbing: flatten/rebuild, trimming, splitting into pieces.
#import "/src/content.typ": *

// flatten + rebuild keeps text, styles and structure.
#let doc = [one #set text(red); two *three* #text(blue)[four five] six]
#let entries = flatten(doc)
#assert(entries.len() > 3)
#assert(entries.any(((c, s)) => s.len() == 2), message: "nested styles are tracked")
#let back = rebuild(entries)
#assert.eq(repr(back), repr(rebuild(flatten(back))), message: "rebuild is stable")
#context assert.eq(measure(back), measure(doc), message: "rebuild preserves layout")

// An empty stream rebuilds to empty content.
#assert.eq(rebuild(()), [])

// trim removes surrounding glue only.
#assert.eq(trim([ #rect() ]).func(), rect)
#assert.eq(trim([x]), [x])

// Text splits after spaces; the last word stays open.
#let toks = tokens-of(flatten([alpha beta gamma]))
#assert.eq(toks.map(t => t.c.text), ("alpha ", "beta ", "gamma"))
#assert.eq(toks.map(t => t.close), (true, true, false))

// Inline wrappers are split and re-wrapped.
#let toks = tokens-of(flatten([a *b c* d]))
#let pieces = pieces-of(toks)
#assert.eq(pieces.len(), 4)
#assert(pieces.at(1).first().c.func() == strong, message: "strong is re-wrapped")
#assert.eq(pieces.at(1).first().c.body.text, "b ")

// Links keep their destination.
#let toks = tokens-of(flatten([#link("https://typst.app")[two words]]))
#assert.eq(toks.len(), 2)
#assert.eq(toks.at(1).c.dest, "https://typst.app")

// CJK text breaks between characters.
#let toks = tokens-of(flatten([漢字かな]))
#assert.eq(toks.len(), 4)
#assert(toks.all(t => t.close))

// Unbreakable inline content stays attached to its word.
#let pieces = pieces-of(tokens-of(flatten([x$y$z w])))
#assert.eq(pieces.len(), 2)

// Tokens rebuild to the same layout as the original paragraph.
#let para = [Some *bold words* and _emphasis_ with #text(red)[colour] here.]
#context assert.eq(
  measure(block(width: 4cm, tokens-content(tokens-of(flatten(para))))),
  measure(block(width: 4cm, para)),
)

// Classification.
#assert(is-block(heading[x]))
#assert(is-block(raw("x", block: true)))
#assert(not is-block(raw("x")))
#assert(not is-block(strong[x]))
#assert(is-hard-stop(pagebreak()))
#assert(is-hard-stop(v(1fr)))
#assert(not is-hard-stop(v(1em)))
#assert(is-hard-stop(figure([x], placement: top)))
#assert(not is-hard-stop(figure([x])))

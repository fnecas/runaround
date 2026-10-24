// The `wrap` function and the markers it produces.

#import "options.typ": check-named, fail, physical
#import "content.typ": trim

#let marker-label = <runaround-wrap>

/// Marks an object around which the following text flows.
///
/// Requires `#show: runaround` once in the document; without it, the object is
/// shown as a plain block.
///
/// Every option defaults to `auto`, which inherits the value given to
/// `runaround` (or the built-in default).
#let wrap(..args) = {
  let pos = args.pos()
  if pos.len() != 1 {
    fail("wrap expects exactly one positional argument (the object to wrap), found " + str(pos.len()))
  }
  let body = pos.first()
  if type(body) == str { body = [#body] }
  if type(body) != content {
    fail("wrap: the object must be content, found " + str(type(body)))
  }
  let opts = check-named("wrap", args.named())
  let object = trim(body)
  if object.func() == figure and object.at("placement", default: none) != none {
    fail("wrap: a floating figure cannot be wrapped; remove its `placement` argument")
  }
  let fallback = context {
    let a = opts.at("align", default: auto)
    align(if a == auto { center } else { physical(a) }, body)
  }
  [#block(width: 100%, {
    metadata((opts: opts, body: object))
    fallback
  })#marker-label]
}

// The `(opts, body)` of a marker produced by `wrap`, or `none`.
#let as-wrap-marker(c) = {
  if c.func() != block or c.at("label", default: none) != marker-label { return none }
  let kids = c.body.children
  if kids.len() == 0 or kids.first().func() != metadata { return none }
  kids.first().value
}

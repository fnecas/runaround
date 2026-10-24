// The document scanner behind `#show: runaround`.

#import "options.typ": check-auto-wrap, check-named, defaults, fail, physical, spec-to-options, to-abs
#import "content.typ": flatten, rebuild, sequence
#import "marker.typ": as-wrap-marker
#import "group.typ": group

// The wrap request `(opts, body)` for a content value, or `none`.
#let as-marker(c, cfg) = {
  let m = as-wrap-marker(c)
  if m != none { return (opts: cfg.defaults + m.opts, body: m.body) }
  for (element, spec) in cfg.rules {
    if c.func() == element {
      // Floating figures stay floats.
      if element == figure and c.at("placement", default: none) != none { return none }
      let resolved = if type(spec) == function { spec(c) } else { spec }
      let opts = spec-to-options(resolved)
      if opts == none { return none }
      return (opts: cfg.defaults + opts, body: c)
    }
  }
  none
}

#let relabel(new, old) = {
  let lbl = old.at("label", default: none)
  if lbl == none { new } else { [#new#lbl] }
}

#let without(fields, ..keys) = {
  for key in keys.pos() { let _ = fields.remove(key, default: none) }
  fields
}

// Containers whose bodies are scanned too.
#let containers = (block, columns, align, pad, list.item, enum.item, quote)


// Rebuilds a container with a new body.
#let rebuild-container(c, body) = {
  let f = c.func()
  let fields = without(c.fields(), "body", "label")
  let new = if f == columns {
    let count = fields.remove("count", default: 2)
    columns(count, ..fields, body)
  } else if f == align {
    align(fields.alignment, body)
  } else if f == enum.item {
    if "number" in fields { enum.item(fields.number, body) } else { enum.item(body) }
  } else if f == list.item {
    list.item(body)
  } else {
    f(..fields, body)
  }
  relabel(new, c)
}

// Scans content; returns `(content, found)` where `found` tells whether any
// wrap object was found (otherwise `content` is the original value).
#let scan-content(body, cfg) = {
  let entries = flatten(body)
  let n = entries.len()

  // Scan the bodies of containers first.
  let found = false
  let i = 0
  while i < n {
    let (c, s) = entries.at(i)
    if c.func() in containers and c.at("body", default: none) != none {
      let (inner, changed) = scan-content(c.body, cfg)
      if changed {
        entries.at(i) = (rebuild-container(c, inner), s)
        found = true
      }
    }
    i += 1
  }

  let marks = entries.map(e => as-marker(e.first(), cfg))
  if marks.all(m => m == none) {
    return if found { (rebuild(entries), true) } else { (body, false) }
  }

  let out = ()
  let i = 0
  while i < n {
    let m = marks.at(i)
    let (c, s) = entries.at(i)
    if m == none {
      out.push((c, s))
      i += 1
      continue
    }
    let mode = m.opts.mode
    if mode == "none" or (mode == "through" and m.opts.behind) {
      // No text needs to move: replace the marker in place.
      let obj = m.body
      out.push((
        context {
          let a = physical(if m.opts.align == auto { center } else { m.opts.align })
          if mode == "none" or target() == "html" {
            align(a, obj)
          } else {
            place(a, dx: to-abs(m.opts.dx, 0pt), obj)
          }
        },
        s,
      ))
      i += 1
      continue
    }
    // The group takes everything up to the next marker.
    let j = i + 1
    while j < n and marks.at(j) == none { j += 1 }
    let candidate = entries.slice(i + 1, j)
    // The group lives at the deepest style level shared with its content.
    let k = s.len()
    for (_, cs) in candidate {
      let d = 0
      let limit = calc.min(k, cs.len())
      while d < limit and cs.at(d).id == s.at(d).id { d += 1 }
      k = d
    }
    let info = (
      opts: m.opts,
      body: rebuild(((m.body, s.slice(k)),)),
      entries: candidate.map(((cc, cs)) => (cc, cs.slice(k))),
    )
    out.push((group(info), s.slice(0, k)))
    i = j
  }
  (rebuild(out), true)
}

/// Makes the text after `wrap`ped objects (and elements matched by
/// `auto-wrap`) flow around them. Use once: `#show: runaround`.
///
/// Named arguments set defaults for every `wrap`. `auto-wrap` maps element
/// names to a mode, a dictionary of options, or a function returning either
/// (or `none` to skip the element), e.g. `auto-wrap: (figure: "after")`.
#let runaround(..args) = {
  let pos = args.pos()
  if pos.len() != 1 {
    fail("runaround is a show rule: use `#show: runaround` or `#show: runaround.with(..)`")
  }
  let named = args.named()
  let rules = check-auto-wrap(named.remove("auto-wrap", default: (:)))
  let cfg = (defaults: defaults + check-named("runaround", named), rules: rules)
  scan-content(pos.first(), cfg).first()
}

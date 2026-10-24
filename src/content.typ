// Content plumbing: flattening and rebuilding content trees, classifying
// elements, and splitting paragraphs at break opportunities.

#let sequence = ([a] + [b]).func()
#let styled = [#set text(red); x].func()
#let space = [a *b*].children.at(1).func()

// Elements that are laid out as blocks (they end a paragraph).
#let block-elements = (
  parbreak,
  heading,
  list.item,
  enum.item,
  terms.item,
  figure,
  block,
  table,
  grid,
  columns,
  align,
  pad,
  place,
  v,
  pagebreak,
  colbreak,
  image,
  rect,
  square,
  circle,
  ellipse,
  polygon,
  line,
  curve,
  stack,
  outline,
  bibliography,
  par,
)

// Elements that are block-level only when their `block` field is true.
#let maybe-block-elements = (raw, math.equation, quote)

#let is-block(c) = {
  let f = c.func()
  if f in block-elements { return true }
  if f in maybe-block-elements { return c.at("block", default: false) == true }
  false
}

// Content after which text must not continue beside an object: the beside
// area is an unbreakable container, and these only work in the normal flow.
#let is-hard-stop(c) = {
  let f = c.func()
  if f in (pagebreak, colbreak, place) { return true }
  if f == v { return type(c.amount) == fraction }
  if f == figure { return c.at("placement", default: none) != none }
  false
}

// Whether a content value is visually empty glue between blocks.
#let is-glue(c) = c.func() in (space, parbreak) or c == []

// Flattens sequences and styled wrappers into an array of `(child, stack)`
// entries, where `stack` lists the enclosing styles (outermost first) as
// `(s: styles, id: int)`. Ids identify styled elements so that `rebuild` can
// restore the exact structure; `base` offsets them to keep nested flattenings
// apart. Iterative, so deep `#set` nesting is fine.
#let flatten(body, base: 0) = {
  let out = ()
  let next-id = base
  let todo = ((body, ()),)
  while todo.len() > 0 {
    let (c, s) = todo.pop()
    let f = c.func()
    if f == sequence {
      let kids = c.children
      let i = kids.len()
      while i > 0 {
        i -= 1
        todo.push((kids.at(i), s))
      }
    } else if f == styled {
      todo.push((c.child, s + ((s: c.styles, id: next-id),)))
      next-id += 1
    } else {
      out.push((c, s))
    }
  }
  out
}

// Inverse of `flatten`: rebuilds content from `(child, stack)` entries.
#let rebuild(entries) = {
  if entries.len() == 0 { return [] }
  let ids = ()
  let styles = ()
  let frames = ((),)
  for (c, s) in entries {
    let k = 0
    let n = calc.min(ids.len(), s.len())
    while k < n and ids.at(k) == s.at(k).id { k += 1 }
    while ids.len() > k {
      let kids = frames.pop()
      let st = styles.pop()
      let _ = ids.pop()
      frames.at(-1).push(styled(sequence(kids), st))
    }
    while ids.len() < s.len() {
      let item = s.at(ids.len())
      ids.push(item.id)
      styles.push(item.s)
      frames.push(())
    }
    frames.at(-1).push(c)
  }
  while ids.len() > 0 {
    let kids = frames.pop()
    let st = styles.pop()
    let _ = ids.pop()
    frames.at(-1).push(styled(sequence(kids), st))
  }
  let top = frames.at(0)
  if top.len() == 1 { top.at(0) } else { sequence(top) }
}

// Removes leading and trailing glue (spaces, parbreaks) from content.
#let trim(body) = {
  if body.func() != sequence { return body }
  let kids = body.children
  let a = 0
  let b = kids.len()
  while a < b and is-glue(kids.at(a)) { a += 1 }
  while b > a and is-glue(kids.at(b - 1)) { b -= 1 }
  if b - a == 1 { kids.at(a) } else { sequence(kids.slice(a, b)) }
}

// --- Paragraph splitting ----------------------------------------------------

// Inline elements whose body can be split, re-wrapping each part.
#let wrappers = (strong, emph, underline, overline, strike, highlight, smallcaps, sub, super, link)

#let rewrap(c, body) = {
  if c.func() == link { return link(c.dest, body) }
  let fields = c.fields()
  let _ = fields.remove("body")
  let _ = fields.remove("label", default: none)
  (c.func())(..fields, body)
}

// Scripts that break between characters rather than at spaces.
#let cjk = regex("[\\p{Han}\\p{Hiragana}\\p{Katakana}\\p{Hangul}]")

// Tokens for a string fragment. `close` marks a break opportunity after the
// token.
#let string-tokens(str-value, s, close) = {
  if not str-value.contains(cjk) {
    return ((c: text(str-value), s: s, close: close),)
  }
  let out = ()
  let run = ""
  for ch in str-value.clusters() {
    run += ch
    if ch.contains(cjk) {
      out.push((c: text(run), s: s, close: true))
      run = ""
    }
  }
  if run != "" {
    out.push((c: text(run), s: s, close: close))
  } else if out.len() > 0 {
    out.at(-1).close = close or out.at(-1).close
  }
  out
}

// Splits paragraph entries into tokens `(c, s, close)`. Text is split after
// spaces (and between CJK characters); splittable inline wrappers are split
// recursively; everything else is an unbreakable token.
#let tokens-of(entries, base: 1000003) = {
  let out = ()
  for (index, (c, s)) in entries.enumerate() {
    let f = c.func()
    if f == text {
      let words = c.text.split(" ")
      let n = words.len()
      let has-cjk = c.text.contains(cjk)
      for (i, w) in words.enumerate() {
        if i < n - 1 {
          if has-cjk { out += string-tokens(w + " ", s, true) } else {
            out.push((c: text(w + " "), s: s, close: true))
          }
        } else if w != "" {
          if has-cjk { out += string-tokens(w, s, false) } else {
            out.push((c: text(w), s: s, close: false))
          }
        }
      }
    } else if f == space or f == linebreak {
      out.push((c: c, s: s, close: true))
    } else if f in wrappers and c.at("label", default: none) == none {
      let inner-base = (base + index + 1) * 7919
      for t in tokens-of(flatten(c.body, base: inner-base), base: inner-base) {
        out.push((c: rewrap(c, t.c), s: s + t.s, close: t.close))
      }
    } else {
      out.push((c: c, s: s, close: false))
    }
  }
  out
}

// Groups tokens into pieces: the units between two break opportunities.
#let pieces-of(tokens) = {
  let pieces = ()
  let current = ()
  for t in tokens {
    current.push(t)
    if t.close {
      pieces.push(current)
      current = ()
    }
  }
  if current.len() > 0 { pieces.push(current) }
  pieces
}

// Content of a run of tokens.
#let tokens-content(tokens) = rebuild(tokens.map(t => (t.c, t.s)))

// A function `(a, b) => content` for pieces `a` to `b` (exclusive). Plain paragraphs (only
// text and spaces, one style) are by far the most common: their prefixes are
// built as one string slice instead of rebuilding every token.
#let range-builder(pieces) = {
  let first-stack = if pieces.len() > 0 { pieces.first().first().s } else { () }
  let plain = pieces.all(p => p.all(t => (
    (t.c.func() == text or t.c.func() == space) and t.s == first-stack
  )))
  if not plain {
    return (a, b) => tokens-content(pieces.slice(a, b).join())
  }
  let piece-text(p) = p.map(t => if t.c.func() == text { t.c.text } else { " " }).join()
  let texts = pieces.map(piece-text)
  let full = texts.join()
  // offsets.at(k) is the byte offset where piece k starts.
  let offsets = (0,)
  for t in texts { offsets.push(offsets.at(-1) + t.len()) }
  (a, b) => if b <= a { [] } else { rebuild(((text(full.slice(offsets.at(a), offsets.at(b))), first-stack),)) }
}

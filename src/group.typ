// Layout of a wrap group: an object plus the content that follows it.

#import "options.typ": default-align, physical, resolve
#import "content.typ": is-glue, is-block, rebuild, tokens-content
#import "fill.typ": fill-column, fill-lines, tail-ids
#import "contour.typ": contour-for

#let probe-label = <runaround-probe>
#let bottom-label = <runaround-bottom>
#let decision-label = <runaround-decision>

// Probes emitted right before a group, so that the measurements cannot depend
// on the group's own output:
// - a zero-height block with no spacing above sits exactly where the
//   preceding content ends, and reports the container width;
// - `place(bottom + left, ..)` reports the bottom and height of the region.
// (A bare `place` would sit after the spacing of the next block only when
// that block stays on the page, which feeds back.)
#let probes = context {
  block(height: 0pt, width: 100%, above: 0pt, below: par.spacing, breakable: false, {
    layout(size => [#metadata(size.width)#probe-label])
  })
  place(bottom + left, layout(size => [#metadata(size.height)#bottom-label]))
}

// Text width of the page, used before the probes have been measured.
#let fallback-width() = {
  let w = if page.width == auto { 21cm } else { page.width }
  let h = if page.height == auto { 29.7cm } else { page.height }
  let m = page.margin
  let auto-margin = 2.5 / 21 * calc.min(w, h)
  let side(key) = {
    let v = if type(m) == dictionary {
      m.at(key, default: m.at("x", default: m.at("rest", default: auto)))
    } else { m }
    if v == auto { auto-margin } else if type(v) == ratio { v * w } else if type(
      v,
    ) == relative { v.ratio * w + v.length.to-absolute() } else { v.to-absolute() }
  }
  let body = w - side("left") - side("right")
  let n = page.columns
  if n <= 1 { return body }
  let g = columns.gutter
  let gutter = if type(g) == ratio { g * body } else if type(g) == relative {
    g.ratio * body + g.length.to-absolute()
  } else { g.to-absolute() }
  (body - (n - 1) * gutter) / n
}

// Measures the object and makes figure captions wrap to the body width.
// Returns the content to place, its visual size, and the horizontal shift
// between the placed content and its visible left edge.
#let prepare-object(body, o, width) = {
  if o.width != auto {
    let obj = block(width: o.width, body)
    let size = measure(obj, width: width)
    return (content: obj, w: size.width, h: size.height, shift: 0pt)
  }
  if body.func() == figure {
    // Narrow only the caption, so that a body sized relative to the container
    // keeps its size.
    let bw = measure(body.body, width: width).width
    let obj = {
      show figure.caption: it => block(width: bw, it)
      body
    }
    let size = measure(obj, width: width)
    // A figure wider than its body centres it: place it so the body lands at
    // the object position.
    let shift = if size.width > bw + 0.5pt { -(size.width - bw) / 2 } else { 0pt }
    return (content: obj, w: calc.min(size.width, bw), h: size.height, shift: shift)
  }
  let size = measure(body, width: width)
  (content: body, w: size.width, h: size.height, shift: 0pt)
}

// Paragraph metrics and other values the fill engine needs.
#let make-ctx(entries) = {
  let indent = par.first-line-indent
  if type(indent) == dictionary {
    indent = if indent.at("all", default: false) { 0pt } else { indent.amount }
  }
  (
    spacing: par.spacing.to-absolute(),
    leading: par.leading.to-absolute(),
    justify: par.justify,
    indent: indent.to-absolute(),
    eps: 0.01pt,
    line: measure([X]).height,
    tail: tail-ids(entries),
  )
}

// Content from a stream position on, in the normal flow. A paragraph split
// at the end of the beside area continues without indent and with normal line
// spacing; the paragraph after it gets back the indent it would have had.
#let rest-content(entries, pos, ctx) = {
  let rest = entries.slice(pos.i)
  if pos.head != none {
    block(above: 0pt, width: 100%, {
      set par(first-line-indent: 0pt)
      tokens-content(pos.head)
    })
    if ctx.indent > 0pt {
      let j = rest.position(e => not is-glue(e.first()))
      if j != none and not is-block(rest.at(j).first()) {
        rest.insert(j, (h(ctx.indent), rest.at(j).last()))
      }
    }
  }
  rebuild(rest)
}

// The sides of an object that receive text.
#let text-sides(o) = {
  if o.mode == "before" { ("left",) } else if o.mode == "after" { ("right",) } else if o.mode == "parallel" {
    ("left", "right")
  } else if o.align == left { ("right",) } else if o.align == right { ("left",) } else { ("left", "right") }
}

// Free segments `(x0, x1)` beside an object whose extent at some height is
// `(l, r)` (absolute, in the container). Sides narrower than `min-width` get
// no text.
#let free-segments(o, width, l, r) = {
  let sides = text-sides(o)
  let segments = ()
  if "left" in sides {
    let x1 = l - o.gap.x
    if x1 >= o.min-width { segments.push((x0: 0pt, x1: x1)) }
  }
  if "right" in sides {
    let x0 = r + o.gap.x
    if width - x0 >= o.min-width { segments.push((x0: x0, x1: width)) }
  }
  segments
}

// The object on its own line, text above and below (mode "none").
#let alone(obj, o, rest) = {
  let shifted = if o.dx != 0pt { move(dx: o.dx, obj.content) } else { obj.content }
  align(o.align, shifted)
  rest
}

// The object with text beside it, followed by the rest in the normal flow.
// `slots(y, h)` gives the free segments for a line at `y` of height `h`. A
// single segment of constant width (`column`) is filled as a column; anything
// else is filled line by line, read across the object.
#let beside(entries, start, obj, o, ox, slots, column, ctx) = {
  let zone = obj.h + o.gap.y
  let filled = if column != none {
    let col = fill-column(entries, start, column.x1 - column.x0, zone, ctx)
    let body = place(top + left, dx: column.x0, block(width: column.x1 - column.x0, col.content))
    (body: body, height: col.height, pos: col.pos, split: col.split)
  } else {
    let r = fill-lines(entries, start, slots, zone, ctx)
    let body = for l in r.lines {
      place(top + left, dx: l.x, dy: l.y, block(width: l.w, l.body))
    }
    (body: body, height: r.height, pos: r.pos, split: r.split)
  }
  // A split paragraph continues right below with line spacing; otherwise the
  // next block keeps its normal spacing, and the object its gap.
  let (height, below) = if filled.split {
    (calc.max(filled.height + ctx.leading, zone), 0pt)
  } else {
    (calc.max(filled.height, zone - ctx.spacing), ctx.spacing)
  }
  block(width: 100%, height: height, breakable: false, above: ctx.spacing, below: below, {
    place(top + left, dx: ox + obj.shift, obj.content)
    filled.body
  })
  rest-content(entries, filled.pos, ctx)
}

#let group-layout(info) = context {
  let entries = info.entries
  let probes = query(selector(probe-label).before(here()))
  let bottoms = query(selector(bottom-label).before(here()))
  let measured = probes.len() > 0 and bottoms.len() > 0
  let width = if measured { probes.last().value } else { fallback-width() }
  let region = if measured { bottoms.last().value } else { none }

  let o = resolve(info.opts, width)
  let ctx = make-ctx(entries)
  let obj = prepare-object(info.body, o, width)

  // Room left on the page below the start of the group.
  let avail = none
  if measured {
    let p = probes.last().location().position()
    let b = bottoms.last().location().position()
    if p.page == b.page and region != none {
      let at-top = p.y - (b.y - region) < 1pt
      avail = b.y - p.y - if at-top { 0pt } else { ctx.spacing }
    }
  }

  let ox = if o.align == left { 0pt } else if o.align == right { width - obj.w } else {
    (width - obj.w) / 2
  }
  ox += o.dx
  let contour = if o.mode == "tight" { contour-for(o.contour, info.body, obj.w, obj.h, width) } else { none }
  let box-segments = free-segments(o, width, ox, ox + obj.w)
  let slots = if contour == none {
    (y, h) => box-segments
  } else {
    (y, h) => {
      let e = contour(y - o.gap.y, y + h + o.gap.y)
      if e == none { ((x0: 0pt, x1: width),) } else { free-segments(o, width, ox + e.at(0), ox + e.at(1)) }
    }
  }
  let column = if contour == none and box-segments.len() == 1 { box-segments.first() } else { none }
  let start = (head: none, i: 0)

  if (region != none and obj.h > region) or (contour == none and box-segments.len() == 0) {
    // An object that cannot fit on any page, or leaves no room for text,
    // is shown on its own.
    alone(obj, o, rest-content(entries, start, ctx))
  } else {
    // If the object does not fit in the room left on the page, that room is
    // first filled with text at full width ("prefill"), and the object starts
    // the next page.
    //
    // Decisions are sticky across layout iterations: a group's decision
    // shifts the content after it, which could flip the decisions of later
    // groups, one wave per iteration. So each group records its decision:
    // - "open": not measured yet (the first iteration);
    // - "fit": final;
    // - "prefill": kept while its text still fits in the room left; otherwise
    //   it becomes "fit", which is always valid (at worst the object moves on
    //   and leaves a gap).
    // Each group changes at most twice, so documents converge in a few
    // iterations however many objects they have. (A cheaper scheme that lets
    // fitting objects stay "open" converges one iteration earlier in simple
    // documents, but cascades again in long ones.)
    //
    // Positions are only trusted when the previous iteration was laid out
    // with the width measured now (the first iteration guesses the width,
    // which is wrong inside containers such as `columns`): until then, the
    // group stays "open". A prefill is also dropped (for good) when the object
    // would fit below it, i.e. when it is no longer needed.
    let previous = {
      let after = query(selector(decision-label).after(here()))
      if after.len() > 0 { after.first().value } else { (kind: "open", width: none) }
    }
    let reliable = avail != none and previous.at("width", default: none) == width
    let decision = if not reliable and previous.kind == "open" {
      (kind: "open")
    } else if previous.kind == "open" {
      if obj.h > avail { (kind: "prefill", limit: calc.max(avail, 0pt)) } else { (kind: "fit") }
    } else if previous.kind == "prefill" and avail != none and (
      previous.height > avail + 0.5pt or avail - previous.height - ctx.spacing >= obj.h
    ) {
      (kind: "fit")
    } else {
      previous
    }
    decision.width = width
    let pre = if decision.kind == "prefill" {
      // Only paragraphs: a heading or list at the page bottom moves on with
      // the object, and their own spacing is not known here.
      fill-column(entries, start, width, decision.limit, ctx, blocks: false)
    } else { none }
    if pre != none { decision.height = pre.height }
    [#metadata(decision)#decision-label]
    if pre != none {
      pre.content
      start = pre.pos
    }
    beside(entries, start, obj, o, ox, slots, column, ctx)
  }
}

// HTML export has no page layout: float the object with CSS instead.
#let html-group(info) = {
  let o = info.opts
  let a = physical(if o.align == auto { default-align.at(o.mode) } else { o.align })
  let side = if o.mode == "after" or (o.mode == "tight" and a == left) {
    "left"
  } else if o.mode == "before" or (o.mode == "tight" and a == right) {
    "right"
  } else { none }
  if side == none {
    info.body
  } else {
    let g = if type(o.gap) == dictionary { o.gap } else { (x: o.gap, y: o.gap) }
    let pt(v) = str(calc.round(v.to-absolute().pt(), digits: 2)) + "pt"
    let x = pt(g.at("x", default: 1em))
    let y = pt(g.at("y", default: 1em))
    let margin = if side == "left" { "0 " + x + " " + y + " 0" } else { "0 0 " + y + " " + x }
    html.elem("div", attrs: (style: "float: " + side + "; margin: " + margin))[#info.body]
  }
  rebuild(info.entries)
}

// A wrap group: probes plus the contextual layout.
#let group(info) = context {
  if target() == "html" {
    html-group(info)
  } else {
    probes
    group-layout(info)
  }
}

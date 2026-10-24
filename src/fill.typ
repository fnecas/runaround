// Filling text into the free space beside obstacles.
//
// Performance matters here: every `measure` is a layout run that Typst keeps
// cached. Paragraphs that fit whole cost one measure; the paragraph crossing
// the bottom is split by estimate plus a short gallop and binary search over a
// bounded prefix. There are no linear scans and nothing outside the beside
// area is measured.

#import "content.typ": is-block, is-glue, is-hard-stop, pieces-of, range-builder, rebuild, tokens-content, tokens-of

// Ids of styled elements that start after the object and contain block-level
// content (`#set` rules at block level). Text inside them cannot go beside an
// object: the beside area is a container, and a hidden `#set page` there would
// be an error.
#let tail-ids(entries) = {
  let ids = (:)
  for (c, s) in entries {
    if s.len() > 0 and (is-block(c) or c.func() == parbreak) {
      for item in s { ids.insert(str(item.id), true) }
    }
  }
  ids
}

#let in-tail(s, ctx) = s.any(item => str(item.id) in ctx.tail)

// The next item of a stream position `(head, i)`. `head` holds the remaining
// tokens of a split paragraph. Items:
// - (kind: "par", tokens or entries, after)
// - (kind: "block", entry, heading, after)
// - (kind: "stop", at) and (kind: "end", at)
#let next-item(entries, pos, ctx) = {
  if pos.head != none {
    return (kind: "par", tokens: pos.head, entries: none, after: (head: none, i: pos.i))
  }
  let n = entries.len()
  let i = pos.i
  while i < n and is-glue(entries.at(i).first()) { i += 1 }
  if i >= n { return (kind: "end", at: (head: none, i: n)) }
  let (c, s) = entries.at(i)
  if is-hard-stop(c) or in-tail(s, ctx) { return (kind: "stop", at: (head: none, i: i)) }
  if is-block(c) {
    return (
      kind: "block",
      entry: (c, s),
      heading: c.func() == heading,
      after: (head: none, i: i + 1),
    )
  }
  let j = i + 1
  while j < n {
    let (c2, s2) = entries.at(j)
    if c2.func() == parbreak or is-block(c2) or is-hard-stop(c2) or in-tail(s2, ctx) { break }
    j += 1
  }
  (kind: "par", tokens: none, entries: entries.slice(i, j), after: (head: none, i: j))
}

#let item-content(item) = {
  if item.kind == "block" { return rebuild((item.entry,)) }
  if item.tokens != none { return tokens-content(item.tokens) }
  rebuild(item.entries)
}

#let height-of(body, width) = measure(block(width: width, body)).height

// Height with greedy line breaking, which is much cheaper than the optimized
// (Knuth–Plass) breaking of justified text. It is used to search; the result
// is then corrected with real measurements, because the two differ slightly
// (the optimized breaking also shrinks spaces and hyphenates more cleverly).
#let greedy-height-of(body, width) = measure(block(width: width, {
  set par(linebreaks: "simple")
  body
})).height

// Splits a paragraph so that its head fits into `avail` at `width`.
// Returns `(head, height, rest)` or `none` if not even one line fits.
#let split-par(item, width, avail, full-height, ctx) = {
  let tokens = if item.tokens != none { item.tokens } else { tokens-of(item.entries) }
  let pieces = pieces-of(tokens)
  let n = pieces.len()
  if n < 2 { return none }
  let lb = if ctx.justify { linebreak(justify: true) } else { [] }
  let part = range-builder(pieces)
  let prefix(k) = part(0, k)
  let height(k) = height-of(prefix(k) + lb, width)
  let fits(k) = greedy-height-of(prefix(k) + lb, width) <= avail + ctx.eps

  // Estimate from line counts: lines that fit, and words per line.
  let line = height-of(prefix(1), width)
  let lead = ctx.leading
  let lines-fit = calc.floor((avail + lead) / (line + lead) + 1e-6)
  if lines-fit < 1 { return none }
  let lines-full = calc.max(1, calc.round((full-height + lead) / (line + lead)))
  let per-line = calc.max(1, calc.floor(n / lines-full))
  let k = calc.clamp(calc.floor(n * lines-fit / lines-full), 1, n - 1)

  // Gallop from the estimate, then bisect. Invariant: fits(lo), !fits(hi).
  let lo = 0
  let hi = n
  let step = calc.max(1, calc.floor(per-line / 2))
  if fits(k) {
    lo = k
    while true {
      let t = calc.min(lo + step, hi - 1)
      if t <= lo { break }
      if fits(t) {
        lo = t
        step *= 2
      } else {
        hi = t
        break
      }
    }
  } else {
    hi = k
    while true {
      let t = calc.max(hi - step, lo + 1)
      if t >= hi { break }
      if fits(t) {
        lo = t
        break
      } else {
        hi = t
        step *= 2
      }
    }
  }
  while hi - lo > 1 {
    let mid = calc.floor((lo + hi) / 2)
    if fits(mid) { lo = mid } else { hi = mid }
  }
  // Correct with the real line breaking (usually one or two measures).
  while lo > 0 and height(lo) > avail + ctx.eps { lo -= 1 }
  while lo < n - 1 and height(lo + 1) <= avail + ctx.eps { lo += 1 }
  if lo == 0 { return none }
  (head: prefix(lo) + lb, height: height(lo), rest: pieces.slice(lo).join())
}

// A paragraph being filled line by line.
#let open-par(item, ctx) = {
  let tokens = if item.tokens != none { item.tokens } else { tokens-of(item.entries) }
  let pieces = pieces-of(tokens)
  let part = range-builder(pieces)
  let natural = measure(part(0, pieces.len())).width
  (
    pieces: pieces,
    k: 0,
    after: item.after,
    // A continuation (tokens) is not indented; a new paragraph may be.
    indent: if item.tokens == none { ctx.indent } else { 0pt },
    avg: natural / calc.max(1, pieces.len()),
    line: height-of(part(0, 1), 10000pt),
    part: part,
  )
}

// The longest run of pieces from `p.k` that fits on one line of `width`.
// Returns `(k, body, height)`; `k == p.k` when not even one piece fits.
#let take-line(p, width, ctx) = {
  let n = p.pieces.len()
  let k0 = p.k
  let lb = if ctx.justify { linebreak(justify: true) } else { [] }
  let body(k) = {
    if k0 == 0 and p.indent > 0pt { h(p.indent) }
    (p.part)(k0, k)
    if k < n { lb }
  }
  // Below the midpoint between one and two lines means one line.
  let one-line = p.line + (p.line + ctx.leading) / 2
  let fits(k) = greedy-height-of(body(k), width) < one-line
  let room = width - if k0 == 0 { p.indent } else { 0pt }
  let est = calc.clamp(k0 + calc.floor(room / p.avg), k0 + 1, n)
  // Gallop away from the estimate, then bisect.
  // Invariant: lo == k0 or fits(lo); hi == n + 1 or not fits(hi).
  let lo = k0
  let hi = n + 1
  let step = 1
  if fits(est) {
    lo = est
    while lo < n {
      let t = calc.min(lo + step, n)
      if fits(t) {
        lo = t
        step *= 2
      } else {
        hi = t
        break
      }
    }
  } else {
    hi = est
    while hi - 1 > k0 {
      let t = calc.max(hi - step, k0 + 1)
      if fits(t) {
        lo = t
        break
      } else {
        hi = t
        step *= 2
      }
    }
  }
  while hi - lo > 1 {
    let mid = calc.floor((lo + hi) / 2)
    if fits(mid) { lo = mid } else { hi = mid }
  }
  // Correct with the real line breaking.
  let real-fits(k) = height-of(body(k), width) < one-line
  while lo > k0 and not real-fits(lo) { lo -= 1 }
  while lo < n and real-fits(lo + 1) { lo += 1 }
  if lo == k0 { return (k: k0, body: none, height: 0pt) }
  let b = body(lo)
  (k: lo, body: b, height: height-of(b, width))
}

// Fills text line by line into the free segments returned by `slots(y, h)`
// (left to right), from `y = 0` while lines fit above `limit`. Used when a
// line has several segments (text on both sides) or segments vary from line
// to line (contours). Returns:
// - lines: placed lines `(x, y, w, body)`
// - height: bottom of the last line
// - pos: the stream position after the filled text
// - split: whether a paragraph continues after the last line
#let fill-lines(entries, pos, slots, limit, ctx) = {
  let lines = ()
  let y = 0pt
  let bottom = 0pt
  let p = none
  let ended = false
  let reached-end = false
  let par-ended = false
  let line-height = height-of([X], 10000pt)
  while not ended {
    let lh = if p != none { p.line } else { line-height }
    if y + lh > limit + ctx.eps { break }
    let band = 0pt
    let segments = slots(y, lh)
    let placed-block = false
    for (index, seg) in segments.enumerate() {
      let width = seg.x1 - seg.x0
      if p == none {
        let item = next-item(entries, pos, ctx)
        if item.kind in ("end", "stop") {
          pos = item.at
          ended = true
          reached-end = item.kind == "end"
          break
        }
        if item.kind == "block" {
          // Blocks go into the first segment of a fresh line if they fit. The
          // segment must be free over the block's whole height: narrow it
          // until the block, measured at that width, fits.
          let body = item-content(item)
          let seg = seg
          let width = width
          let h = height-of(body, width)
          let fits = index == 0
          for _ in range(3) {
            if not fits { break }
            let tall = slots(y, h)
            if tall.len() == 0 or tall.first().x0 != seg.x0 {
              fits = false
              break
            }
            let narrower = tall.first()
            if narrower.x1 - narrower.x0 >= width - ctx.eps { break }
            seg = narrower
            width = seg.x1 - seg.x0
            h = height-of(body, width)
            fits = width >= 0pt and slots(y, h).first() == seg
          }
          if fits and y + h <= limit + ctx.eps {
            lines.push((x: seg.x0, y: y, w: width, body: body, h: h, heading: item.heading, at: pos))
            band = h
            pos = item.after
            placed-block = true
            par-ended = true
            break
          }
          ended = true
          break
        }
        p = open-par(item, ctx)
        par-ended = false
      }
      let r = take-line(p, width, ctx)
      if r.k == p.k { continue }
      lines.push((x: seg.x0, y: y, w: width, body: r.body, h: r.height, heading: false, at: none))
      band = calc.max(band, r.height)
      p.k = r.k
      if p.k >= p.pieces.len() {
        pos = p.after
        p = none
        par-ended = true
      }
    }
    if band > 0pt {
      // Paragraph spacing after a finished paragraph or block, applied before
      // the next line's segments are computed.
      bottom = y + band
      y = bottom + if par-ended or placed-block { ctx.spacing } else { ctx.leading }
    } else if not ended {
      y += lh + ctx.leading
    }
  }
  if p != none {
    pos = (head: p.pieces.slice(p.k).join(), i: p.after.i)
  } else if not reached-end {
    // Keep headings with the text that follows them.
    while lines.len() > 0 and lines.at(-1).heading {
      pos = lines.pop().at
    }
    bottom = lines.fold(0pt, (b, l) => calc.max(b, l.y + l.h))
  }
  (lines: lines, height: bottom, pos: pos, split: p != none)
}

// Fills items from stream position `pos` into a column of `width` and at most
// `limit` high. Returns:
// - content: the column content
// - height: its height
// - pos: the stream position after the column
// - split: whether the last item is the head of a split paragraph
// - empty: whether nothing was placed
#let fill-column(entries, pos, width, limit, ctx, blocks: true) = {
  let parts = ()
  let used = 0pt
  let reached-end = false
  while true {
    let item = next-item(entries, pos, ctx)
    if item.kind in ("end", "stop") {
      pos = item.at
      reached-end = item.kind == "end"
      break
    }
    if item.kind == "block" and not blocks { break }
    let body = item-content(item)
    let sp = if parts.len() > 0 { ctx.spacing } else { 0pt }
    let room = limit - used - sp
    let h = height-of(body, width)
    if h <= room + ctx.eps {
      parts.push((
        body: body,
        split: false,
        heading: item.kind == "block" and item.heading,
        at: pos,
        before: used,
      ))
      used += sp + h
      pos = item.after
      continue
    }
    if item.kind == "par" {
      let r = split-par(item, width, limit - used - sp, h, ctx)
      if r != none {
        parts.push((body: r.head, split: true, heading: false, at: pos, before: used))
        used += sp + r.height
        pos = (head: r.rest, i: item.after.i)
      }
    }
    break
  }
  // Keep headings with the text that follows them.
  while not reached-end and parts.len() > 0 and parts.at(-1).heading {
    let last = parts.pop()
    pos = last.at
    used = last.before
  }
  // The sum of item heights only estimates the spacing between items; verify
  // with one measure of the whole column and drop items that overflow.
  let body = parts.map(p => p.body).join(parbreak())
  let height = if parts.len() > 0 { height-of(body, width) } else { 0pt }
  while parts.len() > 0 and height > limit + ctx.eps {
    let last = parts.pop()
    pos = last.at
    body = parts.map(p => p.body).join(parbreak())
    height = if parts.len() > 0 { height-of(body, width) } else { 0pt }
  }
  (
    content: body,
    height: height,
    pos: pos,
    split: parts.len() > 0 and parts.at(-1).split,
    empty: parts.len() == 0,
  )
}

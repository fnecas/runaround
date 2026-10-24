// Contours for mode "tight".
//
// A contour is a function `(y0, y1) => (l, r)` or `none`: the horizontal
// extent of the object between heights `y0` and `y1`, in its own box
// (0 ≤ l ≤ r ≤ width), or `none` where the object is absent. Contours are
// conservative: they never report less than the object covers.

#import "options.typ": fail

#let contour-plugin = plugin("runaround.wasm")

// Number of bands raster contours are sampled into.
#let raster-rows = 128

// A contour from `rows`: equal horizontal bands top to bottom, each `(l, r)`
// as fractions of the width or `none`.
#let from-rows(rows, w, h) = {
  let n = rows.len()
  (y0, y1) => {
    let a = calc.max(0, calc.floor(y0 / h * n))
    let b = calc.min(n, calc.ceil(y1 / h * n))
    let l = none
    let r = none
    for i in range(a, b) {
      let row = rows.at(i)
      if row != none {
        l = if l == none { row.at(0) } else { calc.min(l, row.at(0)) }
        r = if r == none { row.at(1) } else { calc.max(r, row.at(1)) }
      }
    }
    if l == none { none } else { (l * w, r * w) }
  }
}

// An ellipse filling the box.
#let ellipse-contour(w, h) = (y0, y1) => {
  let a = calc.max(0pt, y0)
  let b = calc.min(h, y1)
  if a >= b { return none }
  // The widest point of the band is the one closest to the centre.
  let c = h / 2
  let d = if a <= c and b >= c { 0pt } else { calc.min(calc.abs(a - c), calc.abs(b - c)) }
  let half = w / 2 * calc.sqrt(calc.max(0, 1 - calc.pow(d / c, 2)))
  (w / 2 - half, w / 2 + half)
}

// A polygon with absolute vertices `((x, y), ..)` in the box.
#let polygon-contour(points) = (y0, y1) => {
  let l = none
  let r = none
  let n = points.len()
  for i in range(n) {
    let (xa, ya) = points.at(i)
    let (xb, yb) = points.at(calc.rem(i + 1, n))
    // Clip the edge to the band and take the x of its end points.
    let lo = calc.max(y0, calc.min(ya, yb))
    let hi = calc.min(y1, calc.max(ya, yb))
    if lo > hi { continue }
    let xs = if ya == yb {
      (xa, xb)
    } else {
      let at(y) = xa + (xb - xa) * ((y - ya) / (yb - ya))
      (at(lo), at(hi))
    }
    for x in xs {
      l = if l == none { x } else { calc.min(l, x) }
      r = if r == none { x } else { calc.max(r, x) }
    }
  }
  if l == none { none } else { (calc.max(0pt, l), r) }
}

// A rectangle with rounded corners of radius `rad`.
#let rounded-contour(w, h, rad) = (y0, y1) => {
  let a = calc.max(0pt, y0)
  let b = calc.min(h, y1)
  if a >= b { return none }
  let rad = calc.min(rad, w / 2, h / 2)
  // Distance into the corner zone of the band's widest point.
  let d = if b <= rad { rad - b } else if a >= h - rad { a - (h - rad) } else { 0pt }
  let inset = if d > 0pt and rad > 0pt {
    rad - calc.sqrt(calc.max(0, 1 - calc.pow(d / rad, 2))) * rad
  } else { 0pt }
  (inset, w - inset)
}

// The largest corner radius of a shape, absolute. Must be called in a context.
#let max-radius(radius, w, h) = {
  let one(v) = {
    if v == none or v == auto { return 0pt }
    if type(v) == ratio { return v * calc.min(w, h) / 2 }
    if type(v) == relative { return v.ratio * calc.min(w, h) / 2 + v.length.to-absolute() }
    if type(v) == length { return v.to-absolute() }
    0pt
  }
  if type(radius) == dictionary { radius.values().map(one).fold(0pt, calc.max) } else { one(radius) }
}

// Image bytes from an image source: bytes, or a path (resolved where the user
// created it). A plain string path is relative to this package, so it cannot
// be read: the caller falls back to the bounding box.
#let image-bytes(source) = {
  if type(source) == bytes { return source }
  if type(source) == path { return read(source, encoding: none) }
  none
}

// Contour of raster image data stretched over the box.
#let raster-contour(data, w, h) = {
  let result = json(contour-plugin.contour(data, bytes(str(raster-rows)), bytes("127")))
  from-rows(result.rows, w, h)
}

// Resolves a point (pair of lengths or ratios) inside a box. Context needed.
#let resolve-point(p, w, h) = {
  let axis(v, size) = {
    if type(v) == ratio { v * size } else if type(v) == relative { v.ratio * size + v.length.to-absolute() } else {
      v.to-absolute()
    }
  }
  (axis(p.at(0), w), axis(p.at(1), h))
}

// A shape element used as a stencil scaled to the box.
#let stencil-contour(shape, w, h) = {
  let f = shape.func()
  if f in (circle, ellipse) { return ellipse-contour(w, h) }
  if f == polygon {
    // Scale the vertices' bounding box to the object's box.
    let pts = shape.vertices.map(p => resolve-point(p, w, h))
    let xs = pts.map(p => p.at(0))
    let ys = pts.map(p => p.at(1))
    let (x0, x1) = (calc.min(..xs), calc.max(..xs))
    let (y0, y1) = (calc.min(..ys), calc.max(..ys))
    let sx = if x1 > x0 { w / (x1 - x0) } else { 1 }
    let sy = if y1 > y0 { h / (y1 - y0) } else { 1 }
    return polygon-contour(pts.map(((x, y)) => ((x - x0) * sx, (y - y0) * sy)))
  }
  if f in (rect, square, box, block) {
    return rounded-contour(w, h, max-radius(shape.at("radius", default: 0pt), w, h))
  }
  fail("contour: unsupported shape `" + repr(f) + "` (use a circle, ellipse, polygon or rect)")
}


// Detects the contour of an object from its content. Context needed.
#let detect(body, w, h, width) = {
  let f = body.func()
  if f in (circle, ellipse) { return ellipse-contour(w, h) }
  if f == polygon {
    return polygon-contour(body.vertices.map(p => resolve-point(p, w, h)))
  }
  if f in (rect, square, box, block) {
    let radius = body.at("radius", default: 0pt)
    let rad = max-radius(radius, w, h)
    if rad > 0pt { return rounded-contour(w, h, rad) }
    return none
  }
  if f == image {
    let data = image-bytes(body.source)
    if data == none { return none }
    return raster-contour(data, w, h)
  }
  if f == figure {
    // The body's contour, plus a full-width band for the caption (and the
    // gap between them).
    let inner = body.body
    let size = measure(inner, width: width)
    let inner-contour = detect(inner, size.width, size.height, width)
    if inner-contour == none { return none }
    let top = figure.caption.position == top and body.at("caption", default: none) != none
    let offset = if top { h - size.height } else { 0pt }
    let dx = (w - size.width) / 2
    return (y0, y1) => {
      let e = inner-contour(y0 - offset, y1 - offset)
      let body-band = if e == none { none } else { (e.at(0) + dx, e.at(1) + dx) }
      let in-caption = if top { y0 < offset } else { y1 > size.height }
      if in-caption { (0pt, w) } else { body-band }
    }
  }
  none
}

// The contour for the `contour` option, or `none` for the bounding box.
// Context needed.
#let contour-for(option, body, w, h, width) = {
  if option == none { return none }
  if option == auto { return detect(body, w, h, width) }
  if type(option) == content { return stencil-contour(option, w, h) }
  if type(option) == array {
    if option.len() < 3 { fail("contour: a polygon needs at least three points") }
    return polygon-contour(option.map(p => resolve-point(p, w, h)))
  }
  if type(option) == function {
    // `y => (l, r)` with ratios of the height and width; sampled into rows.
    let n = 64
    let rows = range(n).map(i => {
      let samples = (i / n, (i + 0.5) / n, (i + 1) / n).map(option)
      let present = samples.filter(s => s != none)
      if present.len() == 0 { none } else {
        (calc.min(..present.map(s => s.at(0) / 100%)), calc.max(..present.map(s => s.at(1) / 100%)))
      }
    })
    return from-rows(rows, w, h)
  }
  if type(option) in (bytes, path) { return raster-contour(image-bytes(option), w, h) }
  none
}

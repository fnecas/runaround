// Option names, defaults, validation and resolution.

#let modes = ("none", "before", "after", "parallel", "through", "tight")

#let defaults = (
  mode: "none",
  align: auto,
  dx: 0pt,
  gap: 1em,
  min-width: 6em,
  width: auto,
  contour: auto,
  behind: true,
)

#let params = defaults.keys()

// Where the object sits when `align` is not given.
#let default-align = (
  "none": center,
  "before": right,
  "after": left,
  "parallel": center,
  "through": center,
  "tight": left,
)

// Elements that `auto-wrap` can match, by name.
#let auto-elements = (
  figure: figure,
  image: image,
  table: table,
  rect: rect,
  square: square,
  circle: circle,
  ellipse: ellipse,
  polygon: polygon,
  block: block,
)

#let fail(message) = panic("runaround: " + message)

#let quoted(names) = names.map(n => "`" + n + "`").join(", ", last: " or ")

#let is-length-like(v) = type(v) in (length, ratio, relative)

#let check-param(who, key, value) = {
  let bad(expected) = fail(
    who + ": `" + key + "` must be " + expected + ", found " + repr(value),
  )
  if key == "mode" {
    if type(value) != str or value not in modes {
      bad("one of " + quoted(modes))
    }
  } else if key == "align" {
    if value != auto {
      if type(value) != alignment or value.y != none {
        bad("`auto` or a horizontal alignment (`left`, `center`, `right`, `start`, `end`)")
      }
    }
  } else if key == "dx" {
    if not is-length-like(value) { bad("a length") }
  } else if key == "gap" {
    if type(value) == dictionary {
      for (k, v) in value {
        if k not in ("x", "y") { bad("a length or a dictionary with keys `x` and `y`") }
        if type(v) != length { bad("a length or a dictionary of lengths") }
      }
    } else if type(value) != length {
      bad("a length or a dictionary with keys `x` and `y`")
    }
  } else if key == "min-width" {
    if not is-length-like(value) { bad("a length or a ratio") }
  } else if key == "width" {
    if value != auto and not is-length-like(value) { bad("`auto` or a length") }
  } else if key == "contour" {
    let ok = (
      value == auto
        or value == none
        or type(value) in (content, array, function, bytes, path)
    )
    if not ok { bad("`auto`, `none`, a shape, an array of points, a function, bytes or a path") }
  } else if key == "behind" {
    if type(value) != bool { bad("a boolean") }
  }
}

// Validates named arguments and returns them unchanged.
#let check-named(who, named) = {
  for (key, value) in named {
    if key not in params {
      fail(who + ": unknown argument `" + key + "` (expected " + quoted(params) + ")")
    }
    check-param(who, key, value)
  }
  named
}

// Normalizes an `auto-wrap` dictionary into an array of (element, spec).
// A spec is a mode string, a dictionary of wrap options, or a function from
// the element to either of those or `none`.
#let check-auto-wrap(value) = {
  if type(value) != dictionary {
    fail("`auto-wrap` must be a dictionary such as `(figure: \"after\")`, found " + repr(value))
  }
  let rules = ()
  for (name, spec) in value {
    if name not in auto-elements {
      fail("auto-wrap: unsupported element `" + name + "` (supported: " + quoted(auto-elements.keys()) + ")")
    }
    if type(spec) == str {
      check-param("auto-wrap", "mode", spec)
    } else if type(spec) == dictionary {
      check-named("auto-wrap", spec)
    } else if type(spec) != function {
      fail("auto-wrap: the value for `" + name + "` must be a mode, a dictionary of options or a function")
    }
    rules.push((auto-elements.at(name), spec))
  }
  rules
}

// Turns an auto-wrap spec (after applying it to the element) into options.
#let spec-to-options(spec) = {
  if spec == none { return none }
  if type(spec) == str {
    check-param("auto-wrap", "mode", spec)
    return (mode: spec)
  }
  if type(spec) == dictionary { return check-named("auto-wrap", spec) }
  fail("auto-wrap: a function must return a mode, a dictionary of options or `none`, found " + repr(spec))
}

// Converts a length, ratio or relative length to an absolute length.
// Must be called in a context.
#let to-abs(value, base) = {
  if type(value) == ratio { return value * base }
  if type(value) == relative { return value.ratio * base + value.length.to-absolute() }
  value.to-absolute()
}

#let rtl-langs = ("ar", "dv", "fa", "he", "ks", "pa", "ps", "sd", "ug", "ur", "yi")

// The physical horizontal alignment (`left`, `center` or `right`).
// Must be called in a context.
#let physical(align) = {
  let x = align.x
  if x in (left, center, right) { return x }
  let rtl = if text.dir == auto { text.lang in rtl-langs } else { text.dir == rtl }
  if x == start { if rtl { right } else { left } } else { if rtl { left } else { right } }
}

// Resolves options to absolute values for an available width.
// Must be called in a context.
#let resolve(opts, width) = {
  let mode = opts.mode
  let align = physical(if opts.align == auto { default-align.at(mode) } else { opts.align })
  let gap = if type(opts.gap) == dictionary {
    let g = defaults.gap
    (x: opts.gap.at("x", default: g), y: opts.gap.at("y", default: g))
  } else {
    (x: opts.gap, y: opts.gap)
  }
  (
    mode: mode,
    align: align,
    dx: to-abs(opts.dx, width),
    gap: (x: gap.x.to-absolute(), y: gap.y.to-absolute()),
    min-width: to-abs(opts.min-width, width),
    width: opts.width,
    contour: opts.contour,
    behind: opts.behind,
  )
}

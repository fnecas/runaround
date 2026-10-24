# runaround — design

`runaround` makes text flow around objects (figures, images, shapes, boxes) in
Typst, in the spirit of LaTeX's `wrapfigure` and the wrap options of LibreOffice
Writer. You mark the *object*; the package finds the text that follows it and
decides where to break it.

Requires Typst ≥ 0.15.

## Goals

1. **Mark the object, not the text.** The user passes neither the text to wrap
   nor the scope of the wrapping.
2. **Decide line and page breaks automatically**, including at page bottoms.
3. **Fast and memory-friendly.** Layout work is linear in the amount of text
   that sits beside objects; text elsewhere costs nothing extra.
4. **Proven by tests**: geometric invariants on real PDF output, visual
   regression, and performance budgets.

## Usage

```typ
#import "@preview/runaround:0.1.0": runaround, wrap
#show: runaround                 // once; place it after your template's show rule

#wrap(mode: "after")[
  #figure(image("cat.jpg", width: 5cm), caption: [A cat.]) <fig:cat>
]
Any text, paragraphs, lists or headings. It flows around @fig:cat
and continues at full width below it.
```

Document-wide defaults and automatic wrapping:

```typ
#show: runaround.with(
  gap: 1em,                       // defaults for every wrap()
  auto-wrap: (figure: "after"),   // wrap every in-flow figure automatically
)
#figure(image("a.png", width: 4cm), caption: [Wraps automatically.])
#wrap(mode: "none")[#figure(...)] // opt one figure out
```

### Why a show rule is needed

Typst 0.15 has no user-defined elements, so no package can add a `wrap:`
parameter to the built-in `figure` or `box`. Shadowing `figure` with a function
would break `#set figure(...)` and `#show figure: ...`. One `#show: runaround`
line, plus `wrap(...)` or the `auto-wrap` option, is the closest equivalent.

## API

### `runaround(body, ..defaults)`

Used as `#show: runaround` or `#show: runaround.with(..)`. It accepts every
`wrap` parameter as a document-wide default, plus:

- `auto-wrap` (named so because `auto` is a reserved word): dictionary mapping an element name to a wrap spec. The spec is a mode
  string, a dictionary of `wrap` parameters, or a function
  `it => spec-or-none`. Floating figures (`placement` other than `none`) are
  never auto-wrapped. A function receives the element as written (unset fields are `auto`). Example: `auto-wrap: (figure: it => if it.body.func() == image { "after" })`.

### `wrap(body, mode:, align:, dx:, gap:, min-width:, width:, contour:, behind:)`

Every parameter defaults to `auto`, which means "inherit from `runaround`".

| Parameter | Default | Meaning |
| --- | --- | --- |
| `mode` | `"none"` | `"none"`, `"before"`, `"after"`, `"parallel"`, `"through"`, `"tight"`; see the modes table |
| `align` | from mode | horizontal position of the object: `left`, `center`, `right` |
| `dx` | `0pt` | extra horizontal offset of the object |
| `gap` | `1em` | space between object and text; a length or `(x: .., y: ..)` |
| `min-width` | `6em` | a free side narrower than this receives no text |
| `width` | `auto` | object width; figure captions wrap to the body width by default |
| `contour` | `auto` | `"tight"` only: `auto`, `none`, a shape (`circle(..)` etc.), polygon points, a function, or image `bytes` |
| `behind` | `true` | `"through"` only: draw the object behind the text |

### Modes

The names follow LibreOffice Writer.

| `mode` | Default `align` | Text placement |
| --- | --- | --- |
| `"none"` | `center` | above and below only |
| `"before"` | `right` | left of the object |
| `"after"` | `left` | right of the object |
| `"parallel"` | `center` | both sides, read line by line across the object (Word/LibreOffice) |
| `"through"` | `center` | unaffected; object drawn behind (or in front of) the text |
| `"tight"` | `left` | follows the object's contour on its free side(s): `left` → text right, `right` → text left, `center` → both |

Without `#show: runaround`, `wrap` renders the object as a plain block (graceful
fallback, same as mode `"none"`).

## Architecture

### 1. Scanner (evaluation time)

Typst applies a selector-less show rule eagerly: `runaround(body)` receives the
raw document before layout. The scanner:

1. Flattens sequences and `styled` wrappers into a stream of
   `(child, style-stack)` entries. It works iteratively, so the depth of `#set`
   nesting doesn't matter. Containers (`block`, `columns`, `align`, `pad`, list
   items, …) are scanned recursively.
2. Detects **markers**: content produced by `wrap()` (a labelled block that
   carries its options in a `metadata` child), and elements matched by `auto-wrap`.
3. Turns each marker plus the content after it, up to the next marker or the end
   of the sequence, into a **group**.
4. Rebuilds the content and its `styled` wrappers. If no marker is found, it
   returns the original content unchanged.

`styled` and `sequence` can be reconstructed by calling `content.func()`, which
the probes on 0.15.1 verified.

### 2. Groups (layout time)

A group is a `context` block. `context` isn't a container, so text after the
object stays in the normal flow: page breaks, floats and `#set page` keep working
there. Each group contains:

- **probes**, read back with `query` (`context` alone can't see the container
  width, and a `layout` container can't hold the rest of the flow, because
  Typst rejects `#set page` inside containers):
  - a zero-height block with `above: 0pt` sits exactly where the preceding
    content ends and reports the container width through `layout`;
  - a `place(bottom + left, ..)` reports the bottom and the height of the
    region.

  A bare `place(left, ..)` probe must not be used: it lands after the spacing
  of the next block only when that block stays on the page. The position then
  depends on the group's own output and the layout oscillates (found by the
  geometry tests).
- **a beside block**: an unbreakable block holding the object and the text placed
  next to it.
- **the leftover**: the rest of the group's content, emitted at full width
  straight into the flow.

The first layout pass guesses the width from the page settings, including page
columns; the following passes use the measured width.

### 3. Fill engine

The engine works on a stream of **items**: paragraphs (split lazily into
break-opportunity pieces: words, and individual CJK characters), block items
(headings, list/enum items, equations, code, tables, …), and markers.

The free space beside the obstacles is a set of horizontal **segments** per
vertical interval:

- **Column fill**: a single segment of constant width (`before`, `after`, or
  `tight` with a rectangular contour). Text goes into one column block of height
  `H`:
  - A paragraph that fits completely costs **one** `measure`.
  - The paragraph that crosses the bottom is split at a word chosen by
    estimate: lines that fit = `⌊(H + leading) / (line-height + leading)⌋`, and
    words ≈ proportional to lines. A short gallop and binary search confirm
    it. The search measures with greedy line breaking
    (`par(linebreaks: "simple")`), which is much cheaper than the optimized
    breaking of justified text. One or two real measurements then step to the
    exact optimum, because the optimized breaking shrinks spaces and can fit
    slightly more. Splits are therefore exactly what Typst itself would break.
  - A final measure of the whole column verifies the result.
  - Plain paragraphs (text only, one style) build their trial prefixes as a
    single string slice instead of rebuilding word tokens.
- **Line fill**: several segments per line, or widths that change from line to
  line (`parallel`, `tight`). Each line segment receives the longest run of
  pieces that fits on one line. The first guess comes from the average piece
  width, then a few measures of one line's worth of text confirm it (greedy
  search, real correction, as above). Lines are stacked with the measured line
  height + `par.leading`, and `par.spacing` at paragraph ends. Spacing is
  applied before the next line's free segments are computed. A block (list
  item, heading, …) gets a segment that is free over its whole height.
- **Split invisibly.** A paragraph is cut where the next word starts a new line
  anyway. The line before the cut gets `linebreak(justify: true)` when the
  paragraph is justified. The continuation gets no indent and line spacing equal
  to `par.leading`.
- **Block items.** A block is placed in the free segment if it fits that width
  and the remaining height; otherwise it, and everything after it, goes into the
  leftover. A heading never stays alone at the bottom of the beside area. Lists
  break between items, and enum numbering continues.
- **Safe stops.** The beside area also ends at hard stops (`pagebreak`,
  `colbreak`, `place`, floating figures, fractional `v`) and at block-level
  `#set` rules that start after the object. Style values are opaque, so a hidden
  `#set page` inside the beside block can't be excluded any other way.

### 4. Page breaks

`avail` = region bottom − group start, from the probes.

- If the object is taller than `avail` but fits on a page, the text first fills
  `avail` at full width ("prefill"). Only paragraphs are used: a heading or list
  at the page bottom moves on with the object, and its own spacing is not known
  here. The beside block then moves to the top of the next page or column on its
  own.
- An object taller than the page body falls back to `"none"`.

**Sticky decisions.** A group's decision shifts the content after it, which can
flip the decisions of later groups, one wave per layout iteration. Typst allows
five, and long documents did not converge (found by the performance test). So
each group records its decision as metadata and reads it back in the next
iteration:

- `open`: not decided yet. A group decides only when the previous iteration was
  laid out with the width measured now, so it doesn't decide from a layout made
  with a guessed width, for example inside `columns`.
- `fit`: final.
- `prefill` (with the height it used): kept while it still fits and is still
  needed. Otherwise it becomes `fit`, which is always valid: at worst the object
  moves on and leaves a gap.

Each group changes at most twice, so documents converge in about three
iterations however many objects they have. The `long_documents_converge` test
covers 50 objects, with identical and with unique paragraphs. A cheaper scheme
that keeps fitting objects "open" saves one iteration in simple documents, but
cascades again in long ones.

### 5. Contours (`tight`)

A contour is sampled into rows of horizontal extents `(l, r)` as fractions of the
object box. A band of text uses the union of the rows it overlaps, widened by
`gap.y`.

- **Shapes**: `circle`, `ellipse`, `polygon` and rounded `rect`/`box` are
  computed analytically from their fields.
- **Figures**: the body's contour, plus the caption's rectangle.
- **Raster images** (PNG/GIF/WebP with alpha): the Rust → WASM plugin
  (`plugin/`) extracts per-row extents from the alpha channel. Typst caches the
  results. A package can only read the user's file when it is given as
  `image(path("cat.png"))` or as bytes. A plain string resolves relative to the
  package; this was verified on 0.15.1.
- **Everything else**: the bounding box.

### 6. Objects close together

When a group's text runs out before its object ends, the group reserves the
object's full height, so the next object starts below it. This matches LaTeX's
wrapfig and never overlaps; it is pinned by the `objects_close_together` test.

*Future work: staggered objects.* The next object could start beside a
protruding one, for example alternating left and right images. Each group would
publish its object region as metadata, and a following group would treat
regions that still overlap its start as extra obstacles. This was deferred
because it needs cross-group introspection, including during page-break
pre-fill, and adds convergence risk.

## Performance strategy

The naive approach measures the growing prefixes 1, 2, 3, … words long until one
overflows, and re-lays out the figure with each measurement. That is quadratic
layout work, and Typst keeps every result in memory. runaround avoids this:

- No linear prefix scans; every split is estimated, then confirmed with a
  bounded number of measures.
- No measurement of "the rest of the document"; only content that may sit beside
  an object is measured.
- Lazy piece splitting: only paragraphs that are actually split are broken into
  pieces.
- Untouched content keeps its identity; documents without markers are returned
  as-is.
- Introspection is limited to two probes and one decision record per group.
  Later layout iterations are almost free thanks to Typst's caches: the first
  pass costs the most, the second and third take a few milliseconds.
- `harness/tests/perf.rs` enforces budgets on compile time and peak memory
  relative to the same text without wrapping (release build, Typst caches
  evicted before each run, peak heap from a counting allocator):

  | Objects | plain (no wrapping) | runaround |
  | --- | --- | --- |
  | 20 | 37 ms, 7.3 MiB | 210–250 ms, 24 MiB |
  | 60 | 114–126 ms, 21 MiB | 470–530 ms, 45–52 MiB |

  runaround grows linearly, at about 6–9 ms per object. The "plain" baseline uses only Typst's embedded fonts and needs
  a single layout pass, so it is unusually cheap. With the CLI and system fonts,
  runaround's overhead measured about 1.9× the plain compile.

## Limitations

- Requires `#show: runaround` once. Content produced inside `context` or by the
  user's own show rules is invisible to the scanner; the `auto-wrap` option covers
  the common case.
- In `parallel` and `tight`, every line is laid out separately, so word spacing
  is slightly less uniform than in a normal paragraph.
- The word just before a split is never hyphenated.
- A block-level `#set` rule placed after an object and before its text stops the
  text beside the object.
- Page-break handling assumes the group sits in the page flow (including
  columns), not inside a fixed-height container.

## What is Rust and what is Typst

- **Package logic is Typst code.** It has to be: text can only be measured and
  broken by Typst's own layout engine (`measure`). A WASM plugin is a pure
  function on bytes, with no fonts and no layout engine.
- **The plugin (`plugin/`, Rust → `wasm32-unknown-unknown`, wasm-minimal-protocol)**
  does the pure geometry: alpha-channel contour extraction for raster images and
  sampling of shapes (ellipse, polygon, rounded rect) into rows. Typst caches
  plugin calls. The plugin is built with `opt-level = "z"`, LTO and stripping to
  stay small.
- **All tooling is Rust**: the test harness (`harness/`) and the build tasks
  (`xtask/`). They pin the `typst` crates to the same version as the supported
  compiler.

## Testing

`cargo test` runs everything. The harness links `typst` 0.15.1 and compiles the
test documents in-process with the repository as the project root. It uses only
the fonts embedded in `typst-assets`, so results are identical on every machine.

1. **Unit** (`tests/unit/*.typ`): Typst `assert`s for pure logic (option
   resolution, contour maths, piece splitting). Each must compile without errors
   or warnings. **Error tests** (`tests/errors/*.typ`) declare the expected
   message in a header comment.
2. **Geometry** (`tests/geometry/*.typ`): the harness walks the laid-out frames
   and gets the exact box of every word (from glyph advances), shape and image,
   with all transforms applied. Assertions:
   - no word overlaps an obstacle widened by its gap;
   - the word sequence equals that of the same document compiled without
     runaround (`--input plain=true`), so nothing is lost or duplicated at
     splits;
   - the space beside an obstacle is filled when enough text follows;
   - no text leaves the text area;
   - expected pages.

   The case matrix covers modes × alignment × justified or ragged × font sizes ×
   columns × page-break positions × lists/headings × multiple objects × RTL.
3. **Visual regression**: pages rendered with `typst-render` and compared with
   `tests/ref/*.png` within a tolerance. `UPDATE_REFS=1 cargo test` regenerates
   the references.
4. **Documentation**: every `typ` code block in `README.md` compiles, with
   `@preview/runaround:<version>` mapped to the working tree.
5. **Performance** (`cargo test --release -- --ignored`): generated large
   documents. A counting global allocator measures exact peak heap. Wall time
   and peak memory have budgets relative to the same document without wrapping.

## Packaging (Typst Universe guidelines)

- **Manifest** (`typst.toml`):
  - `name = "runaround"`, SemVer `version`, `entrypoint = "src/lib.typ"`,
    `compiler = "0.15.0"`.
  - `authors`, `license = "MIT"` matching the `LICENSE` file (exactly that
    name).
  - `description`: one imperative sentence of 40–60 characters without "Typst"
    or "package", e.g. "Flow text around figures, images and shapes."
  - `categories = ["layout"]` (`text` is meant for string and font tools);
    `disciplines` omitted because the package is broadly applicable; `keywords`.
  - `exclude` for README images and docs; never the README or LICENSE.
  - No `homepage` unless a dedicated documentation site exists; `repository`
    once the repository is public.
- **Entrypoint**: `src/lib.typ` contains only imports of the public items
  (`runaround`, `wrap`). Everything else stays private.
- **Code style**: two-space indentation, kebab-case names.
- **Files**: the bundle contains only the manifest, `LICENSE`, `README.md`,
  `src/*.typ` and `src/runaround.wasm`. README illustrations are committed but
  excluded. Tests, Rust sources and build tooling stay in the development
  repository; `cargo xtask bundle` produces `dist/runaround/<version>/` for the
  submission PR.
- **README**:
  - documents every public definition, with examples importing
    `@preview/runaround:<version>`, compiled by the docs test;
  - images use `<picture>` for light and dark variants;
  - no reliance on the h1 heading (Universe removes it);
  - no GitHub alerts, emoji shortcodes or task lists;
  - `typos` run before each release.
- **Files from the user**: no string path parameters. Images for contours come
  as `image(path(..))`, a `path`, or `bytes`. Since 0.15 paths resolve where
  they are created, as the guidelines recommend.
- **Fonts**: none shipped; the tests use the embedded Typst fonts.
- **Safety**: the plugin is pure (no I/O); the package reads only what the user
  passes.
- **Submission**: copy `dist/runaround/<version>/` (never a git submodule) into
  `packages/preview/runaround/<version>/` of a sparse checkout of
  `typst/packages`. Before the PR, run `typst-package-check` and `typos`. Fix
  bugs found after release with patch versions.

The rules come from <https://github.com/typst/packages/tree/main/docs>
(README, manifest, typst, resources, documentation, licensing, tips,
CATEGORIES). Re-check them before each release, because they can be extended.

## Milestones

1. Scaffold (manifest, workspace, harness), marker, scanner, `none`/`through`,
   column fill (`before`/`after`), page breaks, unit and geometry tests.
2. Line fill (`parallel`), block items, lists, headings.
3. `tight`: shape contours, then the WASM alpha-contour plugin.
4. `auto-wrap`, performance suite, README and illustrations, Universe
   packaging.
5. Future: staggered objects (see section 6).

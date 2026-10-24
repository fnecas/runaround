//! Exact geometry of a laid-out document: words, shapes and images.

use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst::visualize::{CurveItem, Geometry as ShapeGeometry, Paint};
use typst_layout::PagedDocument;

/// An axis-aligned rectangle in points (y grows downwards).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// Grows the rectangle by `dx` horizontally and `dy` vertically on each side.
    pub fn inflate(&self, dx: f64, dy: f64) -> Rect {
        Rect { x0: self.x0 - dx, y0: self.y0 - dy, x1: self.x1 + dx, y1: self.y1 + dy }
    }

    /// Whether the interiors overlap by more than `eps` in both directions.
    pub fn overlaps(&self, other: &Rect, eps: f64) -> bool {
        self.x0 + eps < other.x1
            && other.x0 + eps < self.x1
            && self.y0 + eps < other.y1
            && other.y0 + eps < self.y1
    }

    pub fn contains(&self, other: &Rect, eps: f64) -> bool {
        other.x0 >= self.x0 - eps
            && other.x1 <= self.x1 + eps
            && other.y0 >= self.y0 - eps
            && other.y1 <= self.y1 + eps
    }

    fn from_points(points: impl IntoIterator<Item = Point>) -> Rect {
        let mut r = Rect { x0: f64::MAX, y0: f64::MAX, x1: f64::MIN, y1: f64::MIN };
        for p in points {
            r.x0 = r.x0.min(p.x.to_pt());
            r.y0 = r.y0.min(p.y.to_pt());
            r.x1 = r.x1.max(p.x.to_pt());
            r.y1 = r.y1.max(p.y.to_pt());
        }
        r
    }

    /// The bounding box of the rectangle `(0,0)..(w,h)` under a transform.
    fn transformed(min: Point, max: Point, ts: Transform) -> Rect {
        Rect::from_points(
            [
                Point::new(min.x, min.y),
                Point::new(max.x, min.y),
                Point::new(min.x, max.y),
                Point::new(max.x, max.y),
            ]
            .map(|p| p.transform(ts)),
        )
    }
}

/// A word: a maximal run of non-whitespace glyphs within one text run.
#[derive(Debug, Clone)]
pub struct Word {
    /// Zero-based page index.
    pub page: usize,
    pub text: String,
    /// From ascender to descender, across the glyph advances.
    pub rect: Rect,
    pub baseline: f64,
    /// The word ends with a hyphen inserted by hyphenation.
    pub hyphenated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// A shape and its solid fill colour (RGBA), if any.
    Shape(Option<[u8; 4]>),
    Image,
}

/// A shape or image.
#[derive(Debug, Clone)]
pub struct Item {
    pub page: usize,
    pub rect: Rect,
    pub kind: ItemKind,
    /// For shapes: the outline as a closed polygon in page coordinates
    /// (curves flattened), for exact distance checks.
    pub outline: Vec<(f64, f64)>,
}

/// Everything the tests need to know about a laid-out document.
#[derive(Debug, Clone, Default)]
pub struct Geometry {
    /// Page sizes `(width, height)` in points.
    pub pages: Vec<(f64, f64)>,
    /// Words in frame order.
    pub words: Vec<Word>,
    pub items: Vec<Item>,
}

impl Geometry {
    pub fn extract(doc: &PagedDocument) -> Geometry {
        let mut geom = Geometry::default();
        for (index, page) in doc.pages().iter().enumerate() {
            let size = page.frame.size();
            geom.pages.push((size.x.to_pt(), size.y.to_pt()));
            walk(&page.frame, Transform::identity(), index, &mut geom);
        }
        geom
    }

    /// Shapes filled with exactly this colour (RGB hex such as `"ff0000"`).
    pub fn shapes_with_fill(&self, hex: &str) -> Vec<&Item> {
        let rgb = parse_hex(hex);
        self.items
            .iter()
            .filter(|item| match item.kind {
                ItemKind::Shape(Some(c)) => c[..3] == rgb && c[3] == 255,
                _ => false,
            })
            .collect()
    }

    pub fn images(&self) -> Vec<&Item> {
        self.items.iter().filter(|item| item.kind == ItemKind::Image).collect()
    }

    /// Word texts in frame order, with hyphenated words joined back together.
    pub fn word_texts(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut join = false;
        for word in &self.words {
            if join {
                out.last_mut().unwrap().push_str(&word.text);
            } else {
                out.push(word.text.clone());
            }
            join = word.hyphenated;
        }
        out
    }
}

fn parse_hex(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex colour");
    [byte(0), byte(2), byte(4)]
}

fn walk(frame: &Frame, ts: Transform, page: usize, geom: &mut Geometry) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
        match item {
            FrameItem::Group(group) => {
                walk(&group.frame, ts.pre_concat(group.transform), page, geom)
            }
            FrameItem::Text(text) => {
                let metrics = text.font.metrics();
                let top = -metrics.ascender.at(text.size);
                let bottom = -metrics.descender.at(text.size);
                let baseline = Point::zero().transform(ts).y.to_pt();
                let mut x = Abs::zero();
                let mut current: Option<(String, Abs)> = None;
                let mut flush = |current: &mut Option<(String, Abs)>, end: Abs, hyphen: bool| {
                    if let Some((word, start)) = current.take() {
                        let rect =
                            Rect::transformed(Point::new(start, top), Point::new(end, bottom), ts);
                        geom.words.push(Word {
                            page,
                            text: word,
                            rect,
                            baseline,
                            hyphenated: hyphen,
                        });
                    }
                };
                for glyph in &text.glyphs {
                    let range = glyph.range.start as usize..glyph.range.end as usize;
                    let s = text.text.get(range).unwrap_or("");
                    let advance = glyph.x_advance.at(text.size);
                    if s.is_empty() {
                        // A glyph without source text: the hyphen added by
                        // hyphenation at the end of a line.
                        x += advance;
                        flush(&mut current, x, true);
                        continue;
                    }
                    if s.chars().all(char::is_whitespace) {
                        flush(&mut current, x, false);
                    } else {
                        match &mut current {
                            Some((word, _)) => word.push_str(s),
                            None => current = Some((s.to_string(), x)),
                        }
                    }
                    x += advance;
                }
                flush(&mut current, x, false);
            }
            FrameItem::Shape(shape, _) => {
                let (min, max) = match &shape.geometry {
                    ShapeGeometry::Rect(size) => (Point::zero(), size.to_point()),
                    ShapeGeometry::Line(to) => (Point::zero(), *to),
                    ShapeGeometry::Curve(curve) => {
                        let bbox = curve.bbox(None);
                        (bbox.min, bbox.max)
                    }
                };
                let fill = match &shape.fill {
                    Some(Paint::Solid(color)) => Some(color.to_vec4_u8()),
                    _ => None,
                };
                let outline = outline_of(&shape.geometry, ts);
                geom.items.push(Item {
                    page,
                    rect: Rect::transformed(min, max, ts),
                    kind: ItemKind::Shape(fill),
                    outline,
                });
            }
            FrameItem::Image(_, size, _) => {
                geom.items.push(Item {
                    page,
                    rect: Rect::transformed(Point::zero(), size.to_point(), ts),
                    kind: ItemKind::Image,
                    outline: Vec::new(),
                });
            }
            FrameItem::Link(..) | FrameItem::Tag(_) => {}
        }
    }
}

/// The outline of a shape as a polygon in page coordinates. Bézier segments
/// are flattened into short lines.
fn outline_of(geometry: &ShapeGeometry, ts: Transform) -> Vec<(f64, f64)> {
    let pt = |p: Point| {
        let p = p.transform(ts);
        (p.x.to_pt(), p.y.to_pt())
    };
    match geometry {
        ShapeGeometry::Rect(size) => vec![
            pt(Point::zero()),
            pt(Point::new(size.x, Abs::zero())),
            pt(size.to_point()),
            pt(Point::new(Abs::zero(), size.y)),
        ],
        ShapeGeometry::Line(to) => vec![pt(Point::zero()), pt(*to)],
        ShapeGeometry::Curve(curve) => {
            let mut out = Vec::new();
            let mut last = Point::zero();
            for item in curve.0.iter() {
                match item {
                    CurveItem::Move(p) | CurveItem::Line(p) => {
                        out.push(pt(*p));
                        last = *p;
                    }
                    CurveItem::Cubic(c1, c2, p) => {
                        for i in 1..=16 {
                            let t = i as f64 / 16.0;
                            let u = 1.0 - t;
                            let b = |a: Abs, b: Abs, c: Abs, d: Abs| {
                                a * (u * u * u)
                                    + b * (3.0 * u * u * t)
                                    + c * (3.0 * u * t * t)
                                    + d * (t * t * t)
                            };
                            out.push(pt(Point::new(
                                b(last.x, c1.x, c2.x, p.x),
                                b(last.y, c1.y, c2.y, p.y),
                            )));
                        }
                        last = *p;
                    }
                    CurveItem::Close => {}
                }
            }
            out
        }
    }
}

/// Distance between a rectangle and a closed polygon (0 if they overlap).
pub fn rect_polygon_distance(r: &Rect, poly: &[(f64, f64)]) -> f64 {
    if poly.is_empty() {
        return f64::INFINITY;
    }
    let inside_rect = |&(x, y): &(f64, f64)| x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1;
    let in_poly = |x: f64, y: f64| {
        let mut inside = false;
        let n = poly.len();
        for i in 0..n {
            let (xi, yi) = poly[i];
            let (xj, yj) = poly[(i + n - 1) % n];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                inside = !inside;
            }
        }
        inside
    };
    let corners = [(r.x0, r.y0), (r.x1, r.y0), (r.x0, r.y1), (r.x1, r.y1)];
    if corners.iter().any(|&(x, y)| in_poly(x, y)) || poly.iter().any(inside_rect) {
        return 0.0;
    }
    let point_dist = |(x, y): (f64, f64)| {
        let dx = (r.x0 - x).max(0.0).max(x - r.x1);
        let dy = (r.y0 - y).max(0.0).max(y - r.y1);
        (dx * dx + dy * dy).sqrt()
    };
    let mut best = f64::INFINITY;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let steps = (len / 0.5).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            best = best.min(point_dist((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)));
        }
    }
    best
}

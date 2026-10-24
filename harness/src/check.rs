//! Geometric invariants of wrapped layouts.

use crate::geom::{rect_polygon_distance, Geometry, Rect, Word};

/// Parameters a check needs about the document under test.
#[derive(Debug, Clone)]
pub struct Expect {
    /// Fill colour (RGB hex) of obstacle shapes.
    pub obstacle_fill: &'static str,
    /// Horizontal and vertical gap between obstacles and text, in points.
    pub gap: f64,
    /// Font size in points (tolerances scale with it).
    pub font_size: f64,
    /// Page margin in points (all sides).
    pub margin: f64,
    /// Number of obstacles the document must contain.
    pub obstacles: usize,
    /// Whether enough text follows each obstacle to fill the space beside it.
    pub expect_filled: bool,
}

/// Collects violated invariants with readable messages.
#[derive(Debug, Default)]
pub struct Report {
    pub failures: Vec<String>,
}

impl Report {
    fn fail(&mut self, message: String) {
        if self.failures.len() < 20 {
            self.failures.push(message);
        }
    }

    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

fn describe(word: &Word) -> String {
    format!(
        "{:?} on page {} at ({:.1}, {:.1})-({:.1}, {:.1})",
        word.text,
        word.page + 1,
        word.rect.x0,
        word.rect.y0,
        word.rect.x1,
        word.rect.y1
    )
}

/// An obstacle: page, bounding box and outline.
type Obstacle<'a> = (usize, Rect, &'a [(f64, f64)]);

/// Checks all invariants of `actual` against `expect`, using `plain` (the same
/// text laid out without runaround) as the reference word sequence.
pub fn check(actual: &Geometry, plain: &Geometry, expect: &Expect) -> Report {
    let mut report = Report::default();
    let obstacles: Vec<Obstacle> = actual
        .shapes_with_fill(expect.obstacle_fill)
        .iter()
        .map(|item| (item.page, item.rect, item.outline.as_slice()))
        .collect();

    // Guard against vacuous checks.
    if obstacles.len() != expect.obstacles {
        report.fail(format!("found {} obstacles, expected {}", obstacles.len(), expect.obstacles));
    }
    if actual.words.len() < 50 {
        report.fail(format!("only {} words found", actual.words.len()));
    }

    // Punctuation may hang into margins and gaps (Typst's overhang).
    let overhang = 0.4 * expect.font_size;

    // 1. Nothing is lost, duplicated or reordered.
    let got = actual.word_texts();
    let want = plain.word_texts();
    if got != want {
        let first =
            got.iter().zip(&want).position(|(a, b)| a != b).unwrap_or(got.len().min(want.len()));
        let ctx = |v: &[String]| v[first.saturating_sub(3)..(first + 4).min(v.len())].join(" ");
        report.fail(format!(
            "word sequence differs at word {first} ({} words vs {} expected): got `{}`, expected `{}`",
            got.len(),
            want.len(),
            ctx(&got),
            ctx(&want)
        ));
    }

    // 2. No word overlaps an obstacle or its gap. Distances are measured to
    //    the exact outline of the shape, so text following a contour counts.
    for word in &actual.words {
        for (page, rect, outline) in &obstacles {
            if word.page != *page || !word.rect.overlaps(&rect.inflate(expect.gap, expect.gap), 0.0)
            {
                continue;
            }
            let d = rect_polygon_distance(&word.rect, outline);
            if d == 0.0 {
                report.fail(format!("word {} overlaps obstacle {rect:?}", describe(word)));
            } else if d < expect.gap - overhang {
                report.fail(format!("word {} is {d:.1}pt from obstacle {rect:?}", describe(word)));
            }
        }
    }

    // 3. Words do not overlap each other (catches wrong spacing at splits).
    for page in 0..actual.pages.len() {
        let words: Vec<&Word> = actual.words.iter().filter(|w| w.page == page).collect();
        for (i, a) in words.iter().enumerate() {
            for b in &words[i + 1..] {
                // Ascender-to-descender boxes of adjacent lines touch; require
                // a real overlap of a third of the font size.
                if a.rect.overlaps(&b.rect, 0.0) {
                    let dy = (a.rect.y1.min(b.rect.y1) - a.rect.y0.max(b.rect.y0)).abs();
                    let dx = a.rect.x1.min(b.rect.x1) - a.rect.x0.max(b.rect.x0);
                    if dy > expect.font_size * 0.6 && dx > 0.5 {
                        report.fail(format!("words overlap: {} and {}", describe(a), describe(b)));
                    }
                }
            }
        }
    }

    // 4. Text stays within the text area.
    for word in &actual.words {
        let (pw, _) = actual.pages[word.page];
        if word.rect.x0 < expect.margin - overhang || word.rect.x1 > pw - expect.margin + overhang {
            report.fail(format!("word {} leaves the text area", describe(word)));
        }
    }

    // 5. The space beside each obstacle is used, and text resumes right below.
    if expect.expect_filled {
        for (page, rect, outline) in &obstacles {
            let (pw, _) = actual.pages[*page];
            let left_room = rect.x0 - expect.margin;
            let right_room = pw - expect.margin - rect.x1;
            let beside: Vec<&Word> = actual
                .words
                .iter()
                .filter(|w| w.page == *page && w.rect.y0 < rect.y1 && w.rect.y1 > rect.y0)
                .filter(|w| rect_polygon_distance(&w.rect, outline) > 0.0)
                .collect();
            if left_room.max(right_room) < 4.0 * expect.font_size {
                continue; // too narrow for text: nothing to fill
            }
            let lowest = beside.iter().map(|w| w.rect.y1).fold(f64::MIN, f64::max);
            let highest = beside.iter().map(|w| w.rect.y0).fold(f64::MAX, f64::min);
            let pitch = 1.8 * expect.font_size;
            if beside.is_empty() {
                report.fail(format!("no text beside obstacle {rect:?} on page {}", page + 1));
                continue;
            }
            if highest > rect.y0 + pitch {
                report.fail(format!(
                    "text beside obstacle {rect:?} starts {:.1}pt below its top",
                    highest - rect.y0
                ));
            }
            if lowest < rect.y1 - pitch {
                report.fail(format!(
                    "text beside obstacle {rect:?} ends {:.1}pt above its bottom",
                    rect.y1 - lowest
                ));
            }
            // The first line below the obstacle is not far below it.
            let below = actual
                .words
                .iter()
                .filter(|w| w.page == *page && w.rect.y0 >= rect.y1)
                .map(|w| w.rect.y0)
                .fold(f64::MAX, f64::min);
            if below != f64::MAX && below > rect.y1 + expect.gap + 2.5 * pitch {
                report
                    .fail(format!("text resumes {:.1}pt below obstacle {rect:?}", below - rect.y1));
            }
        }
    }

    report
}

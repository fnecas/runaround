//! Geometric invariants over a matrix of generated documents.
//!
//! Each case is compiled twice: with runaround, and as plain text without it.
//! The plain version provides the reference word sequence.

use std::fmt::Write as _;

use runaround_harness::check::{check, Expect};
use runaround_harness::{compile, repo_root, Geometry};

const GAP: f64 = 10.0;
const MARGIN_CM: f64 = 2.0;
const OBSTACLE: &str = "ff0000";

#[derive(Debug, Clone)]
struct Case {
    name: String,
    mode: &'static str,
    align: Option<&'static str>,
    size: f64,
    justify: bool,
    /// Obstacle width and height in cm.
    obj: (f64, f64),
    /// Words of text before the obstacle (moves it down the page).
    pre: usize,
    /// Words of text after the obstacle.
    post: usize,
    columns: u8,
    rich: bool,
    /// Number of wrapped obstacles.
    count: usize,
    /// Obstacle markup with `FILL` for the fill; `None` is a rectangle of `obj`.
    shape: Option<&'static str>,
    /// Whether enough text follows each obstacle to fill the space beside it.
    filled: bool,
}

impl Case {
    fn new(mode: &'static str) -> Case {
        Case {
            name: String::new(),
            mode,
            align: None,
            size: 11.0,
            justify: true,
            obj: (3.0, 3.0),
            pre: 0,
            post: 220,
            columns: 1,
            rich: false,
            count: 1,
            shape: None,
            filled: true,
        }
    }

    fn named(mut self) -> Case {
        self.name = format!(
            "{}-{}-{}pt-{}-{}x{}-pre{}-post{}-col{}{}-n{}",
            self.mode,
            self.align.unwrap_or("auto"),
            self.size,
            if self.justify { "just" } else { "rag" },
            self.obj.0,
            self.obj.1,
            self.pre,
            self.post,
            self.columns,
            if self.rich { "-rich" } else { "" },
            self.count,
        );
        if let Some(shape) = self.shape {
            let tag: String = shape.chars().take_while(|c| c.is_alphabetic()).collect();
            self.name += &format!("-{tag}{}", shape.len());
        }
        self
    }

    fn text(&self, words: usize, seed: usize) -> String {
        // Paragraphs of varying length; with `rich`, inline markup that
        // straddles split points, lists and headings.
        let mut out = String::new();
        let mut left = words;
        let mut k = seed;
        while left > 0 {
            let n = (25 + (k * 37) % 50).min(left);
            if self.rich && k % 3 == 1 {
                let a = n / 3;
                let b = n / 4;
                let _ = write!(
                    out,
                    "#lorem({a}) *#lorem({b})* _#lorem({b})_ $x^2 + y$ #lorem({})\n\n",
                    n.saturating_sub(a + 2 * b).max(1)
                );
            } else if self.rich && k % 5 == 2 {
                let _ = write!(
                    out,
                    "== Section {k}\n\n- #lorem(8)\n- #lorem(12)\n+ #lorem(6)\n+ #lorem(9)\n\n"
                );
            } else {
                let _ = write!(out, "#lorem({n})\n\n");
            }
            left -= n;
            k += 1;
        }
        out
    }

    fn source(&self, plain: bool) -> String {
        let mut s = String::from("#import \"/src/lib.typ\": runaround, wrap\n");
        let cols =
            if self.columns > 1 { format!(", columns: {}", self.columns) } else { String::new() };
        let _ = writeln!(s, "#set page(width: 14cm, height: 20cm, margin: {MARGIN_CM}cm{cols})");
        let _ = writeln!(s, "#set text(size: {}pt, hyphenate: false)", self.size);
        let _ = writeln!(s, "#set par(justify: {})", self.justify);
        if !plain {
            let _ = writeln!(s, "#show: runaround.with(gap: {GAP}pt)");
        }
        s += &self.text(self.pre, 0);
        let per = self.post / self.count;
        for i in 0..self.count {
            let fill = format!("rgb(\"#{OBSTACLE}\")");
            let obj = match self.shape {
                Some(shape) => shape.replace("FILL", &fill),
                None => {
                    format!("rect(width: {}cm, height: {}cm, fill: {fill})", self.obj.0, self.obj.1)
                }
            };
            if plain {
                let _ = writeln!(s, "#{obj}\n");
            } else {
                let align = self.align.map(|a| format!(", align: {a}")).unwrap_or_default();
                let _ = writeln!(s, "#wrap(mode: \"{}\"{align})[#{obj}]", self.mode);
            }
            s += &self.text(per, i + 1);
        }
        s
    }
}

fn run(cases: Vec<Case>) {
    let out_dir = repo_root().join("tests/out/geometry");
    let failures: Vec<String> = std::thread::scope(|scope| {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let chunks: Vec<Vec<Case>> = (0..threads)
            .map(|t| cases.iter().skip(t).step_by(threads).cloned().collect())
            .collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let out_dir = out_dir.clone();
                scope.spawn(move || {
                    let mut failures = Vec::new();
                    for case in chunk {
                        if let Err(message) = run_case(&case) {
                            std::fs::create_dir_all(&out_dir).unwrap();
                            std::fs::write(
                                out_dir.join(format!("{}.typ", case.name)),
                                case.source(false),
                            )
                            .unwrap();
                            failures.push(format!(
                                "{}:\n  {}",
                                case.name,
                                message.replace('\n', "\n  ")
                            ));
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    assert!(
        failures.is_empty(),
        "{} of {} cases failed (sources in tests/out/geometry):\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

fn run_case(case: &Case) -> Result<(), String> {
    let vpath = format!("tests/out/geometry/{}.typ", case.name);
    let actual = compile(&vpath, &case.source(false), &[]).map_err(|e| e.join("\n"))?;
    if !actual.warnings.is_empty() {
        return Err(format!("warnings: {}", actual.warnings.join("; ")));
    }
    let plain = compile(&vpath, &case.source(true), &[]).map_err(|e| e.join("\n"))?;
    let expect = Expect {
        obstacle_fill: OBSTACLE,
        gap: GAP,
        font_size: case.size,
        margin: MARGIN_CM * 72.0 / 2.54,
        obstacles: case.count,
        expect_filled: case.filled,
    };
    let report = check(&Geometry::extract(&actual.doc), &Geometry::extract(&plain.doc), &expect);
    if report.is_ok() {
        Ok(())
    } else {
        Err(report.failures.join("\n"))
    }
}

#[test]
fn single_side_matrix() {
    let mut cases = Vec::new();
    for mode in ["after", "before"] {
        for size in [9.0, 11.0, 14.0] {
            for justify in [true, false] {
                for obj in [(3.0, 3.0), (4.0, 7.0), (2.5, 1.2)] {
                    let mut c = Case::new(mode);
                    c.size = size;
                    c.justify = justify;
                    c.obj = obj;
                    cases.push(c.named());
                }
            }
        }
    }
    run(cases);
}

#[test]
fn page_breaks() {
    // Move the obstacle down the page so that it lands near the bottom.
    let mut cases = Vec::new();
    for mode in ["after", "before"] {
        for pre in [150, 230, 260, 290, 320, 350] {
            for obj in [(3.0, 3.0), (3.5, 6.0)] {
                let mut c = Case::new(mode);
                c.pre = pre;
                c.obj = obj;
                cases.push(c.named());
            }
        }
    }
    run(cases);
}

#[test]
fn alignment_and_tight_rectangles() {
    let mut cases = Vec::new();
    for (mode, align) in [
        ("after", "left"),
        ("after", "center"),
        ("before", "right"),
        ("before", "center"),
        ("tight", "left"),
        ("tight", "right"),
    ] {
        let mut c = Case::new(mode);
        c.align = Some(align);
        cases.push(c.named());
    }
    run(cases);
}

#[test]
fn rich_content_and_several_objects() {
    let mut cases = Vec::new();
    for mode in ["after", "before"] {
        for count in [1, 3] {
            for justify in [true, false] {
                let mut c = Case::new(mode);
                c.rich = true;
                c.count = count;
                c.justify = justify;
                c.post = 600;
                cases.push(c.named());
            }
        }
    }
    run(cases);
}

#[test]
fn columns() {
    let mut cases = Vec::new();
    for mode in ["after", "before"] {
        for pre in [0, 200, 420] {
            let mut c = Case::new(mode);
            c.columns = 2;
            c.obj = (1.8, 2.5);
            c.size = 9.0;
            c.pre = pre;
            c.post = 400;
            cases.push(c.named());
        }
    }
    run(cases);
}

#[test]
fn parallel_line_by_line() {
    let mut cases = Vec::new();
    for size in [9.0, 11.0] {
        for justify in [true, false] {
            for (obj, pre) in [((3.0, 3.0), 0), ((2.5, 5.0), 120), ((3.0, 3.0), 300)] {
                let mut c = Case::new("parallel");
                c.size = size;
                c.justify = justify;
                c.obj = obj;
                c.pre = pre;
                cases.push(c.named());
            }
        }
    }
    for rich in [false, true] {
        let mut c = Case::new("parallel");
        c.rich = rich;
        c.count = 2;
        c.post = 500;
        cases.push(c.named());
    }
    run(cases);
}

#[test]
fn tight_contours() {
    let shapes = [
        "circle(radius: 1.6cm, fill: FILL)",
        "ellipse(width: 4cm, height: 2.4cm, fill: FILL)",
        "polygon(fill: FILL, (0cm, 0cm), (3.5cm, 1.5cm), (0.4cm, 3.5cm))",
        "polygon(fill: FILL, (1.5cm, 0cm), (3cm, 1.5cm), (1.5cm, 3cm), (0cm, 1.5cm))",
        "rect(width: 3cm, height: 3cm, radius: 1.2cm, fill: FILL)",
    ];
    let mut cases = Vec::new();
    for shape in shapes {
        for align in ["left", "right", "center"] {
            for justify in [true, false] {
                let mut c = Case::new("tight");
                c.shape = Some(shape);
                c.align = Some(align);
                c.justify = justify;
                cases.push(c.named());
            }
        }
    }
    // Near page bottoms and with several objects.
    for pre in [250, 300] {
        let mut c = Case::new("tight");
        c.shape = Some(shapes[0]);
        c.pre = pre;
        cases.push(c.named());
    }
    let mut c = Case::new("tight");
    c.shape = Some(shapes[3]);
    c.align = Some("center");
    c.count = 3;
    c.rich = true;
    c.post = 600;
    cases.push(c.named());
    run(cases);
}

#[test]
fn objects_close_together() {
    // Little text between objects: the next object waits until the previous
    // one ends, and nothing overlaps.
    let mut cases = Vec::new();
    for (mode, post) in
        [("after", 40), ("before", 60), ("after", 90), ("parallel", 60), ("tight", 50)]
    {
        for count in [2, 4] {
            let mut c = Case::new(mode);
            c.count = count;
            c.post = post * count;
            c.filled = false;
            if mode == "tight" {
                c.shape = Some("circle(radius: 1.3cm, fill: FILL)");
            }
            cases.push(c.named());
        }
    }
    run(cases);
}

#[test]
fn long_documents_converge() {
    // Many objects, some near page bottoms: decisions must not cascade from
    // one layout iteration to the next (identical and unique paragraphs).
    for unique in [false, true] {
        let mut s = String::from(
            "#import \"/src/lib.typ\": runaround, wrap\n#show: runaround\n#set page(paper: \"a4\")\n#set par(justify: true)\n",
        );
        for i in 0..50 {
            let extra = if unique { i } else { 0 };
            let mode = ["after", "before", "parallel", "tight"][i % 4];
            let shape = if mode == "tight" {
                "circle(radius: 1.4cm, fill: red)".to_string()
            } else {
                format!(
                    "rect(width: {}cm, height: {}cm, fill: red)",
                    3 + i % 3,
                    2.5 + (i % 4) as f64
                )
            };
            let _ = write!(
                s,
                "== Section {i}\n\n#lorem({})\n\n#wrap(mode: \"{mode}\")[#{shape}]\n#lorem({})\n\n#lorem({})\n\n",
                60 + extra,
                120 + extra,
                80 + extra
            );
        }
        let compiled =
            compile("tests/out/long.typ", &s, &[]).unwrap_or_else(|e| panic!("{}", e.join("\n")));
        assert!(compiled.warnings.is_empty(), "unique={unique}: {:?}", compiled.warnings);
    }
}

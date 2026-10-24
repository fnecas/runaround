//! Performance and memory budgets.
//!
//! Run in release mode: `cargo test --release --test perf -- --ignored --nocapture`.
//!
//! A generated document with many wrapped objects is compiled twice: as plain
//! text without wrapping and with runaround. Peak heap is measured exactly with
//! a counting allocator; Typst's caches are evicted before each compilation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use runaround_harness::{compile, Geometry};

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Plain,
    Runaround,
}

fn document(kind: Kind, sections: usize) -> String {
    let mut s = String::new();
    match kind {
        Kind::Runaround => s += "#import \"/src/lib.typ\": runaround, wrap\n#show: runaround\n",
        Kind::Plain => {}
    }
    s += "#set page(paper: \"a4\")\n#set text(size: 11pt)\n#set par(justify: true)\n";
    for i in 0..sections {
        let obj =
            format!("rect(width: {}cm, height: {}cm, fill: red)", 3 + i % 3, 2.5 + (i % 4) as f64);
        let _ = write!(s, "== Section {i}\n\n#lorem(60)\n\n");
        match kind {
            Kind::Plain => {
                let _ = write!(s, "#{obj}\n#lorem(120)\n\n");
            }
            Kind::Runaround => {
                let mode = if i % 2 == 0 { "after" } else { "before" };
                let _ = write!(s, "#wrap(mode: \"{mode}\")[#{obj}]\n#lorem(120)\n\n");
            }
        }
        s += "#lorem(80)\n\n";
    }
    s
}

struct Run {
    time: Duration,
    peak: usize,
    pages: usize,
}

fn measure(kind: Kind, sections: usize) -> Run {
    let source = document(kind, sections);
    typst::comemo::evict(0);
    let base = CURRENT.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let start = Instant::now();
    let compiled =
        compile("tests/out/perf.typ", &source, &[]).unwrap_or_else(|e| panic!("{}", e.join("\n")));
    let time = start.elapsed();
    let peak = PEAK.load(Ordering::Relaxed) - base;
    assert!(compiled.warnings.is_empty(), "warnings: {:?}", compiled.warnings);
    let pages = Geometry::extract(&compiled.doc).pages.len();
    drop(compiled);
    typst::comemo::evict(0);
    Run { time, peak, pages }
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[test]
#[ignore = "run in release mode with --ignored"]
fn budgets() {
    // Warm up fonts and other one-time state.
    measure(Kind::Plain, 2);

    for sections in [20, 60] {
        let plain = measure(Kind::Plain, sections);
        let ours = measure(Kind::Runaround, sections);
        eprintln!("{sections} wrapped objects:");
        for (name, run) in [("plain", &plain), ("runaround", &ours)] {
            eprintln!(
                "  {name:<10} {:>8.0} ms  {:>8.1} MiB peak  {:>3} pages",
                run.time.as_secs_f64() * 1000.0,
                mib(run.peak),
                run.pages
            );
        }
        // Budgets. The plain document uses only the embedded fonts and needs a
        // single layout pass, so it is unusually cheap; runaround needs three
        // passes (introspection) and measures the text beside each object.
        let time_ratio = ours.time.as_secs_f64() / plain.time.as_secs_f64();
        let extra_mib = mib(ours.peak.saturating_sub(plain.peak));
        assert!(time_ratio < 8.0, "runaround takes {time_ratio:.1}x the plain compile time");
        assert!(
            extra_mib < 40.0,
            "runaround needs {extra_mib:.1} MiB more than the plain document"
        );
    }
}

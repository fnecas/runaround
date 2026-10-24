//! Renders the README illustrations: every `docs/*.typ` becomes
//! `docs/<name>-light.png` and `docs/<name>-dark.png`. Multi-page sources are
//! composed into a grid of three columns.

use runaround_harness::{compile_file, repo_root};
use tiny_skia::{Pixmap, PixmapPaint, Transform};
use typst::utils::Scalar;
use typst_render::RenderOptions;

const COLUMNS: usize = 3;
const GUTTER: u32 = 16;

fn main() {
    let docs = repo_root().join("docs");
    let mut sources: Vec<_> = std::fs::read_dir(&docs)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "typ"))
        .collect();
    sources.sort();
    for path in sources {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        for theme in ["light", "dark"] {
            let compiled = compile_file(&format!("docs/{name}.typ"), &[("theme", theme)])
                .unwrap_or_else(|e| panic!("{name}: {}", e.join("\n")));
            assert!(compiled.warnings.is_empty(), "{name}: {:?}", compiled.warnings);
            let opts = RenderOptions { pixel_per_pt: Scalar::new(2.0), render_bleed: false };
            let pages: Vec<Pixmap> =
                compiled.doc.pages().iter().map(|p| typst_render::render(p, &opts)).collect();
            let image = if pages.len() == 1 {
                pages.into_iter().next().unwrap()
            } else {
                grid(&pages, theme)
            };
            let out = docs.join(format!("{name}-{theme}.png"));
            image.save_png(&out).unwrap();
            println!("wrote {} ({}x{})", out.display(), image.width(), image.height());
        }
    }
}

fn grid(pages: &[Pixmap], theme: &str) -> Pixmap {
    let (w, h) = (pages[0].width(), pages[0].height());
    let cols = COLUMNS.min(pages.len()) as u32;
    let rows = pages.len().div_ceil(COLUMNS) as u32;
    let mut out =
        Pixmap::new(cols * w + (cols - 1) * GUTTER, rows * h + (rows - 1) * GUTTER).unwrap();
    if theme == "dark" {
        out.fill(tiny_skia::Color::from_rgba8(0x30, 0x36, 0x3d, 255));
    } else {
        out.fill(tiny_skia::Color::from_rgba8(0xd0, 0xd7, 0xde, 255));
    }
    for (i, page) in pages.iter().enumerate() {
        let (c, r) = ((i % COLUMNS) as u32, (i / COLUMNS) as u32);
        out.draw_pixmap(
            (c * (w + GUTTER)) as i32,
            (r * (h + GUTTER)) as i32,
            page.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
    out
}

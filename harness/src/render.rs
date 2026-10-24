//! Rendering pages to PNG and comparing them with reference images.

use std::path::Path;

use typst::utils::Scalar;
use typst_layout::PagedDocument;
use typst_render::RenderOptions;

/// Pixels per point used for reference images (≈ 144 dpi).
pub const PIXEL_PER_PT: f64 = 2.0;

/// Renders every page and returns one PNG per page.
pub fn render_pngs(doc: &PagedDocument) -> Vec<Vec<u8>> {
    let opts = RenderOptions { pixel_per_pt: Scalar::new(PIXEL_PER_PT), render_bleed: false };
    doc.pages()
        .iter()
        .map(|page| typst_render::render(page, &opts).encode_png().expect("PNG encoding"))
        .collect()
}

/// The result of comparing a rendering with its reference.
#[derive(Debug)]
pub enum Comparison {
    Match,
    /// No reference existed; it was written.
    Created,
    /// The reference was regenerated (`UPDATE_REFS=1`).
    Updated,
    Mismatch {
        differing_pixels: usize,
        total_pixels: usize,
    },
    SizeMismatch,
}

/// Compares `png` with the reference at `path`. Pixels whose channels differ by
/// more than `channel_tolerance` count as different; the images match when at
/// most `max_differing` pixels differ.
pub fn compare_with_reference(
    png: &[u8],
    path: &Path,
    channel_tolerance: u8,
    max_differing: usize,
) -> Comparison {
    let update = std::env::var_os("UPDATE_REFS").is_some_and(|v| v != "0");
    if !path.exists() || update {
        let existed = path.exists();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, png).unwrap();
        return if existed { Comparison::Updated } else { Comparison::Created };
    }
    let actual = tiny_skia::Pixmap::decode_png(png).expect("decode rendering");
    let expected =
        tiny_skia::Pixmap::decode_png(&std::fs::read(path).unwrap()).expect("decode reference");
    if actual.width() != expected.width() || actual.height() != expected.height() {
        return Comparison::SizeMismatch;
    }
    let differing = actual
        .data()
        .chunks_exact(4)
        .zip(expected.data().chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > channel_tolerance))
        .count();
    if differing <= max_differing {
        Comparison::Match
    } else {
        Comparison::Mismatch {
            differing_pixels: differing,
            total_pixels: (actual.width() * actual.height()) as usize,
        }
    }
}

//! Test harness for the `runaround` Typst package.
//!
//! Compiles Typst documents in-process with the same compiler version the
//! package supports, using only the fonts embedded in Typst (so results are
//! identical on every machine), and exposes the laid-out geometry: every word,
//! shape and image with its exact box on the page.

pub mod check;
pub mod geom;
pub mod render;
pub mod world;

pub use geom::{Geometry, Item, Rect, Word};
pub use world::{compile, compile_file, compile_html, Compiled, Input};

use std::path::{Path, PathBuf};

/// The repository root, which is also the Typst project root for tests.
pub fn repo_root() -> &'static Path {
    static ROOT: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    });
    &ROOT
}

/// The package version declared in `typst.toml`.
pub fn package_version() -> String {
    let manifest =
        std::fs::read_to_string(repo_root().join("typst.toml")).expect("typst.toml is readable");
    manifest
        .lines()
        .find_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("version")?.trim_start().strip_prefix('=')?;
            Some(rest.trim().trim_matches('"').to_string())
        })
        .expect("typst.toml declares a version")
}

//! File-based tests:
//! - `tests/unit/*.typ` must compile without errors or warnings (they use
//!   `assert` to test pure logic);
//! - `tests/errors/*.typ` must fail with the message given in their first
//!   line as `// error: <expected substring>`;
//! - `tests/visual/*.typ` are rendered and compared with `tests/ref/*.png`
//!   (`UPDATE_REFS=1` regenerates the references).

use std::path::PathBuf;

use runaround_harness::render::{compare_with_reference, render_pngs, Comparison};
use runaround_harness::{compile_file, repo_root};

fn typ_files(dir: &str) -> Vec<(String, PathBuf)> {
    let mut files: Vec<_> = std::fs::read_dir(repo_root().join(dir))
        .unwrap_or_else(|_| panic!("{dir} exists"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "typ"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (name, p)
        })
        .collect();
    files.sort();
    files
}

fn vpath(dir: &str, name: &str) -> String {
    format!("{dir}/{name}.typ")
}

#[test]
fn unit() {
    let mut failures = Vec::new();
    for (name, _) in typ_files("tests/unit") {
        match compile_file(&vpath("tests/unit", &name), &[]) {
            Ok(c) if c.warnings.is_empty() => {}
            Ok(c) => failures.push(format!("{name}: warnings:\n  {}", c.warnings.join("\n  "))),
            Err(errors) => failures.push(format!("{name}:\n  {}", errors.join("\n  "))),
        }
    }
    assert!(failures.is_empty(), "unit tests failed:\n{}", failures.join("\n"));
}

#[test]
fn errors() {
    let mut failures = Vec::new();
    for (name, path) in typ_files("tests/errors") {
        let text = std::fs::read_to_string(&path).unwrap();
        let expected = text
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("// error: "))
            .unwrap_or_else(|| panic!("{name}: first line must be `// error: <message>`"))
            .trim()
            .to_string();
        match compile_file(&vpath("tests/errors", &name), &[]) {
            Ok(_) => failures.push(format!("{name}: compiled, expected error `{expected}`")),
            Err(errors) => {
                if !errors.iter().any(|e| e.contains(&expected)) {
                    failures.push(format!(
                        "{name}: expected `{expected}`, got:\n  {}",
                        errors.join("\n  ")
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "error tests failed:\n{}", failures.join("\n"));
}

#[test]
fn visual() {
    let mut failures = Vec::new();
    let mut created = Vec::new();
    for (name, _) in typ_files("tests/visual") {
        let compiled = match compile_file(&vpath("tests/visual", &name), &[]) {
            Ok(c) => c,
            Err(errors) => {
                failures.push(format!("{name}:\n  {}", errors.join("\n  ")));
                continue;
            }
        };
        if !compiled.warnings.is_empty() {
            failures.push(format!("{name}: warnings:\n  {}", compiled.warnings.join("\n  ")));
        }
        for (i, png) in render_pngs(&compiled.doc).iter().enumerate() {
            let reference = repo_root().join(format!("tests/ref/{name}-{}.png", i + 1));
            match compare_with_reference(png, &reference, 8, 50) {
                Comparison::Match | Comparison::Updated => {}
                Comparison::Created => created.push(reference.display().to_string()),
                other => {
                    let out = repo_root().join(format!("tests/out/visual/{name}-{}.png", i + 1));
                    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
                    std::fs::write(&out, png).unwrap();
                    failures.push(format!(
                        "{name} page {}: {other:?} (actual: {})",
                        i + 1,
                        out.display()
                    ));
                }
            }
        }
    }
    if !created.is_empty() {
        eprintln!("created references (review them):\n  {}", created.join("\n  "));
    }
    assert!(failures.is_empty(), "visual tests failed:\n{}", failures.join("\n"));
}

#[test]
fn html_export() {
    // Without page layout, objects become CSS floats; nothing is lost.
    let (html, warnings) = runaround_harness::compile_html("tests/visual/figure.typ")
        .unwrap_or_else(|e| panic!("{}", e.join("\n")));
    assert!(html.contains("float: left"), "no left float in:\n{html}");
    assert!(html.contains("float: right"), "no right float in:\n{html}");
    assert!(html.contains("A gradient that has a long caption."), "caption lost");
    assert!(html.contains("As Figure 1 shows") || html.contains("As <a"), "reference text lost");
    // The test document itself uses things HTML export ignores (page setup,
    // shapes); the package must not cause warnings of its own.
    let ours: Vec<_> = warnings.iter().filter(|w| w.contains("/src/")).collect();
    assert!(ours.is_empty(), "warnings from the package: {ours:?}");
}

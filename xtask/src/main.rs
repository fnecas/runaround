//! Development tasks for runaround.
//!
//! - `cargo xtask plugin`: build the WebAssembly plugin and copy it to
//!   `src/runaround.wasm`.
//! - `cargo xtask bundle`: assemble `dist/runaround/<version>/`, the
//!   directory to copy into `packages/preview/` of typst/packages.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let result = match task.as_str() {
        "plugin" => plugin(),
        "bundle" => bundle(),
        _ => Err("usage: cargo xtask <plugin|bundle>".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn plugin() -> Result<(), String> {
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(root())
        .args([
            "build",
            "-p",
            "runaround-plugin",
            "--profile",
            "wasm",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() {
        return Err("building the plugin failed".into());
    }
    let built = root().join("target/wasm32-unknown-unknown/wasm/runaround_plugin.wasm");
    let dest = root().join("src/runaround.wasm");
    fs::copy(&built, &dest).map_err(|e| format!("cannot copy {}: {e}", built.display()))?;
    let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    println!("wrote {} ({:.1} KiB)", dest.display(), size as f64 / 1024.0);
    Ok(())
}

/// The package version from `typst.toml`. It becomes a directory name that is
/// deleted and recreated, so only `major.minor.patch` is accepted.
fn version() -> Result<String, String> {
    let manifest = fs::read_to_string(root().join("typst.toml")).map_err(|e| e.to_string())?;
    let version = manifest
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("version")?
                .trim()
                .strip_prefix('=')
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .ok_or("typst.toml has no version")?;
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3
        || !parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!("invalid version {version:?} in typst.toml"));
    }
    Ok(version)
}

fn copy_tree(from: &Path, to: &Path, filter: &dyn Fn(&Path) -> bool) -> Result<(), String> {
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target, filter)?;
        } else if filter(&path) {
            fs::create_dir_all(to).map_err(|e| e.to_string())?;
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn bundle() -> Result<(), String> {
    let version = version()?;
    let out = root().join("dist/runaround").join(&version);
    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for file in ["typst.toml", "LICENSE", "README.md"] {
        fs::copy(root().join(file), out.join(file)).map_err(|e| format!("{file}: {e}"))?;
    }
    if !root().join("src/runaround.wasm").exists() {
        return Err("src/runaround.wasm is missing; run `cargo xtask plugin` first".into());
    }
    copy_tree(&root().join("src"), &out.join("src"), &|p| {
        p.extension().is_some_and(|x| x == "typ" || x == "wasm")
    })?;
    // README illustrations: committed to typst/packages but excluded from the
    // downloaded archive by `exclude` in typst.toml.
    if root().join("docs").exists() {
        copy_tree(&root().join("docs"), &out.join("docs"), &|_| true)?;
    }
    println!("bundle written to {}", out.display());
    Ok(())
}

//! Every `typ` code block in README.md compiles without errors or warnings,
//! with `@preview/runaround:<version>` resolved to the working tree.

use runaround_harness::{compile, package_version, repo_root};

#[test]
fn readme_examples_compile() {
    let readme = std::fs::read_to_string(repo_root().join("README.md")).unwrap();
    let version = package_version();
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in readme.lines() {
        match (&mut current, line.trim_start()) {
            (None, l) if l.starts_with("```typ") => current = Some(String::new()),
            (Some(block), "```") => {
                blocks.push(std::mem::take(block));
                current = None;
            }
            (Some(block), _) => {
                block.push_str(line);
                block.push('\n');
            }
            _ => {}
        }
    }
    assert!(blocks.len() >= 4, "README has {} typ examples", blocks.len());
    let mut failures = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        for import in block.lines().filter(|l| l.contains("@preview/runaround:")) {
            if !import.contains(&format!("@preview/runaround:{version}")) {
                failures.push(format!(
                    "example {}: import does not use version {version}: {import}",
                    i + 1
                ));
            }
        }
        match compile(&format!("README-{}.typ", i + 1), block, &[]) {
            Ok(c) if c.warnings.is_empty() => {}
            Ok(c) => failures.push(format!("example {}: {}", i + 1, c.warnings.join("; "))),
            Err(e) => failures.push(format!("example {}:\n  {}", i + 1, e.join("\n  "))),
        }
    }
    assert!(failures.is_empty(), "README examples:\n{}", failures.join("\n"));
}

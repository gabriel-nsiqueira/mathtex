//! Diagnostic that normalizes generated files at a base commit and diffs them against the working tree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_BASE: &str = "0d6064b";

fn main() -> std::process::ExitCode {
    let base = std::env::var("MATHTEX_SEMANTIC_DIFF_BASE").unwrap_or_else(|_| DEFAULT_BASE.into());
    let repo_root = repo_root();
    let gen_root = repo_root.join("generated/portable-engine/src");

    // Union of files from both the base commit and the working tree covers added and removed files.
    let mut rel_paths: BTreeSet<String> = BTreeSet::new();
    collect_working_tree(&gen_root, &repo_root, &mut rel_paths);
    collect_base_tree(&base, &repo_root, &mut rel_paths);

    let mut total_with_diff = 0usize;
    let mut parse_failures = 0usize;

    for rel in &rel_paths {
        let before_raw = git_show(&base, rel, &repo_root);
        let after_raw = std::fs::read_to_string(repo_root.join(rel)).ok();

        let before = before_raw.as_deref().map(normalize);
        let after = after_raw.as_deref().map(normalize);

        match (before, after) {
            (Some(Ok(before)), Some(Ok(after))) => {
                if before != after {
                    total_with_diff += 1;
                    println!("=== NORMALIZED DIFF: {rel} ===");
                    print_line_diff(&before, &after);
                    println!();
                }
            }
            (None, Some(Ok(_))) => {
                total_with_diff += 1;
                println!("=== ADDED (not in {base}): {rel} ===\n");
            }
            (Some(Ok(_)), None) => {
                total_with_diff += 1;
                println!("=== REMOVED (gone in working tree): {rel} ===\n");
            }
            (before, after) => {
                if matches!(before, Some(Err(_))) || matches!(after, Some(Err(_))) {
                    parse_failures += 1;
                    println!("=== PARSE FAILURE (cannot normalize): {rel} ===\n");
                }
            }
        }
    }

    println!(
        "semantic_diff: {} file(s) with a normalized difference, {} parse failure(s), base={base}",
        total_with_diff, parse_failures
    );

    if parse_failures > 0 {
        std::process::ExitCode::from(2)
    } else if total_with_diff > 0 {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::from(0)
    }
}

/// Normalizes a Rust source file with `prettyplease`.
fn normalize(source: &str) -> Result<String, syn::Error> {
    let file: syn::File = syn::parse_str(source)?;
    Ok(prettyplease::unparse(&file))
}

/// Prints changed regions between two texts for review.
fn print_line_diff(before: &str, after: &str) {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    // Sliding anchor walk is enough for normalized equivalence review.
    let mut bi = 0;
    let mut ai = 0;
    while bi < before_lines.len() || ai < after_lines.len() {
        if bi < before_lines.len() && ai < after_lines.len() && before_lines[bi] == after_lines[ai]
        {
            bi += 1;
            ai += 1;
            continue;
        }
        let (next_bi, next_ai) = resync(&before_lines, bi, &after_lines, ai);
        for line in &before_lines[bi..next_bi] {
            println!("- {line}");
        }
        for line in &after_lines[ai..next_ai] {
            println!("+ {line}");
        }
        bi = next_bi;
        ai = next_ai;
    }
}

/// Finds the next matching pair `(b, a)` within a bounded window.
fn resync(before: &[&str], bi: usize, after: &[&str], ai: usize) -> (usize, usize) {
    const WINDOW: usize = 200;
    let b_end = (bi + WINDOW).min(before.len());
    let a_end = (ai + WINDOW).min(after.len());
    for offset in 1..WINDOW {
        for (b, before_line) in before.iter().enumerate().take(b_end).skip(bi) {
            let a = ai + offset.saturating_sub(b - bi);
            if a < a_end && *before_line == after[a] {
                return (b, a);
            }
        }
    }
    (b_end, a_end)
}

fn git_show(base: &str, rel: &str, repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("show")
        .arg(format!("{base}:{rel}"))
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout).into_owned()
    })
}

fn collect_working_tree(dir: &Path, repo_root: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_working_tree(&path, repo_root, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(rel) = path.strip_prefix(repo_root) {
                out.insert(rel.to_string_lossy().into_owned());
            }
        }
    }
}

fn collect_base_tree(base: &str, repo_root: &Path, out: &mut BTreeSet<String>) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("ls-tree")
        .arg("-r")
        .arg("--name-only")
        .arg(base)
        .arg("generated/portable-engine/src")
        .output();
    let Ok(output) = output else { return };
    if !output.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.ends_with(".rs") {
            out.insert(line.to_string());
        }
    }
}

fn repo_root() -> PathBuf {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git rev-parse");
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    PathBuf::from(root)
}

//! The pin file: upstream tool versions plus a SHA-256 manifest of every cache input.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::inputs::{Inputs, CACHE_INPUTS};
use crate::model::{io_error, read_file, write_file, PatchError};

/// Pin file path, which lives beside this crate rather than under the output root.
const PIN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/pin.toml");

const INPUTS_SECTION: &str = "[inputs]";

/// Checks the cache against the pin manifest, or rewrites the manifest when `MATHTEX_WEB2C_UPDATE_PIN=1`.
pub(crate) fn verify_or_update(inputs: &Inputs) -> Result<(), PatchError> {
    let pin_path = Path::new(PIN_PATH);
    let actual = input_digests(inputs)?;
    let text = read_file(pin_path)?;
    if std::env::var("MATHTEX_WEB2C_UPDATE_PIN").as_deref() == Ok("1") {
        write_file(pin_path, with_manifest(&text, &actual))?;
        eprintln!(
            "patch_engine: updated input manifest in {}",
            pin_path.display()
        );
        return Ok(());
    }
    let pinned = parse_manifest(&text).map_err(|line| PatchError::MalformedPin {
        path: pin_path.to_path_buf(),
        line,
    })?;
    let files = mismatched(&pinned, &actual);
    if files.is_empty() {
        Ok(())
    } else {
        Err(PatchError::PinMismatch {
            path: pin_path.to_path_buf(),
            files,
        })
    }
}

fn input_digests(inputs: &Inputs) -> Result<Vec<(String, String)>, PatchError> {
    let mut digests = Vec::new();
    for relative in CACHE_INPUTS {
        let path = inputs.cache.join(relative);
        let bytes = std::fs::read(&path).map_err(|error| io_error(&path, error))?;
        digests.push((relative.to_string(), hex(&Sha256::digest(&bytes))));
    }
    Ok(digests)
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("write to String");
    }
    text
}

/// Parses the `[inputs]` table of `"path" = "digest"` lines, or returns the one based bad line.
fn parse_manifest(text: &str) -> Result<BTreeMap<String, String>, usize> {
    let mut manifest = BTreeMap::new();
    let mut in_inputs = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_inputs = line == INPUTS_SECTION;
            continue;
        }
        if !in_inputs {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(index + 1)?;
        let key = quoted(key).ok_or(index + 1)?;
        let value = quoted(value).ok_or(index + 1)?;
        manifest.insert(key.to_string(), value.to_string());
    }
    Ok(manifest)
}

fn quoted(text: &str) -> Option<&str> {
    text.trim().strip_prefix('"')?.strip_suffix('"')
}

/// Cache inputs whose digest is absent from or differs in the manifest, plus stale manifest entries.
fn mismatched(pinned: &BTreeMap<String, String>, actual: &[(String, String)]) -> Vec<String> {
    let mut files = actual
        .iter()
        .filter(|(path, digest)| pinned.get(path) != Some(digest))
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    for path in pinned.keys() {
        if !actual.iter().any(|(actual_path, _)| actual_path == path) {
            files.push(path.clone());
        }
    }
    files
}

/// Replaces the `[inputs]` table, which is always last, keeping everything before it.
fn with_manifest(text: &str, digests: &[(String, String)]) -> String {
    let head = text
        .lines()
        .take_while(|line| line.trim() != INPUTS_SECTION)
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = head.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(INPUTS_SECTION);
    out.push('\n');
    for (path, digest) in digests {
        writeln!(out, "\"{path}\" = \"{digest}\"").expect("write to String");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "# pins\n[upstream]\nc2rust = \"C2Rust 0.22.1\"\n\n[inputs]\n\"a.rs\" = \"11\"\n\"b.pool\" = \"22\"\n";

    fn digests(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(path, digest)| ((*path).to_string(), (*digest).to_string()))
            .collect()
    }

    #[test]
    fn manifest_parses_only_the_inputs_table() {
        let manifest = parse_manifest(PIN).expect("valid pin");
        assert_eq!(manifest.len(), 2);
        assert_eq!(manifest["a.rs"], "11");
    }

    #[test]
    fn malformed_entry_reports_its_line() {
        assert_eq!(parse_manifest("[inputs]\n\"a.rs\" = 11\n"), Err(2));
    }

    #[test]
    fn mismatch_lists_changed_missing_and_stale_files() {
        let pinned = parse_manifest(PIN).expect("valid pin");
        assert!(mismatched(&pinned, &digests(&[("a.rs", "11"), ("b.pool", "22")])).is_empty());
        assert_eq!(
            mismatched(&pinned, &digests(&[("a.rs", "99"), ("c.rs", "33")])),
            ["a.rs", "c.rs", "b.pool"]
        );
    }

    #[test]
    fn update_rewrites_inputs_and_keeps_upstream() {
        let updated = with_manifest(PIN, &digests(&[("a.rs", "99")]));
        assert!(updated.contains("c2rust = \"C2Rust 0.22.1\""));
        assert!(updated.ends_with("[inputs]\n\"a.rs\" = \"99\"\n"));
        assert_eq!(parse_manifest(&updated).expect("valid pin").len(), 1);
    }

    #[test]
    fn digest_is_lowercase_sha256_hex() {
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}

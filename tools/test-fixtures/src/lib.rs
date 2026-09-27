//! Finds the texlive-source checkout at the pinned commit whose test data the mathtex tests read.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Environment variable naming a texlive-source checkout, read before the default cache path.
pub const SOURCE_ENV: &str = "MATHTEX_TEXLIVE_SOURCE";

/// Setting this to 1 turns a missing checkout into a failure instead of a skip.
pub const REQUIRE_ENV: &str = "MATHTEX_REQUIRE_FIXTURES";

/// Computer Modern roman metrics, identical to TeX Live's.
pub const CMR10: &str = "texk/dvipsk/testdata/cmr10.tfm";

/// Computer Modern math italic metrics.
pub const CMMI10: &str = "texk/dvipsk/testdata/cmmi10.tfm";

/// Computer Modern math symbol metrics.
pub const CMSY10: &str = "texk/dvipsk/testdata/cmsy10.tfm";

/// Computer Modern math extension metrics.
pub const CMEX10: &str = "texk/dvipsk/testdata/cmex10.tfm";

/// DejaVu Sans, a TrueType face with quadratic outlines.
pub const DEJAVU_SANS: &str = "libs/gd/libgd-src/tests/freetype/DejaVuSans.ttf";

/// Latin Modern Mono 12, an OpenType face with CFF outlines.
pub const LM_MONO: &str = "texk/dvisvgm/dvisvgm-src/tests/data/lmmono12-regular.otf";

/// Every fixture, the paths `tools/fetch-test-fixtures.sh` checks out.
pub const FILES: &[&str] = &[CMR10, CMMI10, CMSY10, CMEX10, DEJAVU_SANS, LM_MONO];

const PIN: &str = include_str!("../../web2c-import/pin.toml");

/// The texlive-source commit `tools/web2c-import/pin.toml` pins.
#[must_use]
pub fn pinned_commit() -> &'static str {
    PIN.lines()
        .find_map(|line| line.strip_prefix("texlive_source = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("pin.toml records texlive_source")
}

fn required() -> bool {
    std::env::var(REQUIRE_ENV).as_deref() == Ok("1")
}

/// The default checkout, `cache/texlive-source` in the repository, where `tools/fetch-test-fixtures.sh` puts it.
fn default_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache/texlive-source")
}

/// The checkout, `Err` with the reason to skip when there is none, a panic when one is set up wrong.
fn locate() -> Result<PathBuf, String> {
    let fetch = "run tools/fetch-test-fixtures.sh";
    let dir = match std::env::var_os(SOURCE_ENV) {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            assert!(
                dir.is_dir(),
                "{SOURCE_ENV} is set but {} is not a directory, {fetch}",
                dir.display()
            );
            dir
        }
        None => {
            let dir = default_dir();
            if !dir.is_dir() {
                return Err(format!(
                    "no texlive-source checkout at {}, {fetch}",
                    dir.display()
                ));
            }
            dir
        }
    };
    if dir.join(".git").exists() {
        let output = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap_or_else(|error| {
                panic!("cannot check the commit of {}: {error}", dir.display())
            });
        let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert!(
            output.status.success() && head == pinned_commit(),
            "the texlive-source checkout at {} is at {head:?}, not the pinned {}, {fetch}",
            dir.display(),
            pinned_commit()
        );
    }
    Ok(dir)
}

/// The checkout, `None` after a notice on stderr when missing, a panic when required or misconfigured.
pub fn root() -> Option<&'static Path> {
    static ROOT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    match ROOT.get_or_init(locate) {
        Ok(dir) => Some(dir),
        Err(reason) => {
            assert!(!required(), "{REQUIRE_ENV}=1 but {reason}");
            eprintln!("skipping, needs texlive-source fixtures: {reason}");
            None
        }
    }
}

/// The bytes of a fixture, a path relative to the checkout such as [`CMR10`], `None` after a skip notice.
pub fn read(relative: &str) -> Option<Vec<u8>> {
    let path = root()?.join(relative);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{} is unreadable: {error}, the checkout lacks a fixture, run tools/fetch-test-fixtures.sh",
            path.display()
        )
    });
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_names_a_full_commit() {
        let commit = pinned_commit();
        assert_eq!(commit.len(), 40);
        assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn the_fetch_script_checks_out_every_fixture() {
        let script = include_str!("../../fetch-test-fixtures.sh");
        for file in FILES {
            assert!(
                script.lines().any(|line| line.trim() == *file),
                "tools/fetch-test-fixtures.sh does not fetch {file}"
            );
        }
    }
}

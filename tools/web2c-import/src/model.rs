//! Core data model for the Web2C/C2Rust patcher.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::transform::{
    add_self_arg, patch_c_types, patch_resource_search_names, patch_terminal_io_names,
    patch_terminal_writes, prefix_function_calls, prefix_globals, prefix_host_calls,
    scan_identifiers, template_text, PatchSession,
};

/// Bootstrap script that populates the web2c cache, named in input errors.
pub(crate) const BOOTSTRAP_SCRIPT: &str = "tools/bootstrap-texlive-web2c-c2rust.sh";

/// Errors raised while patching translated Web2C output.
#[derive(Debug)]
pub enum PatchError {
    /// The command line arguments were not understood.
    Usage,
    /// A filesystem operation on the named path failed.
    Io {
        /// Path the operation touched.
        path: PathBuf,
        /// Underlying error.
        error: io::Error,
    },
    /// Cache inputs the bootstrap script should have produced are missing.
    MissingInputs {
        /// Every missing input path.
        paths: Vec<PathBuf>,
    },
    /// The pin file could not be parsed.
    MalformedPin {
        /// Pin file path.
        path: PathBuf,
        /// One based line number of the bad line.
        line: usize,
    },
    /// Cache inputs do not match the pin manifest.
    PinMismatch {
        /// Pin file path.
        path: PathBuf,
        /// Cache relative input paths whose digest differs or is absent.
        files: Vec<String>,
    },
    /// A translated function block was malformed.
    MalformedFunction {
        /// Source path containing the malformed function.
        path: PathBuf,
        /// Line number where the malformed function starts.
        line: usize,
    },
    /// A required translated block was missing.
    MissingTranslatedBlock {
        /// Function whose translated block was missing.
        function: &'static str,
    },
    /// The existing generated crate manifest has no package version.
    MissingCrateVersion {
        /// Manifest path.
        path: PathBuf,
    },
    /// Textual patches or templates did not apply as declared.
    PatchMismatches(Vec<PatchMismatch>),
    /// Calls to `EngineFlow` returning functions are missing their `?`.
    UnwrappedFlowCalls(Vec<String>),
}

/// One textual patch or template that did not apply as declared.
#[derive(Debug)]
pub struct PatchMismatch {
    /// Function the patch targets.
    pub function: String,
    /// What went wrong, naming the anchor or template.
    pub detail: String,
}

impl fmt::Display for PatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(formatter, "usage: patch_engine [repo-root]"),
            Self::Io { path, error } => write!(formatter, "{}: {error}", path.display()),
            Self::MissingInputs { paths } => {
                writeln!(formatter, "missing web2c cache inputs:")?;
                for path in paths {
                    writeln!(formatter, "  {}", path.display())?;
                }
                write!(
                    formatter,
                    "run {BOOTSTRAP_SCRIPT} to populate the cache, or point MATHTEX_WEB2C_CACHE at one"
                )
            }
            Self::MalformedPin { path, line } => {
                write!(formatter, "{}:{line}: malformed pin entry", path.display())
            }
            Self::PinMismatch { path, files } => {
                writeln!(formatter, "cache inputs do not match {}:", path.display())?;
                for file in files {
                    writeln!(formatter, "  {file}")?;
                }
                write!(
                    formatter,
                    "rerun {BOOTSTRAP_SCRIPT}, or set MATHTEX_WEB2C_UPDATE_PIN=1 to accept the new inputs"
                )
            }
            Self::MalformedFunction { path, line } => {
                write!(
                    formatter,
                    "malformed function in {} at line {line}",
                    path.display()
                )
            }
            Self::MissingTranslatedBlock { function } => {
                write!(formatter, "missing translated block in {function}")
            }
            Self::MissingCrateVersion { path } => {
                write!(formatter, "{}: no package version", path.display())
            }
            Self::PatchMismatches(mismatches) => {
                write!(formatter, "{} patch mismatch(es):", mismatches.len())?;
                for mismatch in mismatches {
                    write!(formatter, "\n  {}: {}", mismatch.function, mismatch.detail)?;
                }
                Ok(())
            }
            Self::UnwrappedFlowCalls(calls) => {
                write!(formatter, "EngineFlow calls without `?`:")?;
                for call in calls {
                    write!(formatter, "\n  {call}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for PatchError {}

/// Reads a file, attaching its path to any error.
pub(crate) fn read_file(path: &Path) -> Result<String, PatchError> {
    std::fs::read_to_string(path).map_err(|error| io_error(path, error))
}

/// Writes a file, attaching its path to any error.
pub(crate) fn write_file(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), PatchError> {
    std::fs::write(path, contents).map_err(|error| io_error(path, error))
}

/// Wraps an I/O error with the path it concerns.
pub(crate) fn io_error(path: &Path, error: io::Error) -> PatchError {
    PatchError::Io {
        path: path.to_path_buf(),
        error,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RawFunction {
    pub(crate) name: String,
    pub(crate) source: String,
}

impl RawFunction {
    pub(crate) fn patch(
        &self,
        translated_functions: &BTreeSet<String>,
        globals: &BTreeMap<String, String>,
        session: &mut PatchSession,
    ) -> String {
        let local_bindings = self.local_bindings();
        let mut source = self.source.clone();

        // These cannot be AST passes because the input is not yet a valid parse unit.
        source = source.replace("#[no_mangle]\n", "");
        source = source.replace(
            &format!("pub unsafe extern \"C\" fn {}", self.name),
            &format!("pub(crate) unsafe fn {}", self.name),
        );
        source = add_self_arg(source, self.name.as_str());
        source = patch_c_types(source);

        // Lexical passes run before `self.` receivers and `self.state.` paths exist.
        source = patch_terminal_writes(source);
        source = prefix_function_calls(source, translated_functions);
        // Undo the `prefix_function_calls` rewrite on the signature only.
        source = source.replace(
            "pub(crate) unsafe fn (&mut *(self as *mut PortableTexEngine<'_>)).",
            "pub(crate) unsafe fn ",
        );
        source = prefix_host_calls(source);
        source = prefix_globals(source, globals.keys(), &local_bindings);

        // Resource and terminal renames stay textual for bare global and struct field types.
        source = patch_resource_search_names(source);
        source = patch_terminal_io_names(source);

        // Function patches run before parsing so literal anchors survive `prettyplease` line reflow.
        source = session.apply(self, source);

        // AST passes run in order on the live AST, then the method is emitted once.
        source = crate::astpass::run_all(
            &source,
            crate::boundary::RECEIVER_BOUNDARY_CALLS,
            crate::boundary::HOST_SERVICE_CALL_RENAMES,
            &self.vector_state_fields(globals),
        );
        // The AST pass converts `zeqtb` to `.as_mut_ptr()`, making the old pointer annotation wrong.
        crate::transform::patch_paged_eqtb_binding(source)
    }

    /// State fields with a `*mut <T>` vector type (not `*mut FILE`) referenced in this body.
    fn vector_state_fields(&self, globals: &BTreeMap<String, String>) -> BTreeSet<String> {
        let referenced = crate::transform::identifier_names(self.source.as_str());
        globals
            .iter()
            .filter(|(name, raw_type)| {
                crate::transform::is_vector_state_field(raw_type) && referenced.contains(*name)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Globals the emitted body reads, taken from the template when one replaces the function.
    pub(crate) fn referenced_globals(
        &self,
        globals: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        let source = template_text(&self.name).unwrap_or(&self.source);
        let mut referenced = BTreeSet::new();
        scan_identifiers(source, |identifier, _previous, _next| {
            if globals.contains_key(identifier) {
                referenced.insert(identifier.to_string());
            }
        });
        referenced
    }

    pub(crate) fn called_functions(
        &self,
        translated_functions: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut calls = BTreeSet::new();
        scan_identifiers(self.source.as_str(), |identifier, previous, next| {
            if next == Some('(')
                && previous != Some('.')
                && identifier != self.name
                && translated_functions.contains(identifier)
            {
                calls.insert(identifier.to_string());
            }
        });
        calls
    }

    fn local_bindings(&self) -> BTreeSet<String> {
        let mut bindings = BTreeSet::new();
        if let Some(signature_start) = self.source.find('(') {
            if let Some(signature_end) = self.source[signature_start + 1..].find(')') {
                let args = &self.source[signature_start + 1..signature_start + 1 + signature_end];
                for arg in args.split(',') {
                    let arg = arg.trim().strip_prefix("mut ").unwrap_or(arg.trim());
                    if let Some((name, _)) = arg.split_once(':') {
                        let name = name.trim();
                        if !name.is_empty() && name != "_" {
                            bindings.insert(name.to_string());
                        }
                    }
                }
            }
        }

        for line in self.source.lines() {
            let line = line.trim_start();
            let Some(rest) = line
                .strip_prefix("let mut ")
                .or_else(|| line.strip_prefix("let "))
            else {
                continue;
            };
            let end = rest
                .find(|character: char| {
                    character == ':' || character == '=' || character.is_whitespace()
                })
                .unwrap_or(rest.len());
            let name = &rest[..end];
            if !name.is_empty() && name != "_" {
                bindings.insert(name.to_string());
            }
        }

        bindings
    }
}

/// Provenance of an emitted function, recorded in the patch report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FunctionOrigin {
    Translated,
    BoundaryAdapterRequired,
    AdaptedNativeIo,
    StrippedSourceSpecial,
    StrippedWriteWhatsitDiagnostic,
    StrippedPdfExtension,
    Template,
}

impl FunctionOrigin {
    pub(crate) const ALL: [Self; 7] = [
        Self::Translated,
        Self::BoundaryAdapterRequired,
        Self::AdaptedNativeIo,
        Self::StrippedSourceSpecial,
        Self::StrippedWriteWhatsitDiagnostic,
        Self::StrippedPdfExtension,
        Self::Template,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Translated => "Translated",
            Self::BoundaryAdapterRequired => "BoundaryAdapterRequired",
            Self::AdaptedNativeIo => "AdaptedNativeIo",
            Self::StrippedSourceSpecial => "StrippedSourceSpecial",
            Self::StrippedWriteWhatsitDiagnostic => "StrippedWriteWhatsitDiagnostic",
            Self::StrippedPdfExtension => "StrippedPdfExtension",
            Self::Template => "Template",
        }
    }
}

/// One patch report row for an emitted function.
#[derive(Clone, Debug)]
pub(crate) struct FunctionReport {
    pub(crate) symbol: String,
    pub(crate) origin: FunctionOrigin,
    /// Hash of the upstream XeTeX source before any patching.
    pub(crate) upstream_hash: u64,
    pub(crate) external_boundaries: Vec<&'static str>,
}

/// A selected dependency that was dropped because the import policy forbids it.
#[derive(Clone, Debug)]
pub(crate) struct SkippedSymbol {
    pub(crate) symbol: String,
    pub(crate) reason: &'static str,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SelectedFunctions {
    pub(crate) functions: BTreeSet<String>,
    pub(crate) boundary_adapted_native_io: BTreeSet<String>,
    pub(crate) boundary_stripped_output: BTreeSet<String>,
    pub(crate) boundary_stripped_native_font_assembly: BTreeSet<String>,
    pub(crate) prelude_defined: BTreeSet<String>,
    pub(crate) skipped: Vec<SkippedSymbol>,
}

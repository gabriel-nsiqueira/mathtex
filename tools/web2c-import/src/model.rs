//! Core data model for the Web2C/C2Rust patcher.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::io;
use std::path::PathBuf;

use crate::TranslatedSymbolClass;

use crate::transform::{
    add_self_arg, patch_c_types, patch_generic_native_font_path, patch_initialization_function,
    patch_node_source_recording, patch_resource_search_names, patch_terminal_io_names,
    prefix_function_calls, prefix_globals, prefix_host_calls, repair_mangled_char_literals,
    scan_identifiers,
};

#[derive(Debug)]
/// Errors raised while patching translated Web2C output.
pub enum PatchError {
    /// A filesystem operation failed.
    Io(io::Error),
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
    /// Bodies differ in a way the classifier cannot place, fails to prevent emitting a mismatched engine.
    UnclassifiableBodyDiff {
        /// Symbol whose body diff could not be classified.
        symbol: String,
        /// Reason the body diff could not be classified.
        reason: String,
    },
}

impl From<io::Error> for PatchError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
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
            Self::UnclassifiableBodyDiff { symbol, reason } => {
                write!(
                    formatter,
                    "unclassifiable body diff for `{symbol}`: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for PatchError {}

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
        source = prefix_function_calls(source, translated_functions);
        // Undo the `prefix_function_calls` rewrite on the signature only.
        source = source.replace("pub(crate) unsafe fn self.", "pub(crate) unsafe fn ");
        source = source.replace(
            "pub(crate) unsafe fn (&mut *(self as *mut PortableTexEngine<'_>)).",
            "pub(crate) unsafe fn ",
        );
        source = prefix_host_calls(source);
        source = prefix_globals(source, globals.keys(), &local_bindings);
        // Repair char literals mangled by global prefixing in `initialize`.
        source = repair_mangled_char_literals(source);

        // Resource and terminal renames stay textual for bare global and struct field types.
        source = patch_resource_search_names(source);
        source = patch_terminal_io_names(source);

        // These run before parsing so literal anchors survive `prettyplease` line reflow.
        source = patch_generic_native_font_path(source);
        source = patch_initialization_function(source, self.name.as_str());
        source = patch_node_source_recording(source, self.name.as_str());

        // AST passes run in order on the live AST, then the method is emitted once.
        source = crate::astpass::run_all(
            &source,
            crate::boundary::RECEIVER_BOUNDARY_CALLS,
            crate::boundary::HOST_SERVICE_CALL_RENAMES,
            &self.vector_state_fields(globals),
        );
        // The AST pass converts `zeqtb` to `.as_mut_ptr()`, making the old pointer annotation wrong.
        source = crate::transform::patch_paged_eqtb_binding(source);
        source
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

    pub(crate) fn referenced_globals(&self, globals: &BTreeMap<String, String>) -> BTreeSet<String> {
        let mut referenced = BTreeSet::new();
        scan_identifiers(self.source.as_str(), |identifier, _previous, _next| {
            if globals.contains_key(identifier) {
                referenced.insert(identifier.to_string());
            }
        });
        referenced
    }

    pub(crate) fn called_functions(&self, translated_functions: &BTreeSet<String>) -> BTreeSet<String> {
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

#[derive(Clone, Debug)]
pub(crate) struct SkippedSymbol {
    pub(crate) engine: &'static str,
    pub(crate) symbol: String,
    pub(crate) class: TranslatedSymbolClass,
}

impl SkippedSymbol {
    pub(crate) fn new(engine: &'static str, symbol: String, class: TranslatedSymbolClass) -> Self {
        Self {
            engine,
            symbol,
            class,
        }
    }
}


#[derive(Clone, Debug, Default)]
pub(crate) struct SelectedFunctions {
    pub(crate) shared: BTreeSet<String>,
    pub(crate) xetex: BTreeSet<String>,
    pub(crate) boundary_adapted_native_io: BTreeSet<String>,
    pub(crate) boundary_stripped_output: BTreeSet<String>,
    pub(crate) boundary_stripped_native_font_assembly: BTreeSet<String>,
    pub(crate) skipped: Vec<SkippedSymbol>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SharedSourceOrigin {
    TexOnly,
    AdaptedNativeIo,
    StrippedSourceSpecial,
    StrippedWriteWhatsitDiagnostic,
    StrippedPdfExtension,
    EtexProfileGated,
    NativeFontProfileGated,
    XetexOnlyProfileGated,
    TexCompatibleDuplicate,
    TexAdaptedDuplicate,
    BoundaryAdapterRequired,
    XetexWidenedDuplicate,
}

impl SharedSourceOrigin {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TexOnly => "TexOnly",
            Self::AdaptedNativeIo => "AdaptedNativeIo",
            Self::StrippedSourceSpecial => "StrippedSourceSpecial",
            Self::StrippedWriteWhatsitDiagnostic => "StrippedWriteWhatsitDiagnostic",
            Self::StrippedPdfExtension => "StrippedPdfExtension",
            Self::EtexProfileGated => "EtexProfileGated",
            Self::NativeFontProfileGated => "NativeFontProfileGated",
            Self::XetexOnlyProfileGated => "XetexOnlyProfileGated",
            Self::TexCompatibleDuplicate => "TexCompatibleDuplicate",
            Self::TexAdaptedDuplicate => "TexAdaptedDuplicate",
            Self::BoundaryAdapterRequired => "BoundaryAdapterRequired",
            Self::XetexWidenedDuplicate => "XetexWidenedDuplicate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SharedDuplicatePolicy {
    XetexWhenDifferent,
    TexWhenAvailable,
}

impl SharedDuplicatePolicy {
    pub(crate) fn from_env() -> Self {
        match env::var("MATHTEX_WEB2C_SHARED_DUPLICATES").as_deref() {
            Ok("xetex") | Ok("xetex-when-different") => Self::XetexWhenDifferent,
            Ok("tex") | Ok("tex-when-available") => Self::TexWhenAvailable,
            _ => Self::XetexWhenDifferent,
        }
    }
}


#[derive(Clone, Debug)]
pub(crate) struct SharedSourceReport {
    pub(crate) symbol: String,
    pub(crate) origin: SharedSourceOrigin,
    pub(crate) base_hash: Option<u64>,
    pub(crate) xetex_hash: Option<u64>,
    pub(crate) external_boundaries: Vec<&'static str>,
}


#[derive(Clone, Debug)]
pub(crate) struct PatchedSharedFunction {
    pub(crate) source_code: String,
    pub(crate) referenced_globals: BTreeSet<String>,
    pub(crate) source: SharedSourceReport,
    pub(crate) comparison: crate::compare::ComparisonRecord,
}

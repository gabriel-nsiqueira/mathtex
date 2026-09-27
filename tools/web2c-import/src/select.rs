//! Selection of portable functions from the translated XeTeX sources.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{FunctionOrigin, RawFunction, SelectedFunctions, SkippedSymbol};
use crate::transform::{contains_identifier, is_template_function};

/// FNV-1a over the whitespace normalized source, stable across `prettyplease` reflow.
pub(crate) fn source_hash(source: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .bytes()
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// XeTeX call graph closure from the roots, stopping at boundaries, forbidden names and prelude functions.
pub(crate) fn select_portable_functions(
    functions: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    prelude_defined: &BTreeSet<String>,
    roots: impl IntoIterator<Item = String>,
) -> SelectedFunctions {
    let mut selected = SelectedFunctions::default();
    let mut skipped_seen = BTreeSet::new();
    let mut queue = roots.into_iter().collect::<Vec<_>>();

    while let Some(name) = queue.pop() {
        if selected.functions.contains(&name) {
            continue;
        }
        let Some(function) = functions.get(&name) else {
            continue;
        };
        if skip_boundary_or_forbidden(&name, &mut selected, &mut skipped_seen) {
            continue;
        }
        if prelude_defined.contains(&name) {
            selected.prelude_defined.insert(name);
            continue;
        }
        selected.functions.insert(name.clone());
        match patched_function_dependencies(&name) {
            Some(dependencies) => {
                queue.extend(
                    dependencies
                        .iter()
                        .map(|dependency| (*dependency).to_string()),
                );
            }
            None => queue.extend(function.called_functions(translated_functions)),
        }
    }

    selected
}

/// Names of every function the runtime prelude template defines.
pub(crate) fn prelude_defined_functions() -> BTreeSet<String> {
    const PRELUDE: &str = include_str!("../runtime/prelude.rs.in");
    const MARKER: &str = "fn ";
    let mut names = BTreeSet::new();
    for line in PRELUDE.lines() {
        let mut rest = line.trim_start();
        while let Some(after) = rest.find(MARKER) {
            let candidate = &rest[after + MARKER.len()..];
            let end = candidate
                .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .unwrap_or(candidate.len());
            let name = &candidate[..end];
            if candidate[end..].starts_with('(') && !name.is_empty() {
                names.insert(name.to_string());
            }
            rest = &candidate[end..];
        }
    }
    names
}

/// Reverse closure from the abort seeds plus `patched_callers`, every member returns `EngineFlow<T>`.
pub(crate) fn abort_reachable_closure(
    functions: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    patched_callers: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut callers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (caller, function) in functions {
        for callee in function.called_functions(translated_functions) {
            callers.entry(callee).or_default().insert(caller.clone());
        }
    }

    const ABORT_SEEDS: &[&str] = &["jumpout", "error", "zfatalerror", "zoverflow", "zconfusion"];
    let mut closure = BTreeSet::new();
    let mut queue = ABORT_SEEDS
        .iter()
        .map(|seed| (*seed).to_string())
        .filter(|seed| translated_functions.contains(seed))
        .chain(patched_callers.iter().cloned())
        .collect::<Vec<_>>();

    while let Some(name) = queue.pop() {
        if !closure.insert(name.clone()) {
            continue;
        }
        if let Some(direct_callers) = callers.get(&name) {
            for caller in direct_callers {
                if !closure.contains(caller) {
                    queue.push(caller.clone());
                }
            }
        }
    }

    closure
}

/// Dependencies of template functions, whose upstream call graph no longer applies.
fn patched_function_dependencies(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "appendsrcspecial" | "insertsrcspecial" | "zpdferror" | "zprintwritewhatsit" => Some(&[]),
        "initialize" => Some(&[]),
        "openorclosein" => Some(&[
            "scanfourbitint",
            "scanoptionalequals",
            "scanfilename",
            "zpackfilename",
        ]),
        "scanpdfexttoks" => Some(&["zscantoks"]),
        "startinput" => Some(&[
            "scanfilename",
            "zpackfilename",
            "beginfilereading",
            "endfilereading",
            "firmuptheline",
        ]),
        _ => None,
    }
}

fn skip_boundary_or_forbidden(
    name: &str,
    selected: &mut SelectedFunctions,
    skipped_seen: &mut BTreeSet<String>,
) -> bool {
    if is_adapted_native_io_function(name) {
        return false;
    }
    if matches!(name, "openlogfile" | "jumpout") {
        selected.boundary_adapted_native_io.insert(name.to_string());
        return true;
    }
    if matches!(
        name,
        "zshipout" | "buildpage" | "zprunepagetop" | "zoutwhat" | "zloadpicture"
    ) {
        selected.boundary_stripped_output.insert(name.to_string());
        return true;
    }
    if name == "zbuildopentypeassembly" {
        selected
            .boundary_stripped_native_font_assembly
            .insert(name.to_string());
        return true;
    }
    let Some(reason) = crate::forbidden_reason(name) else {
        return false;
    };
    if skipped_seen.insert(name.to_string()) {
        selected.skipped.push(SkippedSymbol {
            symbol: name.to_string(),
            reason,
        });
    }
    true
}

fn is_adapted_native_io_function(name: &str) -> bool {
    matches!(name, "openorclosein" | "startinput")
}

/// Report provenance of an emitted function.
pub(crate) fn function_origin(name: &str, external_boundaries: &[&str]) -> FunctionOrigin {
    match name {
        "appendsrcspecial" | "insertsrcspecial" => FunctionOrigin::StrippedSourceSpecial,
        "zprintwritewhatsit" => FunctionOrigin::StrippedWriteWhatsitDiagnostic,
        "scanpdfexttoks" | "zpdferror" => FunctionOrigin::StrippedPdfExtension,
        _ if is_adapted_native_io_function(name) => FunctionOrigin::AdaptedNativeIo,
        _ if is_template_function(name) => FunctionOrigin::Template,
        _ if !external_boundaries.is_empty() => FunctionOrigin::BoundaryAdapterRequired,
        _ => FunctionOrigin::Translated,
    }
}

/// Host boundary identifiers still present in a patched body, listed in the patch report.
pub(crate) fn unsupported_external_boundaries(source: &str) -> Vec<&'static str> {
    UNSUPPORTED_EXTERNAL_BOUNDARY_IDENTIFIERS
        .iter()
        .copied()
        .filter(|identifier| contains_identifier(source, identifier))
        .collect()
}

const UNSUPPORTED_EXTERNAL_BOUNDARY_IDENTIFIERS: &[&str] = &[
    "CFDictionaryRef",
    "GlyphAssembly",
    "UFILE",
    "aatfontget",
    "aatfontget1",
    "aatfontget2",
    "aatfontgetnamed",
    "aatfontgetnamed1",
    "aatprintfontname",
    "applymapping",
    "applytfmfontmapping",
    "checkfortfmfontmapping",
    "countpdffilepages",
    "free",
    "free_ot_assembly",
    "fputs",
    "getcreationdate",
    "getfiledump",
    "getfilemoddate",
    "getmd5sum",
    "grfontgetnamed",
    "grfontgetnamed1",
    "grprintfontname",
    "initstarttime",
    "loadtfmfontmapping",
    "printglyphname",
    "setinputfileencoding",
    "u_close_file_or_pipe",
    "usingGraphite",
    "zbuildopentypeassembly",
    "znotaatfonterror",
    "znotaatgrfonterror",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn map(functions: &[(&str, &str)]) -> BTreeMap<String, RawFunction> {
        functions
            .iter()
            .map(|(name, body)| {
                let raw = RawFunction {
                    name: (*name).to_string(),
                    source: format!("pub unsafe extern \"C\" fn {name}() {{\n{body}\n}}\n"),
                };
                ((*name).to_string(), raw)
            })
            .collect()
    }

    #[test]
    fn selection_follows_calls_and_stops_at_boundaries() {
        let functions = map(&[
            ("maincontrol", "helper(); zshipout(); fopen();"),
            ("helper", "leaf();"),
            ("leaf", ""),
            ("zshipout", "never();"),
            ("fopen", ""),
            ("unreached", ""),
            ("never", ""),
        ]);
        let translated = functions.keys().cloned().collect::<BTreeSet<_>>();
        let prelude = BTreeSet::from(["leaf".to_string()]);
        let selected = select_portable_functions(
            &functions,
            &translated,
            &prelude,
            ["maincontrol".to_string()],
        );

        assert_eq!(
            selected
                .functions
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["helper", "maincontrol"]
        );
        assert!(selected.prelude_defined.contains("leaf"));
        assert!(selected.boundary_stripped_output.contains("zshipout"));
        assert_eq!(selected.skipped.len(), 1);
        assert_eq!(selected.skipped[0].symbol, "fopen");
        assert_eq!(selected.skipped[0].reason, "NativeIo");
    }

    #[test]
    fn abort_closure_collects_transitive_callers() {
        let functions = map(&[
            ("error", ""),
            ("caller", "error();"),
            ("outer", "caller();"),
            ("pure", "leaf();"),
            ("leaf", ""),
        ]);
        let translated = functions.keys().cloned().collect::<BTreeSet<_>>();
        let closure = abort_reachable_closure(&functions, &translated, &BTreeSet::new());
        assert_eq!(
            closure.iter().map(String::as_str).collect::<Vec<_>>(),
            ["caller", "error", "outer"]
        );
        let patched = BTreeSet::from(["leaf".to_string()]);
        let closure = abort_reachable_closure(&functions, &translated, &patched);
        assert!(closure.contains("leaf") && closure.contains("pure"));
    }

    #[test]
    fn source_hash_ignores_whitespace_layout() {
        assert_eq!(
            source_hash("fn a() {\n    b();\n}"),
            source_hash("fn a() { b(); }")
        );
        assert_ne!(
            source_hash("fn a() { b(); }"),
            source_hash("fn a() { c(); }")
        );
    }
}

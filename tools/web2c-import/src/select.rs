//! Selection of portable functions from translated TeX/eTeX/XeTeX sources.

use std::collections::{BTreeMap, BTreeSet};

use crate::{TranslatedSymbolClass, Web2cSourceManifest};

use crate::compare::{
    dual_profile_record, single_profile_record, ComparisonRecord, SelectedStrategy,
};
use crate::model::{
    PatchError, PatchedSharedFunction, RawFunction, SelectedFunctions, SharedDuplicatePolicy,
    SharedSourceOrigin, SharedSourceReport, SkippedSymbol,
};
use crate::transform::{
    contains_identifier, guard_xetex_patch_function, patched_shared_origin, profile_gated_origin,
};

/// Fails with [`PatchError::UnclassifiableBodyDiff`] when the base/XeTeX body comparison is unclassifiable.
pub(crate) fn patch_shared_function(
    name: &str,
    tex_map: &BTreeMap<String, RawFunction>,
    etex_map: &BTreeMap<String, RawFunction>,
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    globals: &BTreeMap<String, String>,
    shared_duplicate_policy: SharedDuplicatePolicy,
    reachable_from_tex_core: &BTreeSet<String>,
) -> Result<Option<PatchedSharedFunction>, PatchError> {
    let base = etex_map.get(name).or_else(|| tex_map.get(name));
    let base_profile = if etex_map.contains_key(name) {
        "etex"
    } else {
        "tex"
    };
    match (base, xetex_map.get(name)) {
        (Some(base), Some(xetex)) => {
            let base_source = base.patch(translated_functions, globals);
            let xetex_source = xetex.patch(translated_functions, globals);
            let base_hash = Some(source_hash(base_source.as_str()));
            let xetex_hash = Some(source_hash(xetex_source.as_str()));
            let external_boundaries = unsupported_external_boundaries(xetex_source.as_str());

            // Compute strategy before record to ensure the selected body and comparison record agree.
            let strategy = dual_profile_strategy(
                base_source.as_str(),
                xetex_source.as_str(),
                &external_boundaries,
                shared_duplicate_policy,
            );
            let comparison = dual_profile_record(
                name,
                base_profile,
                base_source.as_str(),
                xetex_source.as_str(),
                strategy,
            );
            fail_on_unknown(&comparison)?;

            let (source_code, referenced_globals, origin) = match strategy {
                SelectedStrategy::UseBase if external_boundaries.is_empty() && base_source == xetex_source => {
                    (
                        base_source,
                        base.referenced_globals(globals),
                        patched_shared_origin(name, SharedSourceOrigin::TexCompatibleDuplicate),
                    )
                }
                SelectedStrategy::UseBase => (
                    base_source,
                    base.referenced_globals(globals),
                    patched_shared_origin(name, SharedSourceOrigin::TexAdaptedDuplicate),
                ),
                SelectedStrategy::UseXetex if !external_boundaries.is_empty() => (
                    xetex_source,
                    xetex.referenced_globals(globals),
                    SharedSourceOrigin::BoundaryAdapterRequired,
                ),
                SelectedStrategy::UseXetex | SelectedStrategy::SingleProfile => (
                    xetex_source,
                    xetex.referenced_globals(globals),
                    SharedSourceOrigin::XetexWidenedDuplicate,
                ),
            };
            Ok(Some(PatchedSharedFunction {
                source_code,
                referenced_globals,
                source: SharedSourceReport {
                    symbol: name.to_string(),
                    origin,
                    base_hash,
                    xetex_hash,
                    external_boundaries,
                },
                comparison,
            }))
        }
        (Some(base), None) => {
            let body = base.patch(translated_functions, globals);
            let comparison = single_profile_record(name, base_profile, body.as_str(), Vec::new());
            Ok(Some(PatchedSharedFunction {
                source_code: body,
                referenced_globals: base.referenced_globals(globals),
                source: SharedSourceReport {
                    symbol: name.to_string(),
                    origin: SharedSourceOrigin::EtexProfileGated,
                    base_hash: Some(source_hash(base.source.as_str())),
                    xetex_hash: None,
                    external_boundaries: Vec::new(),
                },
                comparison,
            }))
        }
        (None, Some(xetex)) => {
            // Emitted unguarded, the ~56 in-body is_xetex()/supports_* branches are the actual profile gate.
            let body =
                guard_xetex_patch_function(name, xetex.patch(translated_functions, globals));
            let comparison = single_profile_record(name, "xetex", body.as_str(), Vec::new());
            Ok(Some(PatchedSharedFunction {
                source_code: body,
                referenced_globals: xetex.referenced_globals(globals),
                source: SharedSourceReport {
                    symbol: name.to_string(),
                    origin: profile_gated_origin(name, reachable_from_tex_core),
                    base_hash: None,
                    xetex_hash: Some(source_hash(xetex.source.as_str())),
                    external_boundaries: Vec::new(),
                },
                comparison,
            }))
        }
        (None, None) => Ok(None),
    }
}

/// Encodes the historical selection decision tree for symbols present in both the base and XeTeX profiles.
fn dual_profile_strategy(
    base_source: &str,
    xetex_source: &str,
    external_boundaries: &[&'static str],
    shared_duplicate_policy: SharedDuplicatePolicy,
) -> SelectedStrategy {
    if base_source == xetex_source {
        SelectedStrategy::UseBase
    } else if !external_boundaries.is_empty() {
        SelectedStrategy::UseXetex
    } else if shared_duplicate_policy == SharedDuplicatePolicy::TexWhenAvailable {
        SelectedStrategy::UseBase
    } else {
        SelectedStrategy::UseXetex
    }
}

fn fail_on_unknown(record: &ComparisonRecord) -> Result<(), PatchError> {
    if record.diff.is_unknown() {
        return Err(PatchError::UnclassifiableBodyDiff {
            symbol: record.symbol.clone(),
            reason: record
                .reason_if_not_importable
                .clone()
                .unwrap_or_else(|| "unclassifiable base/xetex body difference".to_string()),
        });
    }
    Ok(())
}


pub(crate) fn source_hash(source: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in normalized_source_for_hash(source).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}


pub(crate) fn normalized_source_for_hash(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}


pub(crate) fn select_portable_functions(
    manifest: &Web2cSourceManifest,
    tex_map: &BTreeMap<String, RawFunction>,
    etex_map: &BTreeMap<String, RawFunction>,
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    shared_roots: BTreeSet<String>,
    xetex_roots: BTreeSet<String>,
) -> SelectedFunctions {
    let mut selected = SelectedFunctions::default();
    let mut shared_queue = shared_roots.into_iter().collect::<Vec<_>>();
    let mut xetex_queue = xetex_roots.into_iter().collect::<Vec<_>>();
    let mut skipped_seen = BTreeSet::new();

    while let Some(name) = shared_queue.pop() {
        if selected.shared.contains(&name) {
            continue;
        }

        let Some(_) = shared_engine_function(name.as_str(), tex_map, etex_map, xetex_map) else {
            continue;
        };
        if skip_forbidden_dependency(
            manifest,
            "tex",
            name.as_str(),
            &mut selected,
            &mut skipped_seen,
        ) {
            continue;
        }

        selected.shared.insert(name.clone());
        for dependency in shared_candidate_dependencies(
            name.as_str(),
            tex_map,
            etex_map,
            xetex_map,
            translated_functions,
        ) {
            if tex_map.contains_key(dependency.as_str())
                || etex_map.contains_key(dependency.as_str())
                || xetex_map.contains_key(dependency.as_str())
            {
                shared_queue.push(dependency);
            }
        }
    }

    while let Some(name) = xetex_queue.pop() {
        if selected.xetex.contains(&name) || selected.shared.contains(&name) {
            continue;
        }

        if tex_map.contains_key(name.as_str()) {
            shared_queue.push(name);
            while let Some(shared_name) = shared_queue.pop() {
                if selected.shared.contains(&shared_name) {
                    continue;
                }
                let Some(_) =
                    shared_engine_function(shared_name.as_str(), tex_map, etex_map, xetex_map)
                else {
                    continue;
                };
                if skip_forbidden_dependency(
                    manifest,
                    "tex",
                    shared_name.as_str(),
                    &mut selected,
                    &mut skipped_seen,
                ) {
                    continue;
                }
                selected.shared.insert(shared_name.clone());
                for dependency in shared_candidate_dependencies(
                    shared_name.as_str(),
                    tex_map,
                    etex_map,
                    xetex_map,
                    translated_functions,
                ) {
                    if tex_map.contains_key(dependency.as_str())
                        || etex_map.contains_key(dependency.as_str())
                        || xetex_map.contains_key(dependency.as_str())
                    {
                        shared_queue.push(dependency);
                    }
                }
            }
            continue;
        }

        let Some(function) = xetex_map.get(&name) else {
            continue;
        };
        if skip_forbidden_dependency(
            manifest,
            "xetex",
            name.as_str(),
            &mut selected,
            &mut skipped_seen,
        ) {
            continue;
        }

        selected.xetex.insert(name.clone());
        for dependency in function.called_functions(translated_functions) {
            if selected.shared.contains(dependency.as_str()) {
                continue;
            }
            if tex_map.contains_key(dependency.as_str())
                || etex_map.contains_key(dependency.as_str())
                || xetex_map.contains_key(dependency.as_str())
            {
                shared_queue.push(dependency);
            } else if xetex_map.contains_key(dependency.as_str()) {
                xetex_queue.push(dependency);
            }
        }

        while let Some(shared_name) = shared_queue.pop() {
            if selected.shared.contains(&shared_name) {
                continue;
            }
            let Some(_) =
                shared_engine_function(shared_name.as_str(), tex_map, etex_map, xetex_map)
            else {
                continue;
            };
            if skip_forbidden_dependency(
                manifest,
                "tex",
                shared_name.as_str(),
                &mut selected,
                &mut skipped_seen,
            ) {
                continue;
            }
            selected.shared.insert(shared_name.clone());
            for dependency in shared_candidate_dependencies(
                shared_name.as_str(),
                tex_map,
                etex_map,
                xetex_map,
                translated_functions,
            ) {
                if tex_map.contains_key(dependency.as_str())
                    || etex_map.contains_key(dependency.as_str())
                    || xetex_map.contains_key(dependency.as_str())
                {
                    shared_queue.push(dependency);
                }
            }
        }
    }

    selected
}


/// BFS from tex and etex symbols, call edges are unioned across all three profiles to compute reachability.
pub(crate) fn reachable_from_tex_core(
    tex_map: &BTreeMap<String, RawFunction>,
    etex_map: &BTreeMap<String, RawFunction>,
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut queue = tex_map
        .keys()
        .chain(etex_map.keys())
        .cloned()
        .collect::<Vec<_>>();

    while let Some(name) = queue.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        for map in [tex_map, etex_map, xetex_map] {
            let Some(function) = map.get(name.as_str()) else {
                continue;
            };
            for callee in function.called_functions(translated_functions) {
                if !reachable.contains(callee.as_str()) {
                    queue.push(callee);
                }
            }
        }
    }

    reachable
}


/// Reverse BFS from {jumpout, error, zfatalerror, zoverflow, zconfusion} to find all transitive callers.
pub(crate) fn abort_reachable_closure(
    tex_map: &BTreeMap<String, RawFunction>,
    etex_map: &BTreeMap<String, RawFunction>,
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
) -> BTreeSet<String> {
    // Call graph unions all three profiles, a body only in XeTeX can still be called from a base/etex body.
    let mut callers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for map in [tex_map, etex_map, xetex_map] {
        for (caller, function) in map {
            for callee in function.called_functions(translated_functions) {
                callers
                    .entry(callee)
                    .or_default()
                    .insert(caller.clone());
            }
        }
    }

    const ABORT_SEEDS: &[&str] = &["jumpout", "error", "zfatalerror", "zoverflow", "zconfusion"];
    let mut closure = BTreeSet::new();
    let mut queue = ABORT_SEEDS
        .iter()
        .map(|seed| (*seed).to_string())
        .filter(|seed| translated_functions.contains(seed))
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


pub(crate) fn shared_engine_function<'a>(
    name: &str,
    tex_map: &'a BTreeMap<String, RawFunction>,
    etex_map: &'a BTreeMap<String, RawFunction>,
    xetex_map: &'a BTreeMap<String, RawFunction>,
) -> Option<&'a RawFunction> {
    etex_map
        .get(name)
        .or_else(|| xetex_map.get(name))
        .or_else(|| tex_map.get(name))
}


pub(crate) fn shared_candidate_dependencies(
    name: &str,
    tex_map: &BTreeMap<String, RawFunction>,
    etex_map: &BTreeMap<String, RawFunction>,
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
) -> BTreeSet<String> {
    if let Some(dependencies) = patched_function_dependencies(name) {
        return dependencies
            .iter()
            .map(|dependency| (*dependency).to_string())
            .collect();
    }

    let mut dependencies = BTreeSet::new();
    if let Some(function) = tex_map.get(name) {
        dependencies.extend(function.called_functions(translated_functions));
    }
    if let Some(function) = etex_map.get(name) {
        dependencies.extend(function.called_functions(translated_functions));
    }
    if let Some(function) = xetex_map.get(name) {
        dependencies.extend(function.called_functions(translated_functions));
    }
    dependencies
}


pub(crate) fn patched_function_dependencies(name: &str) -> Option<&'static [&'static str]> {
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


pub(crate) fn skip_forbidden_dependency(
    manifest: &Web2cSourceManifest,
    engine: &'static str,
    name: &str,
    selected: &mut SelectedFunctions,
    skipped_seen: &mut BTreeSet<(String, &'static str)>,
) -> bool {
    if is_adapted_native_io_function(name) {
        return false;
    }
    if is_boundary_adapted_native_io_function(name) {
        selected.boundary_adapted_native_io.insert(name.to_string());
        return true;
    }
    if is_boundary_stripped_output_function(name) {
        selected.boundary_stripped_output.insert(name.to_string());
        return true;
    }
    if is_boundary_stripped_native_font_assembly_function(name) {
        selected
            .boundary_stripped_native_font_assembly
            .insert(name.to_string());
        return true;
    }
    let class = manifest.classify_symbol(name);
    if !matches!(class, TranslatedSymbolClass::Forbidden(_)) {
        return false;
    }

    let key = (name.to_string(), engine);
    if skipped_seen.insert(key) {
        selected
            .skipped
            .push(SkippedSymbol::new(engine, name.to_string(), class));
    }
    true
}


pub(crate) fn is_adapted_native_io_function(name: &str) -> bool {
    matches!(name, "openorclosein" | "startinput")
}


pub(crate) fn is_boundary_adapted_native_io_function(name: &str) -> bool {
    matches!(name, "openlogfile" | "jumpout")
}


pub(crate) fn is_boundary_stripped_output_function(name: &str) -> bool {
    matches!(
        name,
        "zshipout" | "buildpage" | "zprunepagetop" | "zoutwhat" | "zloadpicture"
    )
}


pub(crate) fn is_boundary_stripped_native_font_assembly_function(name: &str) -> bool {
    matches!(name, "zbuildopentypeassembly")
}


pub(crate) fn unsupported_external_boundaries(source: &str) -> Vec<&'static str> {
    UNSUPPORTED_EXTERNAL_BOUNDARY_IDENTIFIERS
        .iter()
        .copied()
        .filter(|identifier| contains_identifier(source, identifier))
        .collect()
}


pub(crate) const UNSUPPORTED_EXTERNAL_BOUNDARY_IDENTIFIERS: &[&str] = &[
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
    "get_native_glyph_italic_correction",
    "get_native_italic_correction",
    "getcreationdate",
    "getfiledump",
    "getfilemoddate",
    "getfilesize",
    "getfontcharrange",
    "getglyphbounds",
    "getmd5sum",
    "getnativecharheightdepth",
    "getnativecharht",
    "getnativecharic",
    "getnativecharsidebearings",
    "getnativecharwd",
    "grfontgetnamed",
    "grfontgetnamed1",
    "grprintfontname",
    "initstarttime",
    "loadtfmfontmapping",
    "otfontget",
    "otfontget1",
    "otfontget2",
    "otfontget3",
    "printglyphname",
    "set_cp_code",
    "setinputfileencoding",
    "u_close_file_or_pipe",
    "usingGraphite",
    "xmalloc",
    "xrealloc",
    "zbuildopentypeassembly",
    "znotaatfonterror",
    "znotaatgrfonterror",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PatchError;

    fn raw(name: &str, body: &str) -> RawFunction {
        RawFunction {
            name: name.to_string(),
            source: format!(
                "#[no_mangle]\npub unsafe extern \"C\" fn {name}() {{\n{body}\n}}\n"
            ),
        }
    }

    fn map_of(function: RawFunction) -> BTreeMap<String, RawFunction> {
        let mut map = BTreeMap::new();
        map.insert(function.name.clone(), function);
        map
    }

    #[test]
    fn genuinely_divergent_pair_fails_loud() {
        // Synthetic bodies with no structural similarity force the classifier to return Unknown.
        let base_body: String = (0..40).map(|i| format!("    let a{i} = alpha{i};")).collect();
        let xetex_body: String = (0..40).map(|i| format!("    let o{i} = omega{i};")).collect();
        let tex_map = map_of(raw("synthetic_divergent", &base_body));
        let etex_map = BTreeMap::new();
        let xetex_map = map_of(raw("synthetic_divergent", &xetex_body));
        let translated = BTreeSet::from(["synthetic_divergent".to_string()]);
        let globals = BTreeMap::new();

        let reachable =
            reachable_from_tex_core(&tex_map, &etex_map, &xetex_map, &translated);
        let result = patch_shared_function(
            "synthetic_divergent",
            &tex_map,
            &etex_map,
            &xetex_map,
            &translated,
            &globals,
            SharedDuplicatePolicy::XetexWhenDifferent,
            &reachable,
        );

        match result {
            Err(PatchError::UnclassifiableBodyDiff { symbol, .. }) => {
                assert_eq!(symbol, "synthetic_divergent");
            }
            other => panic!("expected UnclassifiableBodyDiff, got {other:?}"),
        }
    }

    #[test]
    fn identical_pair_imports_using_base() {
        let body: String = (0..10).map(|i| format!("    let a{i} = call{i}();")).collect();
        let tex_map = map_of(raw("synthetic_identical", &body));
        let etex_map = BTreeMap::new();
        let xetex_map = map_of(raw("synthetic_identical", &body));
        let translated = BTreeSet::from(["synthetic_identical".to_string()]);
        let globals = BTreeMap::new();

        let reachable =
            reachable_from_tex_core(&tex_map, &etex_map, &xetex_map, &translated);
        let patched = patch_shared_function(
            "synthetic_identical",
            &tex_map,
            &etex_map,
            &xetex_map,
            &translated,
            &globals,
            SharedDuplicatePolicy::XetexWhenDifferent,
            &reachable,
        )
        .expect("identical pair should import")
        .expect("symbol is defined");

        assert_eq!(
            patched.comparison.diff,
            crate::compare::BodyDiff::IdenticalAfterNormalization
        );
        assert_eq!(
            patched.comparison.selected_strategy,
            SelectedStrategy::UseBase
        );
    }

    #[test]
    fn reachability_detects_xetex_only_helpers_the_tex_path_calls() {
        // idlookup is in tex_map only, length is in xetex_map only, simulating a tex.web WEB macro.
        let tex_map = map_of(raw("idlookup", "    let n = length(cs);"));
        let etex_map = BTreeMap::new();
        let xetex_map = {
            let mut map = map_of(raw("length", "    return 0;"));
            // Helper present only in the XeTeX translation, the TeX path never calls it.
            map.extend(map_of(raw("xetex_only_native", "    return 0;")));
            map
        };
        let translated = BTreeSet::from([
            "idlookup".to_string(),
            "length".to_string(),
            "xetex_only_native".to_string(),
        ]);

        let reachable =
            reachable_from_tex_core(&tex_map, &etex_map, &xetex_map, &translated);

        assert!(reachable.contains("idlookup"));
        assert!(reachable.contains("length"));
        assert!(!reachable.contains("xetex_only_native"));

        // TeX-reachable symbols are widened duplicates, others are profile-gated.
        assert_eq!(
            crate::transform::profile_gated_origin("length", &reachable),
            SharedSourceOrigin::XetexWidenedDuplicate,
        );
        assert_eq!(
            crate::transform::profile_gated_origin("xetex_only_native", &reachable),
            SharedSourceOrigin::XetexOnlyProfileGated,
        );
    }

    #[test]
    fn guard_xetex_patch_function_no_longer_wraps_bodies() {
        // Reachability is the safety guarantee, in-body is_xetex() branches are the actual gating.
        let body = "pub(crate) unsafe fn zexample(&mut self) -> integer {\n    return 1;\n}\n";
        let guarded = guard_xetex_patch_function("zexample", body.to_string());
        assert_eq!(guarded, body);
        assert!(!guarded.contains("is_xetex"));
        assert!(!guarded.contains("return core::mem::zeroed()"));
    }
}

//! Orchestration entry point for the Web2C/C2Rust patcher.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::codegen::{
    cargo_source, dispatch_source, functions_split, import_report_markdown, lib_source,
    profiles_source, report_source, runtime_source, translated_etex_startup_primitives,
    translated_xetex_startup_primitives,
};
use crate::boundary::PORTABLE_BOUNDARY_CALLS;
use crate::extract::{extract_functions_from_paths, extract_globals};
use crate::flow::{build_trigger_set, rewrite_fallible_function};
use crate::model::{PatchError, SharedDuplicatePolicy};
use crate::profile::EngineImportProfile;
use crate::select::{
    abort_reachable_closure, patch_shared_function, reachable_from_tex_core,
    select_portable_functions,
};
use crate::transform::guard_xetex_patch_function;
use crate::{TranslatedSymbolClass, Web2cSourceManifest};

/// Runs the Web2C import and emits the portable engine sources.
pub fn run() -> Result<(), PatchError> {
    let repo_root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let input_root = repo_root.join("generated/c2rust");
    let output_root = repo_root.join("generated/portable-engine/src");

    let manifest = Web2cSourceManifest::new();
    let tex_items = extract_functions_from_paths(&[
        input_root.join("tex/src/tex0.rs"),
        input_root.join("tex/src/texini.rs"),
    ])?;
    let etex_items = extract_functions_from_paths(&[
        input_root.join("etex/src/etex0.rs"),
        input_root.join("etex/src/etexini.rs"),
    ])?;
    let xetex_items = extract_functions_from_paths(&[
        input_root.join("xetex/src/xetex0.rs"),
        input_root.join("xetex/src/xetexini.rs"),
    ])?;
    let globals = extract_globals(&[
        input_root.join("tex/src/tex0.rs"),
        input_root.join("tex/src/texini.rs"),
        input_root.join("etex/src/etex0.rs"),
        input_root.join("etex/src/etexini.rs"),
        input_root.join("xetex/src/xetex0.rs"),
        input_root.join("xetex/src/xetexini.rs"),
    ])?;
    let tex_map = tex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let etex_map = etex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let xetex_map = xetex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let translated_functions = tex_items
        .iter()
        .chain(etex_items.iter())
        .chain(xetex_items.iter())
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let shared_duplicate_policy = SharedDuplicatePolicy::from_env();
    // XeTeX provenance functions reachable from shared TeX are widened core functions.
    let reachable = reachable_from_tex_core(
        &tex_map,
        &etex_map,
        &xetex_map,
        &translated_functions,
    );
    // Abort reachable functions return `EngineFlow<T>` with fallible calls wrapped in `?`.
    let mut fallible = abort_reachable_closure(
        &tex_map,
        &etex_map,
        &xetex_map,
        &translated_functions,
    );
    // The `znotaatfonterror` stubs are infallible, so their call sites are not wrapped in `?`.
    fallible.retain(|name| !is_removed_aat_diagnostic_function(name));
    // Wrap closure members, `abort_engine`, and prelude bridge functions that propagate aborts.
    let mut flow_triggers = build_trigger_set(&fallible, PORTABLE_BOUNDARY_CALLS);
    flow_triggers.register_prelude_bridges();
    // Prelude functions are skipped because duplicate `impl` methods fail to compile.
    let prelude_defined = prelude_defined_functions();
    let etex_startup_primitives =
        translated_etex_startup_primitives(&etex_map, &translated_functions, &globals)?;
    let xetex_startup_primitives =
        translated_xetex_startup_primitives(&xetex_map, &translated_functions, &globals)?;
    // Startup primitive methods call `zprimitive`, so this rewrite adds `EngineFlow<()>`.
    let etex_startup_primitives = rewrite_fallible_function(&etex_startup_primitives, &flow_triggers);
    let xetex_startup_primitives =
        rewrite_fallible_function(&xetex_startup_primitives, &flow_triggers);
    let shared_roots = tex_items
        .iter()
        .filter(|item| {
            manifest.classify_symbol(item.name.as_str())
                == TranslatedSymbolClass::SharedTexCoreCandidate
        })
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let xetex_roots = xetex_items
        .iter()
        .filter(|item| {
            manifest.classify_symbol(item.name.as_str())
                == TranslatedSymbolClass::XetexPatchCandidate
        })
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let selected = select_portable_functions(
        &manifest,
        &tex_map,
        &etex_map,
        &xetex_map,
        &translated_functions,
        shared_roots,
        xetex_roots,
    );

    let mut shared = Vec::new();
    let mut xetex = Vec::new();
    let mut skipped = Vec::new();
    let mut referenced_globals = BTreeSet::new();
    let mut shared_sources = Vec::new();
    let mut comparison_records = Vec::new();
    let boundary_adapted_native_io = selected
        .boundary_adapted_native_io
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let boundary_stripped_output = selected
        .boundary_stripped_output
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let boundary_stripped_native_font_assembly = selected
        .boundary_stripped_native_font_assembly
        .iter()
        .cloned()
        .collect::<Vec<_>>();

    for name in &selected.shared {
        if is_removed_aat_diagnostic_function(name) {
            continue;
        }
        let patched = patch_shared_function(
            name,
            &tex_map,
            &etex_map,
            &xetex_map,
            &translated_functions,
            &globals,
            shared_duplicate_policy,
            &reachable,
        )?
        .expect("selected shared function should exist in translated output");
        referenced_globals.extend(patched.referenced_globals);
        shared_sources.push(patched.source);
        comparison_records.push(patched.comparison);
        // Hashes are computed from the body before this rewrite.
        let source_code = if fallible.contains(name) {
            rewrite_fallible_function(&patched.source_code, &flow_triggers)
        } else {
            patched.source_code
        };
        shared.push(source_code);
    }

    for name in &selected.xetex {
        if selected.shared.contains(name) {
            continue;
        }
        if prelude_defined.contains(name.as_str()) {
            // This is handwritten in the runtime prelude.
            continue;
        }
        if is_removed_aat_diagnostic_function(name) {
            continue;
        }
        let item = xetex_map
            .get(name)
            .expect("selected xetex patch function should exist in XeTeX output");
        referenced_globals.extend(item.referenced_globals(&globals));
        let guarded = guard_xetex_patch_function(
            name,
            item.patch(&translated_functions, &globals),
        );
        let guarded = if fallible.contains(name) {
            rewrite_fallible_function(&guarded, &flow_triggers)
        } else {
            guarded
        };
        xetex.push(guarded);
    }
    for skipped_symbol in selected.skipped {
        skipped.push(skipped_symbol);
    }

    fs::create_dir_all(&output_root)?;
    fs::write(
        repo_root.join("generated/portable-engine/Cargo.toml"),
        cargo_source(),
    )?;
    fs::write(
        output_root.join("runtime.rs"),
        runtime_source(
            &globals,
            &referenced_globals,
            &etex_startup_primitives,
            &xetex_startup_primitives,
            &flow_triggers,
        ),
    )?;
    // Split into partial `impl` blocks under `functions/`.
    let functions_dir = output_root.join("functions");
    fs::create_dir_all(&functions_dir)?;
    let split = functions_split(
        "shared TeX core candidates auto-patched from TeX source when compatible",
        &shared,
    );
    let kept_function_files = split
        .files
        .iter()
        .map(|file| file.file_name.clone())
        .chain(std::iter::once("mod.rs".to_string()))
        .collect::<BTreeSet<_>>();
    for file in &split.files {
        fs::write(functions_dir.join(&file.file_name), &file.contents)?;
    }
    fs::write(functions_dir.join("mod.rs"), &split.mod_rs)?;
    remove_stale_functions_files(&functions_dir, &kept_function_files)?;

    // XeTeX patched functions.
    fs::write(output_root.join("dispatch.rs"), dispatch_source(&xetex))?;

    let import_profiles = EngineImportProfile::all_from_recipe();
    fs::write(
        output_root.join("profiles.rs"),
        profiles_source(&import_profiles),
    )?;

    remove_stale_generated_file(output_root.join("shared_tex_core.rs"))?;
    remove_stale_generated_file(output_root.join("xetex_patches.rs"))?;
    remove_stale_generated_file(output_root.join("mod.rs"))?;
    remove_stale_generated_file(output_root.join("stripped_boundaries.rs"))?;
    fs::write(output_root.join("lib.rs"), lib_source(!xetex.is_empty()))?;

    // The pool ships inside the crate so the package builds without the local web2c tree.
    let pool_dir = repo_root.join("generated/portable-engine/pool");
    fs::create_dir_all(&pool_dir)?;
    fs::copy(
        repo_root.join("generated/web2c/xetex/xetex.pool"),
        pool_dir.join("xetex.pool"),
    )?;
    fs::write(
        repo_root.join("generated/portable-engine/patch-report.tsv"),
        report_source(
            shared.len(),
            xetex.len(),
            &shared_sources,
            &comparison_records,
            &boundary_adapted_native_io,
            &boundary_stripped_output,
            &boundary_stripped_native_font_assembly,
            &skipped,
        ),
    )?;
    fs::write(
        repo_root.join("generated/portable-engine/import-report.md"),
        import_report_markdown(&import_profiles, &comparison_records),
    )?;

    println!(
        "patched portable engine sources: {} shared, {} xetex patches, {} skipped",
        shared.len(),
        xetex.len(),
        skipped.len()
    );
    Ok(())
}

/// Selection and classification pass used by tests to assert zero `Unknown` diffs.
#[cfg(test)]
pub(crate) fn collect_shared_comparison_records(
    repo_root: &std::path::Path,
) -> Result<Vec<crate::compare::ComparisonRecord>, PatchError> {
    let input_root = repo_root.join("generated/c2rust");
    let manifest = Web2cSourceManifest::new();
    let tex_items = extract_functions_from_paths(&[
        input_root.join("tex/src/tex0.rs"),
        input_root.join("tex/src/texini.rs"),
    ])?;
    let etex_items = extract_functions_from_paths(&[
        input_root.join("etex/src/etex0.rs"),
        input_root.join("etex/src/etexini.rs"),
    ])?;
    let xetex_items = extract_functions_from_paths(&[
        input_root.join("xetex/src/xetex0.rs"),
        input_root.join("xetex/src/xetexini.rs"),
    ])?;
    let globals = extract_globals(&[
        input_root.join("tex/src/tex0.rs"),
        input_root.join("tex/src/texini.rs"),
        input_root.join("etex/src/etex0.rs"),
        input_root.join("etex/src/etexini.rs"),
        input_root.join("xetex/src/xetex0.rs"),
        input_root.join("xetex/src/xetexini.rs"),
    ])?;
    let tex_map = tex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let etex_map = etex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let xetex_map = xetex_items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let translated_functions = tex_items
        .iter()
        .chain(etex_items.iter())
        .chain(xetex_items.iter())
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let shared_duplicate_policy = SharedDuplicatePolicy::from_env();
    let shared_roots = tex_items
        .iter()
        .filter(|item| {
            manifest.classify_symbol(item.name.as_str())
                == TranslatedSymbolClass::SharedTexCoreCandidate
        })
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let xetex_roots = xetex_items
        .iter()
        .filter(|item| {
            manifest.classify_symbol(item.name.as_str())
                == TranslatedSymbolClass::XetexPatchCandidate
        })
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let selected = select_portable_functions(
        &manifest,
        &tex_map,
        &etex_map,
        &xetex_map,
        &translated_functions,
        shared_roots,
        xetex_roots,
    );
    let reachable = reachable_from_tex_core(
        &tex_map,
        &etex_map,
        &xetex_map,
        &translated_functions,
    );

    let mut comparison_records = Vec::new();
    for name in &selected.shared {
        if is_removed_aat_diagnostic_function(name) {
            continue;
        }
        let patched = patch_shared_function(
            name,
            &tex_map,
            &etex_map,
            &xetex_map,
            &translated_functions,
            &globals,
            shared_duplicate_policy,
            &reachable,
        )?
        .expect("selected shared function should exist in translated output");
        comparison_records.push(patched.comparison);
    }
    Ok(comparison_records)
}

pub(crate) fn remove_stale_generated_file(path: PathBuf) -> Result<(), PatchError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Removes `.rs` files in `functions/` that were not freshly emitted.
pub(crate) fn remove_stale_functions_files(
    functions_dir: &std::path::Path,
    kept: &BTreeSet<String>,
) -> Result<(), PatchError> {
    let read_dir = match fs::read_dir(functions_dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !kept.contains(name) {
            remove_stale_generated_file(path)?;
        }
    }
    Ok(())
}


pub(crate) fn is_removed_aat_diagnostic_function(name: &str) -> bool {
    matches!(name, "znotaatfonterror" | "znotaatgrfonterror")
}

/// Names of functions defined in the runtime prelude template.
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

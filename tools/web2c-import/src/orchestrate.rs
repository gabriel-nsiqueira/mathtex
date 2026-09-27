//! Orchestration entry point for the Web2C/C2Rust patcher.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::boundary::PORTABLE_BOUNDARY_CALLS;
use crate::codegen::{
    cargo_source, fill_build_fingerprint, functions_split, lib_source, pool_string_constants,
    report_source, runtime_source, translated_xetex_startup_primitives, FunctionsSplit,
};
use crate::extract::{extract_functions_from_paths, extract_globals};
use crate::flow::{
    build_trigger_set, calls_prelude_bridge, rewrite_fallible_function, unwrapped_flow_calls,
};
use crate::inputs::Inputs;
use crate::model::{io_error, write_file, FunctionReport, PatchError};
use crate::select::{
    abort_reachable_closure, function_origin, prelude_defined_functions, select_portable_functions,
    source_hash, unsupported_external_boundaries,
};
use crate::transform::PatchSession;
use crate::{CORE_SELECTION_ROOTS, XETEX_SELECTION_ROOTS};

/// Runs the Web2C import and emits the portable engine sources under the repo root argument.
pub fn run() -> Result<(), PatchError> {
    let repo_root = repo_root_argument(env::args_os().skip(1))?;
    let inputs = Inputs::locate()?;
    crate::pin::verify_or_update(&inputs)?;
    let output = generate(&inputs)?;
    write_output(
        &repo_root.join("generated/portable-engine"),
        &inputs,
        &output,
    )?;
    println!(
        "patched portable engine sources: {} functions, {} skipped",
        output.function_count, output.skipped_count
    );
    Ok(())
}

fn repo_root_argument(
    mut args: impl Iterator<Item = std::ffi::OsString>,
) -> Result<PathBuf, PatchError> {
    let root = match (args.next(), args.next()) {
        (None, _) => env::current_dir().map_err(|error| io_error(Path::new("."), error))?,
        (Some(root), None) if !root.to_string_lossy().starts_with('-') => PathBuf::from(root),
        _ => return Err(PatchError::Usage),
    };
    if root.join("generated/portable-engine/Cargo.toml").is_file() {
        Ok(root)
    } else {
        Err(PatchError::Usage)
    }
}

/// Everything the patcher writes, built in memory before any file is touched.
struct GeneratedEngine {
    runtime: String,
    functions: FunctionsSplit,
    report: String,
    function_count: usize,
    skipped_count: usize,
}

fn generate(inputs: &Inputs) -> Result<GeneratedEngine, PatchError> {
    let sources = inputs.translated_sources();
    let items = extract_functions_from_paths(&sources)?;
    let globals = extract_globals(&sources)?;
    let functions = items
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let translated_functions = functions.keys().cloned().collect::<BTreeSet<_>>();

    let roots = CORE_SELECTION_ROOTS
        .iter()
        .chain(XETEX_SELECTION_ROOTS)
        .map(|root| (*root).to_string());
    let selected = select_portable_functions(
        &functions,
        &translated_functions,
        &prelude_defined_functions(),
        roots,
    );

    let mut session = PatchSession::new();
    let startup_primitives = translated_xetex_startup_primitives(
        &functions,
        &translated_functions,
        &globals,
        &mut session,
    )?;
    let mut patched = Vec::new();
    let mut reports = Vec::new();
    let mut referenced_globals = BTreeSet::new();
    for name in &selected.functions {
        let function = &functions[name];
        let source = function.patch(&translated_functions, &globals, &mut session);
        let external_boundaries = unsupported_external_boundaries(&source);
        reports.push(FunctionReport {
            symbol: name.clone(),
            origin: function_origin(name, &external_boundaries),
            upstream_hash: source_hash(&function.source),
            external_boundaries,
        });
        referenced_globals.extend(function.referenced_globals(&globals));
        patched.push((name, source));
    }

    // Patches that call an `EngineFlow` prelude bridge make their function an abort seed.
    let patched_callers = patched
        .iter()
        .filter(|(_, source)| calls_prelude_bridge(source))
        .map(|(name, _)| (*name).clone())
        .collect::<BTreeSet<_>>();
    // Abort reachable functions return `EngineFlow<T>` with fallible calls wrapped in `?`.
    let fallible = abort_reachable_closure(&functions, &translated_functions, &patched_callers);
    let mut flow_triggers = build_trigger_set(&fallible, PORTABLE_BOUNDARY_CALLS);
    flow_triggers.register_prelude_bridges();

    // Startup primitive methods call `zprimitive`, so this rewrite adds `EngineFlow<()>`.
    let startup_primitives = rewrite_fallible_function(&startup_primitives, &flow_triggers);
    let emitted = patched
        .into_iter()
        .map(|(name, source)| {
            if fallible.contains(name) {
                rewrite_fallible_function(&source, &flow_triggers)
            } else {
                source
            }
        })
        .collect::<Vec<_>>();

    let pool = crate::model::read_file(&inputs.pool())?;
    let mut runtime = runtime_source(
        &globals,
        &referenced_globals,
        &startup_primitives,
        &pool_string_constants(&pool)?,
        &flow_triggers,
        &mut session,
    )?;
    session.finish()?;

    let unwrapped = emitted
        .iter()
        .chain([&startup_primitives])
        .flat_map(|block| unwrapped_flow_calls(block, &flow_triggers))
        .collect::<Vec<_>>();
    if !unwrapped.is_empty() {
        return Err(PatchError::UnwrappedFlowCalls(unwrapped));
    }

    let functions = functions_split(&emitted);
    fill_build_fingerprint(&mut runtime, &functions)?;
    Ok(GeneratedEngine {
        runtime,
        functions,
        report: report_source(&reports, &selected),
        function_count: emitted.len(),
        skipped_count: selected.skipped.len(),
    })
}

fn write_output(
    crate_root: &Path,
    inputs: &Inputs,
    output: &GeneratedEngine,
) -> Result<(), PatchError> {
    let manifest_path = crate_root.join("Cargo.toml");
    let manifest = cargo_source(&manifest_path)?;
    write_file(&manifest_path, manifest)?;

    let source_root = crate_root.join("src");
    let functions_dir = source_root.join("functions");
    create_dir_all(&functions_dir)?;
    write_file(&source_root.join("runtime.rs"), &output.runtime)?;
    write_file(&source_root.join("lib.rs"), lib_source())?;
    let mut kept = BTreeSet::from(["mod.rs".to_string()]);
    for file in &output.functions.files {
        write_file(&functions_dir.join(&file.file_name), &file.contents)?;
        kept.insert(file.file_name.clone());
    }
    write_file(&functions_dir.join("mod.rs"), &output.functions.mod_rs)?;
    remove_stale_functions_files(&functions_dir, &kept)?;

    // The pool ships inside the crate so the package builds without the web2c cache.
    let pool_dir = crate_root.join("pool");
    create_dir_all(&pool_dir)?;
    let pool_source = inputs.pool();
    let pool_target = pool_dir.join("xetex.pool");
    fs::copy(&pool_source, &pool_target).map_err(|error| io_error(&pool_source, error))?;
    write_file(&crate_root.join("patch-report.tsv"), &output.report)
}

fn create_dir_all(path: &Path) -> Result<(), PatchError> {
    fs::create_dir_all(path).map_err(|error| io_error(path, error))
}

/// Removes `.rs` files in `functions/` that were not freshly emitted.
fn remove_stale_functions_files(
    functions_dir: &Path,
    kept: &BTreeSet<String>,
) -> Result<(), PatchError> {
    let read_dir = fs::read_dir(functions_dir).map_err(|error| io_error(functions_dir, error))?;
    for entry in read_dir {
        let path = entry
            .map_err(|error| io_error(functions_dir, error))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if kept.contains(name) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&path, error)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_or_flag_arguments_are_usage_errors() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>()
                .into_iter()
        };
        assert!(matches!(
            repo_root_argument(args(&["a", "b"])),
            Err(PatchError::Usage)
        ));
        assert!(matches!(
            repo_root_argument(args(&["--help"])),
            Err(PatchError::Usage)
        ));
        assert!(matches!(
            repo_root_argument(args(&["/nonexistent/mathtex"])),
            Err(PatchError::Usage)
        ));
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = repo_root.to_str().expect("utf-8 repo path");
        assert!(repo_root_argument(args(&[root])).is_ok());
    }

    /// Runs the whole pipeline on the seeded cache, so every patch and flow check must pass.
    #[test]
    fn cached_inputs_generate_cleanly() {
        let inputs = Inputs::locate().expect("seed the web2c cache with the bootstrap script");
        crate::pin::verify_or_update(&inputs).expect("cache matches pin.toml");
        let output = generate(&inputs).unwrap_or_else(|error| panic!("{error}"));
        assert!(output.function_count > 300);
        assert_eq!(output.skipped_count, 0);
    }
}

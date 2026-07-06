//! Generated source emission for the portable engine.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::compare::{BodyDiff, ComparisonRecord};
use crate::model::{
    PatchError, RawFunction, SharedSourceOrigin, SharedSourceReport, SkippedSymbol,
};
use crate::profile::EngineImportProfile;
use crate::transform::{state_field_name, state_field_type};
use crate::EngineKind;

pub(crate) fn translated_etex_startup_primitives(
    etex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    globals: &BTreeMap<String, String>,
) -> Result<String, PatchError> {
    let mainbody = etex_map
        .get("mainbody")
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody",
        })?
        .patch(translated_functions, globals);
    let condition = "&& self.state.formatident == 1303 as i32";
    let condition_end = find_marker_end(&mainbody, &[condition]).ok_or(
        PatchError::MissingTranslatedBlock {
            function: "mainbody:eTeX startup condition",
        },
    )?;
    let block_start = mainbody[condition_end..]
        .find('{')
        .map(|offset| condition_end + offset + 1)
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody:eTeX startup block start",
        })?;
    let block_end_marker =
        "if *self.state.buffer.offset(self.state.curinput.locfield as isize) as i32";
    let block_end = find_ignoring_whitespace(&mainbody[block_start..], block_end_marker)
        .map(|offset| block_start + offset)
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody:eTeX startup block end",
        })?;
    let primitive_block = mainbody[block_start..block_end].trim();
    let mut source = String::new();
    writeln!(
        source,
        "    unsafe fn init_etex_startup_primitives(&mut self) {{"
    )
    .expect("write to String");
    writeln!(
        source,
        "        if !self.supports_etex() || self.is_xetex() {{\n            return;\n        }}"
    )
    .expect("write to String");
    for line in primitive_block.lines() {
        writeln!(source, "        {line}").expect("write to String");
    }
    writeln!(
        source,
        "        self.state.eTeXmode = 1 as eightbits;\n    }}\n"
    )
    .expect("write to String");
    Ok(source)
}


pub(crate) fn translated_xetex_startup_primitives(
    xetex_map: &BTreeMap<String, RawFunction>,
    translated_functions: &BTreeSet<String>,
    globals: &BTreeMap<String, String>,
) -> Result<String, PatchError> {
    let mainbody = xetex_map
        .get("mainbody")
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody",
        })?
        .patch(translated_functions, globals);
    let condition_markers = [
        "&& self.state.formatident as i64 == 66713 as i64",
        "&& self.state.formatident == 66713 as i32",
    ];
    let condition_end = find_marker_end(&mainbody, &condition_markers).ok_or(
        PatchError::MissingTranslatedBlock {
            function: "mainbody:XeTeX startup condition",
        },
    )?;
    let block_start = mainbody[condition_end..]
        .find('{')
        .map(|offset| condition_end + offset + 1)
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody:XeTeX startup block start",
        })?;
    let block_end_marker = "if self.state.nonewcontrolsequence == 0";
    let block_end = find_ignoring_whitespace(&mainbody[block_start..], block_end_marker)
        .map(|offset| block_start + offset)
        .ok_or(PatchError::MissingTranslatedBlock {
            function: "mainbody:XeTeX startup block end",
        })?;
    let primitive_block = trim_trailing_standalone_brace_lines(mainbody[block_start..block_end].trim());
    let mut source = String::new();
    writeln!(
        source,
        "    unsafe fn init_xetex_startup_primitives(&mut self) {{"
    )
    .expect("write to String");
    writeln!(
        source,
        "        if !self.is_xetex() {{\n            return;\n        }}"
    )
    .expect("write to String");
    for line in primitive_block.lines() {
        writeln!(source, "        {line}").expect("write to String");
    }
    writeln!(source, "    }}\n").expect("write to String");
    Ok(source)
}


pub(crate) fn trim_trailing_standalone_brace_lines(source: &str) -> String {
    let mut lines = source.lines().collect::<Vec<_>>();
    while matches!(lines.last(), Some(line) if line.trim() == "}") {
        lines.pop();
    }
    lines.join("\n")
}


pub(crate) fn find_marker_end(source: &str, markers: &[&str]) -> Option<usize> {
    markers.iter().find_map(|marker| {
        find_ignoring_whitespace(source, marker).map(|start| {
            start + matched_len_ignoring_whitespace(&source[start..], marker)
        })
    })
}

/// `mainbody` is reformatted by `prettyplease`, so markers can't be matched as literal substrings.
pub(crate) fn find_ignoring_whitespace(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    for start in 0..=bytes.len() {
        if matches_at_ignoring_whitespace(&haystack[start..], needle) {
            return Some(start);
        }
    }
    None
}

fn matches_at_ignoring_whitespace(text: &str, needle: &str) -> bool {
    matched_end_ignoring_whitespace(text, needle).is_some()
}

/// Returns 0 if no match, callers must first confirm a match via `matches_at_ignoring_whitespace`.
fn matched_len_ignoring_whitespace(text: &str, needle: &str) -> usize {
    matched_end_ignoring_whitespace(text, needle).unwrap_or(0)
}

fn matched_end_ignoring_whitespace(text: &str, needle: &str) -> Option<usize> {
    let tb = text.as_bytes();
    let nb = needle.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut ti = 0;
    let mut ni = 0;
    let mut prev_needle: Option<u8> = None;
    // The match must start at text[0], callers scan every position, so indentation is not skipped.
    while ni < nb.len() {
        if nb[ni].is_ascii_whitespace() {
            while ni < nb.len() && nb[ni].is_ascii_whitespace() {
                ni += 1;
            }
            while ti < tb.len() && tb[ti].is_ascii_whitespace() {
                ti += 1;
            }
            prev_needle = Some(b' ');
            continue;
        }
        // Skip whitespace in text only at token boundaries, never within identifier or number runs.
        if let Some(prev) = prev_needle {
            let at_token_boundary = !is_word(nb[ni]) || !is_word(prev);
            if at_token_boundary {
                while ti < tb.len() && tb[ti].is_ascii_whitespace() {
                    ti += 1;
                }
            }
        }
        if ti >= tb.len() || tb[ti] != nb[ni] {
            return None;
        }
        prev_needle = Some(nb[ni]);
        ti += 1;
        ni += 1;
    }
    Some(ti)
}

pub(crate) struct FunctionsFile {
    pub(crate) file_name: String,
    pub(crate) contents: String,
}

pub(crate) struct FunctionsSplit {
    pub(crate) files: Vec<FunctionsFile>,
    pub(crate) mod_rs: String,
}

/// Splits items into per letter groups without rewriting bytes, as the Phase 5 equivalence gate requires.
pub(crate) fn functions_split(description: &str, items: &[String]) -> FunctionsSplit {
    let mut groups: BTreeMap<char, Vec<&String>> = BTreeMap::new();
    for item in items {
        let name = function_block_name(item)
            .expect("every shared item is a `pub(crate) unsafe fn <name>` block");
        let key = group_key(&name);
        groups.entry(key).or_default().push(item);
    }

    let mut files = Vec::new();
    let mut module_names = Vec::new();
    for (key, group_items) in &groups {
        let module_name = group_module_name(*key);
        module_names.push(module_name.clone());
        let mut contents = String::new();
        writeln!(
            contents,
            "//! {description} (group `{module_name}`).\n//! Generated by `cargo run -p mathtex-web2c-import --bin patch_engine`.\n"
        )
        .expect("write to String");
        writeln!(
            contents,
            "#![allow(dead_code, non_camel_case_types, non_snake_case, unused_assignments, unused_must_use, unused_mut, unused_variables)]\n"
        )
        .expect("write to String");
        writeln!(contents, "use crate::runtime::*;\n").expect("write to String");
        writeln!(
            contents,
            "impl<'resources> PortableTexEngine<'resources> {{\n"
        )
        .expect("write to String");
        for item in group_items {
            contents.push_str(item);
            contents.push('\n');
        }
        contents.push_str("}\n");
        files.push(FunctionsFile {
            file_name: format!("{module_name}.rs"),
            contents,
        });
    }

    let mut mod_rs = String::new();
    writeln!(
        mod_rs,
        "//! {description}.\n//! Generated by `cargo run -p mathtex-web2c-import --bin patch_engine`.\n//!\n//! The shared TeX core is split into per-symbol-group partial `impl` blocks; the\n//! union of their function bodies is the complete shared core."
    )
    .expect("write to String");
    mod_rs.push('\n');
    for module_name in &module_names {
        writeln!(mod_rs, "mod {module_name};").expect("write to String");
    }
    FunctionsSplit { files, mod_rs }
}

fn function_block_name(item: &str) -> Option<String> {
    const PREFIX: &str = "pub(crate) unsafe fn ";
    let start = item.find(PREFIX)? + PREFIX.len();
    let rest = &item[start..];
    let end = rest
        .find(|character: char| character == '(' || character == '<' || character.is_whitespace())
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn group_key(name: &str) -> char {
    match name.chars().next() {
        Some(c) if c.is_ascii_alphabetic() => c.to_ascii_lowercase(),
        _ => '_',
    }
}

fn group_module_name(key: char) -> String {
    if key == '_' {
        "other".to_string()
    } else {
        key.to_string()
    }
}

pub(crate) fn runtime_source(
    globals: &BTreeMap<String, String>,
    referenced_globals: &BTreeSet<String>,
    etex_startup_primitives: &str,
    xetex_startup_primitives: &str,
    flow_triggers: &crate::flow::FlowTriggers,
) -> String {
    let mut source = include_str!("../runtime/prelude.rs.in").to_string();

    let mut state_fields = String::new();
    for name in referenced_globals {
        if name == "mem" {
            continue;
        }
        let Some(ty) = globals.get(name) else {
            continue;
        };
        // zeqtb and hash use PagedView for paged sparse backing, offset calls go through the pager.
        let field = state_field_name(name);
        let ty_str = match field {
            "zeqtb" => "PagedView<memoryword>".to_string(),
            "hash" => "PagedView<twohalves>".to_string(),
            _ => state_field_type(ty),
        };
        writeln!(state_fields, "    pub {field}: {ty_str},").expect("write to String");
    }
    source = source.replace("{state_fields}", &state_fields);
    source = source.replace("{etex_startup_primitives}", etex_startup_primitives);
    source = source.replace("{xetex_startup_primitives}", xetex_startup_primitives);
    if let Some((head, tail)) =
        source.split_once("impl<'resources> PortableTexEngine<'resources> {\n")
    {
        let mut tail = tail.to_string();
        tail = tail.replace("(&mut self, ", "(");
        tail = tail.replace("(&mut self)", "()");
        tail = tail.replace("        &mut self,\n", "");
        tail = tail.replace("self.host_", "Self::host_");
        source = format!("{head}impl<'resources> PortableTexEngine<'resources> {{\n{tail}");
    }
    // Restore &mut self stripped by the tail munge, flow left -> EngineFlow<()> with no params.
    source = source.replace(
        "unsafe fn init_etex_startup_primitives() -> EngineFlow<()> {",
        "unsafe fn init_etex_startup_primitives(&mut self) -> EngineFlow<()> {",
    );
    source = source.replace(
        "unsafe fn init_xetex_startup_primitives() -> EngineFlow<()> {",
        "unsafe fn init_xetex_startup_primitives(&mut self) -> EngineFlow<()> {",
    );

    // Rewrites named prelude bridges to propagate abort via ?, catch points are handwritten and excluded.
    source = crate::flow::rewrite_named_functions(
        source,
        crate::flow::PRELUDE_BRIDGE_FUNCTIONS,
        flow_triggers,
    );

    source
}

pub(crate) fn cargo_source() -> &'static str {
    r#"[package]
name = "mathtex-portable-engine-generated"
version = "0.1.0"
edition = "2021"
description = "Machine generated portable TeX engine core for mathtex, translated from the TeX, eTeX and XeTeX sources"
repository = "https://github.com/gabriel-nsiqueira/mathtex"
license-file = "LICENSE"

[lib]
path = "src/lib.rs"

[dependencies]
"#
}

pub(crate) fn lib_source(_include_xetex_patches: bool) -> String {
    r#"//! Auto-patched portable engine source from Web2C/C2Rust bootstrap output.

pub mod runtime;
// Shared TeX core, split into per-symbol-group partial `impl` blocks under
// `functions/`. The union of those bodies is the complete shared core.
pub(crate) mod functions;
// Generated per-profile import metadata (id, kind, capabilities, source chain,
// pool, allowed host boundaries) for the tex/etex/xetex import profiles.
pub mod profiles;
// Profile-specific (xetex-only) patched functions live in `dispatch`.
pub(crate) mod dispatch;

pub use profiles::{
    ImportCapabilities, ImportProfile, ImportProfileKind, ImportStateBounds, IMPORT_PROFILES,
};

pub use runtime::memory_word_bytes;
pub use runtime::{
    EmptyFontPlatform, EmptyPlatform, EmptyResourceProvider, EngineProfile, EngineProfileKind,
    FontPlatform, GlyphAssembly, PortableClock, PortableFontHandle, PortableFontMetrics,
    PortableFormatImage, PortableLinebreakRequest, PortableMathAssemblyPart, PortableMathKernCorner,
    PortableMathVariant, PortableNativeGlyph, PortableNativeGlyphMetrics, PortableNativeTextMetrics,
    PortableNodeHandle, PortableNodeKind, PortableNodeSnapshot, PortableNodeSourceSpan,
    PortablePlatform, PortableResourceRequestRecord, PortableSourceSpan, PortableTexEngine,
    ResourceKind, ResourceProvider, ResourceRequest,
};
"#
    .to_string()
}

/// Emits per-profile import provenance constants, separate from runtime capability booleans.
pub(crate) fn profiles_source(profiles: &[EngineImportProfile]) -> String {
    let mut source = String::new();
    writeln!(
        source,
        "//! Generated engine import-profile metadata.\n//! Generated by `cargo run -p mathtex-web2c-import --bin patch_engine`.\n//!\n//! Data form of the import-profile specs: which source chain and pool produced\n//! each profile, its capability set, and the host boundaries the importer is\n//! permitted to cross. The runtime capability booleans live on\n//! [`crate::runtime::EngineProfile`]; these constants record import provenance."
    )
    .expect("write to String");
    writeln!(
        source,
        "\n#![allow(dead_code)]\n"
    )
    .expect("write to String");

    source.push_str(PROFILES_TYPE_DEFINITIONS);

    writeln!(
        source,
        "\n/// All formalized engine import profiles, in `tex`/`etex`/`xetex` order."
    )
    .expect("write to String");
    writeln!(
        source,
        "pub static IMPORT_PROFILES: &[ImportProfile] = &[",
    )
    .expect("write to String");
    for profile in profiles {
        source.push_str(&render_profile_literal(profile));
    }
    writeln!(source, "];").expect("write to String");
    source
}

const PROFILES_TYPE_DEFINITIONS: &str = r#"/// Engine kind an import profile targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportProfileKind {
    Tex,
    Etex,
    Xetex,
}

/// Capabilities a profile turns on over the shared TeX core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportCapabilities {
    pub etex: bool,
    pub unicode_scalars: bool,
    pub unicode_math: bool,
    pub native_fonts: bool,
    pub output: bool,
}

/// Inclusive runtime `mem` array bounds for a profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportStateBounds {
    pub mem_min: i32,
    pub mem_top: i32,
}

/// One formalized engine import profile, as generated import metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportProfile {
    /// Stable profile id (`"tex"` / `"etex"` / `"xetex"`).
    pub id: &'static str,
    /// Engine kind this profile imports.
    pub kind: ImportProfileKind,
    /// Ordered source chain: shared WEB source first, then change files.
    pub source_chain: &'static [&'static str],
    /// String pool file linked for this profile.
    pub pool_file: &'static str,
    /// Inclusive `mem` array bounds used at runtime.
    pub state_bounds: ImportStateBounds,
    /// Capabilities enabled over the shared core.
    pub capabilities: ImportCapabilities,
    /// Host-boundary identifiers this profile is permitted to cross.
    pub allowed_boundaries: &'static [&'static str],
}
"#;

fn render_profile_literal(profile: &EngineImportProfile) -> String {
    let mut literal = String::new();
    let kind = match profile.kind {
        EngineKind::Tex => "Tex",
        EngineKind::Etex => "Etex",
        EngineKind::Xetex => "Xetex",
    };
    writeln!(literal, "    ImportProfile {{").expect("write to String");
    writeln!(literal, "        id: {:?},", profile.id).expect("write to String");
    writeln!(literal, "        kind: ImportProfileKind::{kind},").expect("write to String");
    writeln!(literal, "        source_chain: &[").expect("write to String");
    for entry in &profile.source_chain {
        writeln!(literal, "            {entry:?},").expect("write to String");
    }
    writeln!(literal, "        ],").expect("write to String");
    writeln!(literal, "        pool_file: {:?},", profile.pool_file).expect("write to String");
    writeln!(
        literal,
        "        state_bounds: ImportStateBounds {{ mem_min: {}, mem_top: {} }},",
        profile.state_bounds.mem_min, profile.state_bounds.mem_top
    )
    .expect("write to String");
    writeln!(
        literal,
        "        capabilities: ImportCapabilities {{ etex: {}, unicode_scalars: {}, unicode_math: {}, native_fonts: {}, output: {} }},",
        profile.capabilities.etex,
        profile.capabilities.unicode_scalars,
        profile.capabilities.unicode_math,
        profile.capabilities.native_fonts,
        profile.capabilities.output,
    )
    .expect("write to String");
    writeln!(literal, "        allowed_boundaries: &[").expect("write to String");
    for boundary in &profile.allowed_boundaries {
        writeln!(literal, "            {boundary:?},").expect("write to String");
    }
    writeln!(literal, "        ],").expect("write to String");
    writeln!(literal, "    }},").expect("write to String");
    literal
}

pub(crate) fn dispatch_source(xetex_items: &[String]) -> String {
    let mut source = String::new();
    writeln!(
        source,
        "//! Profile-specific (xetex-only) patched functions.\n//! Generated by `cargo run -p mathtex-web2c-import --bin patch_engine`.\n//!\n//! These are functions whose XeTeX body cannot be folded into the shared core\n//! and must be dispatched on the active engine profile. The patcher routes\n//! XeTeX-only patches here, each guarded so non-XeTeX profiles return early."
    )
    .expect("write to String");
    if xetex_items.is_empty() {
        writeln!(
            source,
            "//!\n//! With the current C2Rust inputs there are no such functions, so this is an\n//! empty scaffold rather than fabricated content."
        )
        .expect("write to String");
    }
    writeln!(
        source,
        "\n#![allow(dead_code, non_camel_case_types, non_snake_case, unused_assignments, unused_must_use, unused_mut, unused_variables)]\n"
    )
    .expect("write to String");
    writeln!(source, "use crate::runtime::*;\n").expect("write to String");
    writeln!(
        source,
        "impl<'resources> PortableTexEngine<'resources> {{\n"
    )
    .expect("write to String");
    for item in xetex_items {
        source.push_str(item);
        source.push('\n');
    }
    source.push_str("}\n");
    source
}

pub(crate) fn report_source(
    shared_count: usize,
    xetex_count: usize,
    shared_sources: &[SharedSourceReport],
    comparison_records: &[ComparisonRecord],
    boundary_adapted_native_io: &[String],
    boundary_stripped_output: &[String],
    boundary_stripped_native_font_assembly: &[String],
    skipped: &[SkippedSymbol],
) -> String {
    let diff_by_symbol = comparison_records
        .iter()
        .map(|record| (record.symbol.as_str(), record.diff))
        .collect::<BTreeMap<_, _>>();
    let mut report = String::from(
        "engine\tsymbol\tclass\tbase_hash\txetex_hash\texternal_boundaries\tdiff\n",
    );
    writeln!(report, "summary\tshared_kept\t{shared_count}").expect("write to String");
    writeln!(report, "summary\txetex_kept\t{xetex_count}").expect("write to String");
    for origin in [
        SharedSourceOrigin::TexOnly,
        SharedSourceOrigin::AdaptedNativeIo,
        SharedSourceOrigin::StrippedSourceSpecial,
        SharedSourceOrigin::StrippedWriteWhatsitDiagnostic,
        SharedSourceOrigin::StrippedPdfExtension,
        SharedSourceOrigin::EtexProfileGated,
        SharedSourceOrigin::NativeFontProfileGated,
        SharedSourceOrigin::XetexOnlyProfileGated,
        SharedSourceOrigin::TexCompatibleDuplicate,
        SharedSourceOrigin::TexAdaptedDuplicate,
        SharedSourceOrigin::BoundaryAdapterRequired,
        SharedSourceOrigin::XetexWidenedDuplicate,
    ] {
        let count = shared_sources
            .iter()
            .filter(|source| source.origin == origin)
            .count();
        writeln!(report, "summary\tshared_{}\t{count}", origin.as_str()).expect("write to String");
    }
    for diff in ALL_BODY_DIFFS {
        let count = comparison_records
            .iter()
            .filter(|record| record.diff == *diff)
            .count();
        writeln!(report, "summary\tdiff_{}\t{count}", diff.as_str()).expect("write to String");
    }
    writeln!(
        report,
        "summary\tboundary_AdaptedNativeIo\t{}",
        boundary_adapted_native_io.len()
    )
    .expect("write to String");
    writeln!(
        report,
        "summary\tboundary_StrippedOutput\t{}",
        boundary_stripped_output.len()
    )
    .expect("write to String");
    writeln!(
        report,
        "summary\tboundary_StrippedNativeFontAssembly\t{}",
        boundary_stripped_native_font_assembly.len()
    )
    .expect("write to String");
    for source in shared_sources {
        let base_hash = source
            .base_hash
            .map(|hash| format!("{hash:016x}"))
            .unwrap_or_else(|| "-".to_string());
        let xetex_hash = source
            .xetex_hash
            .map(|hash| format!("{hash:016x}"))
            .unwrap_or_else(|| "-".to_string());
        let external_boundaries = if source.external_boundaries.is_empty() {
            "Clean".to_string()
        } else {
            source.external_boundaries.join(",")
        };
        let diff = diff_by_symbol
            .get(source.symbol.as_str())
            .map(|diff| diff.as_str())
            .unwrap_or("-");
        writeln!(
            report,
            "shared\t{}\t{}\t{}\t{}\t{}\t{}",
            source.symbol,
            source.origin.as_str(),
            base_hash,
            xetex_hash,
            external_boundaries,
            diff,
        )
        .expect("write to String");
    }
    for symbol in boundary_adapted_native_io {
        writeln!(
            report,
            "boundary\t{symbol}\tAdaptedNativeIo\t-\t-\tAllowedManualAdapter\t-"
        )
        .expect("write to String");
    }
    for symbol in boundary_stripped_output {
        writeln!(
            report,
            "boundary\t{symbol}\tStrippedOutput\t-\t-\tAllowedManualAdapter\t-"
        )
        .expect("write to String");
    }
    for symbol in boundary_stripped_native_font_assembly {
        writeln!(
            report,
            "boundary\t{symbol}\tStrippedNativeFontAssembly\t-\t-\tAllowedManualAdapter\t-"
        )
        .expect("write to String");
    }
    for skipped in skipped {
        writeln!(
            report,
            "{}\t{}\t{:?}\t-\t-\t-\t-",
            skipped.engine, skipped.symbol, skipped.class
        )
        .expect("write to String");
    }
    report
}

const ALL_BODY_DIFFS: &[BodyDiff] = &[
    BodyDiff::IdenticalAfterNormalization,
    BodyDiff::OnlyConstantsDiffer,
    BodyDiff::SmallGuardedBehavior,
    BodyDiff::HostBoundaryDifference,
    BodyDiff::StateLayoutDifference,
    BodyDiff::LargeSemanticDivergence,
    BodyDiff::Unknown,
];

pub(crate) fn import_report_markdown(
    profiles: &[EngineImportProfile],
    comparison_records: &[ComparisonRecord],
) -> String {
    let mut report = String::new();
    writeln!(report, "# Portable engine import report").expect("write to String");
    writeln!(report).expect("write to String");
    writeln!(
        report,
        "Generated by `cargo run -p mathtex-web2c-import --bin patch_engine`."
    )
    .expect("write to String");
    writeln!(report).expect("write to String");

    writeln!(report, "## Engine import profiles").expect("write to String");
    writeln!(report).expect("write to String");
    writeln!(
        report,
        "| profile | kind | pool file | source chain | capabilities | allowed boundaries |"
    )
    .expect("write to String");
    writeln!(report, "| --- | --- | --- | --- | --- | --- |").expect("write to String");
    for profile in profiles {
        let caps = capability_summary(profile);
        let chain = profile.source_chain.join(", ");
        let boundaries = if profile.allowed_boundaries.is_empty() {
            "none".to_string()
        } else {
            format!("{} identifiers", profile.allowed_boundaries.len())
        };
        writeln!(
            report,
            "| {} | {:?} | {} | {} | {} | {} |",
            profile.id, profile.kind, profile.pool_file, chain, caps, boundaries
        )
        .expect("write to String");
    }
    writeln!(report).expect("write to String");

    writeln!(report, "## Body-diff classification").expect("write to String");
    writeln!(report).expect("write to String");
    writeln!(report, "| diff class | count |").expect("write to String");
    writeln!(report, "| --- | --- |").expect("write to String");
    for diff in ALL_BODY_DIFFS {
        let count = comparison_records
            .iter()
            .filter(|record| record.diff == *diff)
            .count();
        writeln!(report, "| {} | {} |", diff.as_str(), count).expect("write to String");
    }
    writeln!(report).expect("write to String");
    writeln!(
        report,
        "Total compared symbols: {}.",
        comparison_records.len()
    )
    .expect("write to String");
    writeln!(report).expect("write to String");

    writeln!(report, "## Per-symbol records").expect("write to String");
    writeln!(report).expect("write to String");
    writeln!(
        report,
        "| symbol | profiles | profile body hashes | diff | strategy | required boundaries | reason if not importable |"
    )
    .expect("write to String");
    writeln!(report, "| --- | --- | --- | --- | --- | --- | --- |").expect("write to String");
    for record in comparison_records {
        let boundaries = if record.required_boundaries.is_empty() {
            "none".to_string()
        } else {
            record.required_boundaries.join(", ")
        };
        let reason = record.reason_if_not_importable.as_deref().unwrap_or("-");
        let hashes = record
            .profile_body_hashes
            .iter()
            .map(|(profile, hash)| format!("{profile}={hash:016x}"))
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(
            report,
            "| {} | {} | {} | {} | {:?} | {} | {} |",
            record.symbol,
            record.profiles_present.join("+"),
            hashes,
            record.diff.as_str(),
            record.selected_strategy,
            boundaries,
            reason,
        )
        .expect("write to String");
    }
    report
}

fn capability_summary(profile: &EngineImportProfile) -> String {
    let mut caps = Vec::new();
    if profile.capabilities.etex {
        caps.push("etex");
    }
    if profile.capabilities.unicode_scalars {
        caps.push("unicode_scalars");
    }
    if profile.capabilities.unicode_math {
        caps.push("unicode_math");
    }
    if profile.capabilities.native_fonts {
        caps.push("native_fonts");
    }
    if profile.capabilities.output {
        caps.push("output");
    }
    if caps.is_empty() {
        "core".to_string()
    } else {
        caps.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_insensitive_match_finds_contiguous_marker() {
        let haystack = "    if foo(bar) as i32 { baz }";
        let pos = find_ignoring_whitespace(haystack, "if foo(bar) as i32").unwrap();
        assert_eq!(&haystack[pos..pos + 4], "if f");
    }

    #[test]
    fn whitespace_insensitive_match_tolerates_inserted_breaks() {
        // prettyplease reflows `*self.state.buffer` across lines, the marker must still match.
        let haystack = "        if *self\n            .state\n            .buffer\n            .offset(loc as isize) as i32";
        let pos = find_ignoring_whitespace(
            haystack,
            "if *self.state.buffer.offset(loc as isize) as i32",
        )
        .expect("reflowed marker should still match");
        assert!(haystack[pos..].starts_with("if *self"));
    }

    #[test]
    fn whitespace_insensitive_match_does_not_split_identifiers() {
        // whitespace inside a word run is significant: `ab` does not match `a b`.
        assert!(find_ignoring_whitespace("a b", "ab").is_none());
        // whitespace runs collapse: `a b` matches `a  b`.
        assert!(find_ignoring_whitespace("a  b", "a b").is_some());
    }

    #[test]
    fn whitespace_insensitive_match_does_not_split_numbers() {
        assert!(find_ignoring_whitespace("1 2", "12").is_none());
    }

    #[test]
    fn find_marker_end_returns_position_after_match() {
        let haystack = "x && self.state.formatident == 1303 as i32 {";
        let end = find_marker_end(haystack, &["&& self.state.formatident == 1303 as i32"]).unwrap();
        assert!(haystack[end..].trim_start().starts_with('{'));
    }
}

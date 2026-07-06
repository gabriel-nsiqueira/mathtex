//! Web2C import helpers for classifying translated TeX sources.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

mod astpass;
mod boundary;
mod codegen;
mod compare;
mod extract;
mod flow;
mod model;
mod orchestrate;
mod profile;
mod select;
mod synpass;
mod transform;

pub use model::PatchError;
pub use orchestrate::run;
pub use profile::{
    EngineCapabilities, EngineImportProfile, StateBounds, DEFAULT_STATE_BOUNDS,
};

/// Declares translated TeX units, runtime gated patches, and excluded source families.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Web2cSourceManifest {
    units: Vec<Web2cSourceUnit>,
    forbidden_symbols: Vec<Web2cForbiddenSymbol>,
}

impl Web2cSourceManifest {
    /// Creates a manifest with default forbidden symbol rules.
    #[must_use]
    pub fn new() -> Self {
        Self {
            units: Vec::new(),
            forbidden_symbols: default_forbidden_symbols(),
        }
    }

    /// Adds a source unit to the manifest.
    #[must_use]
    pub fn with_unit(mut self, unit: Web2cSourceUnit) -> Self {
        self.units.push(unit);
        self
    }

    /// Adds a forbidden symbol rule to the manifest.
    #[must_use]
    pub fn with_forbidden_symbol(mut self, symbol: Web2cForbiddenSymbol) -> Self {
        self.forbidden_symbols.push(symbol);
        self
    }

    /// Source units in declaration order.
    #[must_use]
    pub fn units(&self) -> &[Web2cSourceUnit] {
        &self.units
    }

    /// Forbidden symbol rules in declaration order.
    #[must_use]
    pub fn forbidden_symbols(&self) -> &[Web2cForbiddenSymbol] {
        &self.forbidden_symbols
    }

    /// Classifies a translated symbol with manifest rules.
    #[must_use]
    pub fn classify_symbol(&self, symbol: &str) -> TranslatedSymbolClass {
        if let Some(forbidden) = self
            .forbidden_symbols
            .iter()
            .find(|forbidden| forbidden.symbol == symbol)
        {
            return TranslatedSymbolClass::Forbidden(forbidden.reason);
        }

        classify_translated_symbol(symbol)
    }

    /// Audits translated symbols against manifest rules.
    #[must_use]
    pub fn audit_symbols<'a>(
        &self,
        symbols: impl IntoIterator<Item = &'a str>,
    ) -> TranslatedSymbolAuditReport {
        let mut report = TranslatedSymbolAuditReport::default();
        for symbol in symbols {
            report.record(symbol, self.classify_symbol(symbol));
        }
        report
    }

    /// Returns units included for the given engine, TeX and XeTeX include all `SharedTexCore` units.
    pub fn included_for(&self, engine: EngineKind) -> impl Iterator<Item = &Web2cSourceUnit> {
        self.units.iter().filter(move |unit| match unit.role {
            Web2cSourceRole::SharedTexCore => true,
            Web2cSourceRole::EtexPatch => {
                matches!(engine, EngineKind::Etex | EngineKind::Xetex)
            }
            Web2cSourceRole::XetexPatch => matches!(engine, EngineKind::Xetex),
            Web2cSourceRole::Excluded(_) => false,
        })
    }

    /// Validates the manifest without rejecting unknown symbols.
    pub fn validate(&self) -> Result<Web2cManifestReport, Web2cManifestError> {
        self.validate_with_symbol_policy(false)
    }

    /// Like `validate`, but also rejects symbols not yet classified for portable import.
    pub fn validate_portable_import(&self) -> Result<Web2cManifestReport, Web2cManifestError> {
        self.validate_with_symbol_policy(true)
    }

    fn validate_with_symbol_policy(
        &self,
        reject_unknown_symbols: bool,
    ) -> Result<Web2cManifestReport, Web2cManifestError> {
        if !self
            .units
            .iter()
            .any(|unit| unit.role == Web2cSourceRole::SharedTexCore)
        {
            return Err(Web2cManifestError::MissingSharedTexCore);
        }

        for unit in &self.units {
            if unit.include {
                if let Some(reason) = classify_web2c_source_path(unit.path.as_str()) {
                    return Err(Web2cManifestError::ForbiddenIncludedSource {
                        path: unit.path.clone(),
                        reason,
                    });
                }
            }

            if let Web2cSourceRole::Excluded(reason) = unit.role {
                if unit.include {
                    return Err(Web2cManifestError::ForbiddenIncludedSource {
                        path: unit.path.clone(),
                        reason,
                    });
                }
            }
        }

        for unit in &self.units {
            for symbol in &unit.symbols {
                if unit.include {
                    if let TranslatedSymbolClass::Forbidden(reason) = self.classify_symbol(symbol) {
                        return Err(Web2cManifestError::ForbiddenSymbol {
                            symbol: symbol.clone(),
                            path: unit.path.clone(),
                            reason,
                        });
                    }
                }
            }
        }

        for (index, unit) in self.units.iter().enumerate() {
            if !unit.include {
                continue;
            }

            for symbol in &unit.symbols {
                if let Some(duplicate) = self.units[index + 1..]
                    .iter()
                    .filter(|other| other.include)
                    .find(|other| other.symbols.iter().any(|candidate| candidate == symbol))
                {
                    return Err(Web2cManifestError::DuplicateSymbol {
                        symbol: symbol.clone(),
                        first_path: unit.path.clone(),
                        second_path: duplicate.path.clone(),
                    });
                }
            }
        }

        if reject_unknown_symbols {
            for unit in &self.units {
                if !unit.include {
                    continue;
                }

                for symbol in &unit.symbols {
                    if self.classify_symbol(symbol) == TranslatedSymbolClass::Unknown {
                        return Err(Web2cManifestError::UnknownSymbol {
                            symbol: symbol.clone(),
                            path: unit.path.clone(),
                        });
                    }

                    if let Some(expected_class) = expected_symbol_class_for_role(unit.role) {
                        let actual_class = self.classify_symbol(symbol);
                        if actual_class != expected_class {
                            return Err(Web2cManifestError::SymbolRoleMismatch {
                                symbol: symbol.clone(),
                                path: unit.path.clone(),
                                role: unit.role,
                                expected: expected_class,
                                actual: actual_class,
                            });
                        }
                    }
                }
            }
        }

        Ok(self.report())
    }

    fn report(&self) -> Web2cManifestReport {
        Web2cManifestReport {
            shared_tex_units: self
                .units
                .iter()
                .filter(|unit| unit.role == Web2cSourceRole::SharedTexCore && unit.include)
                .count(),
            etex_patch_units: self
                .units
                .iter()
                .filter(|unit| unit.role == Web2cSourceRole::EtexPatch && unit.include)
                .count(),
            xetex_patch_units: self
                .units
                .iter()
                .filter(|unit| unit.role == Web2cSourceRole::XetexPatch && unit.include)
                .count(),
            excluded_units: self.units.iter().filter(|unit| !unit.include).count(),
        }
    }
}

impl Default for Web2cSourceManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Public symbol inventory extracted from raw TeX and XeTeX C2Rust bootstrap output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TranslatedSymbolInventory {
    tex_symbols: Vec<String>,
    xetex_symbols: Vec<String>,
}

impl TranslatedSymbolInventory {
    /// Creates an inventory from raw TeX and XeTeX symbol lists.
    #[must_use]
    pub fn from_symbols<T, X>(tex_symbols: T, xetex_symbols: X) -> Self
    where
        T: IntoIterator,
        T::Item: Into<String>,
        X: IntoIterator,
        X::Item: Into<String>,
    {
        Self {
            tex_symbols: normalize_symbols(tex_symbols),
            xetex_symbols: normalize_symbols(xetex_symbols),
        }
    }

    /// Parses the tab separated inventory emitted by `tools/audit-translated-bootstrap.sh`.
    pub fn parse_tsv(input: &str) -> Result<Self, TranslatedSymbolInventoryError> {
        let mut lines = input.lines();
        match lines.next() {
            Some("engine\tsymbol") => {}
            Some(header) => {
                return Err(TranslatedSymbolInventoryError::InvalidHeader {
                    found: String::from(header),
                });
            }
            None => return Err(TranslatedSymbolInventoryError::MissingHeader),
        }

        let mut tex_symbols = Vec::new();
        let mut xetex_symbols = Vec::new();

        for (offset, line) in lines.enumerate() {
            let line_number = offset + 2;
            let (engine, symbol) = line.split_once('\t').ok_or_else(|| {
                TranslatedSymbolInventoryError::MalformedRow {
                    line: line_number,
                    row: String::from(line),
                }
            })?;

            if symbol.contains('\t') {
                return Err(TranslatedSymbolInventoryError::MalformedRow {
                    line: line_number,
                    row: String::from(line),
                });
            }

            if symbol.is_empty() {
                return Err(TranslatedSymbolInventoryError::EmptySymbol { line: line_number });
            }

            match engine {
                "tex" => tex_symbols.push(symbol),
                "xetex" => xetex_symbols.push(symbol),
                unknown => {
                    return Err(TranslatedSymbolInventoryError::UnknownEngine {
                        line: line_number,
                        engine: String::from(unknown),
                    });
                }
            }
        }

        Ok(Self::from_symbols(tex_symbols, xetex_symbols))
    }

    /// Normalized TeX bootstrap symbols.
    #[must_use]
    pub fn tex_symbols(&self) -> &[String] {
        &self.tex_symbols
    }

    /// Normalized XeTeX bootstrap symbols.
    #[must_use]
    pub fn xetex_symbols(&self) -> &[String] {
        &self.xetex_symbols
    }

    /// Symbols present in both raw TeX and raw XeTeX output.
    #[must_use]
    pub fn shared_duplicate_symbols(&self) -> Vec<String> {
        self.tex_symbols
            .iter()
            .filter(|symbol| self.xetex_symbols.binary_search(symbol).is_ok())
            .cloned()
            .collect()
    }

    /// Symbols present only in raw XeTeX output.
    #[must_use]
    pub fn xetex_only_symbols(&self) -> Vec<String> {
        self.xetex_symbols
            .iter()
            .filter(|symbol| self.tex_symbols.binary_search(symbol).is_err())
            .cloned()
            .collect()
    }

    /// Audits shared duplicate and XeTeX only symbols against the manifest.
    #[must_use]
    pub fn audit(&self, manifest: &Web2cSourceManifest) -> TranslatedBootstrapAuditReport {
        let shared_duplicate_symbols = self.shared_duplicate_symbols();
        let xetex_only_symbols = self.xetex_only_symbols();
        let shared_surface =
            manifest.audit_symbols(shared_duplicate_symbols.iter().map(String::as_str));
        let xetex_patch_surface =
            manifest.audit_symbols(xetex_only_symbols.iter().map(String::as_str));

        TranslatedBootstrapAuditReport {
            tex_symbol_count: self.tex_symbols.len(),
            xetex_symbol_count: self.xetex_symbols.len(),
            shared_duplicate_symbols,
            xetex_only_symbols,
            shared_surface,
            xetex_patch_surface,
        }
    }
}

/// Error parsing a raw translated symbol inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranslatedSymbolInventoryError {
    /// The inventory did not contain a header row.
    MissingHeader,
    /// The header row was not `engine<TAB>symbol`.
    InvalidHeader {
        /// Header value that was found.
        found: String,
    },
    /// A data row did not contain exactly two tab separated columns.
    MalformedRow {
        /// Input line number, one based.
        line: usize,
        /// Row text that failed parsing.
        row: String,
    },
    /// A data row used an unknown engine name.
    UnknownEngine {
        /// Input line number, one based.
        line: usize,
        /// Engine name that was found.
        engine: String,
    },
    /// A data row had an empty symbol column.
    EmptySymbol {
        /// Input line number, one based.
        line: usize,
    },
}

/// Audit report from `TranslatedSymbolInventory::audit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatedBootstrapAuditReport {
    /// Number of normalized raw TeX symbols.
    pub tex_symbol_count: usize,
    /// Number of normalized raw XeTeX symbols.
    pub xetex_symbol_count: usize,
    /// Symbols duplicated between raw TeX and raw XeTeX output.
    pub shared_duplicate_symbols: Vec<String>,
    /// Symbols present only in raw XeTeX output.
    pub xetex_only_symbols: Vec<String>,
    /// Import policy audit for duplicated symbols that should become shared TeX core behavior.
    pub shared_surface: TranslatedSymbolAuditReport,
    /// Import policy audit for XeTeX only symbols that should become runtime gated profile patch behavior.
    pub xetex_patch_surface: TranslatedSymbolAuditReport,
}

impl TranslatedBootstrapAuditReport {
    /// Number of symbols duplicated between the raw TeX and raw XeTeX bootstrap outputs.
    #[must_use]
    pub fn shared_duplicate_count(&self) -> usize {
        self.shared_duplicate_symbols.len()
    }

    /// Number of symbols present only in the raw XeTeX bootstrap output.
    #[must_use]
    pub fn xetex_only_count(&self) -> usize {
        self.xetex_only_symbols.len()
    }

    /// True when XeTeX output shares symbols with TeX output, confirming shared core overlap.
    #[must_use]
    pub fn proves_shared_tex_surface(&self) -> bool {
        !self.shared_duplicate_symbols.is_empty()
    }
}

/// Concrete Web2C/C2Rust bootstrap recipe for one TeX Live source snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Web2cBootstrapRecipe {
    /// Human readable identifier for this bootstrap recipe.
    pub name: &'static str,
    /// Root directory of the TeX Live Web2C source tree.
    pub source_root: &'static str,
    /// Primary `tex.web` source file.
    pub shared_tex_web: &'static str,
    /// Change files treated as shared TeX/Web2C core inputs.
    pub shared_tex_changes: &'static [&'static str],
    /// Full change file chain used by the raw XeTeX Web2C bootstrap command.
    pub xetex_change_chain: &'static [&'static str],
    /// XeTeX/eTeX change files that are patch inputs, excluding shared files.
    pub xetex_patch_changes: &'static [&'static str],
    /// Web2C C output files produced by this bootstrap.
    pub web2c_outputs: &'static [&'static str],
    /// C2Rust Rust output files produced from the Web2C C outputs.
    pub c2rust_outputs: &'static [&'static str],
    /// Bootstrap artifacts used as porting evidence, excluded from compilation as a second engine fork.
    pub bootstrap_only_outputs: &'static [&'static str],
}

impl Web2cBootstrapRecipe {
    /// Returns the portable engine source manifest.
    #[must_use]
    pub fn as_source_manifest(&self) -> Web2cSourceManifest {
        let mut manifest = Web2cSourceManifest::new().with_unit(Web2cSourceUnit::included(
            self.shared_tex_web,
            Web2cSourceRole::SharedTexCore,
        ));

        for change_file in self.shared_tex_changes {
            manifest = manifest.with_unit(Web2cSourceUnit::included(
                *change_file,
                Web2cSourceRole::SharedTexCore,
            ));
        }

        for change_file in self.xetex_patch_changes {
            manifest = manifest.with_unit(Web2cSourceUnit::included(
                *change_file,
                Web2cSourceRole::XetexPatch,
            ));
        }

        for output in self.bootstrap_only_outputs {
            manifest = manifest.with_unit(Web2cSourceUnit::excluded(
                *output,
                ExcludedWeb2cSource::BootstrapOnly,
            ));
        }

        manifest
    }
}

/// TeX Live recipe for generating bootstrap artifacts and reclassifying XeTeX patches over `tex.web`.
#[must_use]
pub const fn texlive_bootstrap_recipe() -> Web2cBootstrapRecipe {
    Web2cBootstrapRecipe {
        name: "texlive-web2c-c2rust-bootstrap",
        source_root: "vendor/texlive-source/texk/web2c",
        shared_tex_web: "tex.web",
        shared_tex_changes: &["tex.ch", "tex-binpool.ch"],
        xetex_change_chain: &[
            "xetexdir/tex.ch0",
            "tex.ch",
            "tracingstacklevels.ch",
            "partoken-102.ch",
            "partoken.ch",
            "locnull-optimize.ch",
            "unbalanced-braces.ch",
            "showstream.ch",
            "xetexdir/xetex.ch",
            "xetexdir/char-warning-xetex.ch",
            "tex-binpool.ch",
        ],
        xetex_patch_changes: &[
            "xetexdir/tex.ch0",
            "tracingstacklevels.ch",
            "partoken-102.ch",
            "partoken.ch",
            "locnull-optimize.ch",
            "unbalanced-braces.ch",
            "showstream.ch",
            "xetexdir/xetex.ch",
            "xetexdir/char-warning-xetex.ch",
        ],
        web2c_outputs: &[
            "generated/web2c/tex/tex0.c",
            "generated/web2c/tex/texini.c",
            "generated/web2c/etex/etex0.c",
            "generated/web2c/etex/etexini.c",
            "generated/web2c/xetex/xetex0.c",
            "generated/web2c/xetex/xetexini.c",
        ],
        c2rust_outputs: &[
            "generated/c2rust/tex/src/tex0.rs",
            "generated/c2rust/tex/src/texini.rs",
            "generated/c2rust/etex/src/etex0.rs",
            "generated/c2rust/etex/src/etexini.rs",
            "generated/c2rust/xetex/src/xetex0.rs",
            "generated/c2rust/xetex/src/xetexini.rs",
        ],
        bootstrap_only_outputs: &[
            "generated/web2c/etex/etex0.c",
            "generated/web2c/etex/etexini.c",
            "generated/c2rust/etex/src/etex0.rs",
            "generated/c2rust/etex/src/etexini.rs",
            "generated/web2c/xetex/xetex0.c",
            "generated/web2c/xetex/xetexini.c",
            "generated/c2rust/xetex/src/xetex0.rs",
            "generated/c2rust/xetex/src/xetexini.rs",
        ],
    }
}

/// One translated source unit from the Web2C/C2Rust import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Web2cSourceUnit {
    /// Source path relative to the imported Web2C tree or generated directory.
    pub path: String,
    /// Role this unit plays in the portable engine manifest.
    pub role: Web2cSourceRole,
    /// Whether this unit is linked into the portable engine.
    pub include: bool,
    /// Top level translated symbols contributed by this unit.
    pub symbols: Vec<String>,
}

impl Web2cSourceUnit {
    /// Creates a source unit that is included in the portable engine under the given role.
    #[must_use]
    pub fn included(path: impl Into<String>, role: Web2cSourceRole) -> Self {
        Self {
            path: path.into(),
            role,
            include: true,
            symbols: Vec::new(),
        }
    }

    /// Creates a source unit that is excluded from the portable engine for the given reason.
    #[must_use]
    pub fn excluded(path: impl Into<String>, reason: ExcludedWeb2cSource) -> Self {
        Self {
            path: path.into(),
            role: Web2cSourceRole::Excluded(reason),
            include: false,
            symbols: Vec::new(),
        }
    }

    /// Records one translated symbol for duplicate linkage checks.
    #[must_use]
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbols.push(symbol.into());
        self
    }
}

/// Translated symbol that must not be linked into the portable engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Web2cForbiddenSymbol {
    /// Symbol name produced by Web2C/C2Rust translation.
    pub symbol: String,
    /// Reason this symbol is forbidden from the portable engine.
    pub reason: ExcludedWeb2cSource,
}

impl Web2cForbiddenSymbol {
    /// Creates a forbidden symbol entry with the given name and exclusion reason.
    #[must_use]
    pub fn new(symbol: impl Into<String>, reason: ExcludedWeb2cSource) -> Self {
        Self {
            symbol: symbol.into(),
            reason,
        }
    }
}

/// Import classification for a symbol from the Web2C/C2Rust bootstrap output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranslatedSymbolClass {
    /// Candidate for the single shared TeX implementation.
    SharedTexCoreCandidate,
    /// Candidate for runtime gated XeTeX/eTeX patch behavior.
    XetexPatchCandidate,
    /// Symbol must be replaced by a portable host boundary or removed.
    Forbidden(ExcludedWeb2cSource),
    /// Symbol has not yet been audited.
    Unknown,
}

/// Classification counts for a set of audited translated symbols.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TranslatedSymbolAuditReport {
    /// Total symbols audited.
    pub total: usize,
    /// Symbols classified as shared TeX core candidates.
    pub shared_tex_core_candidates: usize,
    /// Symbols classified as XeTeX patch candidates.
    pub xetex_patch_candidates: usize,
    /// Symbols from Lua source families.
    pub lua_symbols: usize,
    /// Symbols from callback or node filter source families.
    pub callback_or_node_filter_symbols: usize,
    /// Symbols from shipout or output driver source families.
    pub shipout_or_driver_symbols: usize,
    /// Symbols from native input output source families.
    pub native_io_symbols: usize,
    /// Forbidden symbols from other uncategorized source families.
    pub other_forbidden_symbols: usize,
    /// Forbidden symbols encountered, in audit order.
    pub forbidden_symbols: Vec<String>,
    /// Symbols that need explicit review before import.
    pub unknown_symbols: Vec<String>,
}

impl TranslatedSymbolAuditReport {
    fn record(&mut self, symbol: &str, class: TranslatedSymbolClass) {
        self.total += 1;
        match class {
            TranslatedSymbolClass::SharedTexCoreCandidate => {
                self.shared_tex_core_candidates += 1;
            }
            TranslatedSymbolClass::XetexPatchCandidate => {
                self.xetex_patch_candidates += 1;
            }
            TranslatedSymbolClass::Forbidden(reason) => {
                self.forbidden_symbols.push(String::from(symbol));
                match reason {
                    ExcludedWeb2cSource::LuaRuntime | ExcludedWeb2cSource::Luatex => {
                        self.lua_symbols += 1;
                    }
                    ExcludedWeb2cSource::CallbackOrNodeFilter => {
                        self.callback_or_node_filter_symbols += 1;
                    }
                    ExcludedWeb2cSource::ShipoutOrDriver => {
                        self.shipout_or_driver_symbols += 1;
                    }
                    ExcludedWeb2cSource::NativeIo => {
                        self.native_io_symbols += 1;
                    }
                    ExcludedWeb2cSource::BootstrapOnly | ExcludedWeb2cSource::Other => {
                        self.other_forbidden_symbols += 1;
                    }
                }
            }
            TranslatedSymbolClass::Unknown => {
                self.unknown_symbols.push(String::from(symbol));
            }
        }
    }

    /// True when every audited symbol has a known, permitted import class.
    #[must_use]
    pub fn is_clear_for_portable_import(&self) -> bool {
        self.forbidden_symbols.is_empty() && self.unknown_symbols.is_empty()
    }
}

/// Audit aid for raw C2Rust output, `Unknown` means the symbol needs review before import.
#[must_use]
pub fn classify_translated_symbol(symbol: &str) -> TranslatedSymbolClass {
    if let Some((_, reason)) = DEFAULT_FORBIDDEN_SYMBOLS
        .iter()
        .find(|(forbidden, _)| *forbidden == symbol)
    {
        return TranslatedSymbolClass::Forbidden(*reason);
    }

    if SHARED_TEX_CORE_CANDIDATE_SYMBOLS.contains(&symbol) {
        return TranslatedSymbolClass::SharedTexCoreCandidate;
    }

    if XETEX_PATCH_CANDIDATE_SYMBOLS.contains(&symbol) {
        return TranslatedSymbolClass::XetexPatchCandidate;
    }

    TranslatedSymbolClass::Unknown
}

/// Catches excluded source families at the path level, before symbol inventory audit.
#[must_use]
pub fn classify_web2c_source_path(path: &str) -> Option<ExcludedWeb2cSource> {
    let path = path.to_ascii_lowercase();
    let path = path.as_str();

    if path.contains("callback") || path.contains("node_filter") || path.contains("nodefilter") {
        return Some(ExcludedWeb2cSource::CallbackOrNodeFilter);
    }

    if path.contains("shipout")
        || path.contains("ship_out")
        || path.contains("pdfship")
        || path.contains("dviout")
        || path.contains("output-driver")
        || path.contains("output_driver")
    {
        return Some(ExcludedWeb2cSource::ShipoutOrDriver);
    }

    if path.contains("kpathsea") || path.contains("/kpse") || path.contains("kpse_") {
        return Some(ExcludedWeb2cSource::NativeIo);
    }

    if path.contains("luatex") || path.contains("mfluajit") {
        return Some(ExcludedWeb2cSource::Luatex);
    }

    if path.contains("lua52")
        || path.contains("lua53")
        || path.contains("luajit")
        || path.contains("/lua/")
        || path.contains("/lua-")
        || path.contains("lua-src")
    {
        return Some(ExcludedWeb2cSource::LuaRuntime);
    }

    None
}

/// Role of a translated source unit within the portable engine manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Web2cSourceRole {
    /// Shared TeX source used by TeX, eTeX, and XeTeX profiles.
    SharedTexCore,
    /// eTeX behavior added as runtime gated patch data/code.
    EtexPatch,
    /// XeTeX behavior added as runtime gated patch data/code.
    XetexPatch,
    /// Source excluded from the portable engine with a recorded reason.
    Excluded(ExcludedWeb2cSource),
}

/// Engine kind used by import tooling when selecting source units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineKind {
    /// Plain TeX engine.
    Tex,
    /// eTeX extensions over the shared TeX core.
    Etex,
    /// XeTeX extensions over the shared TeX core.
    Xetex,
}

/// Reason a Web2C/C2Rust source or symbol is excluded from the portable engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExcludedWeb2cSource {
    /// Bootstrap artifact used as porting evidence, excluded from compilation into the shared engine.
    BootstrapOnly,
    /// LuaTeX engine source.
    Luatex,
    /// Embedded Lua runtime or bindings.
    LuaRuntime,
    /// Lua callback or node filter hook, replaced by the host boundary.
    CallbackOrNodeFilter,
    /// Shipout or output driver code, replaced by the IR backend.
    ShipoutOrDriver,
    /// Native filesystem, libc, resource search, or process input output dependency.
    NativeIo,
    /// Excluded for an uncategorized reason.
    Other,
}

/// Summary counts produced by a successful manifest validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Web2cManifestReport {
    /// Number of included shared TeX core source units.
    pub shared_tex_units: usize,
    /// Number of included eTeX patch source units.
    pub etex_patch_units: usize,
    /// Number of included XeTeX patch source units.
    pub xetex_patch_units: usize,
    /// Number of source units excluded from the portable engine.
    pub excluded_units: usize,
}

/// Validation error from `Web2cSourceManifest::validate` or `validate_portable_import`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Web2cManifestError {
    /// No shared TeX core source was declared.
    MissingSharedTexCore,
    /// An excluded source family was marked for inclusion.
    ForbiddenIncludedSource {
        /// Path of the source that was incorrectly included.
        path: String,
        /// Reason the source family is excluded.
        reason: ExcludedWeb2cSource,
    },
    /// Two source units expose the same translated symbol.
    DuplicateSymbol {
        /// Translated symbol name that appears in both units.
        symbol: String,
        /// Path of the first unit declaring the symbol.
        first_path: String,
        /// Path of the second unit declaring the symbol.
        second_path: String,
    },
    /// An included unit exports a symbol forbidden by the import policy.
    ForbiddenSymbol {
        /// Symbol that violates the import policy.
        symbol: String,
        /// Path of the source unit that exported the symbol.
        path: String,
        /// Reason the symbol is forbidden.
        reason: ExcludedWeb2cSource,
    },
    /// An included unit exports a symbol that has not been audited for import.
    UnknownSymbol {
        /// Symbol that has not yet been audited for portable import.
        symbol: String,
        /// Path of the source unit exposing the symbol.
        path: String,
    },
    /// An included unit exports a known portable symbol under the wrong source role.
    SymbolRoleMismatch {
        /// Symbol whose classification contradicts its declared source role.
        symbol: String,
        /// Path of the source unit declaring the role.
        path: String,
        /// Source role declared in the manifest.
        role: Web2cSourceRole,
        /// Classification expected for that role.
        expected: TranslatedSymbolClass,
        /// Classification the symbol actually received.
        actual: TranslatedSymbolClass,
    },
}

const SHARED_TEX_CORE_CANDIDATE_SYMBOLS: &[&str] = &[
    "getstringsstarted",
    "initprim",
    "initialize",
    "macrocall",
    "maincontrol",
    "mlisttohlist",
    "newnoad",
    "zappendtovlist",
    "zcleanbox",
    "zfetch",
    "zmakefraction",
    "zmakeop",
    "zmakeord",
    "zmakeradical",
    "zmakescripts",
    "zmakeunder",
    "zmakevcenter",
    "zmakeover",
    "znewglue",
    "znewkern",
    "znewmath",
    "znewpenalty",
    "znewstyle",
    "zscanglue",
    "zscantoks",
    "zscanspec",
    "zhpack",
    "zvpackage",
];

const XETEX_PATCH_CANDIDATE_SYMBOLS: &[&str] = &[
    "getinputnormalizationstate",
    "gettracingfontsstate",
    "maxhyphenatablelength",
    "scanusvnum",
    "zbuildopentypeassembly",
    "zloadnativefont",
    "zmathxheight",
    "znewnativecharacter",
    "znewnativewordnode",
];

const DEFAULT_FORBIDDEN_SYMBOLS: &[(&str, ExcludedWeb2cSource)] = &[
    ("lua_initialize", ExcludedWeb2cSource::LuaRuntime),
    ("lua_close", ExcludedWeb2cSource::LuaRuntime),
    ("luaopen_tex", ExcludedWeb2cSource::LuaRuntime),
    (
        "callback_register",
        ExcludedWeb2cSource::CallbackOrNodeFilter,
    ),
    ("run_callback", ExcludedWeb2cSource::CallbackOrNodeFilter),
    ("node_filter", ExcludedWeb2cSource::CallbackOrNodeFilter),
    ("ship_out", ExcludedWeb2cSource::ShipoutOrDriver),
    ("shipout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("dvi_out", ExcludedWeb2cSource::ShipoutOrDriver),
    ("pdf_ship_out", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zshipout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("hlistout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("vlistout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("dviswap", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zdvifour", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zdvitwo", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zdvipop", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zdvifontdef", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zdvinativefontdef", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zmovement", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zprunemovements", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zspecialout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zwriteout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zpicout", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zloadpicture", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zoutwhat", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zfireup", ExcludedWeb2cSource::ShipoutOrDriver),
    ("buildpage", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zprunepagetop", ExcludedWeb2cSource::ShipoutOrDriver),
    ("zfreezepagespecs", ExcludedWeb2cSource::ShipoutOrDriver),
    ("kpse_find_file", ExcludedWeb2cSource::NativeIo),
    ("kpse_in_name_ok", ExcludedWeb2cSource::NativeIo),
    ("kpse_out_name_ok", ExcludedWeb2cSource::NativeIo),
    ("fopen", ExcludedWeb2cSource::NativeIo),
    ("open_input", ExcludedWeb2cSource::NativeIo),
    ("open_output", ExcludedWeb2cSource::NativeIo),
    ("open_out_or_pipe", ExcludedWeb2cSource::NativeIo),
    ("close_file", ExcludedWeb2cSource::NativeIo),
    ("close_file_or_pipe", ExcludedWeb2cSource::NativeIo),
    ("openlogfile", ExcludedWeb2cSource::NativeIo),
    ("startinput", ExcludedWeb2cSource::NativeIo),
    ("openorclosein", ExcludedWeb2cSource::NativeIo),
    ("openfmtfile", ExcludedWeb2cSource::NativeIo),
    ("loadfmtfile", ExcludedWeb2cSource::NativeIo),
    ("storefmtfile", ExcludedWeb2cSource::NativeIo),
    ("closefilesandterminate", ExcludedWeb2cSource::NativeIo),
    ("jumpout", ExcludedWeb2cSource::NativeIo),
    ("uexit", ExcludedWeb2cSource::NativeIo),
    ("exit", ExcludedWeb2cSource::NativeIo),
    ("runsystem", ExcludedWeb2cSource::NativeIo),
    ("system", ExcludedWeb2cSource::NativeIo),
];

fn normalize_symbols<I>(symbols: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let mut symbols = symbols.into_iter().map(Into::into).collect::<Vec<_>>();
    symbols.sort();
    symbols.dedup();
    symbols
}

fn default_forbidden_symbols() -> Vec<Web2cForbiddenSymbol> {
    DEFAULT_FORBIDDEN_SYMBOLS
        .iter()
        .map(|(symbol, reason)| Web2cForbiddenSymbol::new(*symbol, *reason))
        .collect()
}

fn expected_symbol_class_for_role(role: Web2cSourceRole) -> Option<TranslatedSymbolClass> {
    match role {
        Web2cSourceRole::SharedTexCore => Some(TranslatedSymbolClass::SharedTexCoreCandidate),
        Web2cSourceRole::EtexPatch | Web2cSourceRole::XetexPatch => {
            Some(TranslatedSymbolClass::XetexPatchCandidate)
        }
        Web2cSourceRole::Excluded(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_reuses_shared_tex_core_for_tex_and_xetex() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(
                Web2cSourceUnit::included("tex.web", Web2cSourceRole::SharedTexCore)
                    .with_symbol("main_control"),
            )
            .with_unit(
                Web2cSourceUnit::included("xetex.ch", Web2cSourceRole::XetexPatch)
                    .with_symbol("xetex_scan_font_identifier"),
            )
            .with_unit(Web2cSourceUnit::excluded(
                "luatex.web",
                ExcludedWeb2cSource::Luatex,
            ));

        let tex_units = manifest
            .included_for(EngineKind::Tex)
            .map(|unit| unit.path.as_str())
            .collect::<Vec<_>>();
        let xetex_units = manifest
            .included_for(EngineKind::Xetex)
            .map(|unit| unit.path.as_str())
            .collect::<Vec<_>>();
        let report = manifest.validate().expect("manifest should validate");

        assert_eq!(tex_units, alloc::vec!["tex.web"]);
        assert_eq!(xetex_units, alloc::vec!["tex.web", "xetex.ch"]);
        assert_eq!(report.shared_tex_units, 1);
        assert_eq!(report.xetex_patch_units, 1);
        assert_eq!(report.excluded_units, 1);
    }

    #[test]
    fn manifest_rejects_included_lua_or_shipout_sources() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(Web2cSourceUnit::included(
                "tex.web",
                Web2cSourceRole::SharedTexCore,
            ))
            .with_unit(Web2cSourceUnit {
                path: "luatex/callbacks.c".into(),
                role: Web2cSourceRole::Excluded(ExcludedWeb2cSource::CallbackOrNodeFilter),
                include: true,
                symbols: Vec::new(),
            });

        let error = manifest
            .validate()
            .expect_err("forbidden source must not be included");

        assert_eq!(
            error,
            Web2cManifestError::ForbiddenIncludedSource {
                path: "luatex/callbacks.c".into(),
                reason: ExcludedWeb2cSource::CallbackOrNodeFilter,
            }
        );
    }

    #[test]
    fn manifest_rejects_forbidden_source_families_even_when_mislabeled() {
        for (path, reason) in [
            ("luatexdir/luatex.web", ExcludedWeb2cSource::Luatex),
            (
                "libs/lua53/lua53-src/src/lapi.c",
                ExcludedWeb2cSource::LuaRuntime,
            ),
            (
                "luatexdir/callbacks.c",
                ExcludedWeb2cSource::CallbackOrNodeFilter,
            ),
            (
                "backend/output-driver.c",
                ExcludedWeb2cSource::ShipoutOrDriver,
            ),
            ("texk/kpathsea/pathsearch.c", ExcludedWeb2cSource::NativeIo),
        ] {
            let manifest = Web2cSourceManifest::new()
                .with_unit(Web2cSourceUnit::included(
                    "tex.web",
                    Web2cSourceRole::SharedTexCore,
                ))
                .with_unit(Web2cSourceUnit::included(
                    path,
                    Web2cSourceRole::SharedTexCore,
                ));

            let error = manifest
                .validate()
                .expect_err("forbidden source family must not be included by relabeling");

            assert_eq!(
                error,
                Web2cManifestError::ForbiddenIncludedSource {
                    path: path.into(),
                    reason,
                }
            );
        }
    }

    #[test]
    fn manifest_rejects_duplicate_symbols_before_linking() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(
                Web2cSourceUnit::included("tex.web", Web2cSourceRole::SharedTexCore)
                    .with_symbol("main_control"),
            )
            .with_unit(
                Web2cSourceUnit::included("xetex.web", Web2cSourceRole::XetexPatch)
                    .with_symbol("main_control"),
            );

        let error = manifest
            .validate()
            .expect_err("duplicate translated symbols should be rejected");

        assert_eq!(
            error,
            Web2cManifestError::DuplicateSymbol {
                symbol: "main_control".into(),
                first_path: "tex.web".into(),
                second_path: "xetex.web".into(),
            }
        );
    }

    #[test]
    fn manifest_ignores_duplicate_symbols_from_excluded_bootstrap_units() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(
                Web2cSourceUnit::included("tex.web", Web2cSourceRole::SharedTexCore)
                    .with_symbol("main_control"),
            )
            .with_unit(
                Web2cSourceUnit::excluded(
                    "generated/c2rust/xetex/src/xetex0.rs",
                    ExcludedWeb2cSource::BootstrapOnly,
                )
                .with_symbol("main_control"),
            );

        let report = manifest
            .validate()
            .expect("excluded raw bootstrap symbols are audit evidence, not linked duplicates");

        assert_eq!(report.shared_tex_units, 1);
        assert_eq!(report.excluded_units, 1);
    }

    #[test]
    fn manifest_rejects_forbidden_symbols_in_included_units() {
        let manifest = Web2cSourceManifest::new().with_unit(
            Web2cSourceUnit::included("tex.web", Web2cSourceRole::SharedTexCore)
                .with_symbol("main_control")
                .with_symbol("ship_out"),
        );

        let error = manifest
            .validate()
            .expect_err("shipout entry point must not be linked");

        assert_eq!(
            error,
            Web2cManifestError::ForbiddenSymbol {
                symbol: "ship_out".into(),
                path: "tex.web".into(),
                reason: ExcludedWeb2cSource::ShipoutOrDriver,
            }
        );
    }

    #[test]
    fn manifest_rejects_concrete_generated_output_driver_symbols() {
        for symbol in ["zshipout", "hlistout", "vlistout", "zspecialout"] {
            let manifest = Web2cSourceManifest::new().with_unit(
                Web2cSourceUnit::included(
                    "generated/c2rust/tex/src/tex0.rs",
                    Web2cSourceRole::SharedTexCore,
                )
                .with_symbol("zhpack")
                .with_symbol(symbol),
            );

            let error = manifest
                .validate()
                .expect_err("generated output-driver symbol must not be linked");

            assert_eq!(
                error,
                Web2cManifestError::ForbiddenSymbol {
                    symbol: symbol.into(),
                    path: "generated/c2rust/tex/src/tex0.rs".into(),
                    reason: ExcludedWeb2cSource::ShipoutOrDriver,
                }
            );
        }
    }

    #[test]
    fn manifest_rejects_concrete_generated_native_io_symbols() {
        for symbol in ["open_input", "open_output", "runsystem", "openfmtfile"] {
            let manifest = Web2cSourceManifest::new().with_unit(
                Web2cSourceUnit::included(
                    "generated/c2rust/tex/src/tex0.rs",
                    Web2cSourceRole::SharedTexCore,
                )
                .with_symbol("zhpack")
                .with_symbol(symbol),
            );

            let error = manifest
                .validate()
                .expect_err("generated native IO symbol must not be linked");

            assert_eq!(
                error,
                Web2cManifestError::ForbiddenSymbol {
                    symbol: symbol.into(),
                    path: "generated/c2rust/tex/src/tex0.rs".into(),
                    reason: ExcludedWeb2cSource::NativeIo,
                }
            );
        }
    }

    #[test]
    fn manifest_allows_layout_symbols_needed_for_fragment_boxes() {
        let manifest = Web2cSourceManifest::new().with_unit(
            Web2cSourceUnit::included(
                "generated/c2rust/tex/src/tex0.rs",
                Web2cSourceRole::SharedTexCore,
            )
            .with_symbol("zhpack")
            .with_symbol("zvpackage")
            .with_symbol("zappendtovlist")
            .with_symbol("newnoad")
            .with_symbol("mlisttohlist"),
        );

        let report = manifest
            .validate()
            .expect("box/list/math layout symbols should remain portable-core candidates");

        assert_eq!(report.shared_tex_units, 1);
    }

    #[test]
    fn classifier_marks_layout_symbols_as_shared_core_candidates() {
        for symbol in [
            "zhpack",
            "zvpackage",
            "zappendtovlist",
            "newnoad",
            "mlisttohlist",
        ] {
            assert_eq!(
                classify_translated_symbol(symbol),
                TranslatedSymbolClass::SharedTexCoreCandidate
            );
        }
    }

    #[test]
    fn classifier_marks_xetex_symbols_as_runtime_patch_candidates() {
        for symbol in [
            "scanusvnum",
            "znewnativewordnode",
            "znewnativecharacter",
            "zloadnativefont",
            "zbuildopentypeassembly",
        ] {
            assert_eq!(
                classify_translated_symbol(symbol),
                TranslatedSymbolClass::XetexPatchCandidate
            );
        }
    }

    #[test]
    fn classifier_marks_native_and_output_symbols_as_forbidden() {
        assert_eq!(
            classify_translated_symbol("zshipout"),
            TranslatedSymbolClass::Forbidden(ExcludedWeb2cSource::ShipoutOrDriver)
        );
        assert_eq!(
            classify_translated_symbol("open_input"),
            TranslatedSymbolClass::Forbidden(ExcludedWeb2cSource::NativeIo)
        );
        assert_eq!(
            Web2cSourceManifest::new().classify_symbol("host_fopen_shim"),
            TranslatedSymbolClass::Unknown
        );
        assert_eq!(
            Web2cSourceManifest::new()
                .with_forbidden_symbol(Web2cForbiddenSymbol::new(
                    "host_fopen_shim",
                    ExcludedWeb2cSource::NativeIo,
                ))
                .classify_symbol("host_fopen_shim"),
            TranslatedSymbolClass::Forbidden(ExcludedWeb2cSource::NativeIo)
        );
    }

    #[test]
    fn classifier_marks_forbidden_source_paths_before_symbol_import() {
        assert_eq!(
            classify_web2c_source_path("texk/web2c/luatexdir/luatex.web"),
            Some(ExcludedWeb2cSource::Luatex)
        );
        assert_eq!(
            classify_web2c_source_path("libs/lua53/lua53-src/src/lvm.c"),
            Some(ExcludedWeb2cSource::LuaRuntime)
        );
        assert_eq!(
            classify_web2c_source_path("texk/web2c/callbacks.c"),
            Some(ExcludedWeb2cSource::CallbackOrNodeFilter)
        );
        assert_eq!(
            classify_web2c_source_path("texk/kpathsea/pathsearch.c"),
            Some(ExcludedWeb2cSource::NativeIo)
        );
        assert_eq!(classify_web2c_source_path("texk/web2c/tex.web"), None);
        assert_eq!(
            classify_web2c_source_path("texk/web2c/xetexdir/xetex.ch"),
            None
        );
    }

    #[test]
    fn audit_report_accepts_known_portable_layout_candidates() {
        let report = Web2cSourceManifest::new().audit_symbols([
            "zhpack",
            "zvpackage",
            "zappendtovlist",
            "newnoad",
            "mlisttohlist",
        ]);

        assert_eq!(report.total, 5);
        assert_eq!(report.shared_tex_core_candidates, 5);
        assert!(report.is_clear_for_portable_import());
    }

    #[test]
    fn audit_report_counts_patch_forbidden_and_unknown_symbols() {
        let report = Web2cSourceManifest::new().audit_symbols([
            "zhpack",
            "zloadnativefont",
            "zshipout",
            "open_input",
            "not_audited_yet",
        ]);

        assert_eq!(report.total, 5);
        assert_eq!(report.shared_tex_core_candidates, 1);
        assert_eq!(report.xetex_patch_candidates, 1);
        assert_eq!(report.shipout_or_driver_symbols, 1);
        assert_eq!(report.native_io_symbols, 1);
        assert_eq!(
            report.forbidden_symbols,
            alloc::vec!["zshipout", "open_input"]
        );
        assert_eq!(report.unknown_symbols, alloc::vec!["not_audited_yet"]);
        assert!(!report.is_clear_for_portable_import());
    }

    #[test]
    fn symbol_inventory_normalizes_extracted_bootstrap_symbols() {
        let inventory = TranslatedSymbolInventory::from_symbols(
            ["zvpackage", "zhpack", "zhpack"],
            ["zloadnativefont", "zhpack", "zloadnativefont"],
        );

        assert_eq!(
            inventory.tex_symbols(),
            &[String::from("zhpack"), String::from("zvpackage")]
        );
        assert_eq!(
            inventory.xetex_symbols(),
            &[String::from("zhpack"), String::from("zloadnativefont")]
        );
    }

    #[test]
    fn symbol_inventory_parses_audit_tsv() {
        let inventory = TranslatedSymbolInventory::parse_tsv(
            "engine\tsymbol\ntex\tzhpack\nxetex\tzhpack\nxetex\tscanusvnum\n",
        )
        .expect("valid audit inventory should parse");

        assert_eq!(inventory.tex_symbols(), &[String::from("zhpack")]);
        assert_eq!(
            inventory.xetex_symbols(),
            &[String::from("scanusvnum"), String::from("zhpack")]
        );
        assert_eq!(
            inventory.shared_duplicate_symbols(),
            alloc::vec![String::from("zhpack")]
        );
        assert_eq!(
            inventory.xetex_only_symbols(),
            alloc::vec![String::from("scanusvnum")]
        );
    }

    #[test]
    fn symbol_inventory_rejects_invalid_tsv() {
        assert_eq!(
            TranslatedSymbolInventory::parse_tsv("").expect_err("header should be required"),
            TranslatedSymbolInventoryError::MissingHeader
        );
        assert_eq!(
            TranslatedSymbolInventory::parse_tsv("engine,name\ntex\tzhpack\n")
                .expect_err("header must match the generated audit format"),
            TranslatedSymbolInventoryError::InvalidHeader {
                found: String::from("engine,name"),
            }
        );
        assert_eq!(
            TranslatedSymbolInventory::parse_tsv("engine\tsymbol\nluatex\tcallback_register\n")
                .expect_err("unknown engines must not be accepted"),
            TranslatedSymbolInventoryError::UnknownEngine {
                line: 2,
                engine: String::from("luatex"),
            }
        );
        assert_eq!(
            TranslatedSymbolInventory::parse_tsv("engine\tsymbol\ntex\n")
                .expect_err("rows must have exactly two columns"),
            TranslatedSymbolInventoryError::MalformedRow {
                line: 2,
                row: String::from("tex"),
            }
        );
        assert_eq!(
            TranslatedSymbolInventory::parse_tsv("engine\tsymbol\ntex\t\n")
                .expect_err("symbols must be non-empty"),
            TranslatedSymbolInventoryError::EmptySymbol { line: 2 }
        );
    }

    #[test]
    fn generated_symbol_inventory_matches_bootstrap_audit_counts() {
        let inventory = TranslatedSymbolInventory::parse_tsv(include_str!(
            "../../../generated/audit/symbols.tsv"
        ))
        .expect("checked generated inventory should parse");
        let report = inventory.audit(&Web2cSourceManifest::new());

        assert_eq!(report.tex_symbol_count, 333);
        assert_eq!(report.xetex_symbol_count, 460);
        assert_eq!(report.shared_duplicate_count(), 330);
        assert_eq!(report.xetex_only_count(), 130);
        assert!(report.proves_shared_tex_surface());
        assert!(!report.shared_surface.is_clear_for_portable_import());
        assert!(!report.xetex_patch_surface.is_clear_for_portable_import());
    }

    #[test]
    fn symbol_inventory_splits_shared_duplicates_from_xetex_only_symbols() {
        let inventory = TranslatedSymbolInventory::from_symbols(
            ["mlisttohlist", "plain_tex_only", "zhpack"],
            ["mlisttohlist", "scanusvnum", "zloadnativefont", "zhpack"],
        );

        assert_eq!(
            inventory.shared_duplicate_symbols(),
            alloc::vec![String::from("mlisttohlist"), String::from("zhpack")]
        );
        assert_eq!(
            inventory.xetex_only_symbols(),
            alloc::vec![String::from("scanusvnum"), String::from("zloadnativefont")]
        );
    }

    #[test]
    fn symbol_inventory_audits_shared_and_xetex_patch_surfaces() {
        let inventory = TranslatedSymbolInventory::from_symbols(
            ["mlisttohlist", "plain_tex_only", "zhpack"],
            [
                "mlisttohlist",
                "scanusvnum",
                "unknown_xetex_symbol",
                "zloadnativefont",
                "zhpack",
            ],
        );

        let report = inventory.audit(&Web2cSourceManifest::new());

        assert_eq!(report.tex_symbol_count, 3);
        assert_eq!(report.xetex_symbol_count, 5);
        assert_eq!(report.shared_duplicate_count(), 2);
        assert_eq!(report.xetex_only_count(), 3);
        assert!(report.proves_shared_tex_surface());
        assert_eq!(report.shared_surface.shared_tex_core_candidates, 2);
        assert!(report.shared_surface.is_clear_for_portable_import());
        assert_eq!(report.xetex_patch_surface.xetex_patch_candidates, 2);
        assert_eq!(
            report.xetex_patch_surface.unknown_symbols,
            alloc::vec!["unknown_xetex_symbol"]
        );
        assert!(!report.xetex_patch_surface.is_clear_for_portable_import());
    }

    #[test]
    fn portable_import_accepts_only_audited_shared_and_patch_symbols() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(
                Web2cSourceUnit::included(
                    "generated/c2rust/tex/src/tex0.rs",
                    Web2cSourceRole::SharedTexCore,
                )
                .with_symbol("zhpack")
                .with_symbol("zvpackage")
                .with_symbol("mlisttohlist"),
            )
            .with_unit(
                Web2cSourceUnit::included("xetexdir/xetex.ch", Web2cSourceRole::XetexPatch)
                    .with_symbol("zloadnativefont")
                    .with_symbol("znewnativewordnode"),
            );

        let report = manifest
            .validate_portable_import()
            .expect("known shared-core and xetex patch symbols should import");

        assert_eq!(report.shared_tex_units, 1);
        assert_eq!(report.xetex_patch_units, 1);
    }

    #[test]
    fn portable_import_rejects_unknown_symbols() {
        let manifest = Web2cSourceManifest::new().with_unit(
            Web2cSourceUnit::included(
                "generated/c2rust/tex/src/tex0.rs",
                Web2cSourceRole::SharedTexCore,
            )
            .with_symbol("zhpack")
            .with_symbol("not_audited_yet"),
        );

        let error = manifest
            .validate_portable_import()
            .expect_err("unknown translated symbols need review before import");

        assert_eq!(
            error,
            Web2cManifestError::UnknownSymbol {
                symbol: "not_audited_yet".into(),
                path: "generated/c2rust/tex/src/tex0.rs".into(),
            }
        );
    }

    #[test]
    fn portable_import_rejects_xetex_patch_symbols_in_shared_core_units() {
        let manifest = Web2cSourceManifest::new().with_unit(
            Web2cSourceUnit::included(
                "generated/c2rust/tex/src/tex0.rs",
                Web2cSourceRole::SharedTexCore,
            )
            .with_symbol("zhpack")
            .with_symbol("zloadnativefont"),
        );

        let error = manifest
            .validate_portable_import()
            .expect_err("xetex behavior must stay in runtime-gated patch units");

        assert_eq!(
            error,
            Web2cManifestError::SymbolRoleMismatch {
                symbol: "zloadnativefont".into(),
                path: "generated/c2rust/tex/src/tex0.rs".into(),
                role: Web2cSourceRole::SharedTexCore,
                expected: TranslatedSymbolClass::SharedTexCoreCandidate,
                actual: TranslatedSymbolClass::XetexPatchCandidate,
            }
        );
    }

    #[test]
    fn portable_import_rejects_shared_symbols_in_xetex_patch_units() {
        let manifest = Web2cSourceManifest::new()
            .with_unit(
                Web2cSourceUnit::included(
                    "generated/c2rust/tex/src/tex0.rs",
                    Web2cSourceRole::SharedTexCore,
                )
                .with_symbol("zhpack"),
            )
            .with_unit(
                Web2cSourceUnit::included("xetexdir/xetex.ch", Web2cSourceRole::XetexPatch)
                    .with_symbol("zvpackage"),
            );

        let error = manifest
            .validate_portable_import()
            .expect_err("shared TeX behavior must not be duplicated into xetex patch units");

        assert_eq!(
            error,
            Web2cManifestError::SymbolRoleMismatch {
                symbol: "zvpackage".into(),
                path: "xetexdir/xetex.ch".into(),
                role: Web2cSourceRole::XetexPatch,
                expected: TranslatedSymbolClass::XetexPatchCandidate,
                actual: TranslatedSymbolClass::SharedTexCoreCandidate,
            }
        );
    }

    #[test]
    fn portable_import_rejects_forbidden_symbols_before_unknown_symbols() {
        let manifest = Web2cSourceManifest::new().with_unit(
            Web2cSourceUnit::included(
                "generated/c2rust/tex/src/tex0.rs",
                Web2cSourceRole::SharedTexCore,
            )
            .with_symbol("not_audited_yet")
            .with_symbol("zshipout"),
        );

        let error = manifest
            .validate_portable_import()
            .expect_err("forbidden symbols are hard removal targets");

        assert_eq!(
            error,
            Web2cManifestError::ForbiddenSymbol {
                symbol: "zshipout".into(),
                path: "generated/c2rust/tex/src/tex0.rs".into(),
                reason: ExcludedWeb2cSource::ShipoutOrDriver,
            }
        );
    }

    #[test]
    fn manifest_accepts_custom_forbidden_symbol_rules() {
        let manifest = Web2cSourceManifest::new()
            .with_forbidden_symbol(Web2cForbiddenSymbol::new(
                "host_fopen_shim",
                ExcludedWeb2cSource::NativeIo,
            ))
            .with_unit(
                Web2cSourceUnit::included("tex-io.c", Web2cSourceRole::SharedTexCore)
                    .with_symbol("host_fopen_shim"),
            );

        let error = manifest
            .validate()
            .expect_err("custom native-io shim must not be linked");

        assert_eq!(
            error,
            Web2cManifestError::ForbiddenSymbol {
                symbol: "host_fopen_shim".into(),
                path: "tex-io.c".into(),
                reason: ExcludedWeb2cSource::NativeIo,
            }
        );
    }

    #[test]
    fn texlive_recipe_maps_xetex_to_runtime_patch_units() {
        let recipe = texlive_bootstrap_recipe();
        let manifest = recipe.as_source_manifest();

        let tex_units = manifest
            .included_for(EngineKind::Tex)
            .map(|unit| unit.path.as_str())
            .collect::<Vec<_>>();
        let xetex_units = manifest
            .included_for(EngineKind::Xetex)
            .map(|unit| unit.path.as_str())
            .collect::<Vec<_>>();
        let report = manifest
            .validate()
            .expect("recipe manifest should validate");

        assert!(tex_units.contains(&"tex.web"));
        assert!(tex_units.contains(&"tex.ch"));
        assert!(!tex_units.iter().any(|unit| unit.contains("xetex")));
        assert!(xetex_units.contains(&"tex.web"));
        assert!(xetex_units.contains(&"xetexdir/xetex.ch"));
        assert!(xetex_units.contains(&"xetexdir/char-warning-xetex.ch"));
        assert_eq!(report.shared_tex_units, 3);
        assert_eq!(report.xetex_patch_units, recipe.xetex_patch_changes.len());
        assert_eq!(report.excluded_units, recipe.bootstrap_only_outputs.len());
    }

    #[test]
    fn texlive_recipe_keeps_raw_xetex_outputs_bootstrap_only() {
        let recipe = texlive_bootstrap_recipe();
        let manifest = recipe.as_source_manifest();

        for output in recipe.bootstrap_only_outputs {
            let unit = manifest
                .units()
                .iter()
                .find(|unit| unit.path == *output)
                .expect("bootstrap-only output should be represented");

            assert!(!unit.include);
            assert_eq!(
                unit.role,
                Web2cSourceRole::Excluded(ExcludedWeb2cSource::BootstrapOnly)
            );
        }
    }

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root is two levels above the crate manifest")
            .to_path_buf()
    }

    #[test]
    fn real_input_classifies_without_unknown_and_imports_all_shared() {
        let records = crate::orchestrate::collect_shared_comparison_records(&repo_root())
            .expect("real C2Rust input must import without fail-loud errors");

        // 411 is stock shared functions plus the changes/src-tracking.ch marker.
        assert_eq!(records.len(), 411);

        let unknown = records
            .iter()
            .filter(|record| record.diff == crate::compare::BodyDiff::Unknown)
            .count();
        assert_eq!(unknown, 0, "real input must yield zero Unknown classifications");
    }

    #[test]
    fn import_profiles_derive_from_recipe() {
        use crate::profile::EngineImportProfile;

        let recipe = texlive_bootstrap_recipe();
        let profiles = EngineImportProfile::all_from_recipe();
        assert_eq!(profiles.len(), 3);

        let xetex = profiles
            .iter()
            .find(|profile| profile.id == "xetex")
            .expect("xetex profile present");

        // The source chain includes all patch change files from the recipe.
        assert!(xetex.source_chain.contains(&recipe.shared_tex_web));
        for change_file in recipe.xetex_patch_changes {
            assert!(
                xetex.source_chain.contains(change_file),
                "xetex source chain should include patch change {change_file}"
            );
        }
        assert!(xetex.capabilities.unicode_scalars);
        assert!(xetex.capabilities.unicode_math);
        assert!(xetex.allows_boundary("getnativecharwd"));

        let tex = profiles
            .iter()
            .find(|profile| profile.id == "tex")
            .expect("tex profile present");
        assert!(!tex.capabilities.etex);
        assert!(tex.allowed_boundaries.is_empty());
    }
}

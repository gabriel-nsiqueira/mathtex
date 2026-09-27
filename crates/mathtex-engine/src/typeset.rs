use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use mathtex_font::{FontError, FontLoader};
use mathtex_ir::{ByteSpan, Fragment, FragmentKind, FragmentMetadata};
use mathtex_portable_engine_generated as pe;

use crate::adapter::{FontTable, HostBoxLog, HostBoxPlatform, LoaderFiles};
use crate::format::Format;
use crate::host_box::HostBoxes;
use crate::lower::{lower, LowerError};

/// Source name of the fragment in the fragment's source map, its spans index the caller's `tex`.
pub const FRAGMENT_SOURCE: &str = "input";

/// Closes the wrapper math, the leading newline keeps a trailing `%` in the fragment from eating it.
const SUFFIX: &str = "\n$}\\csname @@end\\endcsname\\end";

/// Reports lost characters as warnings unless the format already traces or rejects them.
const TRACE_LOST_CHARS: &str = r"\ifnum\tracinglostchars<1 \tracinglostchars=1 \fi";

/// Math mode a fragment is typeset in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MathMode {
    /// Text style math, as between `$` signs.
    Inline,
    /// Display style math, set in a box of its natural width rather than a display.
    Display,
}

impl MathMode {
    fn prefix(self) -> String {
        let style = match self {
            Self::Inline => "",
            Self::Display => r"\displaystyle ",
        };
        format!(r"{TRACE_LOST_CHARS}\hbox{{${style}")
    }

    fn fragment_kind(self) -> FragmentKind {
        match self {
            Self::Inline => FragmentKind::MathInline,
            Self::Display => FragmentKind::MathDisplay,
        }
    }
}

/// Limits and caching of a [`Typesetter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    /// Commands, macro calls and expansions one fragment may run before it fails with [`TypesetError::Budget`].
    pub op_budget: u64,
    /// IR nodes one fragment may lower to before it fails with [`TypesetError::NodeLimit`].
    pub max_nodes: usize,
    /// Fragments the cache keeps, 0 turns caching off.
    pub cache_capacity: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            op_budget: pe::SANDBOX_OP_BUDGET,
            max_nodes: 1 << 20,
            cache_capacity: 64,
        }
    }
}

impl Options {
    /// Sets the per fragment work budget.
    #[must_use]
    pub fn with_op_budget(mut self, op_budget: u64) -> Self {
        self.op_budget = op_budget;
        self
    }

    /// Sets the per fragment IR node cap.
    #[must_use]
    pub fn with_max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = max_nodes;
        self
    }

    /// Sets how many fragments the cache keeps.
    #[must_use]
    pub fn with_cache_capacity(mut self, cache_capacity: usize) -> Self {
        self.cache_capacity = cache_capacity;
        self
    }
}

/// A typeset fragment and the warnings TeX and the host boxes raised.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Typeset {
    /// The laid out fragment, its [`FRAGMENT_SOURCE`] spans are byte offsets into the caller's `tex`.
    pub fragment: Fragment,
    /// Host box warnings, then TeX's warnings in the order TeX printed them.
    pub warnings: Vec<Diagnostic>,
}

/// A warning raised while typesetting.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// What kind of warning it is.
    pub kind: DiagnosticKind,
    /// The warning as TeX or the typesetter wrote it.
    pub message: String,
}

/// Kind of a [`Diagnostic`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// A box is wider or taller than its set size.
    OverfullBox,
    /// A font has no glyph for a character, which is dropped.
    MissingCharacter,
    /// LaTeX substituted a font for one it could not find.
    FontSubstitution,
    /// A host box was missing or rejected and nothing was drawn for it.
    HostBox,
}

/// Why a fragment could not be typeset.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypesetError {
    /// TeX reported an error, which ends the run.
    Tex {
        /// TeX's error message, such as `Undefined control sequence`.
        message: String,
        /// 1 based line of the caller's `tex` TeX was reading, `None` past its end.
        line: Option<u32>,
        /// Byte span of the caller's `tex` holding the token TeX read last.
        span: Option<ByteSpan>,
    },
    /// The sandbox rejected a command, such as a math shift that leaves the math or a file access.
    Sandbox {
        /// What was rejected.
        message: String,
        /// Byte span of the caller's `tex` holding the rejected token.
        span: Option<ByteSpan>,
    },
    /// The fragment ran past [`Options::op_budget`], it is too complex or does not terminate.
    Budget,
    /// The fragment lowers to more than [`Options::max_nodes`] IR nodes.
    NodeLimit {
        /// The cap that was hit.
        limit: usize,
    },
    /// A font the format needs could not be loaded.
    Font {
        /// The font's `\font` spec.
        spec: String,
        /// Why it could not be loaded.
        error: FontError,
    },
    /// The format's state cannot run fragments.
    Format {
        /// Why it cannot.
        message: String,
    },
    /// The fragment is longer than a span can index.
    TooLong,
    /// The run finished without building the fragment's box.
    NoOutput,
    /// The laid out fragment broke an IR invariant, a bug in the lowering.
    Lowering {
        /// The broken invariant.
        message: String,
    },
}

impl fmt::Display for TypesetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tex {
                message,
                line: Some(line),
                ..
            } => write!(f, "TeX error on line {line}: {message}"),
            Self::Tex { message, .. } => write!(f, "TeX error: {message}"),
            Self::Sandbox { message, .. } => f.write_str(message),
            Self::Budget => f.write_str("expression is too complex or did not terminate"),
            Self::NodeLimit { limit } => {
                write!(f, "expression lays out to more than {limit} nodes")
            }
            Self::Font { spec, error } => write!(f, "font {spec}: {error}"),
            Self::Format { message } => write!(f, "format cannot typeset: {message}"),
            Self::TooLong => f.write_str("expression is too long"),
            Self::NoOutput => f.write_str("expression produced no box"),
            Self::Lowering { message } => write!(f, "layout lowering failed: {message}"),
        }
    }
}

impl std::error::Error for TypesetError {}

/// Typesets math fragments against one format, keeping the fonts it loaded across fragments.
pub struct Typesetter<L: FontLoader> {
    format: Format,
    loader: L,
    fonts: FontTable,
    options: Options,
    cache: Cache,
}

impl<L: FontLoader> Typesetter<L> {
    /// Rebinds the format's native fonts through `fonts`, failing when one cannot be loaded.
    pub fn new(format: Format, fonts: L, options: Options) -> Result<Self, TypesetError> {
        let mut table = FontTable::default();
        table
            .restore(&fonts, &format.fonts)
            .map_err(|(spec, error)| TypesetError::Font { spec, error })?;
        Ok(Self {
            format,
            loader: fonts,
            fonts: table,
            options,
            cache: Cache::default(),
        })
    }

    /// Typesets `tex` in a sandbox, `\hostbox{token}` asks `boxes` for its box.
    pub fn typeset(
        &mut self,
        tex: &str,
        mode: MathMode,
        boxes: &dyn HostBoxes,
    ) -> Result<Typeset, TypesetError> {
        if let Some(hit) = self.cache.get(tex, mode, boxes) {
            return Ok(hit);
        }
        let log = RefCell::new(HostBoxLog::default());
        let result = self.run(tex, mode, boxes, &log);
        // Fonts a fragment loaded are unknown to the next fragment's fresh state.
        self.fonts.keep_only(&self.format.fonts);
        let typeset = result?;
        let log = log.into_inner();
        if self.options.cache_capacity > 0 && !log.invalid_token {
            self.cache.insert(
                tex,
                mode,
                &typeset,
                &log.tokens,
                boxes,
                self.options.cache_capacity,
            );
        }
        Ok(typeset)
    }

    /// The font loader, which hosts use to draw the faces a fragment's runs name.
    #[must_use]
    pub fn fonts(&self) -> &L {
        &self.loader
    }

    /// Drops every cached fragment.
    pub fn clear_cache(&mut self) {
        self.cache = Cache::default();
    }

    fn run(
        &mut self,
        tex: &str,
        mode: MathMode,
        boxes: &dyn HostBoxes,
        log: &RefCell<HostBoxLog>,
    ) -> Result<Typeset, TypesetError> {
        let prefix = mode.prefix();
        let body = u32::try_from(prefix.len()).ok();
        let suffix = u32::try_from(prefix.len() + tex.len()).ok();
        let (Some(body), Some(suffix)) = (body, suffix) else {
            return Err(TypesetError::TooLong);
        };
        let wrapped = Wrapped { tex, body };
        let mut engine =
            pe::PortableTexEngine::from_format(&self.format.image, LoaderFiles(&self.loader))
                .with_font_platform(self.fonts.platform(&self.loader))
                .with_platform(HostBoxPlatform { boxes, log });
        let source = format!("{prefix}{tex}{SUFFIX}");
        if !engine.begin_primary_input(FRAGMENT_SOURCE, source.into_bytes()) {
            return Err(TypesetError::Format {
                message: engine
                    .last_error_message()
                    .unwrap_or("the fragment input was refused")
                    .into(),
            });
        }
        engine.set_sandbox(true);
        engine.set_sandbox_op_budget(self.options.op_budget);
        engine.set_sandbox_wrapper_suffix(Some(suffix));
        engine.set_source_tracking(true);
        engine.begin_fragment_capture();
        let ran = engine.run_main_control();
        engine.end_fragment_capture();
        if !ran {
            return Err(wrapped.run_error(&engine));
        }
        let root = engine
            .captured_fragment_root()
            .ok_or(TypesetError::NoOutput)?;
        let metadata = FragmentMetadata {
            format_id: String::new(),
            fragment_kind: mode.fragment_kind(),
        };
        let mut fragment = lower(&engine, root, metadata, self.options.max_nodes).map_err(
            |error| match error {
                LowerError::NodeLimit { limit } => TypesetError::NodeLimit { limit },
                LowerError::UnreadableRoot => TypesetError::NoOutput,
                error => TypesetError::Lowering {
                    message: error.to_string(),
                },
            },
        )?;
        wrapped.rebase(&mut fragment);
        let mut warnings = log.borrow_mut().warnings.split_off(0);
        warnings.extend(transcript_warnings(engine.transcript_bytes()));
        Ok(Typeset { fragment, warnings })
    }
}

impl<L: FontLoader> fmt::Debug for Typesetter<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Typesetter")
            .field("format", &self.format)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

/// The caller's `tex` inside the wrapper, whose bytes start at offset `body` of the primary input.
struct Wrapped<'a> {
    tex: &'a str,
    body: u32,
}

impl Wrapped<'_> {
    /// A wrapper span as a span of the caller's `tex`, `None` unless it lies wholly inside it.
    fn inside(&self, span: ByteSpan) -> Option<ByteSpan> {
        let end = self.body + self.tex.len() as u32;
        (span.start >= self.body && span.start <= span.end && span.end <= end).then(|| ByteSpan {
            start: span.start - self.body,
            end: span.end - self.body,
        })
    }

    /// Moves the fragment's spans into the caller's `tex`, dropping spans of the wrapper and other files.
    fn rebase(&self, fragment: &mut Fragment) {
        let input = fragment
            .source_map
            .sources
            .iter()
            .find(|source| source.name == FRAGMENT_SOURCE)
            .map(|source| source.id);
        for node in &mut fragment.nodes {
            node.primary_source = node.primary_source.and_then(|mut range| {
                range.span = self
                    .inside(range.span)
                    .filter(|_| Some(range.source) == input)?;
                Some(range)
            });
            if let mathtex_ir::LayoutNodeKind::GlyphRun(run) = &mut node.kind {
                let keep = node.primary_source.is_some();
                for glyph in &mut run.glyphs {
                    glyph.cluster = glyph
                        .cluster
                        .and_then(|span| self.inside(span))
                        .filter(|_| keep);
                }
            }
        }
        fragment
            .source_map
            .entries
            .retain_mut(|entry| match self.inside(entry.range.span) {
                Some(span) if Some(entry.range.source) == input => {
                    entry.range.span = span;
                    true
                }
                _ => false,
            });
    }

    /// The error a failed run surfaced, with its line and span in the caller's `tex`.
    fn run_error(&self, engine: &pe::PortableTexEngine<'_>) -> TypesetError {
        let Some(error) = engine.last_error() else {
            let message = match engine.last_abort_status() {
                Some(status) => format!("TeX stopped with status {status}"),
                None => "TeX stopped".into(),
            };
            return TypesetError::Tex {
                message,
                line: None,
                span: None,
            };
        };
        // An error span may reach into the wrapper, such as a group the fragment left open.
        let span = error
            .span
            .as_ref()
            .filter(|span| span.name == FRAGMENT_SOURCE)
            .and_then(|span| {
                let end = self.body + self.tex.len() as u32;
                let (start, stop) = (span.start.max(self.body), span.end.min(end));
                (start <= stop && span.start <= end).then(|| ByteSpan {
                    start: start - self.body,
                    end: stop - self.body,
                })
            });
        let lines = self.tex.split('\n').count();
        let line = u32::try_from(error.line)
            .ok()
            .filter(|&line| line >= 1 && line as usize <= lines);
        let message = error.message.clone();
        match error.kind {
            pe::PortableErrorKind::Budget => TypesetError::Budget,
            pe::PortableErrorKind::Sandbox => TypesetError::Sandbox { message, span },
            pe::PortableErrorKind::Tex => TypesetError::Tex {
                message,
                line,
                span,
            },
        }
    }
}

/// Warnings TeX printed: overfull boxes, missing characters and LaTeX font substitutions.
fn transcript_warnings(transcript: &[u8]) -> Vec<Diagnostic> {
    let transcript = String::from_utf8_lossy(transcript);
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut continues_font_warning = false;
    for line in transcript.lines() {
        let line = line.trim_end();
        if continues_font_warning {
            if let Some(more) = line.strip_prefix("(Font)") {
                if let Some(last) = warnings.last_mut() {
                    last.message.push(' ');
                    last.message.push_str(more.trim());
                }
                continue;
            }
        }
        continues_font_warning = false;
        let kind = if line.starts_with("Overfull \\hbox") || line.starts_with("Overfull \\vbox") {
            DiagnosticKind::OverfullBox
        } else if line.starts_with("Missing character: ") {
            DiagnosticKind::MissingCharacter
        } else if line.starts_with("LaTeX Font Warning: ") {
            continues_font_warning = true;
            DiagnosticKind::FontSubstitution
        } else {
            continue;
        };
        warnings.push(Diagnostic {
            kind,
            message: line.to_string(),
        });
    }
    warnings
}

/// Fragments by mode and source, each valid while the host boxes it used keep their revisions.
#[derive(Default)]
struct Cache {
    tick: u64,
    entries: [HashMap<String, CacheEntry>; 2],
}

struct CacheEntry {
    typeset: Typeset,
    revisions: Vec<(u32, u64)>,
    used: u64,
}

impl Cache {
    fn slot(&mut self, mode: MathMode) -> &mut HashMap<String, CacheEntry> {
        match mode {
            MathMode::Inline => &mut self.entries[0],
            MathMode::Display => &mut self.entries[1],
        }
    }

    fn get(&mut self, tex: &str, mode: MathMode, boxes: &dyn HostBoxes) -> Option<Typeset> {
        self.tick += 1;
        let tick = self.tick;
        let slot = self.slot(mode);
        let entry = slot.get_mut(tex)?;
        let fresh = entry
            .revisions
            .iter()
            .all(|&(token, revision)| boxes.revision(token) == Some(revision));
        if !fresh {
            slot.remove(tex);
            return None;
        }
        entry.used = tick;
        Some(entry.typeset.clone())
    }

    fn insert(
        &mut self,
        tex: &str,
        mode: MathMode,
        typeset: &Typeset,
        tokens: &[u32],
        boxes: &dyn HostBoxes,
        capacity: usize,
    ) {
        let mut revisions = Vec::new();
        for &token in tokens {
            if revisions.iter().any(|&(seen, _)| seen == token) {
                continue;
            }
            let Some(revision) = boxes.revision(token) else {
                return;
            };
            revisions.push((token, revision));
        }
        let len = self.entries.iter().map(HashMap::len).sum::<usize>();
        if len >= capacity {
            self.evict_least_recent();
        }
        let used = self.tick;
        self.slot(mode).insert(
            tex.to_string(),
            CacheEntry {
                typeset: typeset.clone(),
                revisions,
                used,
            },
        );
    }

    fn evict_least_recent(&mut self) {
        let oldest = self
            .entries
            .iter()
            .enumerate()
            .flat_map(|(slot, entries)| {
                entries
                    .iter()
                    .map(move |(tex, entry)| (entry.used, slot, tex.clone()))
            })
            .min();
        if let Some((_, slot, tex)) = oldest {
            self.entries[slot].remove(&tex);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_warnings_pick_out_overfull_boxes_lost_characters_and_font_substitutions() {
        let transcript = concat!(
            "Overfull \\hbox (1.0pt too wide) detected at line 1\n",
            "Missing character: There is no x in font nullfont!\n",
            "LaTeX Font Warning: Font shape `TU/lmr/m/sc' undefined\n",
            "(Font)              using `TU/lmr/m/n' instead on input line 1.\n",
            "Underfull \\hbox (badness 10000) detected at line 1\n",
        );
        let warnings = transcript_warnings(transcript.as_bytes());
        let kinds = warnings.iter().map(|w| w.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                DiagnosticKind::OverfullBox,
                DiagnosticKind::MissingCharacter,
                DiagnosticKind::FontSubstitution
            ]
        );
        assert!(warnings[2]
            .message
            .ends_with("using `TU/lmr/m/n' instead on input line 1."));
    }
}

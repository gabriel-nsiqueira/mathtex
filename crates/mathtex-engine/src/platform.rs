use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Host runtime services that must remain separate from engine semantics.
pub trait Platform {
    /// Returns the diagnostic sink for this platform.
    fn diagnostics(&self) -> &dyn DiagnosticSink;

    /// Returns host resource limits, defaulting to `HostLimits::default()`.
    fn limits(&self) -> HostLimits {
        HostLimits::default()
    }

    /// Deterministic clock value visible to generated TeX code.
    fn clock(&self) -> HostClock {
        HostClock::default()
    }

    /// Called before the engine begins line break iteration for a paragraph.
    fn linebreak_start(&self, _request: LinebreakRequest<'_>) {}

    /// Returns the next line break position, or `None` to use the built in algorithm.
    fn linebreak_next(&self) -> Option<i32> {
        None
    }

    /// Returns the box for a host owned token, or `None` to typeset a zero size box.
    fn host_box(&self, _request: HostBoxRequest) -> Option<HostBox> {
        None
    }
}

impl<T> Platform for &T
where
    T: Platform,
{
    fn diagnostics(&self) -> &dyn DiagnosticSink {
        (*self).diagnostics()
    }

    fn limits(&self) -> HostLimits {
        (*self).limits()
    }

    fn clock(&self) -> HostClock {
        (*self).clock()
    }

    fn linebreak_start(&self, request: LinebreakRequest<'_>) {
        (*self).linebreak_start(request);
    }

    fn linebreak_next(&self) -> Option<i32> {
        (*self).linebreak_next()
    }

    fn host_box(&self, request: HostBoxRequest) -> Option<HostBox> {
        (*self).host_box(request)
    }
}

/// Receiver for diagnostic messages emitted during engine execution.
pub trait DiagnosticSink {
    /// Accepts and handles a single diagnostic message.
    fn emit(&self, diagnostic: Diagnostic);
}

/// A single diagnostic message emitted by the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity level of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Human readable diagnostic text.
    pub message: String,
}

/// Severity level of a diagnostic message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Informational message, no action required.
    Info,
    /// Warning that may indicate a problem but does not stop rendering.
    Warning,
    /// Error that prevented successful rendering.
    Error,
}

/// Host imposed upper bounds on engine resource consumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostLimits {
    /// Maximum input bytes accepted for one fragment.
    pub max_input_bytes: usize,
    /// Maximum resource requests made while executing one fragment.
    pub max_resource_requests: usize,
    /// Maximum layout nodes an engine session should emit.
    pub max_layout_nodes: usize,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1 << 20,
            max_resource_requests: 256,
            max_layout_nodes: 1 << 20,
        }
    }
}

/// Host clock value in seconds and microseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostClock {
    /// Seconds since the host defined epoch.
    pub seconds: i32,
    /// Microseconds within the current second.
    pub micros: i32,
}

/// Parameters for a host driven line break request passed to the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinebreakRequest<'a> {
    /// Generated engine font identifier active for the text.
    pub font: i32,
    /// Generated engine locale identifier active for the text.
    pub locale: i32,
    /// Text slice owned by generated engine memory for this call.
    pub text: &'a [u16],
}

/// Style context a host box is placed in, display math collapses to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostBoxStyle {
    /// Text and display math style.
    Text,
    /// First level script style.
    Script,
    /// Second level script style.
    ScriptScript,
}

/// Parameters for a host box request passed to the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostBoxRequest {
    /// Opaque host owned token identifier scanned from `\hostbox{...}`.
    pub token: i32,
    /// Math style context at the placement site.
    pub style: HostBoxStyle,
    /// Font size context in scaled points at the placement site.
    pub font_size: i32,
}

/// A glyph inside a host box, offsets in scaled points from the box baseline origin, y down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostBoxGlyph {
    /// Glyph identifier in the run's font.
    pub glyph: u16,
    /// Horizontal offset from the box baseline origin.
    pub x: i32,
    /// Vertical offset from the baseline, positive downward.
    pub y: i32,
    /// Horizontal advance in scaled points.
    pub advance: i32,
}

/// One same font glyph sequence inside a host box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostBoxRun {
    /// Host font name carried into the fragment's glyph run.
    pub font_name: String,
    /// Font size for the run in scaled points.
    pub font_size: i32,
    /// Glyphs of the run in visual order.
    pub glyphs: Vec<HostBoxGlyph>,
}

/// A rule inside a host box, `(x, y)` is the bottom left corner, y down from the baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostBoxRule {
    /// Horizontal offset of the rule's left edge from the box baseline origin.
    pub x: i32,
    /// Vertical offset of the rule's bottom edge, positive downward.
    pub y: i32,
    /// Rule width in scaled points.
    pub width: i32,
    /// Rule height in scaled points, extending upward from `y`.
    pub height: i32,
}

/// Host supplied box for one `\hostbox` token: metrics plus render data for the fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostBox {
    /// Box width in scaled points.
    pub width: i32,
    /// Box height above the baseline in scaled points.
    pub height: i32,
    /// Box depth below the baseline in scaled points.
    pub depth: i32,
    /// Glyph runs rendered inside the box.
    pub runs: Vec<HostBoxRun>,
    /// Rules rendered inside the box.
    pub rules: Vec<HostBoxRule>,
}

/// Signals that a host configured resource limit was exceeded.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitError {
    /// Input byte count exceeded the configured maximum.
    InputTooLarge {
        /// Actual byte count seen.
        actual: usize,
        /// Configured byte limit.
        limit: usize,
    },
    /// Resource request count exceeded the configured maximum.
    TooManyResourceRequests {
        /// Actual request count.
        actual: usize,
        /// Configured request limit.
        limit: usize,
    },
    /// Layout node count exceeded the configured maximum.
    TooManyLayoutNodes {
        /// Actual node count.
        actual: usize,
        /// Configured node limit.
        limit: usize,
    },
}

/// Diagnostic sink that discards all messages.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopDiagnosticSink;

impl DiagnosticSink for NoopDiagnosticSink {
    fn emit(&self, _diagnostic: Diagnostic) {}
}

/// Diagnostic sink that accumulates messages, usable without std.
#[derive(Debug, Default)]
pub struct CollectingDiagnosticSink {
    diagnostics: RefCell<Vec<Diagnostic>>,
}

impl CollectingDiagnosticSink {
    /// Creates an empty collecting sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a copy of all accumulated diagnostics without consuming them.
    #[must_use]
    pub fn snapshot(&self) -> Vec<Diagnostic> {
        self.diagnostics.borrow().clone()
    }

    /// Removes and returns all accumulated diagnostics.
    #[must_use]
    pub fn drain(&self) -> Vec<Diagnostic> {
        self.diagnostics.borrow_mut().drain(..).collect()
    }

    /// Returns the number of accumulated diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.diagnostics.borrow().len()
    }

    /// Returns true when no diagnostics have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.borrow().is_empty()
    }

    /// Discards all accumulated diagnostics.
    pub fn clear(&self) {
        self.diagnostics.borrow_mut().clear();
    }
}

impl DiagnosticSink for CollectingDiagnosticSink {
    fn emit(&self, diagnostic: Diagnostic) {
        self.diagnostics.borrow_mut().push(diagnostic);
    }
}

/// Platform implementation composed from a diagnostic sink, host limits, and a clock.
#[derive(Clone, Debug)]
pub struct ConfigurablePlatform<D> {
    diagnostics: D,
    limits: HostLimits,
    clock: HostClock,
}

impl<D> ConfigurablePlatform<D> {
    /// Creates a platform with the given diagnostic sink and limits.
    #[must_use]
    pub fn new(diagnostics: D, limits: HostLimits) -> Self {
        Self {
            diagnostics,
            limits,
            clock: HostClock::default(),
        }
    }

    /// Creates a platform with the given diagnostic sink and default limits.
    #[must_use]
    pub fn with_diagnostics(diagnostics: D) -> Self {
        Self {
            diagnostics,
            limits: HostLimits::default(),
            clock: HostClock::default(),
        }
    }

    /// Returns a copy of this platform with the clock set to `clock`.
    #[must_use]
    pub fn with_clock(mut self, clock: HostClock) -> Self {
        self.clock = clock;
        self
    }

    /// Returns a reference to the underlying diagnostic sink.
    #[must_use]
    pub fn diagnostic_sink(&self) -> &D {
        &self.diagnostics
    }

    /// Returns the configured host limits.
    #[must_use]
    pub fn host_limits(&self) -> HostLimits {
        self.limits
    }

    /// Returns the configured host clock.
    #[must_use]
    pub fn host_clock(&self) -> HostClock {
        self.clock
    }
}

impl<D> Platform for ConfigurablePlatform<D>
where
    D: DiagnosticSink,
{
    fn diagnostics(&self) -> &dyn DiagnosticSink {
        &self.diagnostics
    }

    fn limits(&self) -> HostLimits {
        self.limits
    }

    fn clock(&self) -> HostClock {
        self.clock
    }
}

/// Minimal deterministic platform for tests, embedded use, and early bootstrap.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopPlatform {
    diagnostics: NoopDiagnosticSink,
    limits: HostLimits,
    clock: HostClock,
}

impl NoopPlatform {
    /// Creates a noop platform with the given host limits.
    #[must_use]
    pub fn with_limits(limits: HostLimits) -> Self {
        Self {
            diagnostics: NoopDiagnosticSink,
            limits,
            clock: HostClock::default(),
        }
    }

    /// Creates a noop platform with the given clock value.
    #[must_use]
    pub fn with_clock(clock: HostClock) -> Self {
        Self {
            diagnostics: NoopDiagnosticSink,
            limits: HostLimits::default(),
            clock,
        }
    }
}

impl Platform for NoopPlatform {
    fn diagnostics(&self) -> &dyn DiagnosticSink {
        &self.diagnostics
    }

    fn limits(&self) -> HostLimits {
        self.limits
    }

    fn clock(&self) -> HostClock {
        self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collecting_diagnostic_sink_records_snapshots_and_drains() {
        let sink = CollectingDiagnosticSink::new();

        sink.emit(Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "missing glyph".to_string(),
        });

        assert_eq!(sink.len(), 1);
        assert_eq!(sink.snapshot()[0].message, "missing glyph");
        assert_eq!(sink.drain()[0].severity, DiagnosticSeverity::Warning);
        assert!(sink.is_empty());
    }

}

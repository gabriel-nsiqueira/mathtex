use mathtex_ir::{FontKey, GlyphId, Length, Point};

/// Answers `\hostbox{token}` with a box the host lays out, such as a nested editor field.
pub trait HostBoxes {
    /// The box for one placement of `request.token`, `None` renders nothing and emits a warning.
    fn host_box(&self, request: &HostBoxRequest) -> Option<HostBox>;

    /// A value that changes whenever the boxes for `token` change, `None` makes fragments using it uncachable.
    fn revision(&self, _token: u32) -> Option<u64> {
        None
    }
}

impl<T> HostBoxes for &T
where
    T: HostBoxes + ?Sized,
{
    fn host_box(&self, request: &HostBoxRequest) -> Option<HostBox> {
        (**self).host_box(request)
    }

    fn revision(&self, token: u32) -> Option<u64> {
        (**self).revision(token)
    }
}

/// Host boxes of a host that has none, every `\hostbox` renders nothing and emits a warning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoHostBoxes;

impl HostBoxes for NoHostBoxes {
    fn host_box(&self, _request: &HostBoxRequest) -> Option<HostBox> {
        None
    }

    fn revision(&self, _token: u32) -> Option<u64> {
        Some(0)
    }
}

/// Math style at a host box's placement, display style reports as text as TeX's size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MathStyle {
    /// Text or display style, the text size.
    Text,
    /// Script style.
    Script,
    /// Second level script style.
    ScriptScript,
}

/// One placement of `\hostbox{token}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostBoxRequest {
    /// The number the fragment wrote in `\hostbox{...}`.
    pub token: u32,
    /// Math style at the placement, text outside math.
    pub style: MathStyle,
    /// Size of the current font at the placement.
    pub size: Length,
}

impl HostBoxRequest {
    /// A request for `token` placed in `style` at font size `size`.
    #[must_use]
    pub fn new(token: u32, style: MathStyle, size: Length) -> Self {
        Self { token, style, size }
    }
}

/// A host laid out box, its runs and rules drawn relative to its baseline origin with y down.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostBox {
    /// Width, which TeX spaces the box by.
    pub width: Length,
    /// Height above the baseline.
    pub height: Length,
    /// Depth below the baseline.
    pub depth: Length,
    /// Glyph runs drawn inside the box.
    pub runs: Vec<HostBoxRun>,
    /// Rules drawn inside the box.
    pub rules: Vec<HostBoxRule>,
}

impl HostBox {
    /// An empty box of the given size, a negative size is rejected with a warning when typeset.
    #[must_use]
    pub fn new(width: Length, height: Length, depth: Length) -> Self {
        Self {
            width,
            height,
            depth,
            runs: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Adds a glyph run and returns the box.
    #[must_use]
    pub fn with_run(mut self, run: HostBoxRun) -> Self {
        self.runs.push(run);
        self
    }

    /// Adds a rule and returns the box.
    #[must_use]
    pub fn with_rule(mut self, rule: HostBoxRule) -> Self {
        self.rules.push(rule);
        self
    }
}

/// Glyphs of one face inside a host box, the fragment carries the key so the host can draw them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostBoxRun {
    /// Key of the host's face.
    pub font: FontKey,
    /// Size the glyphs are drawn at.
    pub size: Length,
    /// Glyphs in drawing order.
    pub glyphs: Vec<HostBoxGlyph>,
}

impl HostBoxRun {
    /// A run of `glyphs` in face `font` at `size`.
    #[must_use]
    pub fn new(font: FontKey, size: Length, glyphs: Vec<HostBoxGlyph>) -> Self {
        Self { font, size, glyphs }
    }
}

/// A glyph at its baseline origin, relative to the box's baseline origin with y down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostBoxGlyph {
    /// Glyph index in the run's face.
    pub glyph: GlyphId,
    /// Baseline origin of the glyph.
    pub origin: Point,
}

impl HostBoxGlyph {
    /// Glyph `glyph` with its baseline origin at `origin`.
    #[must_use]
    pub fn new(glyph: GlyphId, origin: Point) -> Self {
        Self { glyph, origin }
    }
}

/// A filled rectangle whose bottom left corner is `origin`, relative to the box's baseline origin with y down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostBoxRule {
    /// Bottom left corner.
    pub origin: Point,
    /// Width to the right of the corner.
    pub width: Length,
    /// Height above the corner.
    pub height: Length,
}

impl HostBoxRule {
    /// A rule `width` wide and `height` tall above `origin`.
    #[must_use]
    pub fn new(origin: Point, width: Length, height: Length) -> Self {
        Self {
            origin,
            width,
            height,
        }
    }
}

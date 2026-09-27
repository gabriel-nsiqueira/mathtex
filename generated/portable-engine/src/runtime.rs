//! Runtime prelude the patcher splices into the generated XeTeX engine as `runtime.rs`.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
// Engine methods spell `self: &mut Self` because codegen strips the `&mut self` shorthand.
#![allow(clippy::needless_arbitrary_self_type)]

pub(crate) type integer = i32;
pub(crate) type real = f64;
pub(crate) type glueratio = f64;
pub(crate) type boolean = i32;
pub(crate) type schar = i8;
pub(crate) type ASCIIcode = integer;
pub(crate) type eightbits = integer;
pub(crate) type poolpointer = integer;
pub(crate) type strnumber = integer;
pub(crate) type savepointer = integer;
pub(crate) type packedUTF16code = u16;
pub(crate) type scaled = integer;
pub(crate) type nonnegativeinteger = integer;
pub(crate) type smallnumber = integer;
pub(crate) type quarterword = integer;
pub(crate) type halfword = integer;
pub(crate) type glueord = integer;
pub(crate) type groupcode = integer;
pub(crate) type internalfontnumber = integer;
pub(crate) type fontindex = integer;
pub(crate) type ninebits = integer;
pub(crate) type triepointer = integer;
pub(crate) type trieopcode = integer;
pub(crate) type hyphpointer = integer;
pub(crate) type UTF16code = u16;
pub(crate) type UnicodeScalar = integer;
pub(crate) type uint16_t = u16;
pub(crate) type UTF8code = integer;
pub(crate) type voidpointer = *mut ();
pub(crate) type string = *mut i8;
pub(crate) type const_string = *const i8;
pub(crate) struct PortableFileHandle {
    name: String,
    package: Option<String>,
    format: integer,
    bytes: Vec<u8>,
    cursor: usize,
    eof_after_failed_read: bool,
    /// Decoding of this file's bytes, `Bytes` for binary inputs such as TFM files.
    encoding: InputEncoding,
    /// One unit lookahead for the UTF-16 surrogate decoder, XeTeX's `savedChar`.
    saved_char: Option<u32>,
}

/// XeTeX input decoding mode from `xetex.h`, `auto` is resolved at open time and ICU mappings read as bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputEncoding {
    /// Raw bytes, one Unicode scalar per byte (XeTeX `RAW`).
    #[default]
    Bytes,
    /// UTF-8 (XeTeX `UTF8`).
    Utf8,
    /// UTF-16 big endian, XeTeX's `UTF16BE`.
    Utf16Be,
    /// UTF-16 little endian, XeTeX's `UTF16LE`.
    Utf16Le,
}
pub(crate) type bytefile = NativeFileHandle;
pub(crate) type unicodefile = NativeFileHandle;
pub type PortableFontHandle = usize;
pub(crate) type FontHandle = PortableFontHandle;
pub(crate) type CFDictionaryRef = voidpointer;
pub(crate) type NativeFileHandle = *mut PortableFileHandle;
pub(crate) type alphafile = NativeFileHandle;
#[allow(clippy::upper_case_acronyms)]
pub(crate) type UFILE = PortableFileHandle;

pub(crate) const true_0: boolean = 1;
pub(crate) const false_0: boolean = 0;
pub(crate) const firstmathfontdimen: integer = 10;
pub(crate) const native_node_size: integer = 6;

/// The empty string, an absent file extension.
pub(crate) const POOL_EMPTY_STRING: strnumber = 65626;
/// The `.tex` extension `\openout` supplies.
pub(crate) const POOL_TEX_EXTENSION: strnumber = 66180;
/// The capacity `overflow` names for a line longer than the buffer.
pub(crate) const POOL_BUFFER_SIZE: strnumber = 65538;
/// The `confusion` label of `prune_page_top`.
pub(crate) const POOL_PRUNING: strnumber = 66395;
/// The prefix `print_err` starts every error message with.
pub(crate) const POOL_ERROR_PREFIX: strnumber = 65544;
/// The inter atom spacing table `magic_offset` indexes.
pub(crate) const POOL_MATH_SPACING: strnumber = 66320;
// XeTeX layout constants from xetex.web, as tangled with the partoken change file that makes max_command 103.
/// The null pointer, `min_halfword`.
pub(crate) const TEX_NULL: halfword = -0x0FFF_FFFF;
/// Outer vertical mode, internal vertical mode is its negation.
pub(crate) const VMODE: integer = 1;
/// Math mode, `vmode + 2 * (max_command + 1)`.
pub(crate) const MMODE: integer = 209;
/// Box context of `\setbox0`, smaller contexts are shift amounts of a box being appended.
pub(crate) const BOX_FLAG: integer = 0o10000000000;
/// `eq_type` of an undefined control sequence, `max_command + 1`.
pub(crate) const EQ_TYPE_UNDEFINED_CS: i16 = 104;
/// `eq_type` of a code table entry, `max_command + 20`.
pub(crate) const EQ_TYPE_DATA: i16 = 123;
/// `eq_level` of a definition made outside every group.
pub(crate) const LEVEL_ONE: i16 = 1;
/// Command code and catcode of an other character.
pub(crate) const CMD_OTHER_CHAR: integer = 12;
/// Command code of the extension primitives, `\Uhostbox` among them.
pub(crate) const CMD_EXTENSION: i16 = 59;
/// Number of the first string with pool storage, below it every string is a single character.
pub(crate) const TOO_BIG_CHAR: strnumber = 65_536;
/// `align_state` outside every alignment, braces move it up and down from here.
pub(crate) const NEUTRAL_ALIGN_STATE: integer = 1_000_000;
/// Math style code of script style, the smaller codes are display and text styles.
pub(crate) const SCRIPT_STYLE: i32 = 4;
/// Math size offset of the script size fonts, twice it is the scriptscript size.
pub(crate) const SCRIPT_SIZE: i32 = 256;
/// Box glue order of finite glue.
pub(crate) const GLUE_ORDER_NORMAL: i16 = 0;
/// Box glue sign of a stretched box.
pub(crate) const GLUE_SIGN_STRETCHING: i16 = 1;
/// Command code of `\relax`.
pub(crate) const CMD_RELAX: integer = 0;
/// Command code of a right brace.
pub(crate) const CMD_RIGHT_BRACE: integer = 2;
/// Command code of a space.
pub(crate) const CMD_SPACER: integer = 10;
/// Command code of `\par`.
pub(crate) const CMD_PAR_END: integer = 13;
/// Command code of `\end` and `\dump`.
pub(crate) const CMD_STOP: integer = 14;
/// Last command code that is not an assignment, `prefixed_command` runs every command above it.
pub(crate) const CMD_MAX_NON_PREFIXED: integer = 71;
/// Input state of a token list level, every other state reads a file line.
pub(crate) const TOKEN_LIST_STATE: quarterword = 0;
/// Input state at the start of a file line, XeTeX's `new_line`.
pub(crate) const NEW_LINE_STATE: quarterword = 33;
/// `read_open` state of a `\read` stream that is not open.
pub(crate) const READ_CLOSED: eightbits = 2;
/// `read_open` state of a `\read` stream opened but not yet read.
pub(crate) const READ_JUST_OPEN: eightbits = 1;
/// Number of `\write` and `\read` streams, higher stream numbers reach the terminal.
pub(crate) const STREAM_COUNT: usize = 16;
/// `font_area` marker of a font loaded through the OpenType or Graphite shaper.
pub(crate) const OTGR_FONT_FLAG: integer = 0xFFFE;
/// Token type of a macro argument, the first token type.
pub(crate) const TOKEN_TYPE_PARAMETER: quarterword = 0;
/// Token type of a single backed up token, the types below it are arguments and templates.
pub(crate) const TOKEN_TYPE_BACKED_UP: quarterword = 3;
/// Token type of a backed up character, XeTeX's `backed_up_char`.
pub(crate) const TOKEN_TYPE_BACKED_UP_CHAR: quarterword = 4;
/// Token type of an inserted token list.
pub(crate) const TOKEN_TYPE_INSERTED: quarterword = 5;
/// Token type of a macro body, the types from here on map their tokens to the invocation.
pub(crate) const TOKEN_TYPE_MACRO: quarterword = 6;
/// Token type of a `\write` token list.
pub(crate) const TOKEN_TYPE_WRITE_TEXT: quarterword = 18;
/// Node type of an hlist box.
pub(crate) const HLIST_NODE: i32 = 0;
/// Node type of a vlist box.
pub(crate) const VLIST_NODE: i32 = 1;
/// Node type of a rule.
pub(crate) const RULE_NODE: i32 = 2;
/// Node type of an insertion.
pub(crate) const INS_NODE: i32 = 3;
/// Node type of a mark.
pub(crate) const MARK_NODE: i32 = 4;
/// Node type of a `\vadjust`.
pub(crate) const ADJUST_NODE: i32 = 5;
/// Node type of a ligature.
pub(crate) const LIGATURE_NODE: i32 = 6;
/// Node type of a discretionary.
pub(crate) const DISC_NODE: i32 = 7;
/// Node type of a whatsit.
pub(crate) const WHATSIT_NODE: i32 = 8;
/// Node type of a math on or off node.
pub(crate) const MATH_NODE: i32 = 9;
/// Node type of glue.
pub(crate) const GLUE_NODE: i32 = 10;
/// Node type of a kern.
pub(crate) const KERN_NODE: i32 = 11;
/// Node type of a penalty.
pub(crate) const PENALTY_NODE: i32 = 12;
/// Node type of an unset alignment box.
pub(crate) const UNSET_NODE: i32 = 13;
/// Node type of a math style change.
pub(crate) const STYLE_NODE: i32 = 14;
/// Node type of a `\mathchoice`.
pub(crate) const CHOICE_NODE: i32 = 15;
/// Noad type of an ordinary atom, the first noad type.
pub(crate) const ORD_NOAD: i32 = 16;
/// Noad type of a generalized fraction.
pub(crate) const FRACTION_NOAD: i32 = 25;
/// Noad type of a `\vcenter`, the last noad type with a nucleus.
pub(crate) const VCENTER_NOAD: i32 = 29;
/// Noad type of a `\left`.
pub(crate) const LEFT_NOAD: i32 = 30;
/// Noad type of a `\right`, the last noad type.
pub(crate) const RIGHT_NOAD: i32 = 31;
/// Whatsit subtype of an `\openout`.
pub(crate) const OPEN_NODE: i32 = 0;
/// Whatsit subtype of a `\write`.
pub(crate) const WRITE_NODE: i32 = 1;
/// Whatsit subtype of a `\closeout`.
pub(crate) const CLOSE_NODE: i32 = 2;
/// Whatsit subtype of a native word.
pub(crate) const NATIVE_WORD_NODE: i32 = 40;
/// Whatsit subtype of a native word ending in a hyphenation point.
pub(crate) const NATIVE_WORD_NODE_AT: i32 = 41;
/// Whatsit subtype of a single native glyph.
pub(crate) const GLYPH_NODE: i32 = 42;
/// Math type of a noad field holding a box.
pub(crate) const SUB_BOX: i32 = 2;
/// `\splittopskip` among the glue parameters.
pub(crate) const SPLIT_TOP_SKIP_CODE: smallnumber = 10;
/// `\vsplit` among the box codes, which index `disc_ptr`.
pub(crate) const VSPLIT_CODE: usize = 3;
/// eqtb location of the current font.
pub(crate) const CUR_FONT_LOC: i64 = 1_206_823;
/// First eqtb location of the math fonts, family 0 at text size.
pub(crate) const MATH_FONT_BASE: i64 = 1_206_824;
/// First eqtb location of the per code point `\catcode` table.
pub(crate) const CAT_CODE_BASE: usize = 1_207_592;
/// First eqtb location of the `\lccode` table.
pub(crate) const LC_CODE_BASE: usize = 2_321_704;
/// First eqtb location of the `\sfcode` table.
pub(crate) const SF_CODE_BASE: usize = 4_549_928;
/// First eqtb location of the `\mathcode` table.
pub(crate) const MATH_CODE_BASE: usize = 5_664_040;
/// First eqtb location of the character substitution table.
pub(crate) const CHAR_SUB_CODE_BASE: usize = 6_778_152;
/// First eqtb location of the integer parameters.
pub(crate) const INT_BASE: usize = 7_892_264;
/// First eqtb location of the per code point `\delcode` table.
pub(crate) const DEL_CODE_BASE: usize = 7_892_607;
/// First eqtb location of the dimension parameters, one past the `\delcode` table.
pub(crate) const DIMEN_BASE: usize = 9_006_719;
/// eqtb location of `\endlinechar`.
pub(crate) const EQTB_END_LINE_CHAR_LOC: i64 = 7_892_312;
/// eqtb location of `\tracinglostchars`, `int_base + tracing_lost_chars_code`.
pub(crate) const EQTB_TRACING_LOST_CHARS_LOC: i64 = 7_892_299;
/// eqtb location of `\XeTeXuseglyphmetrics`, `int_base + eTeX_state_code + XeTeX_use_glyph_metrics_code`.
pub(crate) const EQTB_USE_GLYPH_METRICS_LOC: i64 = 7_892_342;
/// First selector that prints into memory rather than to the terminal or log.
pub(crate) const SELECTOR_PSEUDO: eightbits = 20;
/// Last `eqtb` location, the `hash_extra` control sequence meanings follow it.
pub(crate) const EQTB_SIZE: usize = 9_006_997;
/// Font dimension number of the quad.
pub(crate) const QUAD_CODE: integer = 6;
/// Font dimension number of the x height.
pub(crate) const X_HEIGHT_CODE: integer = 5;
/// Font dimension number XeTeX gives native fonts for the cap height.
pub(crate) const CAP_HEIGHT_CODE: integer = 8;

#[derive(Debug)]
pub(crate) struct EngineAbort {
    status: integer,
}

/// A surfaced TeX error or sandbox rejection, carrying the message the host reports.
#[derive(Debug)]
pub(crate) struct EngineError {
    pub(crate) message: String,
    pub(crate) kind: PortableErrorKind,
}

/// What ended a run with an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableErrorKind {
    /// A TeX error, such as an undefined control sequence.
    Tex,
    /// The sandbox rejected a command, such as a math shift that leaves the wrapper math.
    Sandbox,
    /// The sandbox work budget ran out.
    Budget,
}

/// The error that ended the last run and where the input stood when it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableError {
    pub message: String,
    pub kind: PortableErrorKind,
    /// TeX's `line`, the 1 based line of the innermost input file, 0 before any line was read.
    pub line: integer,
    /// Span of the token read last, `None` while source tracking is off.
    pub span: Option<PortableSourceSpan>,
}

/// The non `Ok` outcomes, kept as a `Result` error so `?` threads them through the generated bodies.
#[derive(Debug)]
pub(crate) enum EngineBreak {
    Abort(EngineAbort),
    Error(EngineError),
}

impl From<EngineAbort> for EngineBreak {
    fn from(abort: EngineAbort) -> Self {
        EngineBreak::Abort(abort)
    }
}

/// Default sandbox budget of commands and expansions, [`PortableTexEngine::set_sandbox_op_budget`] changes it.
pub const SANDBOX_OP_BUDGET: u64 = 2_000_000;

/// Construct frames a render may push before source tracking stops recording enclosing constructs.
const SRC_STACK_CAP: usize = 1 << 20;

/// Request mode of a `\filesize` probe, which reads a resource only to measure it and may find nothing.
pub const FILE_SIZE_PROBE_MODE: &str = "size";

/// Sandbox message for a fragment that leaves or reenters the wrapper math.
const MATH_BREAKOUT_MESSAGE: &str = "math shift ($) is not allowed inside a math expression";

/// Result of engine code that can stop the run, generated bodies shadow `Result` so this alias names it.
pub(crate) type EngineFlow<T> = core::result::Result<T, EngineBreak>;
pub(crate) const nullptr: voidpointer = core::ptr::null_mut::<()>();
pub(crate) const nil: voidpointer = core::ptr::null_mut::<()>();
pub(crate) const maxint: integer = i32::MAX;
pub(crate) const mintrieop: integer = 0;
pub(crate) const trieopsize: i64 = 35111;
pub(crate) const negtrieopsize: i64 = -35111;
pub(crate) const maxtrieop: i64 = 65535;
pub(crate) const hashoffset: integer = 514;
pub(crate) const DIR_SEP: integer = b'/' as integer;
pub(crate) const resource_format_tex_input: integer = 26;
pub(crate) const resource_format_tfm: integer = 3;
pub(crate) const resource_format_format_image: integer = 10;
pub(crate) const resource_format_config: integer = 8;
pub(crate) const resource_format_font_map: integer = 11;
pub(crate) const resource_format_encoding: integer = 44;
pub(crate) const resource_format_font: integer = 47;
pub(crate) const FOPEN_RBIN_MODE: [i8; 3] = [b'r' as i8, b'b' as i8, 0];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableSourceSpan {
    pub name: String,
    pub start: u32,
    pub end: u32,
    /// Provenance role, the IR `SourceRole` discriminant: 0 primary, 1 macro expansion.
    pub role: u8,
}

/// Interned source span id, 0 is none and real spans start at 1, indexing `src_spans` off by one.
pub(crate) type SrcId = u32;

/// One recorded span, byte offsets into the input file named by string number `name`, hashed whole for interning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct RawSpan {
    pub name: strnumber,
    pub start: u32,
    pub end: u32,
    pub role: u8,
}

/// `default_fn` for the `node_src` paged shadow: an unstamped node has span id 0.
fn node_src_default(_i: usize) -> u32 {
    0
}

/// Live byte range of a `node_src` word, the whole `u32`.
fn node_src_sig(_i: usize) -> (usize, usize) {
    (0, 4)
}

/// An enclosing construct frame, a macro call or math group linked to its parent, pending until its group closes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SrcStackCell {
    /// Interned span of the finished construct, 0 while pending or empty.
    pub span: SrcId,
    /// Parent frame index counted from 1, 0 for the root.
    pub parent: u32,
    /// Pending group: input byte offset of the enclosing command's start.
    pub start: u32,
    /// Pending group: string number of the source file the offsets are in.
    pub name: strnumber,
    /// True while a group frame is open (its end offset is not yet known).
    pub pending: bool,
}

/// `default_fn` for the `node_stack` paged shadow: an unstamped node has no enclosing frame (`0`).
fn node_stack_default(_i: usize) -> u32 {
    0
}

/// Live byte range of a `node_stack` word, the whole `u32`.
fn node_stack_sig(_i: usize) -> (usize, usize) {
    (0, 4)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    TexInput,
    Package,
    Class,
    FontDefinition,
    PackageSupport,
    Font,
    Encoding,
    Map,
    Config,
    FormatImage,
    Asset,
    Other(integer),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequest<'a> {
    pub name: &'a str,
    pub kind: ResourceKind,
    pub package: Option<&'a str>,
    pub format: integer,
    pub mode: &'a str,
    pub source: Option<PortableSourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableResourceRequestRecord {
    pub name: String,
    pub kind: ResourceKind,
    pub package: Option<String>,
    pub format: integer,
    pub mode: String,
    pub source: Option<PortableSourceSpan>,
    pub byte_len: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortableFontMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub xheight: i32,
    pub capheight: i32,
    pub slant: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortableNativeGlyph {
    pub glyph_id: u16,
    pub x: i32,
    pub y: i32,
    pub advance: i32,
    pub cluster_start: u32,
    pub cluster_end: u32,
    /// Input byte span of the characters that shaped this glyph, `0..0` when unmapped.
    pub src_start: u32,
    pub src_end: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortableNativeTextMetrics {
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub glyphs: Vec<PortableNativeGlyph>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortableNativeGlyphMetrics {
    pub width: i32,
    pub height: i32,
    pub depth: i32,
}

/// A glyph's advance and control box in points, y up, as XeTeX's `getGlyphWidth` and `getGlyphBounds` give them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PortableGlyphBounds {
    /// Horizontal advance.
    pub advance: f32,
    /// Left edge of the control box.
    pub x_min: f32,
    /// Bottom edge of the control box.
    pub y_min: f32,
    /// Right edge of the control box.
    pub x_max: f32,
    /// Top edge of the control box.
    pub y_max: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PortableNativeGlyphInfo {
    glyphs: Vec<PortableNativeGlyph>,
}

/// A larger OpenType MATH glyph variant, its advance in scaled points and `-1` when there is none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableMathVariant {
    pub glyph: i32,
    pub advance: i32,
}

/// One part of an OpenType MATH glyph assembly in scaled points, as `hb_ot_math_glyph_part_t`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortableMathAssemblyPart {
    pub glyph: i32,
    pub start_connector: i32,
    pub end_connector: i32,
    pub full_advance: i32,
    pub extender: bool,
}

/// A corner of an OpenType `MathKernInfo` record (mirrors `hb_ot_math_kern_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableMathKernCorner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

/// OpenType MATH glyph assembly passed to the engine as an opaque pointer, freed by `free_ot_assembly`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphAssembly {
    pub parts: Vec<PortableMathAssemblyPart>,
}

pub trait FontPlatform {
    fn resolve_font_handle(&mut self, _name: &[i32], _size: i32) -> Option<PortableFontHandle> {
        None
    }

    fn release_font_handle(&mut self, _font: PortableFontHandle, _type_flag: i32) {}

    /// The native font table as `(handle, spec, size)`, which lets a loaded format image rebind its font handles.
    fn font_table(&self) -> Vec<(PortableFontHandle, String, i32)> {
        Vec::new()
    }

    /// Reloads each font of a [`Self::font_table`] snapshot at its original handle, false when one fails.
    fn restore_font_table(&mut self, _table: &[(PortableFontHandle, String, i32)]) -> bool {
        true
    }

    fn font_metrics(&mut self, _font: PortableFontHandle) -> PortableFontMetrics {
        PortableFontMetrics::default()
    }

    fn opentype_font_metrics(&mut self, font: PortableFontHandle) -> PortableFontMetrics {
        self.font_metrics(font)
    }

    fn is_opentype_math_font(&mut self, _font: PortableFontHandle) -> bool {
        false
    }

    /// Whether `font` is shaped by the OpenType shaper, XeTeX's `usingOpenType`.
    fn using_opentype(&mut self, _font: PortableFontHandle) -> bool {
        false
    }

    fn math_symbol_parameter(&mut self, _font: PortableFontHandle, _parameter: i32) -> i32 {
        0
    }

    fn math_extension_parameter(&mut self, _font: PortableFontHandle, _parameter: i32) -> i32 {
        0
    }

    fn opentype_math_constant(&mut self, _font: PortableFontHandle, _constant: i32) -> i32 {
        0
    }

    fn opentype_math_accent_position(&mut self, _font: PortableFontHandle, _glyph: i32) -> i32 {
        0
    }

    /// OpenType MATH italic correction for a glyph, in scaled points.
    fn math_glyph_italic_correction(&mut self, _font: PortableFontHandle, _glyph: i32) -> i32 {
        0
    }

    /// The `index`th larger MATH variant of `glyph`, horizontal or vertical, `None` when there is none.
    fn math_glyph_variant(
        &mut self,
        _font: PortableFontHandle,
        _glyph: i32,
        _index: u16,
        _horizontal: bool,
    ) -> Option<PortableMathVariant> {
        None
    }

    /// The MATH assembly parts for `glyph` in scaled points, empty when it has none.
    fn math_glyph_assembly(
        &mut self,
        _font: PortableFontHandle,
        _glyph: i32,
        _horizontal: bool,
    ) -> Vec<PortableMathAssemblyPart> {
        Vec::new()
    }

    /// The MATH minimum connector overlap for assembly parts, in scaled points.
    fn math_min_connector_overlap(&mut self, _font: PortableFontHandle) -> i32 {
        0
    }

    /// One MATH `MathKernInfo` corner at `correction_height`, both in font design units.
    fn math_kern_at(
        &mut self,
        _font: PortableFontHandle,
        _glyph: i32,
        _corner: PortableMathKernCorner,
        _correction_height: i32,
    ) -> i32 {
        0
    }

    /// Converts points to font design units for `font`, XeTeX's `pointsToUnits`.
    fn math_points_to_units(&mut self, _font: PortableFontHandle, _points: f32) -> f32 {
        0.0
    }

    /// Converts font design units to scaled points for `font`, XeTeX's `D2Fix(unitsToPoints(...))`.
    fn math_units_to_scaled(&mut self, _font: PortableFontHandle, _units: i32) -> i32 {
        0
    }

    /// The point size at which `font` was loaded (`getPointSize`).
    fn math_point_size(&mut self, _font: PortableFontHandle) -> f32 {
        0.0
    }

    fn map_char_to_glyph(&mut self, _font: PortableFontHandle, _codepoint: i32) -> i32 {
        0
    }

    fn map_glyph_to_index(&mut self, _font: PortableFontHandle, _name: &str) -> i32 {
        0
    }

    /// OpenType layout query of `\XeTeXcountglyphs` and the `\XeTeXOT` primitives, `what` is the XeTeX_ext code.
    fn ot_font_get(
        &mut self,
        _font: PortableFontHandle,
        _what: i32,
        _param1: i32,
        _param2: i32,
        _param3: i32,
    ) -> i32 {
        0
    }

    /// The `\font` spec a handle was loaded from, such as `[latinmodern-math.otf]:script=math;ssty=1`.
    fn font_spec(&self, _font: PortableFontHandle) -> Option<String> {
        None
    }

    /// The key the host's font loader gave the face behind a handle.
    fn font_key(&self, _font: PortableFontHandle) -> Option<u64> {
        None
    }

    fn shape_native_text(
        &mut self,
        _font: PortableFontHandle,
        _text: &[u16],
        _use_glyph_metrics: bool,
    ) -> PortableNativeTextMetrics {
        PortableNativeTextMetrics::default()
    }

    fn measure_native_glyph(
        &mut self,
        _font: PortableFontHandle,
        _glyph: u16,
        _use_glyph_metrics: bool,
    ) -> PortableNativeGlyphMetrics {
        PortableNativeGlyphMetrics::default()
    }

    /// Advance and control box of `glyph` at the font's size, all zero for a glyph without an outline.
    fn glyph_bounds(&mut self, _font: PortableFontHandle, _glyph: u16) -> PortableGlyphBounds {
        PortableGlyphBounds::default()
    }

    /// Smallest and largest character codes the font maps to a glyph, as XeTeX's `getFontCharRange`.
    fn char_code_range(&mut self, _font: PortableFontHandle) -> Option<(i32, i32)> {
        None
    }
}

#[derive(Default)]
pub struct EmptyFontPlatform;

impl FontPlatform for EmptyFontPlatform {}

pub trait ResourceProvider {
    fn read(&mut self, request: ResourceRequest<'_>) -> Option<Vec<u8>>;
}

#[derive(Default)]
pub struct EmptyResourceProvider;

impl ResourceProvider for EmptyResourceProvider {
    fn read(&mut self, _request: ResourceRequest<'_>) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortableClock {
    pub seconds: integer,
    pub micros: integer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableLinebreakRequest<'a> {
    pub font: integer,
    pub locale: integer,
    pub text: &'a [uint16_t],
}

// Style context for a host box request, display math collapses to Text to match cursize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableHostBoxStyle {
    Text,
    Script,
    ScriptScript,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableHostBoxRequest {
    pub token: integer,
    pub style: PortableHostBoxStyle,
    pub font_size: scaled,
}

// Glyph offsets are scaled points from the box baseline origin, y positive downward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableHostBoxGlyph {
    pub glyph: u32,
    pub x: scaled,
    pub y: scaled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortableHostBoxRun {
    /// The key the host's font loader gave the run's face.
    pub font_key: u64,
    pub font_size: scaled,
    pub glyphs: Vec<PortableHostBoxGlyph>,
}

// Rule origin is its bottom left corner with y positive downward, height extends upward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableHostBoxRule {
    pub x: scaled,
    pub y: scaled,
    pub width: scaled,
    pub height: scaled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortableHostBox {
    pub width: scaled,
    pub height: scaled,
    pub depth: scaled,
    pub runs: Vec<PortableHostBoxRun>,
    pub rules: Vec<PortableHostBoxRule>,
}

// \Uhostbox is extension cmd 59 with this chr, upstream extension codes stop at 46.
pub(crate) const HOST_BOX_EXTENSION_CODE: i32 = 91;
// Marker whatsit subtypes: pending carries the opaque token id, resolved carries a record index.
pub(crate) const HOST_BOX_PENDING_SUBTYPE: i32 = 91;
pub(crate) const HOST_BOX_RESOLVED_SUBTYPE: i32 = 92;
/// Words in a host box marker whatsit, laid out like a glyph node.
pub(crate) const HOST_BOX_MARKER_SIZE: i32 = 5;

pub trait PortablePlatform {
    fn clock(&mut self) -> PortableClock {
        PortableClock::default()
    }

    fn linebreak_start(&mut self, _request: PortableLinebreakRequest<'_>) {}

    fn linebreak_next(&mut self) -> Option<integer> {
        None
    }

    fn host_box(&mut self, _request: PortableHostBoxRequest) -> Option<PortableHostBox> {
        None
    }
}

#[derive(Default)]
pub struct EmptyPlatform;

impl PortablePlatform for EmptyPlatform {}

impl PortableFileHandle {
    fn new(name: String, package: Option<String>, format: integer, bytes: Vec<u8>) -> Self {
        Self {
            name,
            package,
            format,
            bytes,
            cursor: 0,
            eof_after_failed_read: false,
            encoding: InputEncoding::Bytes,
            saved_char: None,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let Some(byte) = self.bytes.get(self.cursor).copied() else {
            self.eof_after_failed_read = true;
            return None;
        };
        self.cursor += 1;
        Some(byte)
    }

    fn has_remaining(&self) -> bool {
        self.cursor < self.bytes.len() || self.saved_char.is_some()
    }

    /// Byte offset where the next scalar starts, a held back UTF-16 unit began two bytes before the cursor.
    fn next_scalar_offset(&self) -> usize {
        if self.saved_char.is_some() {
            self.cursor.saturating_sub(2)
        } else {
            self.cursor
        }
    }

    fn is_eof(&self) -> bool {
        self.eof_after_failed_read
    }

    /// XeTeX `u_open_in` auto detection: a BOM or leading zero byte picks UTF-16, else UTF-8, skipping a BOM.
    fn resolve_text_encoding_auto(&mut self) {
        let b1 = self.bytes.first().copied();
        let b2 = self.bytes.get(1).copied();
        match (b1, b2) {
            (Some(0xFE), Some(0xFF)) => {
                self.encoding = InputEncoding::Utf16Be;
                self.cursor = 2;
            }
            (Some(0xFF), Some(0xFE)) => {
                self.encoding = InputEncoding::Utf16Le;
                self.cursor = 2;
            }
            (Some(0x00), Some(b2)) if b2 != 0 => {
                self.encoding = InputEncoding::Utf16Be;
            }
            (Some(b1), Some(0x00)) if b1 != 0 => {
                self.encoding = InputEncoding::Utf16Le;
            }
            (Some(0xEF), Some(0xBB)) if self.bytes.get(2).copied() == Some(0xBF) => {
                self.encoding = InputEncoding::Utf8;
                self.cursor = 3;
            }
            _ => {
                self.encoding = InputEncoding::Utf8;
            }
        }
    }

    /// Next Unicode scalar, `None` at the end of input, XeTeX's `get_uni_c`.
    fn next_input_scalar(&mut self) -> Option<u32> {
        // savedChar lookahead (only set by the UTF-16 decoders).
        if let Some(saved) = self.saved_char.take() {
            return Some(saved);
        }
        match self.encoding {
            InputEncoding::Bytes => self.read_byte().map(u32::from),
            InputEncoding::Utf8 => self.next_utf8_scalar(),
            InputEncoding::Utf16Be => self.next_utf16_scalar(true),
            InputEncoding::Utf16Le => self.next_utf16_scalar(false),
        }
    }

    /// UTF-8 branch of `get_uni_c`: a bad continuation byte is pushed back and read as U+FFFD.
    fn next_utf8_scalar(&mut self) -> Option<u32> {
        // offsetsFromUTF8[extraBytes].
        const OFFSETS: [u32; 6] = [
            0x0000_0000,
            0x0000_3080,
            0x000E_2080,
            0x03C8_2080,
            0xFA08_2080,
            0x8208_2080,
        ];
        let lead = self.read_byte()?;
        // bytesFromUTF8[lead]: extra continuation bytes (0..=5).
        let extra: usize = match lead {
            0x00..=0xBF => 0,
            0xC0..=0xDF => 1,
            0xE0..=0xEF => 2,
            0xF0..=0xF7 => 3,
            0xF8..=0xFB => 4,
            0xFC..=0xFF => 5,
        };
        // Assemble in a wider type to mirror C's `int rval` (continuations <<6).
        let mut rval: i64 = i64::from(lead);
        // C `switch(extraBytes)` falls through cases 3, 2 and 1, cases 4 and 5 return the bad result at once.
        if extra >= 4 {
            // Lead bytes 0xF8 to 0xFF start no valid sequence.
            return Some(0xFFFD);
        }
        for _ in 0..extra {
            match self.read_byte() {
                Some(c) if (0x80..0xC0).contains(&c) => {
                    rval = (rval << 6) + i64::from(c);
                }
                Some(c) => {
                    // C `bad_utf8` pushes the offending byte back with `UNGETC`.
                    self.cursor -= 1;
                    let _ = c;
                    return Some(0xFFFD);
                }
                None => {
                    // End of input inside a sequence is `bad_utf8` with nothing to push back.
                    return Some(0xFFFD);
                }
            }
        }
        rval -= i64::from(OFFSETS[extra]);
        if !(0..=0x10_FFFF).contains(&rval) {
            return Some(0xFFFD);
        }
        Some(rval as u32)
    }

    /// UTF-16 branch of `get_uni_c`: an unpaired surrogate reads as U+FFFD, a stray unit after one is kept.
    fn next_utf16_scalar(&mut self, big_endian: bool) -> Option<u32> {
        let unit = self.read_utf16_unit(big_endian)?;
        if (0xD800..=0xDBFF).contains(&unit) {
            match self.read_utf16_unit(big_endian) {
                Some(lo) if (0xDC00..=0xDFFF).contains(&lo) => {
                    Some(0x10000 + (unit - 0xD800) * 0x400 + (lo - 0xDC00))
                }
                Some(lo) => {
                    self.saved_char = Some(lo);
                    Some(0xFFFD)
                }
                None => Some(0xFFFD),
            }
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            Some(0xFFFD)
        } else {
            Some(unit)
        }
    }

    /// One UTF-16 code unit, `None` only at end of input, a missing second byte reads as 0 as C's `GETC` does.
    fn read_utf16_unit(&mut self, big_endian: bool) -> Option<u32> {
        let first = self.read_byte()?;
        let second = self.read_byte().unwrap_or(0);
        let unit = if big_endian {
            (u32::from(first) << 8) + u32::from(second)
        } else {
            u32::from(first) + (u32::from(second) << 8)
        };
        Some(unit)
    }
}

// Mirrors the slice methods generated code calls on storage fields, which are raw pointers here.
#[allow(clippy::wrong_self_convention)]
pub(crate) trait StatePtrCompat<T> {
    fn as_mut_ptr(self) -> *mut T;
}

impl<T> StatePtrCompat<T> for *mut T {
    fn as_mut_ptr(self) -> *mut T {
        self
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union twohalves {
    pub v: TwoHalvesPair,
    pub u: TwoHalvesBytes,
}

impl Default for twohalves {
    fn default() -> Self {
        Self {
            v: TwoHalvesPair::default(),
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct TwoHalvesBytes {
    pub B1: i16,
    pub B0: i16,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct TwoHalvesPair {
    pub LH: halfword,
    pub RH: halfword,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct fourquarters {
    pub u: FourQuarterBytes,
}

// XeTeX quarterwords are 16 bit, `u16` keeps a memory word at 8 bytes and `FourQuarterCastPass` casts accesses.
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct FourQuarterBytes {
    pub B3: u16,
    pub B2: u16,
    pub B1: u16,
    pub B0: u16,
}

pub(crate) type C2RustUnnamed_2 = FourQuarterBytes;

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union memoryword {
    pub gr: glueratio,
    pub hh: twohalves,
    pub u: MemoryInt,
    pub v: MemoryQuarters,
    /// All eight bytes, XeTeX's native glyph info pointer lives here and this engine keeps it zero.
    pub raw: u64,
}

// Images copy words byte for byte, so a word is the same 8 bytes on every target.
const _: () = assert!(core::mem::size_of::<memoryword>() == 8);

impl Default for memoryword {
    fn default() -> Self {
        Self { raw: 0 }
    }
}

/// Whether word 5 of a native word node holds XeTeX's glyph info pointer, this engine keeps it zero.
pub(crate) fn native_glyph_word_is_null(word: &memoryword) -> bool {
    // Sound because every view of the union is plain data.
    let raw = unsafe { word.raw };
    debug_assert_eq!(raw, 0, "a native word node's glyph info word is nonzero");
    raw == 0
}

// XeTeX packs the quarters and the integer at offset 0 of an 8 byte memory word, as this layout does.
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct MemoryQuarters {
    pub QQQQ: fourquarters,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct MemoryInt {
    pub CINT: integer,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union fmemoryword {
    pub u: FontMemoryInt,
    pub v: FontMemoryQuarters,
}

// Zeroes all eight bytes through the widest view, format images copy words byte for byte.
impl Default for fmemoryword {
    fn default() -> Self {
        Self {
            v: FontMemoryQuarters::default(),
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct FontMemoryQuarters {
    pub QQQQ: fourquarters,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct FontMemoryInt {
    pub CINT: integer,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct liststaterecord {
    pub modefield: i16,
    pub headfield: halfword,
    pub tailfield: halfword,
    pub eTeXauxfield: halfword,
    pub pgfield: integer,
    pub mlfield: integer,
    pub auxfield: memoryword,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct instaterecord {
    pub statefield: quarterword,
    pub indexfield: quarterword,
    pub startfield: halfword,
    pub locfield: halfword,
    pub limitfield: halfword,
    pub namefield: halfword,
    /// Span this input level inherits, saved and restored with the whole record by input stack pushes and pops.
    pub spanfield: SrcId,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub(crate) struct ResourceSearchState {
    pub make_tex_discard_errors: boolean,
}

// `eqtb` and `hash` hold a slot per code point in each code table, so both fault pages in from their defaults.
const PAGE_BITS: usize = 12;
const PAGE_LEN: usize = 1 << PAGE_BITS;

/// Live byte range of the `eqtb` word at `i`, the integer and delimiter code regions only use `.u.CINT`.
fn eqtb_sig_range(i: usize) -> (usize, usize) {
    if (INT_BASE..=EQTB_SIZE).contains(&i) {
        (0, 4)
    } else {
        (0, 8)
    }
}

/// `hash` words are `two_halves`, all eight bytes are live.
fn hash_sig_range(_i: usize) -> (usize, usize) {
    (0, 8)
}

/// Compares two values over bytes `off..off + len`, the dead bytes of a word may hold residue.
fn paged_bytes_eq<T>(a: &T, b: &T, off: usize, len: usize) -> bool {
    // Sound because `off + len` stays within `size_of::<T>()`, a read only byte view of two plain values.
    unsafe {
        let pa = (a as *const T as *const u8).add(off);
        let pb = (b as *const T as *const u8).add(off);
        core::slice::from_raw_parts(pa, len) == core::slice::from_raw_parts(pb, len)
    }
}

/// An array over absolute indices `base..end` stored as lazily faulted pages.
pub(crate) struct PagedArray<T: Copy + Default> {
    base: usize,
    end: usize,
    /// Page `n` covers `base + n * PAGE_LEN`, an absent page reads as its defaults.
    pages: Vec<Option<Box<[T]>>>,
    /// Value at an absolute index whose page is absent.
    default_fn: fn(usize) -> T,
    /// Live byte range `(off, len)` of the value at an absolute index, the rest never counts as a change.
    sig_range: fn(usize) -> (usize, usize),
}

impl<T: Copy + Default> PagedArray<T> {
    fn new(
        base: usize,
        end: usize,
        default_fn: fn(usize) -> T,
        sig_range: fn(usize) -> (usize, usize),
    ) -> Self {
        let npages = (end - base).div_ceil(PAGE_LEN);
        PagedArray {
            base,
            end,
            pages: (0..npages).map(|_| None).collect(),
            default_fn,
            sig_range,
        }
    }

    /// Pointer to the value at `abs`, faulting its page in, `abs` must lie in `base..end`.
    #[inline]
    fn ptr(&mut self, abs: usize) -> *mut T {
        let rel = abs - self.base;
        let pg = rel >> PAGE_BITS;
        let off = rel & (PAGE_LEN - 1);
        if self.pages[pg].is_none() {
            let page_base = self.base + pg * PAGE_LEN;
            let f = self.default_fn;
            let page = (0..PAGE_LEN).map(|j| f(page_base + j)).collect::<Vec<_>>();
            self.pages[pg] = Some(page.into_boxed_slice());
        }
        // Sound because the page was just made present and `off < PAGE_LEN`.
        unsafe {
            self.pages[pg]
                .as_mut()
                .unwrap_unchecked()
                .as_mut_ptr()
                .add(off)
        }
    }

    /// Frees every faulted page whose live bytes still equal its defaults.
    fn compact(&mut self) {
        let f = self.default_fn;
        for pg in 0..self.pages.len() {
            let page_base = self.base + pg * PAGE_LEN;
            let is_default = match &self.pages[pg] {
                None => continue,
                Some(page) => (0..PAGE_LEN).all(|j| {
                    let (off, len) = (self.sig_range)(page_base + j);
                    paged_bytes_eq(&page[j], &f(page_base + j), off, len)
                }),
            };
            if is_default {
                self.pages[pg] = None;
            }
        }
    }

    fn resident_bytes(&self) -> usize {
        let elt = core::mem::size_of::<T>();
        self.pages.iter().filter(|p| p.is_some()).count() * PAGE_LEN * elt
            + self.pages.capacity() * core::mem::size_of::<Option<Box<[T]>>>()
    }

    /// Writes `v` at `abs`, ignoring indices outside the array and leaving an absent page absent for its default.
    #[inline]
    fn set(&mut self, abs: usize, v: T)
    where
        T: PartialEq,
    {
        if abs < self.base || abs >= self.end {
            return;
        }
        if self.pages[(abs - self.base) >> PAGE_BITS].is_none() && v == (self.default_fn)(abs) {
            return;
        }
        // Sound because `abs` lies in `base..end`.
        unsafe {
            *self.ptr(abs) = v;
        }
    }

    /// Reads the value at `abs` without faulting its page, so the read only snapshot path can use it.
    #[inline]
    fn get_copy(&self, abs: usize) -> T {
        if abs < self.base || abs >= self.end {
            return (self.default_fn)(abs);
        }
        let rel = abs - self.base;
        match self.pages.get(rel >> PAGE_BITS).and_then(|p| p.as_ref()) {
            Some(page) => page[rel & (PAGE_LEN - 1)],
            None => (self.default_fn)(abs),
        }
    }
}

impl<T: Copy + Default> Clone for PagedArray<T> {
    fn clone(&self) -> Self {
        PagedArray {
            base: self.base,
            end: self.end,
            pages: self.pages.clone(),
            default_fn: self.default_fn,
            sig_range: self.sig_range,
        }
    }
}

// An empty array never consults its default, allocation and decoding install the real one.
impl<T: Copy + Default> Default for PagedArray<T> {
    fn default() -> Self {
        PagedArray::new(0, 0, |_| T::default(), |_| (0, core::mem::size_of::<T>()))
    }
}

/// A `Copy` handle to a [`PagedArray`] that generated code indexes like the raw `eqtb` and `hash` pointers.
pub(crate) struct PagedView<T: Copy + Default>(*mut PagedArray<T>);

impl<T: Copy + Default> Clone for PagedView<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Copy + Default> Copy for PagedView<T> {}

impl<T: Copy + Default> PagedView<T> {
    pub(crate) fn new(arr: &mut PagedArray<T>) -> Self {
        PagedView(arr as *mut PagedArray<T>)
    }
    #[inline]
    pub(crate) fn as_mut_ptr(self) -> Self {
        self
    }
    /// Pointer to the absolute index `count`, generated code computes the whole index before one offset.
    #[inline]
    pub(crate) fn offset(self, count: isize) -> *mut T {
        // Sound because the view is rebound to live storage before generated code runs, as raw array pointers are.
        unsafe { (*self.0).ptr(count as usize) }
    }
}

/// `eqtb` word XeTeX's `initialize` leaves at `i`, the code tables have per table defaults.
fn eqtb_codepoint_default(i: usize) -> memoryword {
    // Start from a zeroed word so compaction compares no indeterminate bytes.
    let mut w: memoryword = unsafe { core::mem::zeroed() };
    if (CAT_CODE_BASE..INT_BASE).contains(&i) {
        let equiv = if i < LC_CODE_BASE {
            CMD_OTHER_CHAR
        } else if (SF_CODE_BASE..MATH_CODE_BASE).contains(&i) {
            1000
        } else if (MATH_CODE_BASE..CHAR_SUB_CODE_BASE).contains(&i) {
            (i - MATH_CODE_BASE) as i32
        } else {
            0
        };
        w.hh.u.B0 = EQ_TYPE_DATA;
        w.hh.u.B1 = LEVEL_ONE;
        w.hh.v.RH = equiv;
    } else if (DEL_CODE_BASE..DIMEN_BASE).contains(&i) {
        w.u.CINT = -1;
    } else if !(CAT_CODE_BASE..=EQTB_SIZE).contains(&i) {
        // Control sequence meanings, below the code tables and in the `hash_extra` region, start undefined.
        w.hh.u.B0 = EQ_TYPE_UNDEFINED_CS;
        w.hh.v.RH = TEX_NULL;
    }
    w
}

/// `hash` word for an untouched slot.
fn hash_codepoint_default(_i: usize) -> twohalves {
    twohalves::default()
}

/// Declares the plain data engine state and its field wise format image codec from one field list.
macro_rules! portable_state {
    ($($(#[$meta:meta])* $vis:vis $field:ident: $ty:ty,)*) => {
        /// Plain data engine state, copied bitwise, every heap owning array lives in [`PortableTexHeap`].
        #[derive(Clone, Copy)]
        pub(crate) struct PortableTexState {
            $($(#[$meta])* $vis $field: $ty,)*
        }

        impl PortableTexState {
            fn encode_fields(&self, out: &mut Vec<u8>) {
                $(<$ty as ImageCodec>::encode(core::slice::from_ref(&self.$field), out);)*
            }

            /// Decodes every field in declaration order into `state`, which must be writable.
            unsafe fn decode_fields(
                state: *mut Self,
                input: &mut ImageReader<'_>,
            ) -> Result<(), FormatImageError> {
                $(
                    <$ty as ImageCodec>::decode(core::ptr::addr_of_mut!((*state).$field), 1, input)
                        .map_err(|error| error.in_field(stringify!($field)))?;
                )*
                Ok(())
            }
        }
    };
}

portable_state! {
    /// Set once `initialize_format_state` completes, every run entry point refuses a state without it.
    initialized: bool,
    /// Source tracking master switch, every hook returns at once while it is false.
    source_tracking: bool,
    /// Span of the command currently executing, latched at the `main_control` dispatch boundary.
    cmd_span: SrcId,
    /// Call site span carried from `macro_call` to the macro body `begin_token_list`.
    pending_call_span: SrcId,
    /// Buffer offset of the token being lexed, captured at the top of the `get_next` loop.
    src_token_start: integer,
    /// String number of the primary input's name.
    src_primary_name: strnumber,
    /// `macro_call` scratch: input byte offset of the invoking control sequence.
    src_call_start: u32,
    /// `macro_call` scratch: name field of the level the macro was read from.
    src_call_name: strnumber,
    /// `macro_call` scratch: state field of that level, 0 for a token list.
    src_call_state: integer,
    /// `macro_call` scratch: token type of that level, captured before argument scanning.
    src_call_index: integer,
    /// `macro_call` scratch: inherited span fallback for a nested call.
    src_call_span: SrcId,
    /// `macro_call` scratch: span of the last token consumed while scanning the arguments.
    src_call_argspan: SrcId,
    /// Span of the latest control sequence lexed from the primary input buffer.
    src_user_cmd_span: SrcId,
    /// Innermost in fragment command replayed from an argument level token list.
    src_anchor_cmd: SrcId,
    /// `macro_call` scratch: `src_user_cmd_span` captured before the macro reads its arguments.
    src_call_user_span: SrcId,
    /// Origin span of the token held in `cur_tok`, restamped by `back_input`.
    src_tok_span: SrcId,
    /// 1 based index of the live enclosing construct frame in `src_stack_cells`, 0 for none.
    cur_stack_head: u32,
    /// Group open span popped at the latest math group close, consumed later in the same arm.
    src_grp_closing: SrcId,
    /// Set once `src_stack_cells` reaches [`SRC_STACK_CAP`], construct frames stop being pushed or popped.
    src_stack_frozen: bool,
    /// Next host box record id, ids are never reused so a marker kept from an earlier fragment finds no record.
    host_box_next_record: integer,
    pub LRproblems: integer,
    pub LRptr: halfword,
    pub OKtointerrupt: boolean,
    pub terminal_output: NativeFileHandle,
    pub activenodesize: smallnumber,
    pub activewidth: [scaled; 7],
    pub actuallooseness: integer,
    pub adjusttail: halfword,
    pub aftertoken: halfword,
    pub alignptr: halfword,
    pub alignstate: integer,
    pub areadelimiter: poolpointer,
    pub aritherror: boolean,
    pub avail: halfword,
    pub background: [scaled; 7],
    pub baseptr: integer,
    pub bchar: halfword,
    pub bcharlabel: *mut fontindex,
    pub bestbet: halfword,
    pub bestheightplusdepth: scaled,
    pub bestline: halfword,
    pub bestplace: [halfword; 4],
    pub bestplglue: [scaled; 4],
    pub bestplline: [halfword; 4],
    pub bestplshort: [scaled; 4],
    pub breadthmax: integer,
    pub breakwidth: [scaled; 7],
    pub buffer: *mut UnicodeScalar,
    pub bufsize: integer,
    pub c: quarterword,
    pub cancelboundary: boolean,
    pub charbase: *mut integer,
    pub condptr: halfword,
    pub cscount: integer,
    pub curactivewidth: [scaled; 7],
    pub curalign: halfword,
    pub curarea: strnumber,
    pub curboundary: integer,
    pub curbox: halfword,
    pub curc: integer,
    pub curchr: halfword,
    pub curcmd: eightbits,
    pub curcs: halfword,
    pub curdir: smallnumber,
    pub curext: strnumber,
    pub curf: internalfontnumber,
    pub curgroup: groupcode,
    pub curhead: halfword,
    pub curi: fourquarters,
    pub curif: smallnumber,
    pub curinput: instaterecord,
    pub curl: halfword,
    pub curlang: eightbits,
    pub curlevel: quarterword,
    pub curlist: liststaterecord,
    pub curloop: halfword,
    pub curmark: [halfword; 5],
    pub curmlist: halfword,
    pub curmu: scaled,
    pub curname: strnumber,
    pub curorder: glueord,
    pub curp: halfword,
    pub curprehead: halfword,
    pub curpretail: halfword,
    pub curptr: halfword,
    pub curq: halfword,
    pub curr: halfword,
    pub curs: integer,
    pub cursize: integer,
    pub curspan: halfword,
    pub curstyle: smallnumber,
    pub curtail: halfword,
    pub curtok: halfword,
    pub curval: integer,
    pub curval1: integer,
    pub curvallevel: eightbits,
    pub deadcycles: integer,
    pub defref: halfword,
    pub deletionsallowed: boolean,
    pub depthbase: *mut integer,
    pub depththreshold: integer,
    pub dig: [eightbits; 23],
    pub discptr: [halfword; 4],
    pub discwidth: scaled,
    pub doingleaders: boolean,
    pub doingspecial: boolean,
    pub dolastlinefit: boolean,
    pub downptr: halfword,
    pub dvibufsize: integer,
    pub dvigone: integer,
    pub dvilimit: integer,
    pub dvioffset: integer,
    pub dviptr: integer,
    pub dynused: integer,
    pub eTeXmode: eightbits,
    pub easyline: halfword,
    pub editline: integer,
    pub editnamelength: integer,
    pub editnamestart: poolpointer,
    pub eightbitp: i32,
    pub emptyfield: twohalves,
    pub eofseen: *mut boolean,
    pub epochseconds: integer,
    pub eqtbtop: halfword,
    pub errorcount: schar,
    pub errorline: integer,
    pub expanddepth: integer,
    pub expanddepthcount: integer,
    pub extdelimiter: poolpointer,
    pub extenbase: *mut integer,
    pub f: internalfontnumber,
    pub falsebchar: halfword,
    pub fewestdemerits: integer,
    pub filelineerrorstylep: i32,
    pub filenamequotechar: UTF16code,
    pub fileoffset: integer,
    pub fillwidth: [scaled; 3],
    pub finalpass: boolean,
    pub first: integer,
    pub firstcount: integer,
    pub firstindent: scaled,
    pub firstp: halfword,
    pub firstwidth: scaled,
    pub fmemptr: fontindex,
    pub fontarea: *mut strnumber,
    pub fontbc: *mut UTF16code,
    pub fontbchar: *mut ninebits,
    pub fontcheck: *mut fourquarters,
    pub fontdsize: *mut scaled,
    pub fontec: *mut UTF16code,
    pub fontfalsebchar: *mut ninebits,
    pub fontflags: *mut i8,
    pub fontglue: *mut halfword,
    pub fontinfo: *mut fmemoryword,
    pub fontinshortdisplay: integer,
    pub fontlayoutengine: *mut voidpointer,
    pub fontletterspace: *mut scaled,
    pub fontmapping: *mut voidpointer,
    pub fontmax: integer,
    pub fontmemsize: integer,
    pub fontname: *mut strnumber,
    pub fontparams: *mut fontindex,
    pub fontptr: internalfontnumber,
    pub fontsize: *mut scaled,
    pub fontused: *mut boolean,
    pub forceeof: boolean,
    pub formatident: strnumber,
    pub fullsourcefilenamestack: *mut strnumber,
    pub g: halfword,
    pub globalprevp: halfword,
    pub grpstack: *mut savepointer,
    pub ha: halfword,
    pub halfbuf: integer,
    pub halferrorline: integer,
    pub haltingonerrorp: boolean,
    pub haltonerrorp: i32,
    pub hash: PagedView<twohalves>,
    pub hashextra: halfword,
    pub hashhigh: halfword,
    pub hashused: halfword,
    pub hb: halfword,
    pub hc: [integer; 4099],
    pub heightbase: *mut integer,
    pub helpline: [strnumber; 6],
    pub helpptr: eightbits,
    pub hf: internalfontnumber,
    pub himemmin: halfword,
    pub history: eightbits,
    pub hliststack: [halfword; 513],
    pub hliststacklevel: i16,
    pub hn: smallnumber,
    pub hu: [integer; 4097],
    pub hyf: [eightbits; 4097],
    pub hyfbchar: halfword,
    pub hyfchar: integer,
    pub hyfdistance: [smallnumber; 35112],
    pub hyfnext: [trieopcode; 35112],
    pub hyfnum: [smallnumber; 35112],
    pub hyphcount: integer,
    pub hyphenchar: *mut integer,
    pub hyphenpassed: smallnumber,
    pub hyphindex: triepointer,
    pub hyphlink: *mut hyphpointer,
    pub hyphlist: *mut halfword,
    pub hyphnext: integer,
    pub hyphsize: integer,
    pub hyphstart: triepointer,
    pub hyphword: *mut strnumber,
    pub iflimit: eightbits,
    pub ifline: integer,
    pub ifstack: *mut halfword,
    pub initcurlang: eightbits,
    pub initlft: boolean,
    pub initlhyf: integer,
    pub initlig: boolean,
    pub initlist: halfword,
    pub initpoolptr: poolpointer,
    pub initrhyf: integer,
    pub initstrptr: strnumber,
    pub iniversion: boolean,
    pub inopen: integer,
    pub inputfile: *mut unicodefile,
    pub inputptr: integer,
    pub inputstack: *mut instaterecord,
    pub insdisc: boolean,
    pub insertpenalties: integer,
    pub insertsrcspecialauto: boolean,
    pub insertsrcspecialeverymath: boolean,
    pub insertsrcspecialeverypar: boolean,
    pub insertsrcspecialeveryvbox: boolean,
    pub interaction: eightbits,
    pub interactionoption: eightbits,
    pub interrupt: integer,
    pub ishyph: boolean,
    pub isincsname: boolean,
    pub italicbase: *mut integer,
    pub jobname: strnumber,
    pub jrandom: eightbits,
    pub justbox: halfword,
    pub kernbase: *mut integer,
    pub resource_search_state: ResourceSearchState,
    pub l: eightbits,
    pub last: integer,
    pub lastbadness: integer,
    pub lastbop: integer,
    pub lastglue: halfword,
    pub lastkern: scaled,
    pub lastleftmostchar: halfword,
    pub lastlinefill: halfword,
    pub lastnodetype: integer,
    pub lastpenalty: integer,
    pub lastrightmostchar: halfword,
    pub lastspecialline: halfword,
    pub lfthit: boolean,
    pub lhyf: integer,
    pub ligaturepresent: boolean,
    pub ligkernbase: *mut integer,
    pub ligstack: halfword,
    pub line: integer,
    pub linediff: integer,
    pub linestack: *mut integer,
    pub loadedfontdesignsize: scaled,
    pub loadedfontflags: i8,
    pub loadedfontletterspace: scaled,
    pub loadedfontmapping: voidpointer,
    pub logfile: alphafile,
    pub logopened: boolean,
    pub lomemmax: halfword,
    pub longhelpseen: boolean,
    pub longstate: eightbits,
    pub magicoffset: integer,
    pub magset: integer,
    pub mainf: internalfontnumber,
    pub mainh: halfword,
    pub maini: fourquarters,
    pub mainj: fourquarters,
    pub maink: fontindex,
    pub mainp: halfword,
    pub mainpp: halfword,
    pub mainppp: halfword,
    pub mains: integer,
    pub mappedtext: *mut UTF16code,
    pub maxbufstack: integer,
    pub maxh: scaled,
    pub maxhyphchar: integer,
    pub maxinopen: integer,
    pub maxinstack: integer,
    pub maxneststack: integer,
    pub maxopused: trieopcode,
    pub maxparamstack: integer,
    pub maxprintline: integer,
    pub maxpush: integer,
    pub maxreghelpline: strnumber,
    pub maxregnum: halfword,
    pub maxsavestack: integer,
    pub maxstrings: integer,
    pub maxv: scaled,
    pub membot: integer,
    pub memend: halfword,
    pub memmax: integer,
    pub memmin: integer,
    pub memtop: integer,
    pub microseconds: integer,
    pub minimaldemerits: [integer; 4],
    pub minimumdemerits: integer,
    pub mlistpenalties: boolean,
    pub mltexenabledp: boolean,
    pub mltexp: boolean,
    pub nameinprogress: boolean,
    pub namelength: integer,
    pub nameoffile: *mut UTF8code,
    pub nativefonttypeflag: integer,
    pub nativelen: integer,
    pub nativetext: *mut UTF16code,
    pub nativetextsize: integer,
    pub nest: *mut liststaterecord,
    pub nestptr: integer,
    pub nestsize: integer,
    pub nonewcontrolsequence: boolean,
    pub noshrinkerroryet: boolean,
    pub nullcharacter: fourquarters,
    pub nulldelimiter: fourquarters,
    pub oldselectorignorederr: eightbits,
    pub oldsetting: eightbits,
    pub openparens: integer,
    pub opstart: [integer; 256],
    pub outputactive: boolean,
    pub outputcanend: boolean,
    pub packbeginline: integer,
    pub pagecontents: eightbits,
    pub pagemaxdepth: scaled,
    pub pagesofar: [scaled; 8],
    pub pagetail: halfword,
    pub parambase: *mut integer,
    pub paramptr: integer,
    pub paramsize: integer,
    pub paramstack: *mut halfword,
    pub parloc: halfword,
    pub partoken: halfword,
    pub passive: halfword,
    pub passnumber: halfword,
    pub pdflastxpos: integer,
    pub pdflastypos: integer,
    pub poolptr: poolpointer,
    pub poolsize: integer,
    pub preadjusttail: halfword,
    pub prevclass: integer,
    pub prim: [twohalves; 2101],
    pub primused: halfword,
    pub printednode: halfword,
    pub pseudofiles: halfword,
    pub pstack: [halfword; 9],
    pub quotedfilename: boolean,
    pub radix: smallnumber,
    pub randoms: [integer; 55],
    pub randomseed: scaled,
    pub readfile: [unicodefile; 16],
    pub readopen: [eightbits; 17],
    pub readyalready: integer,
    pub restrictedshell: i32,
    pub rhyf: integer,
    pub rightptr: halfword,
    pub rover: halfword,
    pub rthit: boolean,
    pub sachain: halfword,
    pub salevel: quarterword,
    pub sanull: memoryword,
    pub saroot: [halfword; 8],
    pub savearitherror: boolean,
    pub savenativelen: integer,
    pub saveptr: integer,
    pub savesize: integer,
    pub savestack: *mut memoryword,
    pub scannerstatus: eightbits,
    pub secondindent: scaled,
    pub secondpass: boolean,
    pub secondwidth: scaled,
    pub selector: eightbits,
    pub setboxallowed: boolean,
    pub shellenabledp: i32,
    pub shownmode: i16,
    pub skewchar: *mut integer,
    pub skipline: integer,
    pub sourcefilenamestack: *mut strnumber,
    pub spaceclass: integer,
    pub speclog: [integer; 29],
    pub stacksize: integer,
    pub stopatspace: boolean,
    pub stringvacancies: integer,
    pub strpool: *mut packedUTF16code,
    pub strptr: strnumber,
    pub strstart: *mut poolpointer,
    pub tally: integer,
    pub tempptr: halfword,
    pub termin: unicodefile,
    pub termoffset: integer,
    pub texremainder: scaled,
    pub tfmfile: bytefile,
    pub tfmtemp: i32,
    pub threshold: integer,
    pub totalpages: integer,
    pub totalshrink: [scaled; 4],
    pub totalstretch: [scaled; 4],
    pub trickbuf: [UnicodeScalar; 256],
    pub trickcount: integer,
    pub triec: *mut packedUTF16code,
    pub triehash: *mut triepointer,
    pub triel: *mut triepointer,
    pub triemax: triepointer,
    pub triemin: [triepointer; 65536],
    pub trienotready: boolean,
    pub trieo: *mut trieopcode,
    pub trieoplang: [eightbits; 35112],
    pub trieopptr: integer,
    pub trieopval: [trieopcode; 35112],
    pub trieptr: triepointer,
    pub trier: *mut triepointer,
    pub triesize: integer,
    pub trietaken: *mut boolean,
    pub trietrc: *mut quarterword,
    pub trietrl: *mut triepointer,
    pub trietro: *mut triepointer,
    pub trieused: [trieopcode; 256],
    pub twotothe: [integer; 31],
    pub useerrhelp: boolean,
    pub varused: integer,
    pub warningindex: halfword,
    pub widthbase: *mut integer,
    pub writefile: [alphafile; 16],
    pub writeloc: halfword,
    pub writeopen: [boolean; 18],
    pub xtxligaturepresent: boolean,
    pub zeqtb: PagedView<memoryword>,
    pub zmem: *mut memoryword,
    pub zzzaa: [quarterword; 1114734],
    pub zzzab: [integer; 70223],
}

/// Declares [`PortableTexHeap`] and its allocation, pointer refresh and image codec from one field list.
macro_rules! portable_heap {
    (
        sized { $($sized:ident: $sized_ty:ty [$bound:ident] => $sized_ptr:ident,)* }
        trie_scratch { $($scratch:ident: $scratch_ty:ty => $scratch_ptr:ident,)* }
        transient { $($(#[$transient_meta:meta])* $transient:ident: $transient_ty:ty,)* }
    ) => {
        /// Heap owning engine arrays, `Clone` deep copies them so no two states ever share a buffer.
        #[derive(Clone, Default)]
        pub(crate) struct PortableTexHeap {
            /// Main memory, indexed from `memmin`.
            mem: Vec<memoryword>,
            /// Paged `hash`, from absolute index `hashoffset` through `eqtbtop`.
            hash_paged: PagedArray<twohalves>,
            /// Paged `eqtb`, from absolute index 0 through `eqtbtop`.
            eqtb_paged: PagedArray<memoryword>,
            /// Platform font handles per font number, stored as integers in images.
            fontlayoutengine_storage: Vec<voidpointer>,
            /// Font mapping handles per font number, always null in this engine.
            fontmapping_storage: Vec<voidpointer>,
            /// Native text run buffer, grown by `ensure_nativetext_capacity` to `nativetextsize`.
            nativetext_storage: Vec<UTF16code>,
            $($sized: Vec<$sized_ty>,)*
            $($scratch: Vec<$scratch_ty>,)*
            $($(#[$transient_meta])* $transient: $transient_ty,)*
        }

        /// Allocates every array at the sizes already stored in `state`.
        fn allocate_heap_arrays(state: &PortableTexState, heap: &mut PortableTexHeap) {
            heap.mem = zeroed_vec(slots(state.memtop - state.memmin));
            heap.eqtb_paged = PagedArray::new(
                0,
                slots(state.eqtbtop),
                eqtb_codepoint_default,
                eqtb_sig_range,
            );
            heap.hash_paged = PagedArray::new(
                hashoffset as usize,
                slots(state.eqtbtop),
                hash_codepoint_default,
                hash_sig_range,
            );
            heap.fontlayoutengine_storage = vec![nullptr; slots(state.fontmax)];
            heap.fontmapping_storage = vec![nullptr; slots(state.fontmax)];
            heap.nativetext_storage = Vec::new();
            $(heap.$sized = zeroed_vec(slots(state.$bound));)*
            $(heap.$scratch = zeroed_vec(slots(state.triesize));)*
        }

        /// Points every raw array field of `state` at its storage in `heap`.
        fn refresh_runtime_pointers(state: &mut PortableTexState, heap: &mut PortableTexHeap) {
            state.zmem = pointer_or_null(&mut heap.mem).wrapping_offset(-(state.memmin as isize));
            state.hash = PagedView::new(&mut heap.hash_paged);
            state.zeqtb = PagedView::new(&mut heap.eqtb_paged);
            state.fontlayoutengine = pointer_or_null(&mut heap.fontlayoutengine_storage);
            state.fontmapping = pointer_or_null(&mut heap.fontmapping_storage);
            state.nativetext = pointer_or_null(&mut heap.nativetext_storage);
            $(state.$sized_ptr = pointer_or_null(&mut heap.$sized);)*
            $(state.$scratch_ptr = pointer_or_null(&mut heap.$scratch);)*
        }

        /// Frees the trie builder scratch, only valid once the trie is packed.
        fn free_trie_scratch(heap: &mut PortableTexHeap) {
            $(heap.$scratch = Vec::new();)*
        }

        /// Bytes allocated for every engine array, the dominant resident footprint.
        fn heap_array_bytes(heap: &PortableTexHeap) -> usize {
            let mut total = vec_bytes(&heap.mem)
                + heap.eqtb_paged.resident_bytes()
                + heap.hash_paged.resident_bytes()
                + vec_bytes(&heap.fontlayoutengine_storage)
                + vec_bytes(&heap.fontmapping_storage)
                + vec_bytes(&heap.nativetext_storage);
            $(total += vec_bytes(&heap.$sized);)*
            $(total += vec_bytes(&heap.$scratch);)*
            total
        }

        fn encode_heap(state: &PortableTexState, heap: &PortableTexHeap, out: &mut Vec<u8>) {
            // Only the live low and high regions of `mem` are stored, the free gap between them is not.
            let lo_len = heap.mem.len().min(slots(state.lomemmax - state.memmin));
            let hi_start = usize::try_from(state.himemmin - state.memmin).map_or(0, |start| start.min(heap.mem.len()));
            let hi_len = heap.mem.len().min(slots(state.memend - state.memmin)).saturating_sub(hi_start);
            write_vec_ranges(out, &heap.mem, &[(0, lo_len), (hi_start, hi_len)]);
            write_paged(out, &heap.eqtb_paged);
            write_paged(out, &heap.hash_paged);
            write_handles(out, &heap.fontlayoutengine_storage);
            write_handles(out, &heap.fontmapping_storage);
            write_vec_ranges(out, &heap.nativetext_storage, &[(0, heap.nativetext_storage.len())]);
            $(write_vec_used(out, &heap.$sized);)*
            $(write_vec_used(out, &heap.$scratch);)*
        }

        /// Decodes the arrays of an image whose `state` already passed [`validate_state`].
        fn decode_heap(
            state: &PortableTexState,
            input: &mut ImageReader<'_>,
        ) -> Result<PortableTexHeap, FormatImageError> {
            let live = |bound: integer| if state.initialized { slots(bound) } else { 0 };
            let eqtb_end = live(state.eqtbtop);
            let hash_base = if state.initialized { hashoffset as usize } else { 0 };
            let scratch_len = live(state.triesize);
            // Packing the trie frees its builder scratch, an unpacked trie still needs it.
            let scratch_ok = |len: usize| len == scratch_len || (len == 0 && state.trienotready == false_0);
            Ok(PortableTexHeap {
                mem: read_vec(input, "mem", |len| len == live(state.memtop - state.memmin))?,
                eqtb_paged: read_paged(input, "eqtb", 0, eqtb_end, eqtb_codepoint_default, eqtb_sig_range)?,
                hash_paged: read_paged(input, "hash", hash_base, eqtb_end, hash_codepoint_default, hash_sig_range)?,
                fontlayoutengine_storage: read_handles(input, "fontlayoutengine", live(state.fontmax))?,
                fontmapping_storage: read_handles(input, "fontmapping", live(state.fontmax))?,
                nativetext_storage: read_vec(input, "nativetext", |len| len == state.nativetextsize as usize)?,
                $($sized: read_vec(input, stringify!($sized), |len| len == live(state.$bound))?,)*
                $($scratch: read_vec(input, stringify!($scratch), scratch_ok)?,)*
                $($transient: Default::default(),)*
            })
        }
    };
}

portable_heap! {
    sized {
        buffer_storage: UnicodeScalar [bufsize] => buffer,
        nest_storage: liststaterecord [nestsize] => nest,
        savestack_storage: memoryword [savesize] => savestack,
        inputstack_storage: instaterecord [stacksize] => inputstack,
        inputfile_storage: unicodefile [maxinopen] => inputfile,
        eofseen_storage: boolean [maxinopen] => eofseen,
        linestack_storage: integer [maxinopen] => linestack,
        grpstack_storage: savepointer [maxinopen] => grpstack,
        ifstack_storage: halfword [maxinopen] => ifstack,
        sourcefilenamestack_storage: strnumber [maxinopen] => sourcefilenamestack,
        fullsourcefilenamestack_storage: strnumber [maxinopen] => fullsourcefilenamestack,
        paramstack_storage: halfword [paramsize] => paramstack,
        hyphword_storage: strnumber [hyphsize] => hyphword,
        hyphlist_storage: halfword [hyphsize] => hyphlist,
        hyphlink_storage: hyphpointer [hyphsize] => hyphlink,
        strstart_storage: poolpointer [maxstrings] => strstart,
        strpool_storage: packedUTF16code [poolsize] => strpool,
        fontinfo_storage: fmemoryword [fontmemsize] => fontinfo,
        bcharlabel_storage: fontindex [fontmax] => bcharlabel,
        charbase_storage: integer [fontmax] => charbase,
        widthbase_storage: integer [fontmax] => widthbase,
        heightbase_storage: integer [fontmax] => heightbase,
        depthbase_storage: integer [fontmax] => depthbase,
        italicbase_storage: integer [fontmax] => italicbase,
        ligkernbase_storage: integer [fontmax] => ligkernbase,
        kernbase_storage: integer [fontmax] => kernbase,
        extenbase_storage: integer [fontmax] => extenbase,
        parambase_storage: integer [fontmax] => parambase,
        fontarea_storage: strnumber [fontmax] => fontarea,
        fontname_storage: strnumber [fontmax] => fontname,
        fontbc_storage: UTF16code [fontmax] => fontbc,
        fontec_storage: UTF16code [fontmax] => fontec,
        fontbchar_storage: ninebits [fontmax] => fontbchar,
        fontfalsebchar_storage: ninebits [fontmax] => fontfalsebchar,
        fontcheck_storage: fourquarters [fontmax] => fontcheck,
        fontdsize_storage: scaled [fontmax] => fontdsize,
        fontsize_storage: scaled [fontmax] => fontsize,
        fontflags_storage: i8 [fontmax] => fontflags,
        fontglue_storage: halfword [fontmax] => fontglue,
        fontletterspace_storage: scaled [fontmax] => fontletterspace,
        fontparams_storage: fontindex [fontmax] => fontparams,
        fontused_storage: boolean [fontmax] => fontused,
        hyphenchar_storage: integer [fontmax] => hyphenchar,
        skewchar_storage: integer [fontmax] => skewchar,
        trietrc_storage: quarterword [triesize] => trietrc,
        trietrl_storage: triepointer [triesize] => trietrl,
        trietro_storage: triepointer [triesize] => trietro,
    }
    trie_scratch {
        triec_storage: packedUTF16code => triec,
        triehash_storage: triepointer => triehash,
        triel_storage: triepointer => triel,
        trieo_storage: trieopcode => trieo,
        trier_storage: triepointer => trier,
        trietaken_storage: boolean => trietaken,
    }
    transient {
        /// Source tracking intern table, `SrcId` minus 1 indexes it in first touch order.
        src_spans: Vec<RawSpan>,
        /// Source tracking dedup map, equal spans share one `SrcId`.
        src_dedup: std::collections::HashMap<RawSpan, SrcId>,
        /// Paged shadow of `mem` holding the `SrcId` stamped on the node at each address.
        node_src: PagedArray<u32>,
        /// Per UTF-16 unit source ids of the native run being collected, consumed when it is shaped.
        src_native_offsets: Vec<SrcId>,
        /// Arena of parent linked enclosing construct frames, addressed 1 based by `cur_stack_head`.
        src_stack_cells: Vec<SrcStackCell>,
        /// Opening token spans of the live math groups and `\left` constructs.
        src_grp_stack: Vec<SrcId>,
        /// Paged shadow of `mem` holding the `cur_stack_head` live when each node was allocated.
        node_stack: PagedArray<u32>,
        /// Input byte range of the scalar in each buffer slot, the limit slot ends past the line break.
        src_buf_bytes: Vec<(u32, u32)>,
    }
}

// Engine array sizes, a web2c `array[0..N]` gets N+1 slots, see `slots`.
const MEM_TOP: integer = 999_999;
const HASH_EXTRA: integer = 600_000;
const BUF_SIZE: integer = 200_000;
const NEST_SIZE: integer = 1_000;
const MAX_IN_OPEN: integer = 15;
const PARAM_SIZE: integer = 20_000;
const SAVE_SIZE: integer = 200_000;
const STACK_SIZE: integer = 10_000;
const POOL_SIZE: integer = 6_250_000;
const MAX_STRINGS: integer = 500_000;
const FONT_MEM_SIZE: integer = 1_000_000;
const FONT_MAX: integer = 500;
const TRIE_SIZE: integer = 1_100_000;
const HYPH_SIZE: integer = 8_191;

/// Slot count of a web2c `array[0..bound]`, zero for a negative bound.
fn slots(bound: integer) -> usize {
    usize::try_from(bound).map_or(0, |bound| bound + 1)
}

fn vec_bytes<T>(v: &Vec<T>) -> usize {
    v.capacity() * core::mem::size_of::<T>()
}

/// Magic of a serialized format image, followed by the little endian image layout version.
const PORTABLE_FORMAT_MAGIC: [u8; 6] = *b"MTXfmt";

/// Image layout version, 7 dropped pointer width from the tag and the pointer view of memory words.
const PORTABLE_FORMAT_VERSION: u16 = 7;

/// Hash of the generated sources and layout constants, so an image from another build is refused.
const BUILD_FINGERPRINT: u64 = 0xe96abe755e6fab55;

/// Validation tag bytes: magic, layout version, byte order and build fingerprint.
const VALIDATION_TAG_LEN: usize = 17;

/// Header bytes: validation tag, body length and body checksum.
const HEADER_LEN: usize = VALIDATION_TAG_LEN + 16;

/// Byte order of the plain values an image copies, the only target property an image depends on.
const fn byte_order() -> u8 {
    if cfg!(target_endian = "big") {
        1
    } else {
        0
    }
}

fn validation_tag() -> [u8; VALIDATION_TAG_LEN] {
    let mut tag = [0u8; VALIDATION_TAG_LEN];
    tag[..6].copy_from_slice(&PORTABLE_FORMAT_MAGIC);
    tag[6..8].copy_from_slice(&PORTABLE_FORMAT_VERSION.to_le_bytes());
    tag[8] = byte_order();
    tag[9..].copy_from_slice(&BUILD_FINGERPRINT.to_le_bytes());
    tag
}

/// Word wise multiplicative checksum of an image body, it detects corruption, not tampering.
fn image_checksum(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut hash = bytes.len() as u64;
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let mut word = [0u8; 8];
        word.copy_from_slice(chunk);
        hash = (hash.rotate_left(5) ^ u64::from_le_bytes(word)).wrapping_mul(K);
    }
    for &byte in chunks.remainder() {
        hash = (hash.rotate_left(5) ^ u64::from(byte)).wrapping_mul(K);
    }
    hash
}

/// Why a serialized format image was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatImageError {
    /// The bytes do not start with the format image magic.
    NotAnImage,
    /// The image was written with another image layout version.
    Version(u16),
    /// The image was written on a target with the other byte order.
    ByteOrder,
    /// The image was written by another engine build.
    OtherBuild,
    /// The image ends before a part its header or body declares.
    Truncated,
    /// Bytes remain after the last part.
    TrailingBytes,
    /// The body checksum does not match the header.
    ChecksumMismatch,
    /// A state field holds a value its type cannot hold.
    InvalidField(&'static str),
    /// Decoded sizes disagree with each other or with the layout this build allocates.
    Geometry(&'static str),
}

impl FormatImageError {
    fn in_field(self, field: &'static str) -> Self {
        match self {
            Self::InvalidField(_) => Self::InvalidField(field),
            other => other,
        }
    }
}

impl core::fmt::Display for FormatImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAnImage => f.write_str("not a format image"),
            Self::Version(found) => write!(
                f,
                "format image layout version {found}, this build reads {PORTABLE_FORMAT_VERSION}"
            ),
            Self::ByteOrder => f.write_str("format image is from a target with the other byte order"),
            Self::OtherBuild => f.write_str("format image is from another engine build"),
            Self::Truncated => f.write_str("format image is truncated"),
            Self::TrailingBytes => f.write_str("format image has trailing bytes"),
            Self::ChecksumMismatch => f.write_str("format image checksum mismatch"),
            Self::InvalidField(field) => write!(f, "format image field `{field}` holds an invalid value"),
            Self::Geometry(what) => write!(f, "format image `{what}` has an inconsistent size"),
        }
    }
}

impl std::error::Error for FormatImageError {}

/// Cursor over image bytes whose reads fail instead of running past the end.
struct ImageReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ImageReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], FormatImageError> {
        let end = self
            .cursor
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(FormatImageError::Truncated)?;
        let slice = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, FormatImageError> {
        let mut word = [0u8; 4];
        word.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(word))
    }

    fn u64(&mut self) -> Result<u64, FormatImageError> {
        let mut word = [0u8; 8];
        word.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(word))
    }

    fn usize(&mut self) -> Result<usize, FormatImageError> {
        usize::try_from(self.u64()?).map_err(|_| FormatImageError::Truncated)
    }
}

/// Field wise image encoding, implemented only for types valid as all zero bytes.
trait ImageCodec: Sized {
    /// Encoded bytes per value.
    const ENCODED_SIZE: usize;

    /// Whether the value is all zero, used to drop the zero tail of an array.
    fn is_zero(&self) -> bool;

    /// Appends the encoding of every value in `items`.
    fn encode(items: &[Self], out: &mut Vec<u8>);

    /// Decodes `count` values into `dst`, which must be valid for `count` writes.
    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError>;
}

/// Bytes of plain values, the caller guarantees `T` has no padding and no uninitialized bytes.
unsafe fn plain_bytes<T>(items: &[T]) -> &[u8] {
    core::slice::from_raw_parts(items.as_ptr().cast::<u8>(), core::mem::size_of_val(items))
}

macro_rules! plain_image_codec {
    ($($ty:ty),* $(,)?) => {$(
        impl ImageCodec for $ty {
            const ENCODED_SIZE: usize = core::mem::size_of::<$ty>();

            fn is_zero(&self) -> bool {
                unsafe { plain_bytes(core::slice::from_ref(self)) }.iter().all(|&byte| byte == 0)
            }

            fn encode(items: &[Self], out: &mut Vec<u8>) {
                out.extend_from_slice(unsafe { plain_bytes(items) });
            }

            unsafe fn decode(
                dst: *mut Self,
                count: usize,
                input: &mut ImageReader<'_>,
            ) -> Result<(), FormatImageError> {
                let len = count
                    .checked_mul(Self::ENCODED_SIZE)
                    .ok_or(FormatImageError::Truncated)?;
                let bytes = input.take(len)?;
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst.cast::<u8>(), len);
                Ok(())
            }
        }
    )*};
}

// No padding and every byte initialized, the word unions are zero filled by their `Default`.
plain_image_codec!(
    i8,
    i16,
    u16,
    i32,
    u32,
    u64,
    memoryword,
    fmemoryword,
    twohalves,
    fourquarters,
    instaterecord,
    ResourceSearchState,
);

impl ImageCodec for bool {
    const ENCODED_SIZE: usize = 1;

    fn is_zero(&self) -> bool {
        !*self
    }

    fn encode(items: &[Self], out: &mut Vec<u8>) {
        out.extend(items.iter().map(|&value| u8::from(value)));
    }

    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError> {
        for (index, &byte) in input.take(count)?.iter().enumerate() {
            let value = match byte {
                0 => false,
                1 => true,
                _ => return Err(FormatImageError::InvalidField("bool")),
            };
            dst.add(index).write(value);
        }
        Ok(())
    }
}

// Raw pointers are process local, an image stores nothing for them and decodes them as null.
impl<T> ImageCodec for *mut T {
    const ENCODED_SIZE: usize = 0;

    fn is_zero(&self) -> bool {
        true
    }

    fn encode(_items: &[Self], _out: &mut Vec<u8>) {}

    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        _input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError> {
        for index in 0..count {
            dst.add(index).write(core::ptr::null_mut());
        }
        Ok(())
    }
}

// Views are rebound by `refresh_runtime_pointers` after decoding.
impl<T: Copy + Default> ImageCodec for PagedView<T> {
    const ENCODED_SIZE: usize = 0;

    fn is_zero(&self) -> bool {
        true
    }

    fn encode(_items: &[Self], _out: &mut Vec<u8>) {}

    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        _input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError> {
        for index in 0..count {
            dst.add(index).write(PagedView(core::ptr::null_mut()));
        }
        Ok(())
    }
}

impl<T: ImageCodec, const N: usize> ImageCodec for [T; N] {
    const ENCODED_SIZE: usize = T::ENCODED_SIZE * N;

    fn is_zero(&self) -> bool {
        self.iter().all(T::is_zero)
    }

    fn encode(items: &[Self], out: &mut Vec<u8>) {
        T::encode(items.as_flattened(), out);
    }

    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError> {
        let count = count.checked_mul(N).ok_or(FormatImageError::Truncated)?;
        T::decode(dst.cast::<T>(), count, input)
    }
}

// Field by field, the record has padding after `modefield`.
impl ImageCodec for liststaterecord {
    const ENCODED_SIZE: usize = 2 + 5 * 4 + 8;

    fn is_zero(&self) -> bool {
        self.modefield == 0
            && [self.headfield, self.tailfield, self.eTeXauxfield, self.pgfield, self.mlfield]
                .iter()
                .all(|&field| field == 0)
            && self.auxfield.is_zero()
    }

    fn encode(items: &[Self], out: &mut Vec<u8>) {
        for record in items {
            i16::encode(&[record.modefield], out);
            i32::encode(
                &[
                    record.headfield,
                    record.tailfield,
                    record.eTeXauxfield,
                    record.pgfield,
                    record.mlfield,
                ],
                out,
            );
            memoryword::encode(&[record.auxfield], out);
        }
    }

    unsafe fn decode(
        dst: *mut Self,
        count: usize,
        input: &mut ImageReader<'_>,
    ) -> Result<(), FormatImageError> {
        for index in 0..count {
            let record = dst.add(index);
            i16::decode(core::ptr::addr_of_mut!((*record).modefield), 1, input)?;
            i32::decode(core::ptr::addr_of_mut!((*record).headfield), 1, input)?;
            i32::decode(core::ptr::addr_of_mut!((*record).tailfield), 1, input)?;
            i32::decode(core::ptr::addr_of_mut!((*record).eTeXauxfield), 1, input)?;
            i32::decode(core::ptr::addr_of_mut!((*record).pgfield), 1, input)?;
            i32::decode(core::ptr::addr_of_mut!((*record).mlfield), 1, input)?;
            memoryword::decode(core::ptr::addr_of_mut!((*record).auxfield), 1, input)?;
        }
        Ok(())
    }
}

/// Writes the length of `v`, then only the element ranges in `ranges`, the rest decodes as zero.
fn write_vec_ranges<T: ImageCodec>(out: &mut Vec<u8>, v: &[T], ranges: &[(usize, usize)]) {
    out.extend_from_slice(&(v.len() as u64).to_le_bytes());
    let ranges = ranges.iter().filter(|&&(_, len)| len > 0).collect::<Vec<_>>();
    out.extend_from_slice(&(ranges.len() as u32).to_le_bytes());
    for &&(start, len) in &ranges {
        out.extend_from_slice(&(start as u64).to_le_bytes());
        out.extend_from_slice(&(len as u64).to_le_bytes());
        T::encode(&v[start..start + len], out);
    }
}

/// Writes `v` without its zero tail, most engine arrays are sized far beyond what a format uses.
fn write_vec_used<T: ImageCodec>(out: &mut Vec<u8>, v: &[T]) {
    let used = v.iter().rposition(|item| !item.is_zero()).map_or(0, |last| last + 1);
    write_vec_ranges(out, v, &[(0, used)]);
}

/// Decodes a vector from [`write_vec_ranges`], refusing a length `accept` rejects before allocating.
fn read_vec<T: ImageCodec>(
    input: &mut ImageReader<'_>,
    what: &'static str,
    accept: impl Fn(usize) -> bool,
) -> Result<Vec<T>, FormatImageError> {
    let full = input.usize()?;
    if !accept(full) {
        return Err(FormatImageError::Geometry(what));
    }
    let ranges = input.u32()?;
    let mut v = Vec::<T>::with_capacity(full);
    // Sound because every `ImageCodec` type is valid as zero bytes, and ranges stay within `full`.
    unsafe {
        core::ptr::write_bytes(v.as_mut_ptr(), 0, full);
        v.set_len(full);
        for _ in 0..ranges {
            let start = input.usize()?;
            let len = input.usize()?;
            if start.checked_add(len).is_none_or(|end| end > full) {
                return Err(FormatImageError::Geometry(what));
            }
            T::decode(v.as_mut_ptr().add(start), len, input)?;
        }
    }
    Ok(v)
}

/// Writes font handles as integers, they index the font platform rather than memory.
fn write_handles(out: &mut Vec<u8>, handles: &[voidpointer]) {
    let values = handles.iter().map(|&handle| handle as usize as u64).collect::<Vec<_>>();
    write_vec_used(out, &values);
}

fn read_handles(
    input: &mut ImageReader<'_>,
    what: &'static str,
    expected: usize,
) -> Result<Vec<voidpointer>, FormatImageError> {
    read_vec::<u64>(input, what, |len| len == expected)?
        .into_iter()
        .map(|value| {
            usize::try_from(value)
                .map(|handle| handle as voidpointer)
                .map_err(|_| FormatImageError::InvalidField(what))
        })
        .collect()
}

/// Writes a [`PagedArray`] as its geometry and only its faulted pages.
fn write_paged<T: ImageCodec + Copy + Default>(out: &mut Vec<u8>, arr: &PagedArray<T>) {
    for value in [arr.base, arr.end] {
        out.extend_from_slice(&(value as u64).to_le_bytes());
    }
    out.extend_from_slice(&(arr.pages.len() as u64).to_le_bytes());
    let present = arr.pages.iter().enumerate().filter(|(_, page)| page.is_some()).count();
    out.extend_from_slice(&(present as u32).to_le_bytes());
    for (index, page) in arr.pages.iter().enumerate() {
        if let Some(page) = page {
            out.extend_from_slice(&(index as u64).to_le_bytes());
            T::encode(page, out);
        }
    }
}

/// Decodes a [`write_paged`] array whose geometry must be `base..end`.
fn read_paged<T: ImageCodec + Copy + Default>(
    input: &mut ImageReader<'_>,
    what: &'static str,
    base: usize,
    end: usize,
    default_fn: fn(usize) -> T,
    sig_range: fn(usize) -> (usize, usize),
) -> Result<PagedArray<T>, FormatImageError> {
    let geometry = [input.usize()?, input.usize()?];
    if geometry != [base, end] {
        return Err(FormatImageError::Geometry(what));
    }
    let npages = input.usize()?;
    if npages != (end - base).div_ceil(PAGE_LEN) {
        return Err(FormatImageError::Geometry(what));
    }
    let mut pages: Vec<Option<Box<[T]>>> = (0..npages).map(|_| None).collect();
    for _ in 0..input.u32()? {
        let slot = pages
            .get_mut(input.usize()?)
            .ok_or(FormatImageError::Geometry(what))?;
        let mut page = vec![T::default(); PAGE_LEN];
        // Sound because `page` holds exactly `PAGE_LEN` values.
        unsafe { T::decode(page.as_mut_ptr(), PAGE_LEN, input)? };
        *slot = Some(page.into_boxed_slice());
    }
    Ok(PagedArray {
        base,
        end,
        pages,
        default_fn,
        sig_range,
    })
}

/// Cross checks the decoded scalars that generated code indexes arrays by against this build's layout.
fn validate_state(state: &PortableTexState) -> Result<(), FormatImageError> {
    let check = |ok: bool, what: &'static str| {
        if ok {
            Ok(())
        } else {
            Err(FormatImageError::Geometry(what))
        }
    };
    if !state.initialized {
        return Ok(());
    }
    check(state.memmin == 0 && state.membot == 0, "memmin")?;
    check(state.memtop == MEM_TOP && state.memmax == MEM_TOP, "memtop")?;
    check(
        0 <= state.lomemmax
            && state.lomemmax < state.himemmin
            && state.himemmin <= state.memend
            && state.memend <= state.memmax,
        "lomemmax, himemmin and memend",
    )?;
    check(
        state.hashextra == HASH_EXTRA && state.eqtbtop == EQTB_SIZE as halfword + HASH_EXTRA,
        "eqtbtop",
    )?;
    check(
        state.bufsize == BUF_SIZE
            && state.nestsize == NEST_SIZE
            && state.maxinopen == MAX_IN_OPEN
            && state.paramsize == PARAM_SIZE
            && state.savesize == SAVE_SIZE
            && state.stacksize == STACK_SIZE
            && state.hyphsize == HYPH_SIZE
            && state.triesize == TRIE_SIZE,
        "stack sizes",
    )?;
    check(
        state.poolsize == POOL_SIZE && (0..=state.poolsize).contains(&state.poolptr),
        "poolptr",
    )?;
    check(
        state.maxstrings == MAX_STRINGS
            && state.strptr >= TOO_BIG_CHAR
            && state.strptr - TOO_BIG_CHAR <= state.maxstrings,
        "strptr",
    )?;
    check(
        state.fontmemsize == FONT_MEM_SIZE && (0..=state.fontmemsize).contains(&state.fmemptr),
        "fmemptr",
    )?;
    check(
        state.fontmax == FONT_MAX && (0..=state.fontmax).contains(&state.fontptr),
        "fontptr",
    )?;
    check(state.nativetextsize >= 0, "nativetextsize")
}

/// Nulls every process local file slot and marks every stream closed, as TeX's `initialize` does.
fn close_file_slots(state: &mut PortableTexState) {
    state.readfile = [core::ptr::null_mut(); STREAM_COUNT];
    state.writefile = [core::ptr::null_mut(); STREAM_COUNT];
    state.termin = core::ptr::null_mut();
    state.logfile = core::ptr::null_mut();
    state.tfmfile = core::ptr::null_mut();
    state.terminal_output = core::ptr::null_mut();
    state.nameoffile = core::ptr::null_mut();
    state.mappedtext = core::ptr::null_mut();
    state.readopen = [READ_CLOSED; STREAM_COUNT + 1];
    state.writeopen = [false_0; STREAM_COUNT + 2];
}

fn boxed_state_zeroed() -> Box<PortableTexState> {
    let mut state = Box::<PortableTexState>::new_uninit();
    // Sound because all zero bytes are a valid value for every plain field of the state.
    unsafe {
        core::ptr::write_bytes(state.as_mut_ptr(), 0, 1);
        state.assume_init()
    }
}

/// Copies `state` straight into a new box, the state is several megabytes and must not cross the stack.
fn boxed_state_copy(state: &PortableTexState) -> Box<PortableTexState> {
    let mut copy = Box::<PortableTexState>::new_uninit();
    // Sound because `PortableTexState` is `Copy`, a bitwise copy is a valid value.
    unsafe {
        core::ptr::copy_nonoverlapping(state, copy.as_mut_ptr(), 1);
        copy.assume_init()
    }
}

/// Sets the array sizes of a fresh INITEX state and allocates its arrays.
fn allocate_initial_arrays(state: &mut PortableTexState, heap: &mut PortableTexHeap) {
    state.iniversion = true_0;
    state.membot = 0;
    state.memmin = state.membot;
    // Math fragments use a tiny fraction of TeX's worst case memory, the LaTeX format build fits well.
    state.memtop = MEM_TOP;
    state.memmax = state.memtop;
    state.hashextra = HASH_EXTRA;
    state.eqtbtop = EQTB_SIZE as halfword + state.hashextra;
    state.bufsize = BUF_SIZE;
    state.nestsize = NEST_SIZE;
    state.maxinopen = MAX_IN_OPEN;
    state.paramsize = PARAM_SIZE;
    state.savesize = SAVE_SIZE;
    state.stacksize = STACK_SIZE;
    state.dvibufsize = 16_384;
    state.poolsize = POOL_SIZE;
    state.maxstrings = MAX_STRINGS;
    state.fontmemsize = FONT_MEM_SIZE;
    state.fontmax = FONT_MAX;
    // The LaTeX `\patterns` need over 700K trie nodes, the builder scratch is freed once packed.
    state.triesize = TRIE_SIZE;
    state.hyphsize = HYPH_SIZE;
    state.primused = 2_100;
    state.errorline = 79;
    state.halferrorline = 50;
    state.maxprintline = 79;
    state.expanddepth = 10_000;
    allocate_heap_arrays(state, heap);
    refresh_runtime_pointers(state, heap);
}

/// Resets per run input state and compacts arrays so a snapshot keeps only what a format needs.
fn seal_as_format_snapshot(state: &mut PortableTexState, heap: &mut PortableTexHeap) {
    state.curinput = instaterecord::default();
    state.inputptr = 0;
    state.inopen = 0;
    state.baseptr = 0;
    state.scannerstatus = 0;
    state.warningindex = 0;
    state.defref = TEX_NULL;
    state.paramptr = 0;
    state.alignstate = NEUTRAL_ALIGN_STATE;
    state.first = 0;
    state.last = 0;
    state.line = 0;
    state.openparens = 0;
    close_file_slots(state);
    heap.inputstack_storage.fill(instaterecord::default());
    heap.inputfile_storage.fill(core::ptr::null_mut());
    heap.eofseen_storage.fill(false_0);
    heap.linestack_storage.fill(0);
    heap.grpstack_storage.fill(TEX_NULL);
    heap.ifstack_storage.fill(TEX_NULL);
    heap.sourcefilenamestack_storage.fill(0);
    heap.fullsourcefilenamestack_storage.fill(0);
    heap.paramstack_storage.fill(0);
    // Pages still equal to their band defaults are dropped, the bulk of the eqtb and hash savings.
    heap.eqtb_paged.compact();
    heap.hash_paged.compact();
    // A packed trie never runs the builder again, an unpacked one still needs its scratch lazily.
    if state.trienotready == false_0 {
        free_trie_scratch(heap);
    }
    refresh_runtime_pointers(state, heap);
}

/// Serializes a state as a format image: header, plain fields, then the arrays.
fn encode_image(state: &PortableTexState, heap: &PortableTexHeap) -> Vec<u8> {
    let mut out = vec![0u8; HEADER_LEN];
    state.encode_fields(&mut out);
    encode_heap(state, heap, &mut out);
    let body_len = (out.len() - HEADER_LEN) as u64;
    let checksum = image_checksum(&out[HEADER_LEN..]);
    out[..VALIDATION_TAG_LEN].copy_from_slice(&validation_tag());
    out[VALIDATION_TAG_LEN..VALIDATION_TAG_LEN + 8].copy_from_slice(&body_len.to_le_bytes());
    out[VALIDATION_TAG_LEN + 8..HEADER_LEN].copy_from_slice(&checksum.to_le_bytes());
    out
}

/// Checks each part of a validation tag in order, so the error names the first that differs.
fn check_validation_tag(tag: &[u8]) -> Result<(), FormatImageError> {
    let expected = validation_tag();
    if tag[..6] != expected[..6] {
        return Err(FormatImageError::NotAnImage);
    }
    let version = u16::from_le_bytes([tag[6], tag[7]]);
    if version != PORTABLE_FORMAT_VERSION {
        return Err(FormatImageError::Version(version));
    }
    if tag[8] != expected[8] {
        return Err(FormatImageError::ByteOrder);
    }
    if tag[9..] != expected[9..] {
        return Err(FormatImageError::OtherBuild);
    }
    Ok(())
}

/// Decodes and fully validates an [`encode_image`] image.
fn decode_image(
    bytes: &[u8],
) -> Result<(Box<PortableTexState>, Box<PortableTexHeap>), FormatImageError> {
    let mut header = ImageReader::new(bytes);
    check_validation_tag(header.take(VALIDATION_TAG_LEN)?)?;
    let body_len = header.usize()?;
    let checksum = header.u64()?;
    let body = &bytes[HEADER_LEN..];
    if body.len() < body_len {
        return Err(FormatImageError::Truncated);
    }
    if body.len() > body_len {
        return Err(FormatImageError::TrailingBytes);
    }
    if image_checksum(body) != checksum {
        return Err(FormatImageError::ChecksumMismatch);
    }
    let mut input = ImageReader::new(body);
    let mut state = boxed_state_zeroed();
    // Sound because `state` is a valid, uniquely owned state that each field decode overwrites in place.
    unsafe { PortableTexState::decode_fields(&mut *state, &mut input)? };
    validate_state(&state)?;
    // The dense native text buffer must fit the remaining input, which bounds its allocation.
    if (state.nativetextsize as usize).saturating_mul(2) > input.remaining() {
        return Err(FormatImageError::Geometry("nativetextsize"));
    }
    close_file_slots(&mut state);
    state.cur_stack_head = 0;
    let mut heap = Box::new(decode_heap(&state, &mut input)?);
    if input.remaining() != 0 {
        return Err(FormatImageError::TrailingBytes);
    }
    refresh_runtime_pointers(&mut state, &mut heap);
    Ok((state, heap))
}

fn zeroed_vec<T>(len: usize) -> Vec<T>
where
    T: Clone + Default,
{
    vec![T::default(); len]
}

fn pointer_or_null<T>(storage: &mut Vec<T>) -> *mut T {
    if storage.is_empty() {
        core::ptr::null_mut()
    } else {
        storage.as_mut_ptr()
    }
}

/// XeTeX `right_brace_token + "}"`, opening the braces `\write` wraps its token list in.
const WRITE_RIGHT_BRACE_TOKEN: halfword = 4_194_429;
/// XeTeX `end_write_token`, the frozen `\endwrite` control sequence.
const WRITE_END_TOKEN: halfword = 34_749_089;
/// XeTeX `left_brace_token + "{"`, closing the braces `\write` wraps its token list in.
const WRITE_LEFT_BRACE_TOKEN: halfword = 2_097_275;

/// A sealed engine state, decoded and validated once when loaded, that every engine instance clones.
pub struct PortableFormatImage {
    state: Box<PortableTexState>,
    heap: Box<PortableTexHeap>,
}

/// An engine node handle, only the engine hands these out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortableNodeHandle(i32);

/// What a node in engine memory is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableNodeKind {
    /// An hlist box.
    HorizontalBox,
    /// A vlist box.
    VerticalBox,
    /// A rule.
    Rule,
    /// An insertion.
    Insertion,
    /// A mark.
    Mark,
    /// A `\vadjust`.
    Adjustment,
    /// A ligature, its original characters are the snapshot's list.
    Ligature,
    /// A discretionary.
    Discretionary,
    /// An `\openout`, `\write`, `\closeout` or `\special` whatsit.
    OutputWhatsit,
    /// Any other whatsit.
    Whatsit,
    /// A math on or off node, its width is `\mathsurround`.
    Math,
    /// Glue, with a leader box when it is a leader.
    Glue,
    /// A kern.
    Kern,
    /// A penalty.
    Penalty,
    /// An unset box of an alignment still being built.
    UnsetBox,
    /// A math noad.
    Noad(PortableNoadKind),
    /// A math style change.
    Style,
    /// A `\mathchoice`.
    Choice,
    /// A character of a TFM font.
    Character,
    /// A native word.
    NativeWord,
    /// A single native glyph.
    NativeGlyph,
    /// A resolved host box marker, its character is the host box record id.
    HostBoxRef,
    /// A node type this engine does not name.
    Unknown(i32),
}

/// The atom type of a math noad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableNoadKind {
    /// `ord_noad`.
    Ord,
    /// `op_noad`.
    Op,
    /// `bin_noad`.
    Bin,
    /// `rel_noad`.
    Rel,
    /// `open_noad`.
    Open,
    /// `close_noad`.
    Close,
    /// `punct_noad`.
    Punct,
    /// `inner_noad`.
    Inner,
    /// `radical_noad`.
    Radical,
    /// `fraction_noad`.
    Fraction,
    /// `under_noad`.
    Under,
    /// `over_noad`.
    Over,
    /// `accent_noad`.
    Accent,
    /// `vcenter_noad`.
    Vcenter,
    /// `left_noad`.
    Left,
    /// `right_noad`.
    Right,
}

/// Noad kinds in type order from [`ORD_NOAD`] to [`RIGHT_NOAD`].
const NOAD_KINDS: [PortableNoadKind; 16] = [
    PortableNoadKind::Ord,
    PortableNoadKind::Op,
    PortableNoadKind::Bin,
    PortableNoadKind::Rel,
    PortableNoadKind::Open,
    PortableNoadKind::Close,
    PortableNoadKind::Punct,
    PortableNoadKind::Inner,
    PortableNoadKind::Radical,
    PortableNoadKind::Fraction,
    PortableNoadKind::Under,
    PortableNoadKind::Over,
    PortableNoadKind::Accent,
    PortableNoadKind::Vcenter,
    PortableNoadKind::Left,
    PortableNoadKind::Right,
];

/// A read only copy of one node's fields, dimensions in scaled points.
#[derive(Clone, Debug, PartialEq)]
pub struct PortableNodeSnapshot {
    /// The node.
    pub handle: PortableNodeHandle,
    /// What the node is.
    pub kind: PortableNodeKind,
    /// The raw subtype byte, the span count of an unset box.
    pub subtype: i32,
    /// Where the node came from when source tracking is on.
    pub source: Option<PortableSourceSpan>,
    /// The next node in its list.
    pub link: Option<PortableNodeHandle>,
    /// Font of a character, ligature, native word or native glyph.
    pub font: i32,
    /// Character of a character or ligature, glyph of a native glyph, record id of a host box marker.
    pub character: i32,
    /// Width, the natural width of glue and the surround of a math node.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Depth.
    pub depth: i32,
    /// Shift of a box.
    pub shift: i32,
    /// Contents of a box, or the original characters of a ligature.
    pub list: Option<PortableNodeHandle>,
    /// Leader box or rule of leader glue.
    pub leader: Option<PortableNodeHandle>,
    /// Shaped glyphs of a native word or glyph.
    pub native_glyphs: Vec<PortableNativeGlyph>,
    /// Box glue set ratio from `hpack` or `vpack`.
    pub glue_set: f64,
    /// Box glue sign: 0 normal, 1 stretching, 2 shrinking.
    pub glue_sign: i32,
    /// Box glue order that stretches or shrinks.
    pub glue_order: i32,
    /// Stretch of glue or of an unset box.
    pub glue_stretch: i32,
    /// Shrink of glue or of an unset box.
    pub glue_shrink: i32,
    /// Order of the stretch of glue or of an unset box.
    pub glue_stretch_order: i32,
    /// Order of the shrink of glue or of an unset box.
    pub glue_shrink_order: i32,
}

impl Clone for PortableFormatImage {
    fn clone(&self) -> Self {
        let (state, heap) = self.instantiate();
        Self { state, heap }
    }
}

impl PortableFormatImage {
    /// The image of a state before `initialize_format_state`, engines built from it only initialize.
    pub fn empty() -> Self {
        let mut state = boxed_state_zeroed();
        let mut heap = Box::<PortableTexHeap>::default();
        refresh_runtime_pointers(&mut state, &mut heap);
        Self { state, heap }
    }

    /// Seals a deep copy of a live engine state.
    fn from_engine_state(state: &PortableTexState, heap: &PortableTexHeap) -> Self {
        let mut state = boxed_state_copy(state);
        let mut heap = Box::new(heap.clone());
        seal_as_format_snapshot(&mut state, &mut heap);
        Self { state, heap }
    }

    /// Deep copies the sealed state for a new engine, rebinding its pointers to the copy.
    fn instantiate(&self) -> (Box<PortableTexState>, Box<PortableTexHeap>) {
        let mut state = boxed_state_copy(&self.state);
        let mut heap = Box::new((*self.heap).clone());
        refresh_runtime_pointers(&mut state, &mut heap);
        (state, heap)
    }

    /// Returns the validation tag stored at the start of every image this build writes.
    #[must_use]
    pub fn validation_tag() -> [u8; VALIDATION_TAG_LEN] {
        validation_tag()
    }

    /// Reports whether a serialized image carries this build's validation tag.
    #[must_use]
    pub fn bytes_match_target(bytes: &[u8]) -> bool {
        bytes.get(..VALIDATION_TAG_LEN) == Some(&validation_tag()[..])
    }

    /// Decodes and validates a static image once, later instantiation only clones the decoded state.
    pub fn from_static_bytes(bytes: &'static [u8]) -> Result<Self, FormatImageError> {
        Self::from_bytes(bytes)
    }

    /// Serializes this image, the bytes are deterministic and load in this build on any target of the same byte order.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_image(&self.state, &self.heap)
    }

    /// Decodes and validates an image produced by [`Self::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FormatImageError> {
        let (state, heap) = decode_image(bytes)?;
        Ok(Self { state, heap })
    }

    /// Resident bytes of the engine arrays once instantiated from this image.
    #[must_use]
    pub fn state_array_bytes(&self) -> usize {
        heap_array_bytes(&self.heap)
    }
}

/// Size in bytes of one TeX `memory_word`, 8 as in XeTeX.
#[must_use]
pub fn memory_word_bytes() -> usize {
    core::mem::size_of::<memoryword>()
}

pub struct PortableTexEngine<'resources> {
    pub(crate) state: Box<PortableTexState>,
    pub(crate) heap: Box<PortableTexHeap>,
    pub(crate) resources: Box<dyn ResourceProvider + 'resources>,
    pub(crate) fonts: Box<dyn FontPlatform + 'resources>,
    pub(crate) platform: Box<dyn PortablePlatform + 'resources>,
    pub(crate) nameoffile_storage: Vec<UTF8code>,
    native_glyph_infos: std::collections::BTreeMap<i32, PortableNativeGlyphInfo>,
    // Host box render payloads by record id, markers in node memory carry only the id.
    hostbox_records: std::collections::BTreeMap<integer, PortableHostBox>,
    character_protrusions: std::collections::BTreeMap<(integer, u32, integer), integer>,
    pub(crate) resource_requests: usize,
    pub(crate) resource_request_records: Vec<PortableResourceRequestRecord>,
    pub(crate) virtual_files: std::collections::BTreeMap<String, Vec<u8>>,
    pub(crate) transcript_bytes: Vec<u8>,
    pub(crate) current_input_package_owner: Option<String>,
    pub(crate) stripped_page_builds: usize,
    pub(crate) stripped_shipouts: usize,
    pub(crate) stripped_special_outputs: usize,
    pub(crate) stripped_picture_loads: usize,
    pub(crate) stripped_source_specials: usize,
    pub(crate) stripped_write_whatsit_diagnostics: usize,
    pub(crate) stripped_pdf_extensions: usize,
    pub(crate) stripped_page_top_prunes: usize,
    pub(crate) last_stripped_shipout_box: Option<PortableNodeHandle>,
    pub(crate) fragment_capture_enabled: bool,
    pub(crate) format_initialization: bool,
    pub(crate) captured_fragment_root: Option<PortableNodeHandle>,
    pub(crate) last_abort_status: Option<integer>,
    /// The most recent surfaced [`EngineError`] with its input position.
    pub(crate) last_error: Option<PortableError>,
    /// Whether fragment input is sandboxed, math breakouts, file access and runaway loops then end the run.
    pub(crate) sandbox: bool,
    /// Math nesting depth under the sandbox, the wrapper math is depth 1 and returning to 0 closes it.
    pub(crate) sandbox_math_depth: i32,
    /// Set once the wrapper math has opened, a later reopening at depth 0 is a breakout.
    pub(crate) sandbox_math_opened: bool,
    /// Sandbox work consumed this run, one unit per command, macro call and expansion.
    pub(crate) sandbox_ops: u64,
    /// Sandbox work allowed per run before the fragment is rejected.
    pub(crate) sandbox_op_budget: u64,
    /// Primary input byte offset of the host wrapper's suffix, only a math shift from there closes the wrapper math.
    pub(crate) sandbox_suffix_start: Option<u32>,
    /// Bytes of the error message being printed, from `print_err` to the start of `error`.
    error_message_capture: Option<Vec<u8>>,
    /// The last complete error message, taken when the error surfaces.
    captured_error_message: Option<String>,
}

pub(crate) fn zround(value: real) -> integer {
    value.round() as integer
}

/// Message a run entry point reports for an engine whose state was never initialized.
const UNINITIALIZED_STATE_MESSAGE: &str = "engine state is not initialized";

impl<'resources> PortableTexEngine<'resources> {
    pub fn from_format<R>(format: &PortableFormatImage, resources: R) -> Self
    where
        R: ResourceProvider + 'resources,
    {
        let (state, heap) = format.instantiate();
        Self {
            state,
            heap,
            resources: Box::new(resources),
            fonts: Box::<EmptyFontPlatform>::default(),
            platform: Box::<EmptyPlatform>::default(),
            nameoffile_storage: Vec::new(),
            native_glyph_infos: std::collections::BTreeMap::new(),
            hostbox_records: std::collections::BTreeMap::new(),
            character_protrusions: std::collections::BTreeMap::new(),
            resource_requests: 0,
            resource_request_records: Vec::new(),
            virtual_files: std::collections::BTreeMap::new(),
            transcript_bytes: Vec::new(),
            current_input_package_owner: None,
            stripped_page_builds: 0,
            stripped_shipouts: 0,
            stripped_special_outputs: 0,
            stripped_picture_loads: 0,
            stripped_source_specials: 0,
            stripped_write_whatsit_diagnostics: 0,
            stripped_pdf_extensions: 0,
            stripped_page_top_prunes: 0,
            last_stripped_shipout_box: None,
            fragment_capture_enabled: false,
            format_initialization: false,
            captured_fragment_root: None,
            last_abort_status: None,
            last_error: None,
            sandbox: false,
            sandbox_math_depth: 0,
            sandbox_math_opened: false,
            sandbox_ops: 0,
            sandbox_op_budget: SANDBOX_OP_BUDGET,
            sandbox_suffix_start: None,
            error_message_capture: None,
            captured_error_message: None,
        }
    }

    pub fn with_font_platform<F>(mut self, fonts: F) -> Self
    where
        F: FontPlatform + 'resources,
    {
        self.fonts = Box::new(fonts);
        self
    }

    pub fn with_platform<P>(mut self, platform: P) -> Self
    where
        P: PortablePlatform + 'resources,
    {
        self.platform = Box::new(platform);
        self
    }

    /// Runs INITEX initialization, the only entry point that accepts an uninitialized state.
    pub fn initialize_format_state(self: &mut Self) -> bool {
        self.state.initialized = false;
        let completed = self.catch_engine_abort(|engine| unsafe {
            let this = engine as *mut PortableTexEngine<'_>;
            allocate_initial_arrays(&mut engine.state, &mut engine.heap);
            engine.initialize();
            engine.state.eTeXmode = 1 as eightbits;
            if engine.getstringsstarted()? == 0 {
                Self::abort_engine(this, 1 as integer)?;
            }
            engine.initprim()?;
            engine.init_xetex_startup_primitives()?;
            engine.register_host_box_primitive()?;
            engine.state.initstrptr = engine.state.strptr;
            engine.state.initpoolptr = engine.state.poolptr;
            // Replaces the `mainbody` assignment `magic_offset := str_start[math_spacing] - 9 * ord_noad`.
            let math_spacing = Self::pool_string_index(POOL_MATH_SPACING).unwrap_or_default();
            engine.state.magicoffset = *engine.state.strstart.offset(math_spacing) - 9 * ORD_NOAD;
            engine.state.alignstate = NEUTRAL_ALIGN_STATE;
            Ok(())
        });
        self.state.initialized = completed;
        completed
    }

    pub(crate) fn ensure_nativetext_capacity(
        engine: &mut PortableTexEngine<'_>,
        required: integer,
    ) {
        let required = required.max(0) as usize;
        if engine.heap.nativetext_storage.len() < required {
            engine.heap.nativetext_storage.resize(required, UTF16code::default());
        }
        engine.state.nativetext = pointer_or_null(&mut engine.heap.nativetext_storage);
    }

    unsafe fn init_xetex_startup_primitives(&mut self) -> EngineFlow<()> {
        self.state.nonewcontrolsequence = false_0 as boolean;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66755 as i64 as strnumber, 59 as i32 as quarterword, 41 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66756 as i64 as strnumber, 59 as i32 as quarterword, 42 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66757 as i64 as strnumber, 59 as i32 as quarterword, 43 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66758 as i64 as strnumber, 59 as i32 as quarterword, 46 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66759 as i64 as strnumber,
                73 as i32 as quarterword,
                1206306 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66760 as i64 as strnumber, 59 as i32 as quarterword, 23 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66816 as i64 as strnumber, 71 as i32 as quarterword, 3 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66817 as i64 as strnumber, 71 as i32 as quarterword, 19 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66115 as i64 as strnumber, 111 as i32 as quarterword, 5 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66818 as i64 as strnumber, 71 as i32 as quarterword, 27 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66819 as i64 as strnumber,
                111 as i32 as quarterword,
                33 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66820 as i64 as strnumber, 71 as i32 as quarterword, 28 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66821 as i64 as strnumber, 71 as i32 as quarterword, 29 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66822 as i64 as strnumber, 71 as i32 as quarterword, 30 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66823 as i64 as strnumber, 71 as i32 as quarterword, 31 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66824 as i64 as strnumber, 71 as i32 as quarterword, 32 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66825 as i64 as strnumber, 71 as i32 as quarterword, 33 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66826 as i64 as strnumber, 71 as i32 as quarterword, 34 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66827 as i64 as strnumber, 71 as i32 as quarterword, 35 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66828 as i64 as strnumber, 71 as i32 as quarterword, 36 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66829 as i64 as strnumber, 71 as i32 as quarterword, 37 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66830 as i64 as strnumber, 71 as i32 as quarterword, 38 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66831 as i64 as strnumber, 71 as i32 as quarterword, 39 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66832 as i64 as strnumber, 71 as i32 as quarterword, 40 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66833 as i64 as strnumber, 71 as i32 as quarterword, 41 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66834 as i64 as strnumber, 71 as i32 as quarterword, 42 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66835 as i64 as strnumber,
                111 as i32 as quarterword,
                34 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66836 as i64 as strnumber,
                111 as i32 as quarterword,
                35 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66837 as i64 as strnumber,
                111 as i32 as quarterword,
                36 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66838 as i64 as strnumber, 71 as i32 as quarterword, 43 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66839 as i64 as strnumber, 71 as i32 as quarterword, 44 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66840 as i64 as strnumber, 71 as i32 as quarterword, 45 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66841 as i64 as strnumber, 71 as i32 as quarterword, 46 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66842 as i64 as strnumber, 71 as i32 as quarterword, 47 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66843 as i64 as strnumber, 71 as i32 as quarterword, 48 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66844 as i64 as strnumber, 71 as i32 as quarterword, 49 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66845 as i64 as strnumber, 71 as i32 as quarterword, 50 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66846 as i64 as strnumber, 71 as i32 as quarterword, 55 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66847 as i64 as strnumber,
                111 as i32 as quarterword,
                37 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66848 as i64 as strnumber, 71 as i32 as quarterword, 51 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66849 as i64 as strnumber, 71 as i32 as quarterword, 52 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66850 as i64 as strnumber, 71 as i32 as quarterword, 53 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66851 as i64 as strnumber, 71 as i32 as quarterword, 54 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66861 as i64 as strnumber,
                73 as i32 as quarterword,
                1206305 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66862 as i64 as strnumber,
                74 as i32 as quarterword,
                7892325 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66863 as i64 as strnumber,
                74 as i32 as quarterword,
                7892326 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66864 as i64 as strnumber,
                74 as i32 as quarterword,
                7892327 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66865 as i64 as strnumber,
                74 as i32 as quarterword,
                7892328 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66866 as i64 as strnumber,
                74 as i32 as quarterword,
                7892329 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66867 as i64 as strnumber,
                74 as i32 as quarterword,
                7892330 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66868 as i64 as strnumber,
                74 as i32 as quarterword,
                7892331 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66869 as i64 as strnumber,
                74 as i32 as quarterword,
                7892332 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66870 as i64 as strnumber,
                74 as i32 as quarterword,
                7892333 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66871 as i64 as strnumber,
                74 as i32 as quarterword,
                7892335 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66885 as i64 as strnumber, 71 as i32 as quarterword, 20 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66886 as i64 as strnumber, 71 as i32 as quarterword, 21 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66887 as i64 as strnumber, 71 as i32 as quarterword, 22 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66888 as i64 as strnumber, 71 as i32 as quarterword, 23 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66889 as i64 as strnumber, 71 as i32 as quarterword, 24 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66890 as i64 as strnumber, 71 as i32 as quarterword, 56 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66891 as i64 as strnumber, 71 as i32 as quarterword, 57 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66892 as i64 as strnumber, 71 as i32 as quarterword, 58 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66893 as i64 as strnumber, 71 as i32 as quarterword, 59 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66894 as i64 as strnumber, 71 as i32 as quarterword, 60 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66895 as i64 as strnumber, 71 as i32 as quarterword, 61 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66896 as i64 as strnumber, 71 as i32 as quarterword, 62 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66897 as i64 as strnumber, 19 as i32 as quarterword, 4 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66899 as i64 as strnumber, 19 as i32 as quarterword, 5 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66900 as i64 as strnumber, 112 as i32 as quarterword, 1 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66901 as i64 as strnumber, 112 as i32 as quarterword, 5 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66902 as i64 as strnumber, 19 as i32 as quarterword, 6 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66906 as i64 as strnumber, 83 as i32 as quarterword, 2 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66288 as i64 as strnumber, 49 as i32 as quarterword, 1 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66910 as i64 as strnumber,
                74 as i32 as quarterword,
                7892334 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66911 as i64 as strnumber,
                74 as i32 as quarterword,
                7892339 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66912 as i64 as strnumber,
                74 as i32 as quarterword,
                7892341 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66913 as i64 as strnumber,
                74 as i32 as quarterword,
                7892342 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66914 as i64 as strnumber,
                74 as i32 as quarterword,
                7892343 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66915 as i64 as strnumber,
                74 as i32 as quarterword,
                7892340 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66916 as i64 as strnumber,
                74 as i32 as quarterword,
                7892344 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66917 as i64 as strnumber,
                74 as i32 as quarterword,
                7892347 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66918 as i64 as strnumber,
                74 as i32 as quarterword,
                7892348 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66919 as i64 as strnumber,
                74 as i32 as quarterword,
                7892349 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66920 as i64 as strnumber,
                74 as i32 as quarterword,
                7892350 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66761 as i64 as strnumber, 59 as i32 as quarterword, 44 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66762 as i64 as strnumber, 59 as i32 as quarterword, 45 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66921 as i64 as strnumber, 33 as i32 as quarterword, 6 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66922 as i64 as strnumber, 33 as i32 as quarterword, 7 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66923 as i64 as strnumber, 33 as i32 as quarterword, 10 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66924 as i64 as strnumber, 33 as i32 as quarterword, 11 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66933 as i64 as strnumber, 107 as i32 as quarterword, 2 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66935 as i64 as strnumber, 98 as i32 as quarterword, 1 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66164 as i64 as strnumber, 105 as i32 as quarterword, 1 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66936 as i64 as strnumber,
                108 as i32 as quarterword,
                17 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66937 as i64 as strnumber,
                108 as i32 as quarterword,
                18 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66938 as i64 as strnumber,
                108 as i32 as quarterword,
                19 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66939 as i64 as strnumber,
                108 as i32 as quarterword,
                20 as i32,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66623 as i64 as strnumber, 95 as i32 as quarterword, 8 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66945 as i64 as strnumber, 71 as i32 as quarterword, 67 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66946 as i64 as strnumber, 71 as i32 as quarterword, 68 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66947 as i64 as strnumber, 71 as i32 as quarterword, 69 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66948 as i64 as strnumber, 71 as i32 as quarterword, 70 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66952 as i64 as strnumber, 71 as i32 as quarterword, 25 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66953 as i64 as strnumber, 71 as i32 as quarterword, 26 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66954 as i64 as strnumber, 71 as i32 as quarterword, 63 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66955 as i64 as strnumber, 71 as i32 as quarterword, 64 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66956 as i64 as strnumber, 71 as i32 as quarterword, 65 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66957 as i64 as strnumber, 71 as i32 as quarterword, 66 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66958 as i64 as strnumber, 18 as i32 as quarterword, 5 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66959 as i64 as strnumber, 113 as i32 as quarterword, 5 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66960 as i64 as strnumber, 113 as i32 as quarterword, 6 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66961 as i64 as strnumber, 113 as i32 as quarterword, 7 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66962 as i64 as strnumber, 113 as i32 as quarterword, 8 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66963 as i64 as strnumber, 113 as i32 as quarterword, 9 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66968 as i64 as strnumber, 24 as i32 as quarterword, 2 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(66969 as i64 as strnumber, 24 as i32 as quarterword, 3 as i32)?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66970 as i64 as strnumber,
                85 as i32 as quarterword,
                1206563 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66971 as i64 as strnumber,
                85 as i32 as quarterword,
                1206564 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66972 as i64 as strnumber,
                85 as i32 as quarterword,
                1206565 as i64 as halfword,
            )?;
        (&mut *(self as *mut PortableTexEngine<'_>))
            .zprimitive(
                66973 as i64 as strnumber,
                85 as i32 as quarterword,
                1206566 as i64 as halfword,
            )?;
        if *self.state.buffer.offset(self.state.curinput.locfield as isize) == 42 as i32
        {
            self.state.curinput.locfield += 1;
        }
        self.state.eTeXmode = 1 as eightbits;
        self.state.maxregnum = 32767 as i32 as halfword;
        self.state.maxreghelpline = 66965 as i64 as strnumber;
        Ok(())
    }

    pub fn begin_primary_input(self: &mut Self, name: &str, bytes: Vec<u8>) -> bool {
        let mut started = false;
        let completed = self.run_initialized(|engine| {
            started = unsafe { engine.begin_primary_input_raw(name, bytes) }? != 0;
            Ok(())
        });
        completed && started
    }

    pub fn run_main_control(self: &mut Self) -> bool {
        self.run_initialized(|engine| unsafe { engine.maincontrol() })
    }

    pub fn run_format_initialization(self: &mut Self) -> bool {
        self.format_initialization = true;
        let completed = self.run_initialized(|engine| unsafe { engine.maincontrol() });
        self.format_initialization = false;
        completed
    }

    /// Starts capturing a fragment root and drops the previous fragment's host box records.
    pub fn begin_fragment_capture(self: &mut Self) {
        self.fragment_capture_enabled = true;
        self.captured_fragment_root = None;
        // Ids are never reused, so a marker kept in a global box register from a dropped record resolves to none.
        self.hostbox_records.clear();
    }

    pub fn end_fragment_capture(self: &mut Self) {
        self.fragment_capture_enabled = false;
    }

    /// Runs `run` through [`Self::catch_engine_abort`], refusing a state never initialized.
    fn run_initialized<F>(self: &mut Self, run: F) -> bool
    where
        F: FnOnce(&mut Self) -> EngineFlow<()>,
    {
        if !self.state.initialized {
            self.last_abort_status = None;
            self.last_error = Some(PortableError {
                message: UNINITIALIZED_STATE_MESSAGE.into(),
                kind: PortableErrorKind::Tex,
                line: 0,
                span: None,
            });
            return false;
        }
        self.catch_engine_abort(run)
    }

    fn catch_engine_abort<F>(self: &mut Self, run: F) -> bool
    where
        F: FnOnce(&mut Self) -> EngineFlow<()>,
    {
        // Status 0, the normal end of `\end` or `\dump`, completes the run and any other status aborts it.
        self.last_abort_status = None;
        self.last_error = None;
        match run(self) {
            Ok(()) => self.last_abort_status.is_none(),
            Err(EngineBreak::Abort(EngineAbort { status: 0 })) => {
                self.last_abort_status = None;
                true
            }
            Err(EngineBreak::Abort(EngineAbort { status })) => {
                self.last_abort_status = Some(status);
                false
            }
            Err(EngineBreak::Error(error)) => {
                self.last_error = Some(PortableError {
                    message: error.message,
                    kind: error.kind,
                    line: self.state.line,
                    span: self.src_span_by_id(self.state.src_tok_span),
                });
                false
            }
        }
    }

    pub fn snapshot_format(&self) -> PortableFormatImage {
        PortableFormatImage::from_engine_state(&self.state, &self.heap)
    }

    /// Seals this engine's state in place as a format image, avoiding the copy [`Self::snapshot_format`] makes.
    #[must_use]
    pub fn into_format(self) -> PortableFormatImage {
        // Packing the trie forbids later `\patterns`, callers with a final format call `finalize_trie` first.
        let mut state = self.state;
        let mut heap = self.heap;
        seal_as_format_snapshot(&mut state, &mut heap);
        PortableFormatImage { state, heap }
    }

    /// Packs the hyphenation trie as `\dump` would, false when it overflows, only once no more `\patterns` follow.
    #[must_use]
    pub fn finalize_trie(self: &mut Self) -> bool {
        let saved = self.format_initialization;
        self.format_initialization = false;
        let packed = self.run_initialized(|engine| {
            if engine.state.trienotready != 0 && !engine.state.triehash.is_null() {
                unsafe { engine.inittrie() }?;
            }
            Ok(())
        });
        self.format_initialization = saved;
        packed
    }

    pub fn resource_request_count(&self) -> usize {
        self.resource_requests
    }

    pub fn resource_request_records(&self) -> &[PortableResourceRequestRecord] {
        self.resource_request_records.as_slice()
    }

    pub fn transcript_bytes(&self) -> &[u8] {
        self.transcript_bytes.as_slice()
    }

    /// The first interned span of the primary input, if any.
    pub fn primary_input_source_span(&self) -> Option<PortableSourceSpan> {
        if !self.state.source_tracking {
            return None;
        }
        let primary = self.state.src_primary_name;
        let raw = self
            .heap
            .src_spans
            .iter()
            .find(|raw| raw.name == primary)?;
        let name = self.pool_string(raw.name)?;
        Some(PortableSourceSpan {
            name,
            start: raw.start,
            end: raw.end,
            role: raw.role,
        })
    }

    pub fn stripped_page_build_count(&self) -> usize {
        self.stripped_page_builds
    }

    pub fn stripped_shipout_count(&self) -> usize {
        self.stripped_shipouts
    }

    pub fn stripped_special_output_count(&self) -> usize {
        self.stripped_special_outputs
    }

    pub fn stripped_picture_load_count(&self) -> usize {
        self.stripped_picture_loads
    }

    pub fn stripped_source_special_count(&self) -> usize {
        self.stripped_source_specials
    }

    pub fn stripped_write_whatsit_diagnostic_count(&self) -> usize {
        self.stripped_write_whatsit_diagnostics
    }

    pub fn stripped_pdf_extension_count(&self) -> usize {
        self.stripped_pdf_extensions
    }

    pub fn stripped_page_top_prune_count(&self) -> usize {
        self.stripped_page_top_prunes
    }

    pub fn last_stripped_shipout_box(&self) -> Option<PortableNodeHandle> {
        self.last_stripped_shipout_box
    }

    pub fn captured_fragment_root(&self) -> Option<PortableNodeHandle> {
        self.captured_fragment_root
    }

    /// Host box render payload by record id, as referenced by `HostBoxRef` marker nodes.
    pub fn host_box_record(&self, index: usize) -> Option<&PortableHostBox> {
        self.hostbox_records.get(&integer::try_from(index).ok()?)
    }

    /// Size in scaled points font number `font` was loaded at.
    pub fn font_at_size(&self, font: integer) -> integer {
        if font < 0 {
            return 0;
        }
        self.heap
            .fontsize_storage
            .get(font as usize)
            .copied()
            .unwrap_or(0)
    }

    /// The `\font` name of a font number, the XeTeX spec of a native font or the TFM name.
    pub fn font_name(&self, font: integer) -> Option<String> {
        if font < 0 {
            return None;
        }
        let name = self.heap.fontname_storage.get(font as usize).copied()?;
        self.pool_string(name)
    }

    /// The spec a native font number was loaded with, from the font platform.
    pub fn native_font_spec(&self, font: integer) -> Option<String> {
        let handle = Self::font_handle_for_number(self, font)?;
        self.fonts.font_spec(handle)
    }

    /// The host font key of a native font number, from the font platform.
    pub fn native_font_key(&self, font: integer) -> Option<u64> {
        let handle = Self::font_handle_for_number(self, font)?;
        self.fonts.font_key(handle)
    }

    /// The attached font platform's native font table, packaged with a serialized format image.
    pub fn native_font_table(&self) -> Vec<(PortableFontHandle, String, i32)> {
        self.fonts.font_table()
    }

    /// Rebinds a loaded image's native fonts from a [`Self::native_font_table`] snapshot, false when one fails.
    pub fn restore_native_font_table(
        self: &mut Self,
        table: &[(PortableFontHandle, String, i32)],
    ) -> bool {
        self.fonts.restore_font_table(table)
    }

    pub fn last_abort_status(&self) -> Option<integer> {
        self.last_abort_status
    }

    /// The message of the last run's surfaced TeX error, `None` after a clean run or a bare abort.
    pub fn last_error_message(&self) -> Option<&str> {
        self.last_error.as_ref().map(|error| error.message.as_str())
    }

    /// The last run's surfaced error with its kind and input position, `None` after a clean run or a bare abort.
    pub fn last_error(&self) -> Option<&PortableError> {
        self.last_error.as_ref()
    }

    /// Switches the fragment sandbox and resets its per run bookkeeping.
    pub fn set_sandbox(self: &mut Self, on: bool) {
        self.sandbox = on;
        self.sandbox_math_depth = 0;
        self.sandbox_math_opened = false;
        self.sandbox_ops = 0;
    }

    /// Sets the sandbox work budget, counted in commands, macro calls and expansions per run.
    pub fn set_sandbox_op_budget(self: &mut Self, budget: u64) {
        self.sandbox_op_budget = budget;
    }

    /// Marks where the host wrapper's suffix starts in the primary input, as a byte offset.
    pub fn set_sandbox_wrapper_suffix(self: &mut Self, start: Option<u32>) {
        self.sandbox_suffix_start = start;
    }

    /// Reads one node's fields, `None` for a handle outside node memory.
    pub fn snapshot_node(&self, handle: PortableNodeHandle) -> Option<PortableNodeSnapshot> {
        let node = handle.0 as halfword;
        let word = self.node_word(node, 0)?;
        let raw_kind = unsafe { word.hh.u.B0 as i32 };
        let subtype = unsafe { word.hh.u.B1 as i32 };
        let kind = self.node_kind(raw_kind, node, subtype);
        let mut snapshot = PortableNodeSnapshot {
            handle,
            kind,
            subtype,
            source: self.resolve_node_src(node),
            link: self.node_link(node),
            font: 0,
            character: 0,
            width: 0,
            height: 0,
            depth: 0,
            shift: 0,
            list: None,
            leader: None,
            native_glyphs: Vec::new(),
            glue_set: 0.0,
            glue_sign: 0,
            glue_order: 0,
            glue_stretch: 0,
            glue_shrink: 0,
            glue_stretch_order: 0,
            glue_shrink_order: 0,
        };
        let scaled_at = |offset: halfword| self.node_scaled(node, offset).unwrap_or_default();
        // Byte `B0` of a two halves word is the type field, `B1` the subtype field.
        let quarters_at = |at: halfword, offset: halfword| {
            self.node_word(at, offset)
                .map_or((0, 0), |word| unsafe { (word.hh.u.B0 as i32, word.hh.u.B1 as i32) })
        };
        match kind {
            PortableNodeKind::Character => {
                snapshot.font = raw_kind;
                snapshot.character = subtype;
                (snapshot.width, snapshot.height, snapshot.depth) =
                    self.character_metrics(raw_kind, subtype).unwrap_or_default();
            }
            PortableNodeKind::HorizontalBox | PortableNodeKind::VerticalBox => {
                snapshot.width = scaled_at(1);
                snapshot.depth = scaled_at(2);
                snapshot.height = scaled_at(3);
                snapshot.shift = scaled_at(4);
                snapshot.list = self.node_field_link(node, 5);
                (snapshot.glue_sign, snapshot.glue_order) = quarters_at(node, 5);
                snapshot.glue_set = self.node_word(node, 6).map_or(0.0, |word| unsafe { word.gr });
            }
            PortableNodeKind::UnsetBox => {
                // An unset box keeps its total stretch in the glue set word and its shrink in the shift word.
                snapshot.width = scaled_at(1);
                snapshot.depth = scaled_at(2);
                snapshot.height = scaled_at(3);
                snapshot.glue_shrink = scaled_at(4);
                snapshot.list = self.node_field_link(node, 5);
                (snapshot.glue_shrink_order, snapshot.glue_stretch_order) = quarters_at(node, 5);
                snapshot.glue_stretch = scaled_at(6);
            }
            PortableNodeKind::Rule => {
                snapshot.width = scaled_at(1);
                snapshot.depth = scaled_at(2);
                snapshot.height = scaled_at(3);
            }
            PortableNodeKind::Glue => {
                let fields = self.node_word(node, 1);
                snapshot.leader = fields.and_then(|word| Self::node_handle_from_raw(unsafe { word.hh.v.RH }));
                if let Some(spec) = fields.map(|word| unsafe { word.hh.v.LH }) {
                    snapshot.width = self.node_scaled(spec, 1).unwrap_or_default();
                    snapshot.glue_stretch = self.node_scaled(spec, 2).unwrap_or_default();
                    snapshot.glue_shrink = self.node_scaled(spec, 3).unwrap_or_default();
                    (snapshot.glue_stretch_order, snapshot.glue_shrink_order) = quarters_at(spec, 0);
                }
            }
            PortableNodeKind::Kern | PortableNodeKind::Math => snapshot.width = scaled_at(1),
            PortableNodeKind::Ligature => {
                // The ligature character word sits after the node header and links to the original characters.
                (snapshot.font, snapshot.character) = quarters_at(node, 1);
                (snapshot.width, snapshot.height, snapshot.depth) = self
                    .character_metrics(snapshot.font, snapshot.character)
                    .unwrap_or_default();
                snapshot.list = self.node_field_link(node, 1);
            }
            PortableNodeKind::NativeWord | PortableNodeKind::NativeGlyph => {
                let native = self.node_word(node, 4);
                snapshot.font = native.map_or(0, |word| unsafe { word.v.QQQQ.u.B1 as i32 });
                snapshot.character = native.map_or(0, |word| unsafe { word.v.QQQQ.u.B2 as i32 });
                snapshot.width = scaled_at(1);
                snapshot.depth = scaled_at(2);
                snapshot.height = scaled_at(3);
                snapshot.native_glyphs = self
                    .native_glyph_infos
                    .get(&node)
                    .map(|info| info.glyphs.clone())
                    .unwrap_or_default();
            }
            PortableNodeKind::HostBoxRef => {
                // Host box markers carry their record id in the payload word after the metrics.
                snapshot.character = scaled_at(4);
                snapshot.width = scaled_at(1);
                snapshot.depth = scaled_at(2);
                snapshot.height = scaled_at(3);
            }
            _ => {}
        }
        Some(snapshot)
    }

    fn node_kind(&self, raw_kind: i32, node: halfword, subtype: i32) -> PortableNodeKind {
        if node >= self.state.himemmin {
            return PortableNodeKind::Character;
        }
        match raw_kind {
            HLIST_NODE => PortableNodeKind::HorizontalBox,
            VLIST_NODE => PortableNodeKind::VerticalBox,
            RULE_NODE => PortableNodeKind::Rule,
            INS_NODE => PortableNodeKind::Insertion,
            MARK_NODE => PortableNodeKind::Mark,
            ADJUST_NODE => PortableNodeKind::Adjustment,
            LIGATURE_NODE => PortableNodeKind::Ligature,
            DISC_NODE => PortableNodeKind::Discretionary,
            WHATSIT_NODE => match subtype {
                NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT => PortableNodeKind::NativeWord,
                GLYPH_NODE => PortableNodeKind::NativeGlyph,
                HOST_BOX_RESOLVED_SUBTYPE => PortableNodeKind::HostBoxRef,
                0..=3 => PortableNodeKind::OutputWhatsit,
                _ => PortableNodeKind::Whatsit,
            },
            MATH_NODE => PortableNodeKind::Math,
            GLUE_NODE => PortableNodeKind::Glue,
            KERN_NODE => PortableNodeKind::Kern,
            PENALTY_NODE => PortableNodeKind::Penalty,
            UNSET_NODE => PortableNodeKind::UnsetBox,
            STYLE_NODE => PortableNodeKind::Style,
            CHOICE_NODE => PortableNodeKind::Choice,
            ORD_NOAD..=RIGHT_NOAD => PortableNodeKind::Noad(NOAD_KINDS[(raw_kind - ORD_NOAD) as usize]),
            other => PortableNodeKind::Unknown(other),
        }
    }

    /// Copies the side table glyphs of node `src` to node `dest`, returning the glyph count.
    pub(crate) fn copy_native_glyph_info(
        this: &mut Self,
        src: halfword,
        dest: halfword,
    ) -> quarterword {
        if let Some(info) = this.native_glyph_infos.get(&src).cloned() {
            let glyph_count = info.glyphs.len().min(i32::MAX as usize) as quarterword;
            this.native_glyph_infos.insert(dest, info);
            glyph_count
        } else {
            this.native_glyph_infos.remove(&dest);
            0
        }
    }

    /// Drops the side table glyphs of a node being freed.
    pub(crate) fn release_native_glyph_info(this: &mut Self, node: halfword) {
        if !this.native_glyph_infos.is_empty() {
            this.native_glyph_infos.remove(&node);
        }
    }

    fn font_handle_for_number(this: &Self, font: integer) -> Option<FontHandle> {
        if font < 0 || font > this.state.fontptr {
            return None;
        }
        let handle = *this.heap.fontlayoutengine_storage.get(font as usize)? as FontHandle;
        (handle != 0).then_some(handle)
    }

    unsafe fn node_index_for_pointer(
        engine: &PortableTexEngine<'_>,
        node: voidpointer,
    ) -> Option<halfword> {
        if node.is_null() || engine.state.zmem.is_null() {
            return None;
        }
        let base = engine.state.zmem as isize;
        let address = node as isize;
        let word_size = core::mem::size_of::<memoryword>() as isize;
        if word_size == 0 || address < base {
            return None;
        }
        let bytes = address - base;
        if bytes % word_size != 0 {
            return None;
        }
        let index = (bytes / word_size) as halfword;
        if index < engine.state.memmin || index > engine.state.memmax {
            None
        } else {
            Some(index)
        }
    }

    unsafe fn native_node_font(mem: *mut memoryword, node: halfword) -> integer {
        (*mem.offset((node + 4) as isize)).v.QQQQ.u.B1 as integer
    }

    unsafe fn native_node_text<'a>(mem: *mut memoryword, node: halfword) -> &'a [u16] {
        let len = (*mem.offset((node + 4) as isize)).v.QQQQ.u.B2 as usize;
        if len == 0 {
            return &[];
        }
        core::slice::from_raw_parts(
            mem.offset((node + native_node_size) as isize) as *const memoryword as *const u16,
            len,
        )
    }

    unsafe fn write_native_node_metrics(
        mem: *mut memoryword,
        node: halfword,
        width: i32,
        height: i32,
        depth: i32,
    ) {
        (*mem.offset((node + 1) as isize)).u.CINT = width;
        (*mem.offset((node + 2) as isize)).u.CINT = depth;
        (*mem.offset((node + 3) as isize)).u.CINT = height;
    }

    fn node_word(&self, node: halfword, offset: halfword) -> Option<memoryword> {
        if node == TEX_NULL {
            return None;
        }
        let index = node.checked_add(offset)?;
        if index > self.state.memmax {
            return None;
        }
        let slot = usize::try_from(index.checked_sub(self.state.memmin)?).ok()?;
        self.heap.mem.get(slot).copied()
    }

    fn node_link(&self, node: halfword) -> Option<PortableNodeHandle> {
        let word = self.node_word(node, 0)?;
        let link = unsafe { word.hh.v.RH };
        Self::node_handle_from_raw(link)
    }

    fn node_field_link(&self, node: halfword, offset: halfword) -> Option<PortableNodeHandle> {
        let word = self.node_word(node, offset)?;
        let link = unsafe { word.hh.v.RH };
        Self::node_handle_from_raw(link)
    }

    fn node_scaled(&self, node: halfword, offset: halfword) -> Option<i32> {
        let word = self.node_word(node, offset)?;
        Some(unsafe { word.u.CINT })
    }

    // Bounds checked, a stale or foreign handle can put any high memory word here.
    fn character_metrics(&self, font: i32, character: i32) -> Option<(i32, i32, i32)> {
        if font < 0 || character < 0 || font > self.state.fontptr {
            return None;
        }
        let base = |storage: &[integer]| storage.get(font as usize).copied();
        let char_base = base(&self.heap.charbase_storage)?;
        let fontinfo = |index: i32| self.heap.fontinfo_storage.get(usize::try_from(index).ok()?).copied();
        let scaled_at = |table: i32, index: i32| {
            fontinfo(table.checked_add(index)?).map(|word| unsafe { word.u.CINT })
        };
        let char_info = unsafe { fontinfo(char_base.checked_add(character)?)?.v.QQQQ };
        // The height_depth byte packs the height index in its high nibble, as tex.web's char_height.
        let height_depth = i32::from(char_info.u.B1);
        Some((
            scaled_at(base(&self.heap.widthbase_storage)?, i32::from(char_info.u.B0))?,
            scaled_at(base(&self.heap.heightbase_storage)?, height_depth / 16)?,
            scaled_at(base(&self.heap.depthbase_storage)?, height_depth % 16)?,
        ))
    }

    fn node_handle_from_raw(raw: halfword) -> Option<PortableNodeHandle> {
        if raw == TEX_NULL {
            None
        } else {
            Some(PortableNodeHandle(raw))
        }
    }

    unsafe fn current_resource_name(engine: *mut PortableTexEngine<'_>) -> Option<String> {
        let engine = engine.as_ref()?;
        if engine.state.nameoffile.is_null() || engine.state.namelength <= 0 {
            return None;
        }

        let mut bytes = Vec::with_capacity(engine.state.namelength as usize);
        for index in 1..=engine.state.namelength {
            let value = *engine.state.nameoffile.offset(index as isize);
            if value <= 0 {
                continue;
            }
            bytes.push(value as u8);
        }
        Some(String::from_utf8_lossy(bytes.as_slice()).into_owned())
    }

    /// XeTeX `getencodingmodeandinfo` for the name in `nameoffile`, without ICU unknown names read as raw bytes.
    pub(crate) unsafe fn get_encoding_mode_and_info(
        engine: *mut PortableTexEngine<'_>,
        info: *mut integer,
    ) -> integer {
        if !info.is_null() {
            *info = 0;
        }
        let name = Self::current_resource_name(engine).unwrap_or_default();
        let lowered = name.trim().to_ascii_lowercase();
        match lowered.as_str() {
            "auto" => 0,
            "utf8" | "utf-8" => 1,
            // XeTeX reads the host endian `utf16` as big endian.
            "utf16" | "utf-16" | "utf16be" | "utf-16be" => 2,
            "utf16le" | "utf-16le" => 3,
            // Without ICU, any other encoding name reads raw bytes.
            _ => 4,
        }
    }

    unsafe fn mode_string(mode: const_string) -> String {
        if mode.is_null() {
            return String::new();
        }

        let mut bytes = Vec::new();
        let mut cursor = mode;
        while *cursor != 0 {
            bytes.push(*cursor as u8);
            cursor = cursor.add(1);
        }
        String::from_utf8_lossy(bytes.as_slice()).into_owned()
    }

    // Bounds checked against the pool storage, callers can pass any string number.
    fn pool_string(&self, string: strnumber) -> Option<String> {
        let index = usize::try_from(Self::pool_string_index(string)?).ok()?;
        let start = usize::try_from(*self.heap.strstart_storage.get(index)?).ok()?;
        let end = usize::try_from(*self.heap.strstart_storage.get(index.checked_add(1)?)?).ok()?;
        let units = self.heap.strpool_storage.get(start..end)?;
        Some(
            char::decode_utf16(units.iter().copied())
                .map(|codepoint| codepoint.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect(),
        )
    }

    pub(crate) fn pool_string_index(string: strnumber) -> Option<isize> {
        if string < 0 {
            return None;
        }
        let index = if string >= TOO_BIG_CHAR { string - TOO_BIG_CHAR } else { string };
        isize::try_from(index).ok()
    }

    fn resource_kind(name: &str, format: integer) -> ResourceKind {
        match format {
            resource_format_tex_input | 0 => Self::tex_resource_kind(name),
            resource_format_tfm | resource_format_font => ResourceKind::Font,
            resource_format_encoding => ResourceKind::Encoding,
            resource_format_font_map => ResourceKind::Map,
            resource_format_config => ResourceKind::Config,
            resource_format_format_image => ResourceKind::FormatImage,
            other => ResourceKind::Other(other),
        }
    }

    fn tex_resource_kind(name: &str) -> ResourceKind {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let name = name.to_ascii_lowercase();
        if name.ends_with(".sty") {
            return ResourceKind::Package;
        }
        if name.ends_with(".cls") {
            return ResourceKind::Class;
        }
        if name.ends_with(".fd") {
            return ResourceKind::FontDefinition;
        }
        if name.ends_with(".clo")
            || name.ends_with(".def")
            || name.ends_with(".ldf")
            || name.ends_with(".cfg")
        {
            return ResourceKind::PackageSupport;
        }
        ResourceKind::TexInput
    }

    fn resource_kind_for_open(
        engine: &PortableTexEngine<'_>,
        name: &str,
        format: integer,
    ) -> ResourceKind {
        let kind = Self::resource_kind(name, format);
        if kind == ResourceKind::TexInput
            && Self::active_package_owner(engine).is_some()
            && Self::looks_like_package_asset(name)
        {
            ResourceKind::Asset
        } else {
            kind
        }
    }

    fn resource_package_owner(
        engine: &PortableTexEngine<'_>,
        name: &str,
        kind: ResourceKind,
    ) -> Option<String> {
        match kind {
            ResourceKind::Package | ResourceKind::Class => Self::resource_stem(name),
            ResourceKind::PackageSupport | ResourceKind::FontDefinition | ResourceKind::Asset => {
                Self::active_package_owner(engine)
            }
            _ => None,
        }
    }

    fn active_package_owner(engine: &PortableTexEngine<'_>) -> Option<String> {
        engine.current_input_package_owner.clone()
    }

    fn looks_like_package_asset(name: &str) -> bool {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let name = name.to_ascii_lowercase();
        !(name.ends_with(".tex")
            || name.ends_with(".ltx")
            || name.ends_with(".sty")
            || name.ends_with(".cls")
            || name.ends_with(".fd")
            || name.ends_with(".clo")
            || name.ends_with(".def")
            || name.ends_with(".ldf")
            || name.ends_with(".cfg"))
    }

    fn resource_stem(name: &str) -> Option<String> {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        if stem.is_empty() {
            None
        } else {
            Some(stem.to_string())
        }
    }

    fn source_index(value: usize) -> u32 {
        value.min(u32::MAX as usize) as u32
    }

    fn virtual_file_key(name: &str) -> String {
        let mut name = name;
        while let Some(stripped) = name.strip_prefix("./") {
            name = stripped;
        }
        name.to_string()
    }

    fn normalized_runtime_resource_name(mut name: &str) -> &str {
        while let Some(stripped) = name.strip_prefix("./") {
            name = stripped;
        }
        name
    }

    // Hand written `EngineFlow` bridge, `Ok(None)` means nothing was interned and the flow pass leaves it alone.
    unsafe fn intern_static_pool_string(
        engine: &mut PortableTexEngine<'_>,
        text: &str,
    ) -> EngineFlow<Option<strnumber>> {
        if engine.state.strpool.is_null() || engine.state.strstart.is_null() {
            return Ok(None);
        }

        let Ok(needed) = integer::try_from(text.encode_utf16().count()) else {
            return Ok(None);
        };
        let Some(next_pool) = engine.state.poolptr.checked_add(needed) else {
            return Ok(None);
        };
        if next_pool > engine.state.poolsize {
            return Ok(None);
        }

        for unit in text.encode_utf16() {
            *engine.state.strpool.offset(engine.state.poolptr as isize) = unit as packedUTF16code;
            engine.state.poolptr += 1;
        }

        Ok(Some(engine.makestring()?))
    }

    unsafe fn append_text_to_pool(engine: &mut PortableTexEngine<'_>, text: &str) -> boolean {
        if engine.state.strpool.is_null() {
            return false_0;
        }
        let needed = match integer::try_from(text.encode_utf16().count()) {
            Ok(needed) => needed,
            Err(_) => return false_0,
        };
        let Some(next_pool) = engine.state.poolptr.checked_add(needed) else {
            return false_0;
        };
        if next_pool > engine.state.poolsize {
            return false_0;
        }
        for unit in text.encode_utf16() {
            *engine.state.strpool.offset(engine.state.poolptr as isize) = unit as packedUTF16code;
            engine.state.poolptr += 1;
        }
        true_0
    }

    /// `\filesize` of a TeX input: the provider's byte length, or nothing for a missing file as in pdfTeX.
    pub(crate) unsafe fn boundary_get_file_size(
        engine: *mut PortableTexEngine<'_>,
        string: integer,
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        let Some(name) = engine.pool_string(string as strnumber) else {
            return;
        };
        let name = Self::normalized_runtime_resource_name(name.as_str()).to_string();
        let kind = Self::resource_kind_for_open(engine, &name, resource_format_tex_input);
        let package = Self::resource_package_owner(engine, &name, kind);
        let size = match engine.virtual_files.get(&Self::virtual_file_key(&name)) {
            Some(bytes) => Some(bytes.len()),
            None => engine
                .resources
                .read(ResourceRequest {
                    name: &name,
                    kind,
                    package: package.as_deref(),
                    format: resource_format_tex_input,
                    mode: FILE_SIZE_PROBE_MODE,
                    source: None,
                })
                .map(|bytes| bytes.len()),
        };
        engine.resource_requests = engine.resource_requests.saturating_add(1);
        engine.resource_request_records.push(PortableResourceRequestRecord {
            name,
            kind,
            package,
            format: resource_format_tex_input,
            mode: FILE_SIZE_PROBE_MODE.to_string(),
            source: None,
            byte_len: size.map(Self::source_index),
        });
        // An empty expansion is how expl3 and `\IfFileExists` learn a file is missing.
        if let Some(size) = size {
            Self::append_text_to_pool(engine, size.to_string().as_str());
        }
    }

    pub(crate) unsafe fn load_pool_strings(
        engine: &mut PortableTexEngine<'_>,
        spare_size: integer,
    ) -> EngineFlow<integer> {
        if engine.state.strpool.is_null() || engine.state.strstart.is_null()
            || spare_size <= 0
        {
            return Ok(0);
        }
        let mut used = 0_i32;
        let mut last = 0_i32;
        for line in include_str!("../pool/xetex.pool").lines() {
            if line.starts_with('*') {
                break;
            }
            let bytes = line.as_bytes();
            let text = if bytes.len() >= 2 && bytes[0].is_ascii_digit()
                && bytes[1].is_ascii_digit()
            {
                &line[2..]
            } else {
                line
            };
            let units = text.encode_utf16().count().min(i32::MAX as usize) as integer;
            used = used.saturating_add(units);
            if used >= spare_size
                || engine.state.poolptr.saturating_add(units) > engine.state.poolsize
            {
                return Ok(0);
            }
            for unit in text.encode_utf16() {
                *engine.state.strpool.offset(engine.state.poolptr as isize) = unit
                    as packedUTF16code;
                engine.state.poolptr += 1;
            }
            last = engine.makestring()?;
        }
        Ok(last)
    }

    pub(crate) unsafe fn boundary_open_log_file(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        let Some(engine) = engine.as_mut() else {
            return Ok(());
        };
        let old_setting = engine.state.selector;
        if engine.state.jobname == 0 {
            if let Some(jobname) = Self::intern_static_pool_string(engine, "texput")? {
                engine.state.jobname = jobname;
            }
        }
        engine.state.logopened = true_0 as boolean;
        engine.state.selector = (old_setting as i32 + 2).clamp(0, 21) as eightbits;
        Ok(())
    }

    pub(crate) unsafe fn boundary_jump_out(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<core::convert::Infallible> {
        // A history of at most warnings is a normal end with status 0, anything worse aborts with status 1.
        let status = if let Some(engine) = engine.as_ref() {
            if engine.state.history as i32 <= 1 {
                0 as integer
            } else {
                1 as integer
            }
        } else {
            1 as integer
        };
        Self::abort_engine(engine, status)
    }

    pub(crate) unsafe fn boundary_shipout(
        engine: *mut PortableTexEngine<'_>,
        box_node: halfword,
    ) {
        if let Some(engine) = engine.as_mut() {
            engine.stripped_shipouts = engine.stripped_shipouts.saturating_add(1);
            engine.last_stripped_shipout_box = Some(PortableNodeHandle(box_node));
        }
    }

    /// `box_end` hook: takes the first box appended to the outer vertical list as the fragment root.
    pub(crate) unsafe fn boundary_capture_fragment_box(
        engine: *mut PortableTexEngine<'_>,
        box_node: halfword,
        mode: integer,
        boxcontext: integer,
    ) -> boolean {
        let Some(engine) = engine.as_mut() else {
            return false_0;
        };
        // Boxes inside `\vbox`, `\vtop` and `\vcenter` are in internal vertical mode, which is negative.
        if !engine.fragment_capture_enabled
            || engine.captured_fragment_root.is_some()
            || mode != VMODE
            || boxcontext >= BOX_FLAG
        {
            return false_0;
        }
        engine.captured_fragment_root = Some(PortableNodeHandle(box_node));
        true_0
    }

    /// `build_page`: the page builder is stripped, contributions stay on the vertical list.
    pub(crate) unsafe fn boundary_build_page(engine: *mut PortableTexEngine<'_>) {
        if let Some(engine) = engine.as_mut() {
            engine.stripped_page_builds = engine.stripped_page_builds.saturating_add(1);
        }
    }

    /// XeTeX `prune_page_top`: frees or saves what precedes a split's first box, then adds `\splittopskip` glue.
    pub(crate) unsafe fn boundary_prune_page_top(
        engine: *mut PortableTexEngine<'_>,
        mut p: halfword,
        saving: boolean,
    ) -> EngineFlow<halfword> {
        let Some(this) = engine.as_mut() else {
            return Ok(p);
        };
        this.stripped_page_top_prunes = this.stripped_page_top_prunes.saturating_add(1);
        let mem = this.state.zmem;
        let temp_head = this.state.memtop - 3;
        let mut prev_p = temp_head;
        (*mem.offset(temp_head as isize)).hh.v.RH = p;
        // Last node moved to the split discards, `r` in xetex.web.
        let mut last_discard = TEX_NULL;
        while p != TEX_NULL {
            match (*mem.offset(p as isize)).hh.u.B0 as i32 {
                HLIST_NODE | VLIST_NODE | RULE_NODE => {
                    let q = (*engine).znewskipparam(SPLIT_TOP_SKIP_CODE)?;
                    let this = &mut *engine;
                    (*mem.offset(prev_p as isize)).hh.v.RH = q;
                    (*mem.offset(q as isize)).hh.v.RH = p;
                    // `new_skip_param` leaves the fresh glue spec in `temp_ptr`.
                    let spec = this.state.tempptr;
                    let width = (*mem.offset((spec + 1) as isize)).u.CINT;
                    let height = (*mem.offset((p + 3) as isize)).u.CINT;
                    (*mem.offset((spec + 1) as isize)).u.CINT =
                        if width > height { width - height } else { 0 };
                    p = TEX_NULL;
                }
                WHATSIT_NODE | MARK_NODE | INS_NODE => {
                    prev_p = p;
                    p = (*mem.offset(prev_p as isize)).hh.v.RH;
                }
                GLUE_NODE | KERN_NODE | PENALTY_NODE => {
                    let q = p;
                    p = (*mem.offset(q as isize)).hh.v.RH;
                    (*mem.offset(q as isize)).hh.v.RH = TEX_NULL;
                    (*mem.offset(prev_p as isize)).hh.v.RH = p;
                    if saving != false_0 {
                        let this = &mut *engine;
                        if this.state.discptr[VSPLIT_CODE] == TEX_NULL {
                            this.state.discptr[VSPLIT_CODE] = q;
                        } else {
                            (*mem.offset(last_discard as isize)).hh.v.RH = q;
                        }
                        last_discard = q;
                    } else {
                        (*engine).zflushnodelist(q)?;
                    }
                }
                _ => match (*engine).zconfusion(POOL_PRUNING)? {},
            }
        }
        Ok((*mem.offset(temp_head as isize)).hh.v.RH)
    }

    pub(crate) unsafe fn boundary_special_out(
        engine: *mut PortableTexEngine<'_>,
        node: halfword,
    ) -> EngineFlow<()> {
        let Some(engine) = engine.as_mut() else {
            return Ok(());
        };
        if node < engine.state.memmin || node > engine.state.memend {
            engine.stripped_special_outputs = engine
                .stripped_special_outputs
                .saturating_add(1);
            return Ok(());
        }
        let mem = engine.state.zmem.as_mut_ptr();
        if (*mem.offset(node as isize)).hh.u.B0 as i32 == WHATSIT_NODE {
            match (*mem.offset(node as isize)).hh.u.B1 as i32 {
                OPEN_NODE => {
                    Self::boundary_open_write_whatsit(engine, node);
                    return Ok(());
                }
                WRITE_NODE => {
                    Self::boundary_write_whatsit(engine, node)?;
                    return Ok(());
                }
                CLOSE_NODE => {
                    Self::boundary_close_write_whatsit(engine, node);
                    return Ok(());
                }
                _ => {}
            }
        }
        engine.stripped_special_outputs = engine
            .stripped_special_outputs
            .saturating_add(1);
        Ok(())
    }

    unsafe fn boundary_open_write_whatsit(engine: &mut PortableTexEngine<'_>, node: halfword) {
        let mem = engine.state.zmem.as_mut_ptr();
        let stream = (*mem.offset((node + 1) as isize)).hh.v.LH as usize;
        if stream >= STREAM_COUNT {
            return;
        }

        if engine.state.writeopen[stream] != 0 {
            Self::boundary_close_write_stream(engine, stream);
        }

        engine.state.curname = (*mem.offset((node + 1) as isize)).hh.v.RH as strnumber;
        engine.state.curarea = (*mem.offset((node + 2) as isize)).hh.v.LH as strnumber;
        engine.state.curext = (*mem.offset((node + 2) as isize)).hh.v.RH as strnumber;
        if engine.state.curext == POOL_EMPTY_STRING {
            engine.state.curext = POOL_TEX_EXTENSION;
        }
        engine.zpackfilename(engine.state.curname, engine.state.curarea, engine.state.curext);
        let Some(name) = Self::current_resource_name(engine as *mut PortableTexEngine<'_>) else {
            return;
        };
        let handle = Box::new(PortableFileHandle::new(
            name,
            None,
            resource_format_tex_input,
            Vec::new(),
        ));
        engine.state.writefile[stream] = Box::into_raw(handle);
        engine.state.writeopen[stream] = true_0;
    }

    unsafe fn boundary_write_whatsit(
        engine: &mut PortableTexEngine<'_>,
        node: halfword,
    ) -> EngineFlow<()> {
        let mem = engine.state.zmem.as_mut_ptr();
        let stream = (*mem.offset((node + 1) as isize)).hh.v.LH as usize;
        if stream >= STREAM_COUNT || engine.state.writeopen[stream] == 0 {
            return Ok(());
        }
        let q = engine.getavail()?;
        (*mem.offset(q as isize)).hh.v.LH = WRITE_RIGHT_BRACE_TOKEN;
        let r = engine.getavail()?;
        (*mem.offset(q as isize)).hh.v.RH = r;
        (*mem.offset(r as isize)).hh.v.LH = WRITE_END_TOKEN;
        engine.zbegintokenlist(q, TOKEN_TYPE_INSERTED)?;
        engine
            .zbegintokenlist(
                (*mem.offset((node + 1) as isize)).hh.v.RH,
                TOKEN_TYPE_WRITE_TEXT,
            )?;
        let q = engine.getavail()?;
        (*mem.offset(q as isize)).hh.v.LH = WRITE_LEFT_BRACE_TOKEN;
        engine.zbegintokenlist(q, TOKEN_TYPE_INSERTED)?;
        let old_mode = engine.state.curlist.modefield;
        engine.state.curlist.modefield = 0;
        engine.state.curcs = engine.state.writeloc;
        engine.zscantoks(false_0, true_0)?;
        engine.state.curlist.modefield = old_mode;
        engine.gettoken()?;
        if engine.state.curtok != WRITE_END_TOKEN {
            while engine.state.curtok != WRITE_END_TOKEN {
                engine.gettoken()?;
            }
        }
        engine.endtokenlist()?;
        let old_setting = engine.state.selector;
        engine.state.selector = stream as eightbits;
        engine.ztokenshow(engine.state.defref);
        engine.println();
        engine.state.selector = old_setting;
        engine.zflushlist(engine.state.defref);
        Ok(())
    }

    unsafe fn boundary_close_write_whatsit(engine: &mut PortableTexEngine<'_>, node: halfword) {
        let mem = engine.state.zmem.as_mut_ptr();
        let stream = (*mem.offset((node + 1) as isize)).hh.v.LH as usize;
        if stream < STREAM_COUNT {
            Self::boundary_close_write_stream(engine, stream);
        }
    }

    unsafe fn boundary_close_write_stream(engine: &mut PortableTexEngine<'_>, stream: usize) {
        if stream >= STREAM_COUNT || engine.state.writeopen[stream] == 0 {
            return;
        }
        let file = engine.state.writefile[stream];
        engine.state.writefile[stream] = core::ptr::null_mut();
        engine.state.writeopen[stream] = false_0;
        if file.is_null() {
            return;
        }
        let handle = Box::from_raw(file);
        let key = Self::virtual_file_key(handle.name.as_str());
        engine.virtual_files.insert(key, handle.bytes);
    }

    pub(crate) unsafe fn boundary_load_picture(
        engine: *mut PortableTexEngine<'_>,
        _is_pdf: boolean,
    ) {
        if let Some(engine) = engine.as_mut() {
            engine.stripped_picture_loads = engine.stripped_picture_loads.saturating_add(1);
        }
    }

    pub(crate) fn record_stripped_source_special(engine: *mut PortableTexEngine<'_>) {
        if let Some(engine) = unsafe { engine.as_mut() } {
            engine.stripped_source_specials = engine.stripped_source_specials.saturating_add(1);
        }
    }

    pub(crate) fn record_stripped_write_whatsit_diagnostic(engine: *mut PortableTexEngine<'_>) {
        if let Some(engine) = unsafe { engine.as_mut() } {
            engine.stripped_write_whatsit_diagnostics =
                engine.stripped_write_whatsit_diagnostics.saturating_add(1);
        }
    }

    pub(crate) fn record_stripped_pdf_extension(engine: *mut PortableTexEngine<'_>) {
        if let Some(engine) = unsafe { engine.as_mut() } {
            engine.stripped_pdf_extensions = engine.stripped_pdf_extensions.saturating_add(1);
        }
    }

    /// Only TeX text inputs get XeTeX's `u_open_in` encoding sniff, binary inputs such as TFM files read raw.
    fn resolve_input_encoding(handle: &mut PortableFileHandle) {
        let is_text = handle.format == resource_format_tex_input || handle.format == 0;
        if is_text {
            handle.resolve_text_encoding_auto();
        } else {
            handle.encoding = InputEncoding::Bytes;
        }
    }

    pub(crate) unsafe fn boundary_open_input(
        engine: *mut PortableTexEngine<'_>,
        file: *mut NativeFileHandle,
        format: integer,
        mode: const_string,
    ) -> boolean {
        let Some(engine) = engine.as_mut() else {
            if !file.is_null() {
                *file = core::ptr::null_mut();
            }
            return false_0;
        };
        let Some(name) = Self::current_resource_name(engine as *mut PortableTexEngine<'_>) else {
            if !file.is_null() {
                *file = core::ptr::null_mut();
            }
            return false_0;
        };
        let mode = Self::mode_string(mode);
        let request_kind = Self::resource_kind_for_open(engine, name.as_str(), format);
        let request_package = Self::resource_package_owner(engine, name.as_str(), request_kind);
        let source: Option<PortableSourceSpan> = None;
        let request = ResourceRequest {
            name: name.as_str(),
            kind: request_kind,
            package: request_package.as_deref(),
            format,
            mode: mode.as_str(),
            source: source.clone(),
        };
        engine.resource_requests = engine.resource_requests.saturating_add(1);
        let virtual_key = Self::virtual_file_key(name.as_str());
        let bytes = if let Some(bytes) = engine.virtual_files.get(&virtual_key) {
            bytes.clone()
        } else if let Some(bytes) = engine.resources.read(request) {
            bytes
        } else {
            engine.resource_request_records.push(PortableResourceRequestRecord {
                name,
                kind: request_kind,
                package: request_package,
                format,
                mode,
                source,
                byte_len: None,
            });
            if !file.is_null() {
                *file = core::ptr::null_mut();
            }
            return false_0;
        };
        let byte_len = Self::source_index(bytes.len());
        engine.resource_request_records.push(PortableResourceRequestRecord {
            name: name.clone(),
            kind: request_kind,
            package: request_package.clone(),
            format,
            mode: mode.clone(),
            source,
            byte_len: Some(byte_len),
        });
        let mut handle = Box::new(PortableFileHandle::new(
            name,
            request_package,
            format,
            bytes,
        ));
        // Text inputs get XeTeX's `u_open_in` encoding sniff, binary inputs stay raw `Bytes`.
        Self::resolve_input_encoding(&mut handle);
        if format == resource_format_tfm {
            engine.state.tfmtemp = handle.read_byte().map_or(-1, |byte| byte as integer);
        }
        if !file.is_null() {
            *file = Box::into_raw(handle);
            return true_0;
        }
        drop(handle);
        false_0
    }

    unsafe fn begin_primary_input_raw(
        self: &mut Self,
        name: &str,
        bytes: Vec<u8>,
    ) -> EngineFlow<boolean> {
        if self.state.inputfile.is_null() || self.state.sourcefilenamestack.is_null()
            || self.state.fullsourcefilenamestack.is_null()
            || self.state.buffer.is_null()
        {
            return Ok(false_0);
        }
        let Some(source_name) = Self::intern_static_pool_string(self, name)? else {
            return Ok(false_0);
        };
        self.beginfilereading()?;
        let slot = self.state.curinput.indexfield as isize;
        let mut handle = Box::new(
            PortableFileHandle::new(
                name.to_string(),
                None,
                resource_format_tex_input,
                bytes,
            ),
        );
        Self::resolve_input_encoding(&mut handle);
        *self.state.inputfile.offset(slot) = Box::into_raw(handle);
        self.state.curinput.namefield = source_name as halfword;
        *self.state.sourcefilenamestack.offset(slot) = source_name;
        *self.state.fullsourcefilenamestack.offset(slot) = source_name;
        self.state.curinput.statefield = NEW_LINE_STATE;
        self.state.line = 1 as integer;
        self.state.src_primary_name = source_name;
        self.reset_source_tracking_registers();
        if Self::boundary_input_line(
            self as *mut PortableTexEngine<'_>,
            *self.state.inputfile.offset(slot) as NativeFileHandle,
        )? == 0
        {
            self.endfilereading();
            return Ok(false_0);
        }
        self.firmuptheline()?;
        let eqtb = self.state.zeqtb.as_mut_ptr();
        let endline_char = (*eqtb.offset(EQTB_END_LINE_CHAR_LOC as isize)).u.CINT;
        if !(0..=255).contains(&endline_char) {
            self.state.curinput.limitfield -= 1;
        } else {
            *self.state.buffer.offset(self.state.curinput.limitfield as isize) = endline_char
                as UnicodeScalar;
        }
        self.state.first = (self.state.curinput.limitfield as i32 + 1) as integer;
        self.state.curinput.locfield = self.state.curinput.startfield;
        Ok(true_0)
    }

    /// XeTeX `input_line`: decodes one line into `buffer[first..last]` and records each slot's input bytes.
    pub(crate) unsafe fn boundary_input_line(
        engine: *mut PortableTexEngine<'_>,
        file: NativeFileHandle,
    ) -> EngineFlow<boolean> {
        let Some(engine) = engine.as_mut() else {
            return Ok(false_0);
        };
        if file.is_null() || engine.state.buffer.is_null() {
            return Ok(false_0);
        }

        let handle = &mut *file;
        if !handle.has_remaining() {
            return Ok(false_0);
        }

        let first = engine.state.first.max(0) as usize;
        let limit = engine.state.bufsize.max(0) as usize;
        let mut last = first;
        // Byte offset where the line's content stops, before its terminator.
        let content_end;
        loop {
            let start = handle.next_scalar_offset();
            let Some(scalar) = handle.next_input_scalar() else {
                content_end = handle.cursor;
                break;
            };
            match scalar {
                0x0A => {
                    content_end = start;
                    break;
                }
                0x0D => {
                    content_end = start;
                    // A CR swallows a following LF, the peek is undone when the next scalar is not LF.
                    let saved_cursor = handle.cursor;
                    let saved_lookahead = handle.saved_char;
                    let was_eof = handle.eof_after_failed_read;
                    if handle.next_input_scalar() != Some(0x0A) {
                        handle.cursor = saved_cursor;
                        handle.saved_char = saved_lookahead;
                        handle.eof_after_failed_read = was_eof;
                    }
                    break;
                }
                scalar => {
                    if last >= limit {
                        // TeX reports the partial line as the error context before giving up.
                        engine.state.curinput.locfield = first as halfword;
                        engine.state.curinput.limitfield = last as halfword - 1;
                        let size = engine.state.bufsize;
                        match engine.zoverflow(POOL_BUFFER_SIZE, size)? {}
                    }
                    *engine.state.buffer.add(last) = scalar as UnicodeScalar;
                    Self::record_slot_bytes(engine, last, (Self::source_index(start), 0));
                    last += 1;
                }
            }
        }
        let decoded_last = last;
        let line_break_end = Self::source_index(handle.cursor);

        while last > first && *engine.state.buffer.add(last - 1) == b' ' as UnicodeScalar {
            last -= 1;
        }
        if last >= engine.state.maxbufstack.max(0) as usize {
            engine.state.maxbufstack = last as integer;
        }
        // Each slot ends where the next begins, and the limit slot holding `\endlinechar` spans the line break.
        let kept_end = if last < decoded_last {
            engine.heap.src_buf_bytes[last].0
        } else {
            Self::source_index(content_end)
        };
        for slot in first..last {
            let end = if slot + 1 < last {
                engine.heap.src_buf_bytes[slot + 1].0
            } else {
                kept_end
            };
            engine.heap.src_buf_bytes[slot].1 = end;
        }
        Self::record_slot_bytes(engine, last, (kept_end, line_break_end));

        if handle.format == resource_format_tex_input || handle.format == 0 {
            engine.current_input_package_owner = handle.package.clone();
        }
        // A token that straddles the line break is lexed in the refilling `get_next` call, so restart its span.
        if engine.state.source_tracking {
            engine.state.src_token_start = first as integer;
        }
        engine.state.last = last as integer;
        Ok(true_0)
    }

    fn record_slot_bytes(engine: &mut PortableTexEngine<'_>, slot: usize, bytes: (u32, u32)) {
        let map = &mut engine.heap.src_buf_bytes;
        if map.len() <= slot {
            map.resize(slot + 1, (0, 0));
        }
        map[slot] = bytes;
    }

    pub(crate) unsafe fn boundary_read_byte(_file: NativeFileHandle) -> integer {
        if _file.is_null() {
            return -1;
        }
        (&mut *_file).read_byte().map_or(-1, |byte| byte as integer)
    }

    pub(crate) unsafe fn boundary_end_of_file(_file: NativeFileHandle) -> integer {
        if _file.is_null() || (&*_file).is_eof() {
            1
        } else {
            0
        }
    }

    pub(crate) unsafe fn boundary_flush_file(_file: NativeFileHandle) -> integer {
        0
    }

    /// `wlog` and file output: a null handle is the log, which the transcript records.
    pub(crate) unsafe fn boundary_write_byte(
        engine: *mut PortableTexEngine<'_>,
        character: integer,
        file: NativeFileHandle,
    ) -> integer {
        if !file.is_null() {
            if let Ok(byte) = u8::try_from(character) {
                (*file).bytes.push(byte);
            }
            return character;
        }
        if let Some(engine) = engine.as_mut() {
            if let Ok(byte) = u8::try_from(character) {
                engine.transcript_bytes.push(byte);
            }
        }
        character
    }

    /// `wterm`: terminal output joins the transcript only until the log opens, the log repeats it after that.
    pub(crate) unsafe fn boundary_write_terminal_byte(
        engine: *mut PortableTexEngine<'_>,
        character: integer,
        _file: NativeFileHandle,
    ) -> integer {
        if let Some(engine) = engine.as_mut() {
            if engine.state.logopened == false_0 {
                if let Ok(byte) = u8::try_from(character) {
                    engine.transcript_bytes.push(byte);
                }
            }
        }
        character
    }

    /// `print_nl` hook: starts capturing an error message once `print_err` has printed its prefix.
    pub(crate) fn begin_error_message(this: &mut Self, s: strnumber) {
        if s == POOL_ERROR_PREFIX {
            this.error_message_capture = Some(Vec::new());
        }
    }

    /// `print_file_line` hook: the prefix `print_err` uses in file line error style.
    pub(crate) fn begin_file_line_error_message(this: &mut Self) {
        this.error_message_capture = Some(Vec::new());
    }

    /// `print_raw_char` hook: records a message byte while an error message is being printed.
    pub(crate) fn capture_error_byte(this: &mut Self, s: UnicodeScalar) {
        if this.state.selector < SELECTOR_PSEUDO {
            if let (Some(capture), Ok(byte)) = (this.error_message_capture.as_mut(), u8::try_from(s)) {
                capture.push(byte);
            }
        }
    }

    /// `error` hook: the message is complete once `error` starts, before it prints its period and context.
    pub(crate) fn end_error_message(this: &mut Self) {
        if let Some(capture) = this.error_message_capture.take() {
            this.captured_error_message = Some(String::from_utf8_lossy(&capture).into_owned());
        }
    }

    pub(crate) unsafe fn boundary_close_file(_file: NativeFileHandle) {
        if !_file.is_null() {
            drop(Box::from_raw(_file));
        }
    }

    pub(crate) unsafe fn get_seconds_and_micros(
        engine: *mut PortableTexEngine<'_>,
        seconds: *mut integer,
        micros: *mut integer,
    ) {
        let clock = engine
            .as_mut()
            .map(|engine| engine.platform.clock())
            .unwrap_or_default();
        if !seconds.is_null() {
            *seconds = clock.seconds;
        }
        if !micros.is_null() {
            *micros = clock.micros;
        }
    }

    pub(crate) unsafe fn linebreak_start(
        engine: *mut PortableTexEngine<'_>,
        font: integer,
        locale: integer,
        text: *mut uint16_t,
        text_length: integer,
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        let text = if text.is_null() || text_length <= 0 {
            &[]
        } else {
            core::slice::from_raw_parts(text as *const uint16_t, text_length as usize)
        };
        engine
            .platform
            .linebreak_start(PortableLinebreakRequest { font, locale, text });
    }

    pub(crate) unsafe fn linebreak_next(engine: *mut PortableTexEngine<'_>) -> integer {
        engine
            .as_mut()
            .and_then(|engine| engine.platform.linebreak_next())
            .unwrap_or(-1)
    }

    // Registers \Uhostbox during format initialization so it dumps with the format eqtb and hash.
    pub(crate) unsafe fn register_host_box_primitive(self: &mut Self) -> EngineFlow<()> {
        let eqtb = self.state.zeqtb.as_mut_ptr();
        let name = b"Uhostbox";
        for (index, byte) in name.iter().enumerate() {
            *self.state.buffer.add(index) = *byte as UnicodeScalar;
        }
        let saved = self.state.nonewcontrolsequence;
        self.state.nonewcontrolsequence = 0 as boolean;
        let cs = (&mut *(self as *mut PortableTexEngine<'_>))
            .zidlookup(0 as integer, name.len() as integer)?;
        self.state.nonewcontrolsequence = saved;
        (*eqtb.offset(cs as isize)).hh.u.B1 = LEVEL_ONE;
        (*eqtb.offset(cs as isize)).hh.u.B0 = CMD_EXTENSION;
        (*eqtb.offset(cs as isize)).hh.v.RH = HOST_BOX_EXTENSION_CODE as halfword;
        Ok(())
    }

    // A five word marker whatsit laid out like a glyph node, metrics in words 1 to 3 and its payload in word 4.
    unsafe fn host_box_marker(
        engine: *mut PortableTexEngine<'_>,
        subtype: i32,
        payload: integer,
        width: scaled,
        height: scaled,
        depth: scaled,
    ) -> EngineFlow<halfword> {
        let this = &mut *engine;
        let marker = this.zgetnode(HOST_BOX_MARKER_SIZE)?;
        let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
        (*mem.offset(marker as isize)).hh.u.B0 = WHATSIT_NODE as i16;
        (*mem.offset(marker as isize)).hh.u.B1 = subtype as i16;
        (*mem.offset(marker as isize)).hh.v.RH = TEX_NULL;
        (*mem.offset((marker as i32 + 1 as i32) as isize)).u.CINT = width;
        (*mem.offset((marker as i32 + 2 as i32) as isize)).u.CINT = depth;
        (*mem.offset((marker as i32 + 3 as i32) as isize)).u.CINT = height;
        (*mem.offset((marker as i32 + 4 as i32) as isize)).u.CINT = payload;
        Ok(marker)
    }

    // Asks the host for the box behind `token`, None falls back to a deterministic zero size box.
    unsafe fn host_box_build(
        engine: *mut PortableTexEngine<'_>,
        token: integer,
        style: PortableHostBoxStyle,
        font_size: scaled,
    ) -> EngineFlow<halfword> {
        let this = &mut *engine;
        let response = this.platform.host_box(PortableHostBoxRequest {
            token,
            style,
            font_size,
        });
        let hbox = this.newnullbox()?;
        let Some(host_box) = response else {
            return Ok(hbox);
        };
        let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
        (*mem.offset((hbox as i32 + 1 as i32) as isize)).u.CINT = host_box.width;
        (*mem.offset((hbox as i32 + 2 as i32) as isize)).u.CINT = host_box.depth;
        (*mem.offset((hbox as i32 + 3 as i32) as isize)).u.CINT = host_box.height;
        let id = this.state.host_box_next_record;
        this.state.host_box_next_record = id.saturating_add(1);
        let (width, height, depth) = (host_box.width, host_box.height, host_box.depth);
        this.hostbox_records.insert(id, host_box);
        let marker = Self::host_box_marker(
            engine,
            HOST_BOX_RESOLVED_SUBTYPE,
            id,
            width,
            height,
            depth,
        )?;
        (*mem.offset((hbox as i32 + 5 as i32) as isize)).hh.v.RH = marker;
        Ok(hbox)
    }

    // \Uhostbox scan site: math defers to the mlist pass for style, other modes resolve now as Text.
    pub(crate) unsafe fn host_box_insert(
        engine: *mut PortableTexEngine<'_>,
        token: integer,
    ) -> EngineFlow<()> {
        let Some(this) = engine.as_mut() else {
            return Ok(());
        };
        let mode = (this.state.curlist.modefield as i32).abs();
        if mode == MMODE {
            let marker =
                Self::host_box_marker(engine, HOST_BOX_PENDING_SUBTYPE, token, 0, 0, 0)?;
            let this = &mut *engine;
            let placeholder = this.newnullbox()?;
            let noad = this.newnoad()?;
            let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
            (*mem.offset((placeholder as i32 + 5 as i32) as isize)).hh.v.RH = marker;
            (*mem.offset((noad as i32 + 1 as i32) as isize)).hh.v.RH = SUB_BOX;
            (*mem.offset((noad as i32 + 1 as i32) as isize)).hh.v.LH = placeholder;
            (*mem.offset(this.state.curlist.tailfield as isize)).hh.v.RH = noad;
            this.state.curlist.tailfield = noad;
        } else {
            let eqtb = this.state.zeqtb.as_mut_ptr();
            let font = (*eqtb.offset(CUR_FONT_LOC as isize)).hh.v.RH as integer;
            let font_size = this.font_at_size(font);
            let hbox =
                Self::host_box_build(engine, token, PortableHostBoxStyle::Text, font_size)?;
            // `box_end` appends like `\hbox`, with baselineskip in vertical mode and space factor 1000 in text.
            let this = &mut *engine;
            this.state.curbox = hbox;
            this.zboxend(0)?;
        }
        Ok(())
    }

    // mlist pass hook: swap a pending placeholder nucleus for the host's box under curstyle.
    pub(crate) unsafe fn host_box_resolve_noad(
        engine: *mut PortableTexEngine<'_>,
        q: halfword,
    ) -> EngineFlow<()> {
        let Some(this) = engine.as_mut() else {
            return Ok(());
        };
        if q < 0 || q >= this.state.himemmin {
            return Ok(());
        }
        let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
        // A placeholder can reach the nucleus of any noad with one, through a math group collapse.
        let noad_type = (*mem.offset(q as isize)).hh.u.B0 as i32;
        if !(ORD_NOAD..=VCENTER_NOAD).contains(&noad_type) || noad_type == FRACTION_NOAD {
            return Ok(());
        }
        let style_code = this.state.curstyle as i32;
        Self::host_box_resolve_field(engine, (q as i32 + 1 as i32) as halfword, style_code)
    }

    // clean_box hook: fields copied out of single atom groups (scripts, fractions) resolve here.
    pub(crate) unsafe fn host_box_resolve_field(
        engine: *mut PortableTexEngine<'_>,
        field: halfword,
        style_code: i32,
    ) -> EngineFlow<()> {
        let Some(this) = engine.as_mut() else {
            return Ok(());
        };
        if field < 0 {
            return Ok(());
        }
        let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
        // The field must hold a box whose list is a pending marker whatsit.
        if (*mem.offset(field as isize)).hh.v.RH != SUB_BOX {
            return Ok(());
        }
        let placeholder = (*mem.offset(field as isize)).hh.v.LH;
        if placeholder < 0 || placeholder == TEX_NULL {
            return Ok(());
        }
        let marker = (*mem.offset((placeholder as i32 + 5 as i32) as isize)).hh.v.RH;
        if marker < 0 || marker == TEX_NULL || marker >= this.state.himemmin {
            return Ok(());
        }
        if (*mem.offset(marker as isize)).hh.u.B0 as i32 != WHATSIT_NODE
            || (*mem.offset(marker as isize)).hh.u.B1 as i32 != HOST_BOX_PENDING_SUBTYPE
        {
            return Ok(());
        }
        let token = (*mem.offset((marker as i32 + 4 as i32) as isize)).u.CINT;
        // Styles map to sizes as `mlist_to_hlist` maps them, display and text styles share the text size.
        let size = if style_code < SCRIPT_STYLE {
            0
        } else {
            SCRIPT_SIZE * ((style_code - 2) / 2)
        };
        let style = match size {
            0 => PortableHostBoxStyle::Text,
            SCRIPT_SIZE => PortableHostBoxStyle::Script,
            _ => PortableHostBoxStyle::ScriptScript,
        };
        // Size context comes from the family 2 symbol font at the active math size.
        let eqtb = this.state.zeqtb.as_mut_ptr();
        let font = (*eqtb.offset((MATH_FONT_BASE + 2 + size as i64) as isize))
            .hh
            .v
            .RH as integer;
        let font_size = this.font_at_size(font);
        let hbox = Self::host_box_build(engine, token, style, font_size)?;
        // The placeholder carries the `\hostbox` span, copy it to its replacement box and that box's marker.
        Self::src_carry_copy(engine as *mut Self, placeholder, hbox);
        let mem: *mut memoryword = (*engine).state.zmem.as_mut_ptr();
        let resolved_marker = (*mem.offset((hbox as i32 + 5 as i32) as isize)).hh.v.RH;
        if resolved_marker >= 0 && resolved_marker != TEX_NULL {
            Self::src_carry_copy(engine as *mut Self, placeholder, resolved_marker);
        }
        (&mut *engine).zflushnodelist(placeholder)?;
        let this = &mut *engine;
        let mem: *mut memoryword = this.state.zmem.as_mut_ptr();
        (*mem.offset(field as isize)).hh.v.LH = hbox;
        Ok(())
    }

    pub(crate) unsafe fn abort_engine(
        engine: *mut PortableTexEngine<'_>,
        status: integer,
    ) -> EngineFlow<core::convert::Infallible> {
        if let Some(engine) = engine.as_mut() {
            engine.last_abort_status = Some(status);
        }
        Err(EngineBreak::Abort(EngineAbort { status }))
    }

    /// Stops the run with `message` when the sandbox is on, a no op otherwise.
    pub(crate) unsafe fn sandbox_reject(
        engine: *mut PortableTexEngine<'_>,
        message: &str,
    ) -> EngineFlow<()> {
        if let Some(engine) = engine.as_ref() {
            if engine.sandbox {
                return Err(EngineBreak::Error(EngineError {
                    message: message.into(),
                    kind: PortableErrorKind::Sandbox,
                }));
            }
        }
        Ok(())
    }

    /// `init_math` sandbox hook: math nested in text is allowed, reopening math after the wrapper closed is not.
    pub(crate) unsafe fn sandbox_open_math(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        if let Some(engine) = engine.as_mut() {
            if engine.sandbox {
                if engine.sandbox_math_depth == 0 && engine.sandbox_math_opened {
                    return Err(EngineBreak::Error(EngineError {
                        message: MATH_BREAKOUT_MESSAGE.into(),
                        kind: PortableErrorKind::Sandbox,
                    }));
                }
                engine.sandbox_math_opened = true;
                engine.sandbox_math_depth += 1;
            }
        }
        Ok(())
    }

    /// `after_math` sandbox hook: leaves one math level, only the wrapper suffix may close the wrapper math.
    pub(crate) unsafe fn sandbox_close_math(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        if let Some(engine) = engine.as_mut() {
            if engine.sandbox && engine.sandbox_math_depth > 0 {
                if engine.sandbox_math_depth == 1 && !engine.token_is_from_wrapper_suffix() {
                    return Err(EngineBreak::Error(EngineError {
                        message: MATH_BREAKOUT_MESSAGE.into(),
                        kind: PortableErrorKind::Sandbox,
                    }));
                }
                engine.sandbox_math_depth -= 1;
            }
        }
        Ok(())
    }

    /// Whether the token just read comes from the marked wrapper suffix, true when the host marked none.
    fn token_is_from_wrapper_suffix(&self) -> bool {
        let Some(suffix) = self.sandbox_suffix_start else {
            return true;
        };
        // A backed up list replays a token its file level already consumed, so the file level tells the origin.
        let mut level = self.state.curinput;
        let mut below = usize::try_from(self.state.inputptr).unwrap_or(0);
        while level.statefield == TOKEN_LIST_STATE
            && (level.indexfield == TOKEN_TYPE_BACKED_UP || level.indexfield == TOKEN_TYPE_BACKED_UP_CHAR)
        {
            let Some(next) = below.checked_sub(1).and_then(|index| self.heap.inputstack_storage.get(index)) else {
                return false;
            };
            level = *next;
            below -= 1;
        }
        if level.statefield == TOKEN_LIST_STATE || level.namefield != self.state.src_primary_name {
            return false;
        }
        // `loc` is one past the last character the file level consumed.
        let consumed = (level.locfield - 1).clamp(level.startfield, level.limitfield.max(level.startfield));
        usize::try_from(consumed)
            .ok()
            .and_then(|slot| self.heap.src_buf_bytes.get(slot))
            .is_some_and(|&(start, _)| start >= suffix)
    }

    /// Sandbox budget tick from `main_control`, `macro_call` and `expand`, so expansion loops end too.
    pub(crate) unsafe fn sandbox_tick(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        if let Some(engine) = engine.as_mut() {
            if engine.sandbox {
                engine.sandbox_ops = engine.sandbox_ops.saturating_add(1);
                if engine.sandbox_ops > engine.sandbox_op_budget {
                    return Err(EngineBreak::Error(EngineError {
                        message: "expression is too complex or did not terminate".into(),
                        kind: PortableErrorKind::Budget,
                    }));
                }
            }
        }
        Ok(())
    }

    /// `main_control` dispatch hook: after the wrapper math closes only braces, assignments and `\end` may run.
    pub(crate) unsafe fn sandbox_check_command(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        let Some(engine) = engine.as_ref() else {
            return Ok(());
        };
        if !engine.sandbox || !engine.sandbox_math_opened || engine.sandbox_math_depth != 0 {
            return Ok(());
        }
        // LaTeX's `\everymath` leaves an `\aftergroup` assignment that runs once the wrapper math closes.
        match engine.state.curcmd as integer {
            CMD_RELAX | CMD_RIGHT_BRACE | CMD_SPACER | CMD_PAR_END | CMD_STOP => Ok(()),
            cmd if cmd > CMD_MAX_NON_PREFIXED => Ok(()),
            _ => Err(EngineBreak::Error(EngineError {
                message: MATH_BREAKOUT_MESSAGE.into(),
                kind: PortableErrorKind::Sandbox,
            })),
        }
    }

    /// `\write` hook: rejects a sandboxed write to a stream open to a file, other streams reach the terminal.
    pub(crate) unsafe fn sandbox_check_write_stream(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        let Some(this) = engine.as_ref() else {
            return Ok(());
        };
        let mem = this.state.zmem;
        let stream = (*mem.offset((this.state.curlist.tailfield + 1) as isize)).hh.v.LH;
        let to_file = usize::try_from(stream)
            .ok()
            .and_then(|stream| this.state.writeopen.get(stream))
            .is_some_and(|&open| open != false_0);
        if to_file {
            Self::sandbox_reject(engine, "\\write to a file is not allowed inside a math expression")?;
        }
        Ok(())
    }

    /// `error` hook: every TeX error inside a render ends the run with its message instead of recovering.
    pub(crate) unsafe fn surface_error(
        engine: *mut PortableTexEngine<'_>,
    ) -> EngineFlow<()> {
        let message = engine
            .as_mut()
            .and_then(|engine| engine.captured_error_message.take())
            .unwrap_or_else(|| "TeX error".into());
        Err(EngineBreak::Error(EngineError {
            message,
            kind: PortableErrorKind::Tex,
        }))
    }

    /// Stops the run with XeTeX's `pdf_error` message, which upstream ends with `succumb`.
    pub(crate) unsafe fn fatal_pdf_error(
        engine: *mut PortableTexEngine<'_>,
        t: strnumber,
        p: strnumber,
    ) -> EngineFlow<()> {
        let message = engine.as_ref().map_or_else(
            || "Error".into(),
            |engine| {
                let text = |string: strnumber| engine.pool_string(string).unwrap_or_default();
                if t == 0 {
                    format!("Error: {}", text(p))
                } else {
                    format!("Error ({}): {}", text(t), text(p))
                }
            },
        );
        Err(EngineBreak::Error(EngineError {
            message,
            kind: PortableErrorKind::Tex,
        }))
    }

    // Source tracking hooks, all no ops while `source_tracking` is off.

    /// Enables or disables source tracking, clearing every table so the calls around it may come in any order.
    pub fn set_source_tracking(self: &mut Self, on: bool) {
        self.state.source_tracking = on;
        self.reset_source_tracking_registers();
        self.heap.src_spans.clear();
        self.heap.src_dedup.clear();
        let end = if on { self.heap.mem.len() } else { 0 };
        self.heap.node_src = PagedArray::new(0, end, node_src_default, node_src_sig);
        self.heap.node_stack = PagedArray::new(0, end, node_stack_default, node_stack_sig);
    }

    /// Clears the per render source tracking registers, shared by the tracking switch and input start.
    fn reset_source_tracking_registers(self: &mut Self) {
        self.state.cmd_span = 0;
        self.state.pending_call_span = 0;
        self.state.src_token_start = 0;
        self.state.src_call_start = 0;
        self.state.src_call_name = 0;
        self.state.src_call_state = 0;
        self.state.src_call_index = 0;
        self.state.src_call_span = 0;
        self.state.src_call_argspan = 0;
        self.state.src_user_cmd_span = 0;
        self.state.src_tok_span = 0;
        self.state.src_anchor_cmd = 0;
        self.state.src_grp_closing = 0;
        self.state.src_call_user_span = 0;
        self.state.curinput.spanfield = 0;
        self.state.cur_stack_head = 0;
        self.state.src_stack_frozen = false;
        self.heap.src_grp_stack.clear();
        self.heap.src_native_offsets.clear();
        self.heap.src_stack_cells.clear();
    }

    /// Whether source tracking is currently enabled.
    pub fn source_tracking_enabled(&self) -> bool {
        self.state.source_tracking
    }

    /// Interns a span, equal spans share their first `SrcId`.
    fn intern_span_raw(self: &mut Self, name: strnumber, start: u32, end: u32, role: u8) -> SrcId {
        let span = RawSpan { name, start, end, role };
        if let Some(&id) = self.heap.src_dedup.get(&span) {
            return id;
        }
        self.heap.src_spans.push(span);
        let id = self.heap.src_spans.len() as SrcId;
        self.heap.src_dedup.insert(span, id);
        id
    }

    /// Input byte offset of buffer position `loc` on the current line, past the limit is past the line break.
    fn src_buf_offset(&self, loc: integer) -> u32 {
        let start = self.state.curinput.startfield;
        let limit = self.state.curinput.limitfield;
        let slot = |index: integer| {
            usize::try_from(index)
                .ok()
                .and_then(|index| self.heap.src_buf_bytes.get(index))
                .copied()
                .unwrap_or_default()
        };
        if loc > limit && limit >= start {
            slot(limit).1
        } else {
            slot(loc.max(start)).0
        }
    }

    /// The span stamped on `node`, `None` when it is unstamped or its source name is unreadable.
    pub(crate) fn resolve_node_src(&self, node: halfword) -> Option<PortableSourceSpan> {
        if !self.state.source_tracking || node < 0 {
            return None;
        }
        self.src_span_by_id(self.heap.node_src.get_copy(node as usize))
    }

    /// The interned span `id`, `None` for id 0, while tracking is off, or when its source name is unreadable.
    fn src_span_by_id(&self, id: SrcId) -> Option<PortableSourceSpan> {
        if !self.state.source_tracking || id == 0 {
            return None;
        }
        let raw = *self.heap.src_spans.get((id - 1) as usize)?;
        let name = self.pool_string(raw.name)?;
        Some(PortableSourceSpan {
            name,
            start: raw.start,
            end: raw.end,
            role: raw.role,
        })
    }

    /// `get_next` hook at the top of its loop: marks where the next token starts, after skipped input.
    pub(crate) unsafe fn src_mark_token_start(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if engine.state.source_tracking && engine.state.curinput.statefield != TOKEN_LIST_STATE {
            engine.state.src_token_start = engine.state.curinput.locfield as integer;
        }
    }

    /// `get_next` hook at its end: the span of a token just read from a file line.
    pub(crate) unsafe fn src_record_buffer_span(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || engine.state.curinput.statefield == TOKEN_LIST_STATE {
            return;
        }
        // Names up to 19 are the terminal, `\read` streams and pseudo files, their tokens take the inherited span.
        if engine.state.curinput.namefield <= 19 {
            engine.state.src_tok_span = engine.state.curinput.spanfield;
            return;
        }
        let a = engine.src_buf_offset(engine.state.src_token_start);
        let b = engine.src_buf_offset(engine.state.curinput.locfield as integer);
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let name = engine.state.curinput.namefield as strnumber;
        let id = engine.intern_span_raw(name, lo, hi, 0);
        engine.state.curinput.spanfield = id;
        engine.state.src_tok_span = id;
        // A control sequence read from the primary input outside argument scanning is the command the user typed.
        if engine.state.curcs != 0
            && name == engine.state.src_primary_name
            && engine.state.scannerstatus as i32 == 0
        {
            engine.state.src_user_cmd_span = id;
            // Reading the buffer ends any token list replay and with it the replayed anchor command.
            engine.state.src_anchor_cmd = 0;
        }
    }

    /// `get_next` token list hook: records the origin of the token about to be read from the current list.
    pub(crate) unsafe fn src_tokenlist_span(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        // Macro bodies and the lists after them map to their invocation, never to where they were defined.
        if engine.state.curinput.indexfield >= TOKEN_TYPE_MACRO {
            engine.state.src_tok_span = engine.state.curinput.spanfield;
            return;
        }
        // Parameter, template, backed up and inserted lists replay cells that carry their own origin.
        let cell = engine.state.curinput.locfield;
        let id = if cell >= 0 {
            engine.heap.node_src.get_copy(cell as usize)
        } else {
            0
        };
        if id != 0 {
            engine.state.curinput.spanfield = id;
        }
        engine.state.src_tok_span = engine.state.curinput.spanfield;
    }

    /// `back_input` hook: stamps the backed up cell with the origin frozen when its token was last read.
    pub(crate) unsafe fn src_back_input_stamp(engine: *mut Self, p: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || p < 0 {
            return;
        }
        let id = engine.state.src_tok_span;
        if id != 0 {
            engine.heap.node_src.set(p as usize, id);
        }
    }

    /// `main_control` hook after `get_x_token`: latches the command's span before it scans its arguments.
    pub(crate) unsafe fn src_latch_cmd_span(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if engine.state.source_tracking {
            engine.state.cmd_span = engine.state.curinput.spanfield;
        }
    }

    /// `get_avail` hook: stamps a one word node with the current input span and construct stack head.
    pub(crate) unsafe fn src_stamp_avail(engine: *mut Self, node: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if engine.state.source_tracking && node >= 0 {
            let id = engine.state.curinput.spanfield;
            engine.heap.node_src.set(node as usize, id);
            engine.heap.node_stack.set(node as usize, engine.state.cur_stack_head);
        }
    }

    /// `get_node` hook: stamps the whole node with the construct span and stack head, clearing a stale occupant.
    pub(crate) unsafe fn src_stamp_node_range(engine: *mut Self, node: halfword, size: integer) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || node < 0 || size <= 0 {
            return;
        }
        let id = engine.state.cmd_span;
        let head = engine.state.cur_stack_head;
        let base = node as usize;
        for i in 0..size as usize {
            engine.heap.node_src.set(base + i, id);
            engine.heap.node_stack.set(base + i, head);
        }
    }

    /// `copy_node_list` hook: a copy keeps the span and enclosing constructs of a stamped original.
    pub(crate) unsafe fn src_carry_copy(engine: *mut Self, src: halfword, dst: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || src < 0 || dst < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(src as usize);
        if id != 0 {
            engine.heap.node_src.set(dst as usize, id);
            let head = engine.heap.node_stack.get_copy(src as usize);
            engine.heap.node_stack.set(dst as usize, head);
        }
    }

    /// `src_token_copy` hook: a copied token cell keeps the span and enclosing constructs of the cell it copies.
    pub(crate) unsafe fn src_carry_token_span(engine: *mut Self, dest: halfword, src: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || dest < 0 || src < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(src as usize);
        if id != 0 {
            engine.heap.node_src.set(dest as usize, id);
            let head = engine.heap.node_stack.get_copy(src as usize);
            engine.heap.node_stack.set(dest as usize, head);
        }
    }

    /// `new_character` hook: a TFM character takes the latched command's span, so `\char98` maps to the command.
    pub(crate) unsafe fn src_stamp_char(engine: *mut Self, node: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || node < 0 {
            return;
        }
        let id = engine.state.cmd_span;
        if id != 0 {
            engine.heap.node_src.set(node as usize, id);
        }
        engine.heap.node_stack.set(node as usize, engine.state.cur_stack_head);
    }

    /// `mlist_to_hlist` hook per noad: what the noad builds takes the noad's own span.
    pub(crate) unsafe fn src_mlist_repoint(engine: *mut Self, q: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || q < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(q as usize);
        if id != 0 {
            engine.state.cmd_span = id;
        }
    }

    /// `scan_math` hook: records the span of a one character field so `clean_box` can pass it on.
    pub(crate) unsafe fn src_stamp_field(engine: *mut Self, field: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || field < 0 {
            return;
        }
        let id = engine.state.curinput.spanfield;
        if id != 0 {
            engine.heap.node_src.set(field as usize, id);
        }
    }

    /// `clean_box` hook: the noad it builds for a one character field takes the field's span.
    pub(crate) unsafe fn src_carry_field(engine: *mut Self, field: halfword, noad: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || field < 0 || noad < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(field as usize);
        let head = engine.heap.node_stack.get_copy(field as usize);
        if id != 0 {
            // Stamps the whole four word noad, the mlist hook reads its first word and the nucleus its second.
            for i in 0..4usize {
                engine.heap.node_src.set(noad as usize + i, id);
            }
        }
        if head != 0 {
            for i in 0..4usize {
                engine.heap.node_stack.set(noad as usize + i, head);
            }
        }
    }

    /// `make_ord` hook: a nucleus glyph takes its field's span and enclosing constructs, the noad keeps its own.
    pub(crate) unsafe fn src_carry_nucleus(engine: *mut Self, noad: halfword, glyph: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || noad < 0 || glyph < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(noad as usize + 1);
        if id != 0 {
            engine.heap.node_src.set(glyph as usize, id);
        }
        let head = engine.heap.node_stack.get_copy(noad as usize + 1);
        engine.heap.node_stack.set(glyph as usize, head);
    }

    /// `handle_right_brace` hook: a group reduced to one character keeps its span in the field it collapses into.
    pub(crate) unsafe fn src_carry_collapse(engine: *mut Self, noad: halfword, field: halfword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || noad < 0 || field < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(noad as usize);
        if id != 0 {
            engine.heap.node_src.set(field as usize, id);
        }
        let head = engine.heap.node_stack.get_copy(noad as usize);
        if head != 0 {
            engine.heap.node_stack.set(field as usize, head);
        }
    }

    /// `clean_box` entry hook: saves `cmd_span` across recursive cleaning.
    pub(crate) unsafe fn src_save_cmd_span(engine: *mut Self) -> u32 {
        engine.as_ref().map_or(0, |engine| engine.state.cmd_span)
    }

    /// `clean_box` exit hook: restores `cmd_span` so what is built after takes the construct's span.
    pub(crate) unsafe fn src_restore_cmd_span(engine: *mut Self, saved: u32) {
        if let Some(engine) = engine.as_mut() {
            if engine.state.source_tracking {
                engine.state.cmd_span = saved;
            }
        }
    }

    /// Pushes a construct frame as the live top, the stack freezes at head 0 once it holds [`SRC_STACK_CAP`].
    fn src_stack_push(self: &mut Self, span: SrcId, start: u32, name: strnumber, pending: bool) {
        if self.state.src_stack_frozen {
            return;
        }
        if self.heap.src_stack_cells.len() >= SRC_STACK_CAP {
            self.state.src_stack_frozen = true;
            self.state.cur_stack_head = 0;
            return;
        }
        self.heap.src_stack_cells.push(SrcStackCell {
            span,
            parent: self.state.cur_stack_head,
            start,
            name,
            pending,
        });
        self.state.cur_stack_head = self.heap.src_stack_cells.len() as u32;
    }

    /// Pops the live top frame, restoring its parent as the head.
    fn src_stack_pop(self: &mut Self) {
        let head = self.state.cur_stack_head;
        if self.state.src_stack_frozen || head == 0 {
            return;
        }
        if let Some(cell) = self.heap.src_stack_cells.get((head - 1) as usize) {
            self.state.cur_stack_head = cell.parent;
        }
    }

    /// `end_token_list` hook: pops a macro frame and keeps the furthest reaching argument span for the hull.
    pub(crate) unsafe fn src_end_token_list(engine: *mut Self, token_type: i32) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let cand = engine.state.curinput.spanfield;
        if cand != 0 {
            let cand_end = engine
                .heap
                .src_spans
                .get((cand - 1) as usize)
                .map(|r| r.end)
                .unwrap_or(0);
            let cur_end = if engine.state.src_call_argspan != 0 {
                engine
                    .heap
                    .src_spans
                    .get((engine.state.src_call_argspan - 1) as usize)
                    .map(|r| r.end)
                    .unwrap_or(0)
            } else {
                0
            };
            if engine.state.src_call_argspan == 0 || cand_end > cur_end {
                engine.state.src_call_argspan = cand;
            }
        }
        if token_type == TOKEN_TYPE_MACRO {
            engine.src_stack_pop();
        }
    }

    /// `scan_math` hook at a `{`: pushes a pending frame from the command's start, finished at the matching `}`.
    pub(crate) unsafe fn src_scan_math_group_open(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let cmd = engine.state.cmd_span;
        let (start, name) = if cmd != 0 {
            match engine.heap.src_spans.get((cmd - 1) as usize) {
                Some(raw) => (raw.start, raw.name),
                None => (0, 0),
            }
        } else {
            (0, 0)
        };
        engine.src_stack_push(0, start, name, true);
        // The opening brace span, a bare group pulls its fraction's start back to it.
        engine.heap.src_grp_stack.push(engine.state.curinput.spanfield);
    }

    /// `handle_right_brace` hook for a math group: finishes the pending frame at the closing brace and pops it.
    pub(crate) unsafe fn src_scan_math_group_close(engine: *mut Self) {
        let eptr = engine;
        let Some(e) = engine.as_mut() else {
            return;
        };
        if !e.state.source_tracking {
            return;
        }
        // `src_construct_extend_to_loc`, later in the same arm, uses this opening brace span.
        e.state.src_grp_closing = e.heap.src_grp_stack.pop().unwrap_or(0);
        let grp = e.state.src_grp_closing;
        // A generalized fraction built in this group gets the whole group's span.
        let aux = e.state.curlist.auxfield.u.CINT;
        let frac = if aux > 0 && aux != TEX_NULL { aux } else { -1 };
        if !e.state.src_stack_frozen && e.state.cur_stack_head != 0 {
            let idx = (e.state.cur_stack_head - 1) as usize;
            if let Some(cell) = e.heap.src_stack_cells.get(idx).copied() {
                // Only a pending group frame in the current file is finished, any other top frame is only popped.
                if cell.pending
                    && cell.name != 0
                    && e.state.curinput.statefield != TOKEN_LIST_STATE
                    && cell.name == e.state.curinput.namefield as strnumber
                {
                    let end = e.src_buf_offset(e.state.curinput.locfield as integer);
                    let (lo, hi) = if cell.start <= end {
                        (cell.start, end)
                    } else {
                        (end, cell.start)
                    };
                    let id = e.intern_span_raw(cell.name, lo, hi, 1);
                    if let Some(c) = e.heap.src_stack_cells.get_mut(idx) {
                        c.span = id;
                        c.pending = false;
                    }
                }
                e.state.cur_stack_head = cell.parent;
            }
        }
        if frac >= 0 {
            Self::src_construct_extent(eptr, frac, grp);
        }
    }

    /// `math_radical` and `math_ac` hook: pulls the new noad's start back to the command the user typed.
    pub(crate) unsafe fn src_construct_anchor(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let noad = engine.state.curlist.tailfield as i32;
        if noad < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(noad as usize);
        if id == 0 {
            return;
        }
        let Some(raw) = engine.heap.src_spans.get((id - 1) as usize).copied() else {
            return;
        };
        // A replayed nested construct anchors to its own replayed command, never to the outer user command.
        let anchor = (engine.state.src_anchor_cmd != 0)
            .then(|| engine.heap.src_spans.get((engine.state.src_anchor_cmd - 1) as usize).copied())
            .flatten()
            .filter(|u| u.name == raw.name);
        let chosen = match anchor {
            Some(u) => {
                if u.start < raw.start {
                    Some(u)
                } else {
                    None
                }
            }
            None => (engine.state.src_user_cmd_span != 0)
                .then(|| engine.heap.src_spans.get((engine.state.src_user_cmd_span - 1) as usize).copied())
                .flatten()
                .filter(|u| u.name == raw.name && u.start < raw.start),
        };
        let Some(u) = chosen else {
            return;
        };
        let newid = engine.intern_span_raw(raw.name, u.start, raw.end, raw.role);
        engine.heap.node_src.set(noad as usize, newid);
    }

    /// `handle_right_brace` hook: a construct noad whose nucleus group closed extends its span through the brace.
    pub(crate) unsafe fn src_construct_extend_to_loc(engine: *mut Self) {
        let e = match engine.as_ref() {
            Some(e) => e,
            None => return,
        };
        if !e.state.source_tracking {
            return;
        }
        // Only the noad's own nucleus, its `+1` field, extends it, a closing script field does not.
        let field = (*e
            .state
            .savestack
            .offset((e.state.saveptr as i32 + 0 as i32) as isize))
            .u
            .CINT;
        let noad = e.state.curlist.tailfield as i32;
        let grp = e.state.src_grp_closing;
        if noad < 0 || field != noad + 1 {
            return;
        }
        Self::src_construct_extent(engine, noad, grp);
    }

    /// Extends a construct noad's span from the earlier of its command and group open to the end read so far.
    pub(crate) unsafe fn src_construct_extent(engine: *mut Self, noad: halfword, group_open: SrcId) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking || noad < 0 {
            return;
        }
        let id = engine.heap.node_src.get_copy(noad as usize);
        if id == 0 {
            return;
        }
        let Some(raw) = engine.heap.src_spans.get((id - 1) as usize).copied() else {
            return;
        };
        let (end, name) = if engine.state.curinput.statefield != TOKEN_LIST_STATE {
            (
                engine.src_buf_offset(engine.state.curinput.locfield as integer),
                engine.state.curinput.namefield as strnumber,
            )
        } else {
            let sid = engine.state.curinput.spanfield;
            if sid == 0 {
                return;
            }
            let Some(braw) = engine.heap.src_spans.get((sid - 1) as usize).copied() else {
                return;
            };
            (braw.end, braw.name)
        };
        if raw.name != name {
            return;
        }
        let end = end.max(raw.end);
        let mut start = raw.start;
        if group_open != 0 {
            if let Some(g) = engine.heap.src_spans.get((group_open - 1) as usize).copied() {
                if g.name == name && g.start < start {
                    start = g.start;
                }
            }
        }
        if start == raw.start && end == raw.end {
            return;
        }
        let newid = engine.intern_span_raw(name, start, end, raw.role);
        engine.heap.node_src.set(noad as usize, newid);
    }

    /// `math_left_right` hook: `\left` opens a group span that `\right` closes over its delimiters.
    pub(crate) unsafe fn src_leftright(engine: *mut Self, p: halfword, t: integer) {
        let Some(eng) = engine.as_mut() else {
            return;
        };
        if !eng.state.source_tracking {
            return;
        }
        if t == LEFT_NOAD {
            let cmd = eng.state.cmd_span;
            eng.heap.src_grp_stack.push(cmd);
        } else if t == RIGHT_NOAD {
            let open = eng.heap.src_grp_stack.pop().unwrap_or(0);
            Self::src_construct_extent(engine, p, open);
        }
    }

    /// Spans of the constructs strictly enclosing a node, innermost first and in the node's own source.
    pub fn node_enclosing_spans(&self, handle: PortableNodeHandle) -> Vec<PortableSourceSpan> {
        let node = handle.0 as halfword;
        let mut out: Vec<PortableSourceSpan> = Vec::new();
        if !self.state.source_tracking || node < 0 {
            return out;
        }
        let primary = self.resolve_node_src(node);
        let mut head = self.heap.node_stack.get_copy(node as usize);
        let mut guard = 0u32;
        while head != 0 {
            guard += 1;
            if guard > 4096 {
                break;
            }
            let Some(cell) = self.heap.src_stack_cells.get((head - 1) as usize) else {
                break;
            };
            let parent = cell.parent;
            let span_id = cell.span;
            head = parent;
            if span_id == 0 {
                continue;
            }
            let Some(raw) = self.heap.src_spans.get((span_id - 1) as usize) else {
                continue;
            };
            let Some(name) = self.pool_string(raw.name) else {
                continue;
            };
            if let Some(p) = primary.as_ref() {
                // An enclosing construct contains the node's span and is not equal to it.
                if name != p.name || raw.start > p.start || raw.end < p.end {
                    continue;
                }
                if raw.start == p.start && raw.end == p.end {
                    continue;
                }
            }
            if let Some(last) = out.last() {
                if last.name == name && last.start == raw.start && last.end == raw.end {
                    continue;
                }
            }
            out.push(PortableSourceSpan {
                name,
                start: raw.start,
                end: raw.end,
                role: 1,
            });
        }
        out
    }

    /// `begin_token_list` hook: a macro body takes the call site span, other lists keep the inherited span.
    pub(crate) unsafe fn src_begin_token_list(engine: *mut Self, t: quarterword) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        if t == TOKEN_TYPE_MACRO {
            let call = engine.state.pending_call_span;
            if call != 0 {
                engine.state.curinput.spanfield = call;
            }
            engine.state.pending_call_span = 0;
            // The macro frame is popped when its body ends, nodes the body allocates record it.
            engine.src_stack_push(call, 0, 0, false);
        }
    }

    /// `\futurelet` hook: saves the first peeked token's origin before the second token read overwrites it.
    pub(crate) unsafe fn src_capture_tok_span(engine: *mut Self) -> u32 {
        engine.as_ref().map_or(0, |engine| engine.state.src_tok_span)
    }

    /// Restores the origin [`Self::src_capture_tok_span`] saved, before the first token is backed up.
    pub(crate) unsafe fn src_restore_tok_span(engine: *mut Self, saved: u32) {
        if let Some(engine) = engine.as_mut() {
            if engine.state.source_tracking {
                engine.state.src_tok_span = saved;
            }
        }
    }

    /// `macro_call` entry hook: records the invoking control sequence for the invocation span.
    pub(crate) unsafe fn src_macro_begin(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let span = engine.state.curinput.spanfield;
        engine.state.src_call_span = span;
        engine.state.src_call_state = engine.state.curinput.statefield as integer;
        engine.state.src_call_index = engine.state.curinput.indexfield as integer;
        engine.state.src_call_name = engine.state.curinput.namefield as strnumber;
        engine.state.src_call_argspan = 0;
        // The user command before the arguments are read, so a construct among them does not become the anchor.
        engine.state.src_call_user_span = engine.state.src_user_cmd_span;
        engine.state.src_call_start = if span != 0 {
            engine
                .heap
                .src_spans
                .get((span - 1) as usize)
                .map(|s| s.start)
                .unwrap_or(0)
        } else {
            0
        };
        // A macro read from a parameter list of the primary input is typed argument content, the anchor command.
        let from_argument_or_template = engine.state.src_call_state == TOKEN_LIST_STATE as integer
            && engine.state.src_call_index < TOKEN_TYPE_BACKED_UP as integer;
        if from_argument_or_template && span != 0 {
            if let Some(raw) = engine.heap.src_spans.get((span - 1) as usize).copied() {
                if raw.name == engine.state.src_primary_name {
                    engine.state.src_anchor_cmd = span;
                }
            }
        }
    }

    /// Hull in one source of the invoking control sequence and its argument tokens, 0 when nothing is found.
    unsafe fn src_arg_hull(self: &mut Self, n: integer) -> SrcId {
        let mut lo = u32::MAX;
        let mut hi = 0u32;
        let mut found = false;
        let mut name: strnumber = self.state.src_primary_name;
        if self.state.src_call_span != 0 {
            if let Some(raw) = self
                .heap
                .src_spans
                .get((self.state.src_call_span - 1) as usize)
            {
                name = raw.name;
                lo = raw.start;
                hi = raw.end;
                found = true;
            }
        }
        // Arguments before the command start belong to a definition site elsewhere and are left out.
        let cmd_start = if found { lo } else { 0 };
        let zmem = self.state.zmem;
        let lo_b = self.state.memmin;
        let hi_b = self.state.memmax;
        for i in 0..n.max(0) {
            let mut p = self.state.pstack[i as usize];
            let mut guard = 0i32;
            while p >= lo_b && p <= hi_b && p != TEX_NULL {
                guard += 1;
                if guard > 100000 {
                    break;
                }
                let id = self.heap.node_src.get_copy(p as usize);
                if id != 0 {
                    if let Some(raw) = self.heap.src_spans.get((id - 1) as usize) {
                        if raw.name == name && raw.start >= cmd_start {
                            if found {
                                lo = lo.min(raw.start);
                                hi = hi.max(raw.end);
                            } else {
                                lo = raw.start;
                                hi = raw.end;
                                found = true;
                            }
                        }
                    }
                }
                p = (*zmem.offset(p as isize)).hh.v.RH;
            }
        }
        // Reaches to the final argument's closing brace, from the last popped argument span or the current one.
        for cand in [self.state.src_call_argspan, self.state.curinput.spanfield] {
            if cand == 0 {
                continue;
            }
            if let Some(raw) = self.heap.src_spans.get((cand - 1) as usize) {
                if raw.name == name && raw.start >= cmd_start {
                    if found {
                        lo = lo.min(raw.start);
                        hi = hi.max(raw.end);
                    } else {
                        lo = raw.start;
                        hi = raw.end;
                        found = true;
                    }
                }
            }
        }
        if found && hi > lo {
            self.intern_span_raw(name, lo, hi, 1)
        } else {
            0
        }
    }

    /// `macro_call` hook before the body starts: sets the call site span the body inherits.
    pub(crate) unsafe fn src_macro_set_pending(engine: *mut Self, n: integer) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        if engine.state.src_call_index == TOKEN_TYPE_PARAMETER as integer && engine.state.src_call_span == 0 {
            // A library helper without a span of its own inherits the enclosing user command's span.
            engine.state.pending_call_span = 0;
        } else if engine.state.src_call_index == TOKEN_TYPE_PARAMETER as integer {
            // An argument replay recovers its own hull from its scanned arguments.
            let hull = engine.src_arg_hull(n);
            engine.state.pending_call_span = if hull != 0 {
                hull
            } else {
                engine.state.src_call_span
            };
        } else if engine.state.curinput.statefield != TOKEN_LIST_STATE {
            // A call read from the file spans from the user command to the end of its arguments.
            let end = engine.src_buf_offset(engine.state.curinput.locfield as integer);
            let name = engine.state.curinput.namefield as strnumber;
            let mut start = engine.state.src_call_start.min(end);
            let user = engine.state.src_call_user_span;
            if user != 0 {
                if let Some(raw) = engine.heap.src_spans.get((user - 1) as usize) {
                    if raw.name == name && raw.start <= start {
                        start = raw.start;
                    }
                }
            }
            let id = engine.intern_span_raw(name, start, end, 1);
            engine.state.pending_call_span = id;
        } else if n > 0 {
            // A macro body call that took arguments joins the inherited span with its own arguments.
            let hull = engine.src_arg_hull(n);
            engine.state.pending_call_span = if hull != 0 {
                hull
            } else {
                engine.state.src_call_span
            };
        } else {
            engine.state.pending_call_span = engine.state.src_call_span;
        }
    }

    /// Main loop hook: records a span per UTF-16 unit of the character just added to `nativetext`.
    pub(crate) unsafe fn src_native_run_push(engine: *mut Self) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let nativelen = engine.state.nativelen.max(0) as usize;
        // A supplementary scalar takes two UTF-16 units.
        let units = if engine.state.curchr as i64 > 65535 { 2 } else { 1 };
        let prev = nativelen.saturating_sub(units);
        if prev == 0 {
            engine.heap.src_native_offsets.clear();
        }
        if engine.heap.src_native_offsets.len() > prev {
            engine.heap.src_native_offsets.truncate(prev);
        }
        let id = engine.state.curinput.spanfield;
        while engine.heap.src_native_offsets.len() < nativelen {
            engine.heap.src_native_offsets.push(id);
        }
    }

    /// Maps each shaped glyph to the input span of the characters in its cluster, using the offsets once.
    pub(crate) unsafe fn src_resolve_native_glyphs(
        engine: *mut Self,
        node: halfword,
        text: &[u16],
        glyphs: &mut [PortableNativeGlyph],
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        if !engine.state.source_tracking {
            return;
        }
        let offsets = core::mem::take(&mut engine.heap.src_native_offsets);
        // Only a run whose offsets cover exactly this node's text is mapped.
        if text.is_empty() || offsets.len() != text.len() {
            return;
        }
        if node < 0 {
            return;
        }
        // Only characters from the node's own source contribute.
        let node_id = engine.heap.node_src.get_copy(node as usize);
        if node_id == 0 {
            return;
        }
        let node_name = match engine.heap.src_spans.get((node_id - 1) as usize) {
            Some(raw) => raw.name,
            None => return,
        };
        // Clusters are UTF-8 byte offsets into the shaped text, so each byte gets its character's span.
        let s = String::from_utf16_lossy(text);
        let n = s.len();
        let mut byte_src = vec![0u32; n];
        let mut u16i = 0usize;
        for (b, ch) in s.char_indices() {
            let id = offsets.get(u16i).copied().unwrap_or(0);
            let upper = (b + ch.len_utf8()).min(n);
            for slot in byte_src.iter_mut().take(upper).skip(b) {
                *slot = id;
            }
            u16i += ch.len_utf16();
        }
        for gi in 0..glyphs.len() {
            let cs = (glyphs[gi].cluster_start as usize).min(n);
            // Clusters rise left to right, a cluster ends where the next greater one starts.
            let mut ce = n;
            for g2 in glyphs.iter() {
                let c2 = (g2.cluster_start as usize).min(n);
                if c2 > cs && c2 < ce {
                    ce = c2;
                }
            }
            let mut lo = u32::MAX;
            let mut hi = 0u32;
            let mut found = false;
            for &id in byte_src.iter().take(ce).skip(cs) {
                if id == 0 {
                    continue;
                }
                let Some(raw) = engine.heap.src_spans.get((id - 1) as usize) else {
                    continue;
                };
                if raw.name != node_name {
                    continue;
                }
                lo = lo.min(raw.start);
                hi = hi.max(raw.end);
                found = true;
            }
            if found && hi > lo {
                glyphs[gi].src_start = lo;
                glyphs[gi].src_end = hi;
            }
        }
    }

    /// Every interned span in display form.
    pub fn input_source_spans(&self) -> Vec<PortableSourceSpan> {
        self.heap
            .src_spans
            .iter()
            .filter_map(|raw| {
                let name = self.pool_string(raw.name)?;
                Some(PortableSourceSpan {
                    name,
                    start: raw.start,
                    end: raw.end,
                    role: raw.role,
                })
            })
            .collect()
    }

    pub(crate) unsafe fn resolve_font_handle(
        engine: *mut PortableTexEngine<'_>,
        name: *mut ASCIIcode,
        size: integer,
    ) -> FontHandle {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let name_len = engine.state.namelength.max(0) as usize;
        let name = if name.is_null() || name_len == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(name as *const i32, name_len)
        };
        engine.fonts.resolve_font_handle(name, size).unwrap_or(0)
    }

    pub(crate) unsafe fn measure_font_metrics(
        engine: *mut PortableTexEngine<'_>,
        font: FontHandle,
        ascent: *mut integer,
        descent: *mut integer,
        xheight: *mut integer,
        capheight: *mut integer,
        slant: *mut integer,
    ) {
        let metrics = engine
            .as_mut()
            .map(|engine| engine.fonts.font_metrics(font))
            .unwrap_or_default();
        if !ascent.is_null() {
            *ascent = metrics.ascent;
        }
        if !descent.is_null() {
            *descent = metrics.descent;
        }
        if !xheight.is_null() {
            *xheight = metrics.xheight;
        }
        if !capheight.is_null() {
            *capheight = metrics.capheight;
        }
        if !slant.is_null() {
            *slant = metrics.slant;
        }
    }

    pub(crate) unsafe fn get_native_mathsy_parameter(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        param: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.math_symbol_parameter(font_handle, param)
    }

    pub(crate) unsafe fn get_native_mathex_parameter(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        param: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.math_extension_parameter(font_handle, param)
    }

    /// XeTeX `get_ot_math_ital_corr`: a MATH italic correction in scaled points.
    pub(crate) unsafe fn get_ot_math_ital_corr(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        glyph: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.math_glyph_italic_correction(font_handle, glyph)
    }

    /// XeTeX `get_ot_math_variant`: the `v`th variant of `g` with its advance in `adv`, which is `-1` when none.
    pub(crate) unsafe fn get_ot_math_variant(
        engine: *mut PortableTexEngine<'resources>,
        f_0: integer,
        g_0: integer,
        v: integer,
        adv: *mut integer,
        horiz: integer,
    ) -> integer {
        if !adv.is_null() {
            *adv = -1;
        }
        let Some(engine) = engine.as_mut() else {
            return g_0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, f_0) else {
            return g_0;
        };
        let index = u16::try_from(v).unwrap_or(u16::MAX);
        match engine
            .fonts
            .math_glyph_variant(font_handle, g_0, index, horiz != 0)
        {
            Some(variant) => {
                if !adv.is_null() {
                    *adv = variant.advance;
                }
                variant.glyph
            }
            None => g_0,
        }
    }

    /// XeTeX `get_ot_assembly_ptr`: the assembly of `g` as an owned pointer, null when there is none.
    pub(crate) unsafe fn get_ot_assembly_ptr(
        engine: *mut PortableTexEngine<'resources>,
        f_0: integer,
        g_0: integer,
        horiz: integer,
    ) -> voidpointer {
        let Some(engine) = engine.as_mut() else {
            return nullptr;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, f_0) else {
            return nullptr;
        };
        let parts = engine
            .fonts
            .math_glyph_assembly(font_handle, g_0, horiz != 0);
        if parts.is_empty() {
            return nullptr;
        }
        let assembly = Box::new(GlyphAssembly { parts });
        Box::into_raw(assembly) as voidpointer
    }

    /// XeTeX `ot_min_connector_overlap` for font `f`, in scaled points.
    pub(crate) unsafe fn ot_min_connector_overlap(
        engine: *mut PortableTexEngine<'resources>,
        f_0: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, f_0) else {
            return 0;
        };
        engine.fonts.math_min_connector_overlap(font_handle)
    }

    /// Control box of glyph `glyph` in native font `f`, all zero when `f` is not a native font.
    fn native_glyph_bounds(this: &mut Self, f: integer, glyph: integer) -> PortableGlyphBounds {
        match (Self::font_handle_for_number(this, f), u16::try_from(glyph)) {
            (Some(handle), Ok(glyph)) => this.fonts.glyph_bounds(handle, glyph),
            _ => PortableGlyphBounds::default(),
        }
    }

    /// Control box of the glyph native font `f` maps character `c` to.
    fn native_char_bounds(this: &mut Self, f: integer, c: integer) -> PortableGlyphBounds {
        let Some(handle) = Self::font_handle_for_number(this, f) else {
            return PortableGlyphBounds::default();
        };
        let glyph = this.fonts.map_char_to_glyph(handle, c);
        Self::native_glyph_bounds(this, f, glyph)
    }

    /// Font dimension `n` of font `f`, zero outside the font memory.
    fn font_dimen(this: &Self, f: integer, n: integer) -> scaled {
        usize::try_from(f)
            .ok()
            .and_then(|f| this.heap.parambase_storage.get(f))
            .and_then(|&base| usize::try_from(base.checked_add(n)?).ok())
            .and_then(|index| this.heap.fontinfo_storage.get(index))
            .map_or(0, |word| unsafe { word.u.CINT })
    }

    /// `\XeTeXletterspace` of font `f`, which XeTeX adds to every native italic correction.
    fn font_letter_space(this: &Self, f: integer) -> scaled {
        usize::try_from(f)
            .ok()
            .and_then(|f| this.heap.fontletterspace_storage.get(f))
            .copied()
            .unwrap_or(0)
    }

    /// XeTeX `D2Fix`: points to scaled points, truncating after adding half a unit.
    fn d2fix(points: f32) -> scaled {
        (f64::from(points) * 65536.0 + 0.5) as scaled
    }

    /// XeTeX `getGlyphItalCorr`: how far the control box reaches right of the advance.
    fn glyph_italic_correction(bounds: PortableGlyphBounds) -> f32 {
        if bounds.x_max > bounds.advance {
            bounds.x_max - bounds.advance
        } else {
            0.0
        }
    }

    /// Left and right side bearings, the right one measured inward from the advance.
    fn side_bearings(bounds: PortableGlyphBounds) -> (scaled, scaled) {
        (Self::d2fix(bounds.x_min), Self::d2fix(bounds.advance - bounds.x_max))
    }

    /// XeTeX `getnativecharheightdepth`, snapping to the baseline, x height and cap height within a 25th quad.
    fn snapped_char_height_depth(this: &mut Self, f: integer, c: integer) -> (scaled, scaled) {
        let bounds = Self::native_char_bounds(this, f, c);
        let mut height = Self::d2fix(bounds.y_max);
        let mut depth = Self::d2fix(-bounds.y_min);
        let fuzz = Self::font_dimen(this, f, QUAD_CODE) / 25;
        let snap = |value: &mut scaled, target: scaled| {
            if (*value - target).abs() <= fuzz {
                *value = target;
            }
        };
        snap(&mut depth, 0);
        snap(&mut height, 0);
        snap(&mut height, Self::font_dimen(this, f, X_HEIGHT_CODE));
        snap(&mut height, Self::font_dimen(this, f, CAP_HEIGHT_CODE));
        (height, depth)
    }

    /// XeTeX `getnativecharwd`.
    pub(crate) unsafe fn native_char_width(engine: *mut Self, f: integer, c: integer) -> scaled {
        engine.as_mut().map_or(0, |this| Self::d2fix(Self::native_char_bounds(this, f, c).advance))
    }

    /// XeTeX `getnativecharht`.
    pub(crate) unsafe fn native_char_height(engine: *mut Self, f: integer, c: integer) -> scaled {
        engine.as_mut().map_or(0, |this| Self::snapped_char_height_depth(this, f, c).0)
    }

    /// XeTeX `getnativechardp`.
    pub(crate) unsafe fn native_char_depth(engine: *mut Self, f: integer, c: integer) -> scaled {
        engine.as_mut().map_or(0, |this| Self::snapped_char_height_depth(this, f, c).1)
    }

    /// XeTeX `getnativecharheightdepth`.
    pub(crate) unsafe fn native_char_height_depth(
        engine: *mut Self,
        f: integer,
        c: integer,
        height: *mut scaled,
        depth: *mut scaled,
    ) {
        let (h, d) = engine
            .as_mut()
            .map_or((0, 0), |this| Self::snapped_char_height_depth(this, f, c));
        if !height.is_null() {
            *height = h;
        }
        if !depth.is_null() {
            *depth = d;
        }
    }

    /// XeTeX `getnativecharsidebearings`.
    pub(crate) unsafe fn native_char_side_bearings(
        engine: *mut Self,
        f: integer,
        c: integer,
        lsb: *mut scaled,
        rsb: *mut scaled,
    ) {
        let (left, right) = engine
            .as_mut()
            .map_or((0, 0), |this| Self::side_bearings(Self::native_char_bounds(this, f, c)));
        if !lsb.is_null() {
            *lsb = left;
        }
        if !rsb.is_null() {
            *rsb = right;
        }
    }

    /// XeTeX `getnativecharic`: the overhang past the advance plus the letter spacing.
    pub(crate) unsafe fn native_char_italic_correction(engine: *mut Self, f: integer, c: integer) -> scaled {
        let Some(this) = engine.as_mut() else {
            return 0;
        };
        let (_, rsb) = Self::side_bearings(Self::native_char_bounds(this, f, c));
        Self::font_letter_space(this, f) - rsb.min(0)
    }

    /// XeTeX `getglyphbounds`: edges 1 to 4 are the left bearing, height, right bearing and depth of glyph `gid`.
    pub(crate) unsafe fn glyph_bound(engine: *mut Self, f: integer, edge: integer, gid: integer) -> scaled {
        let Some(this) = engine.as_mut() else {
            return 0;
        };
        let bounds = Self::native_glyph_bounds(this, f, gid);
        let (a, b) = if edge & 1 != 0 {
            (bounds.x_min, bounds.advance - bounds.x_max)
        } else {
            (bounds.y_max, -bounds.y_min)
        };
        Self::d2fix(if edge <= 2 { a } else { b })
    }

    /// XeTeX `getfontcharrange`: the first or last character code native font `f` maps.
    pub(crate) unsafe fn font_char_range(engine: *mut Self, f: integer, first: integer) -> integer {
        let Some(this) = engine.as_mut() else {
            return 0;
        };
        let Some(handle) = Self::font_handle_for_number(this, f) else {
            return 0;
        };
        this.fonts
            .char_code_range(handle)
            .map_or(0, |(low, high)| if first != 0 { low } else { high })
    }

    /// XeTeX `get_native_italic_correction`: italic correction of a native word's last glyph plus letter spacing.
    pub(crate) unsafe fn native_word_italic_correction(engine: *mut Self, node: voidpointer) -> scaled {
        let Some(this) = engine.as_mut() else {
            return 0;
        };
        let Some(node) = Self::node_index_for_pointer(this, node) else {
            return 0;
        };
        let Some(last) = this.native_glyph_infos.get(&node).and_then(|info| info.glyphs.last()) else {
            return 0;
        };
        let glyph = integer::from(last.glyph_id);
        let font = Self::native_node_font(this.state.zmem, node);
        let bounds = Self::native_glyph_bounds(this, font, glyph);
        Self::d2fix(Self::glyph_italic_correction(bounds)) + Self::font_letter_space(this, font)
    }

    /// XeTeX `get_native_glyph_italic_correction`.
    pub(crate) unsafe fn native_glyph_italic_correction(engine: *mut Self, node: voidpointer) -> scaled {
        let Some(this) = engine.as_mut() else {
            return 0;
        };
        let Some(node) = Self::node_index_for_pointer(this, node) else {
            return 0;
        };
        let mem = this.state.zmem;
        let font = Self::native_node_font(mem, node);
        let glyph = (*mem.offset((node + 4) as isize)).v.QQQQ.u.B2 as integer;
        Self::d2fix(Self::glyph_italic_correction(Self::native_glyph_bounds(this, font, glyph)))
    }

    /// Height of a math glyph in points, XeTeXOTMath.cpp `glyph_height` takes it unrounded from the control box.
    fn math_glyph_height(engine: &mut PortableTexEngine<'resources>, f_0: integer, g_0: integer) -> f32 {
        Self::native_glyph_bounds(engine, f_0, g_0).y_max
    }

    /// Depth of a math glyph in points, XeTeXOTMath.cpp `glyph_depth`.
    fn math_glyph_depth(engine: &mut PortableTexEngine<'resources>, f_0: integer, g_0: integer) -> f32 {
        -Self::native_glyph_bounds(engine, f_0, g_0).y_min
    }

    /// One MATH kern corner of glyph `g` at a height in design units, in design units.
    unsafe fn math_kern_at(
        engine: &mut PortableTexEngine<'resources>,
        f_0: integer,
        g_0: integer,
        height: integer,
        corner: PortableMathKernCorner,
    ) -> integer {
        let Some(font_handle) = Self::font_handle_for_number(engine, f_0) else {
            return 0;
        };
        engine.fonts.math_kern_at(font_handle, g_0, corner, height)
    }

    /// XeTeX `get_ot_math_kern`: the cut in kern between base glyph `g` and script glyph `sg`, in scaled points.
    pub(crate) unsafe fn get_ot_math_kern(
        engine: *mut PortableTexEngine<'resources>,
        f_0: integer,
        g_0: integer,
        sf: integer,
        sg: integer,
        cmd: integer,
        shift_scaled: integer,
    ) -> integer {
        const SUP_CMD: integer = 0;
        const SUB_CMD: integer = 1;
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, f_0) else {
            return 0;
        };
        let Some(sfont_handle) = Self::font_handle_for_number(engine, sf) else {
            return 0;
        };

        let g_height_pt = Self::math_glyph_height(engine, f_0, g_0);
        let g_depth_pt = Self::math_glyph_depth(engine, f_0, g_0);
        let sg_height_pt = Self::math_glyph_height(engine, sf, sg);
        let sg_depth_pt = Self::math_glyph_depth(engine, sf, sg);

        // XeTeXOTMath.cpp works in design units of the base glyph's font.
        let g_height = engine.fonts.math_points_to_units(font_handle, g_height_pt) as integer;
        let g_depth = engine.fonts.math_points_to_units(font_handle, g_depth_pt) as integer;
        let sg_height = engine
            .fonts
            .math_points_to_units(sfont_handle, sg_height_pt) as integer;
        let sg_depth = engine
            .fonts
            .math_points_to_units(sfont_handle, sg_depth_pt) as integer;
        // `Fix2D` divides in double before `pointsToUnits` narrows to float.
        let shift_pt = (f64::from(shift_scaled) / 65536.0) as f32;
        let shift = engine.fonts.math_points_to_units(font_handle, shift_pt) as integer;

        let f_size = engine.fonts.math_point_size(font_handle);
        let sf_size = engine.fonts.math_point_size(sfont_handle);
        if f_size == 0.0 {
            return 0;
        }
        let scale_factor = sf_size / f_size;

        // Mixed int and float expressions evaluate in `f32` and truncate once, as C does in XeTeXOTMath.cpp.
        let truncate = |value: f32| value as integer;
        let mut rval: integer;
        if cmd == SUP_CMD {
            let kern = Self::math_kern_at(
                engine,
                f_0,
                g_0,
                truncate(shift as f32 - scale_factor * sg_depth as f32),
                PortableMathKernCorner::TopRight,
            );
            let skern =
                Self::math_kern_at(engine, sf, sg, -sg_depth, PortableMathKernCorner::BottomLeft);
            let top_kern = truncate(kern as f32 + scale_factor * skern as f32);

            let kern =
                Self::math_kern_at(engine, f_0, g_0, g_height, PortableMathKernCorner::TopRight);
            let skern = Self::math_kern_at(
                engine,
                sf,
                sg,
                truncate((g_height - shift) as f32 / scale_factor),
                PortableMathKernCorner::BottomLeft,
            );
            let bot_kern = truncate(kern as f32 + scale_factor * skern as f32);

            rval = if top_kern > bot_kern { top_kern } else { bot_kern };
        } else if cmd == SUB_CMD {
            let kern = Self::math_kern_at(
                engine,
                f_0,
                g_0,
                truncate(scale_factor * sg_height as f32 - shift as f32),
                PortableMathKernCorner::BottomRight,
            );
            let skern =
                Self::math_kern_at(engine, sf, sg, sg_height, PortableMathKernCorner::TopLeft);
            let top_kern = truncate(kern as f32 + scale_factor * skern as f32);

            let kern =
                Self::math_kern_at(engine, f_0, g_0, -g_depth, PortableMathKernCorner::BottomRight);
            let skern = Self::math_kern_at(
                engine,
                sf,
                sg,
                truncate((shift - g_depth) as f32 / scale_factor),
                PortableMathKernCorner::TopLeft,
            );
            let bot_kern = truncate(kern as f32 + scale_factor * skern as f32);

            rval = if top_kern > bot_kern { top_kern } else { bot_kern };
        } else {
            return 0;
        }

        rval = engine.fonts.math_units_to_scaled(font_handle, rval);
        rval
    }

    pub(crate) unsafe fn get_native_word_cp(
        engine: *mut PortableTexEngine<'resources>,
        node: voidpointer,
        side: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(node_index) = Self::node_index_for_pointer(engine, node) else {
            return 0;
        };
        let Some(info) = engine.native_glyph_infos.get(&node_index) else {
            return 0;
        };
        let glyph = if side == 0 {
            info.glyphs.first()
        } else {
            info.glyphs.last()
        };
        let Some(glyph) = glyph else {
            return 0;
        };
        let font = Self::native_node_font(engine.state.zmem, node_index);
        Self::character_protrusion(engine, font, u32::from(glyph.glyph_id), side)
    }

    pub(crate) unsafe fn get_native_glyph(
        engine: *mut PortableTexEngine<'resources>,
        node: voidpointer,
        index: u32,
    ) -> uint16_t {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(node_index) = Self::node_index_for_pointer(engine, node) else {
            return 0;
        };
        engine
            .native_glyph_infos
            .get(&node_index)
            .and_then(|info| info.glyphs.get(index as usize))
            .map_or(0, |glyph| glyph.glyph_id)
    }

    pub(crate) unsafe fn get_character_protrusion(
        engine: *mut PortableTexEngine<'_>,
        font: integer,
        code: u32,
        side: integer,
    ) -> integer {
        engine
            .as_mut()
            .map_or(0, |engine| Self::character_protrusion(engine, font, code, side))
    }

    pub(crate) fn character_protrusion(
        engine: &PortableTexEngine<'_>,
        font: integer,
        code: u32,
        side: integer,
    ) -> integer {
        engine
            .character_protrusions
            .get(&(font, code, side))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) unsafe fn set_character_protrusion(
        engine: *mut PortableTexEngine<'_>,
        font: integer,
        code: u32,
        side: integer,
        value: integer,
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        let key = (font, code, side);
        if value == 0 {
            engine.character_protrusions.remove(&key);
        } else {
            engine.character_protrusions.insert(key, value);
        }
    }

    pub(crate) unsafe fn get_opentype_math_constant(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        constant: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.opentype_math_constant(font_handle, constant)
    }

    pub(crate) unsafe fn get_opentype_math_accent_position(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        glyph: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.opentype_math_accent_position(font_handle, glyph)
    }

    pub(crate) unsafe fn map_char_to_glyph(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
        ch: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine.fonts.map_char_to_glyph(font_handle, ch)
    }

    pub(crate) unsafe fn map_glyph_to_index(
        engine: *mut PortableTexEngine<'resources>,
        font: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        let Some(name) = engine.pool_string(engine.state.curname) else {
            return 0;
        };
        engine.fonts.map_glyph_to_index(font_handle, name.as_str())
    }

    /// XeTeX `ot_font_get` for the `\XeTeXOT` and `\XeTeXcountglyphs` primitives.
    pub(crate) unsafe fn ot_font_get(
        engine: *mut PortableTexEngine<'resources>,
        what: integer,
        font: integer,
        param1: integer,
        param2: integer,
        param3: integer,
    ) -> integer {
        let Some(engine) = engine.as_mut() else {
            return 0;
        };
        let Some(font_handle) = Self::font_handle_for_number(engine, font) else {
            return 0;
        };
        engine
            .fonts
            .ot_font_get(font_handle, what, param1, param2, param3)
    }

    pub(crate) unsafe fn is_opentype_math_font(
        engine: *mut PortableTexEngine<'_>,
        font: FontHandle,
    ) -> boolean {
        engine
            .as_mut()
            .map(|engine| engine.fonts.is_opentype_math_font(font) as boolean)
            .unwrap_or(false_0)
    }

    pub(crate) unsafe fn using_opentype(
        engine: *mut PortableTexEngine<'_>,
        font: FontHandle,
    ) -> boolean {
        engine
            .as_mut()
            .map(|engine| engine.fonts.using_opentype(font) as boolean)
            .unwrap_or(false_0)
    }

    pub(crate) unsafe fn release_font_engine(
        engine: *mut PortableTexEngine<'_>,
        font: FontHandle,
        type_flag: integer,
    ) {
        if let Some(engine) = engine.as_mut() {
            engine.fonts.release_font_handle(font, type_flag);
        }
    }

    pub(crate) unsafe fn measure_opentype_font_metrics(
        engine: *mut PortableTexEngine<'_>,
        font: FontHandle,
        ascent: *mut integer,
        descent: *mut integer,
        xheight: *mut integer,
        capheight: *mut integer,
        slant: *mut integer,
    ) {
        let metrics = engine
            .as_mut()
            .map(|engine| engine.fonts.opentype_font_metrics(font))
            .unwrap_or_default();
        if !ascent.is_null() {
            *ascent = metrics.ascent;
        }
        if !descent.is_null() {
            *descent = metrics.descent;
        }
        if !xheight.is_null() {
            *xheight = metrics.xheight;
        }
        if !capheight.is_null() {
            *capheight = metrics.capheight;
        }
        if !slant.is_null() {
            *slant = metrics.slant;
        }
    }

    pub(crate) unsafe fn measure_native_node(
        engine: *mut PortableTexEngine<'resources>,
        node: voidpointer,
        use_glyph_metrics: integer,
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        let Some(node_index) = Self::node_index_for_pointer(engine, node) else {
            return;
        };
        let mem = engine.state.zmem;
        let font_number = Self::native_node_font(mem, node_index);
        let Some(font_handle) = Self::font_handle_for_number(engine, font_number) else {
            return;
        };
        let text = Self::native_node_text(mem, node_index);
        let mut metrics =
            engine
                .fonts
                .shape_native_text(font_handle, text, use_glyph_metrics != 0);
        Self::src_resolve_native_glyphs(
            engine as *mut PortableTexEngine<'resources>,
            node_index,
            text,
            metrics.glyphs.as_mut_slice(),
        );
        Self::write_native_node_metrics(
            mem,
            node_index,
            metrics.width,
            metrics.height,
            metrics.depth,
        );
        (*mem.offset((node_index + 4) as isize)).v.QQQQ.u.B3 =
            (metrics.glyphs.len().min(i32::MAX as usize) as quarterword) as u16;
        (*mem.offset((node_index + 5) as isize)).raw = 0;
        engine.native_glyph_infos.insert(
            node_index,
            PortableNativeGlyphInfo {
                glyphs: metrics.glyphs,
            },
        );
    }

    pub(crate) unsafe fn measure_native_glyph(
        engine: *mut PortableTexEngine<'resources>,
        node: voidpointer,
        use_glyph_metrics: integer,
    ) {
        let Some(engine) = engine.as_mut() else {
            return;
        };
        let Some(node_index) = Self::node_index_for_pointer(engine, node) else {
            return;
        };
        let mem = engine.state.zmem;
        let font_number = Self::native_node_font(mem, node_index);
        let Some(font_handle) = Self::font_handle_for_number(engine, font_number) else {
            return;
        };
        let glyph = (*mem.offset((node_index + 4) as isize)).v.QQQQ.u.B2 as u16;
        let mut metrics = engine
            .fonts
            .measure_native_glyph(font_handle, glyph, use_glyph_metrics != 0);
        // Without `\XeTeXuseglyphmetrics` XeTeX gives every glyph the font's ascent and descent.
        if use_glyph_metrics == 0 {
            let font = usize::try_from(font_number).unwrap_or(usize::MAX);
            metrics.height = engine.heap.heightbase_storage.get(font).copied().unwrap_or(0);
            metrics.depth = engine.heap.depthbase_storage.get(font).copied().unwrap_or(0);
        }
        Self::write_native_node_metrics(
            mem,
            node_index,
            metrics.width,
            metrics.height,
            metrics.depth,
        );
        (*mem.offset((node_index + 4) as isize)).v.QQQQ.u.B3 = (1 as quarterword) as u16;
        // XeTeX's glyph info pointer would sit at word 5, one past this five word node, the side table holds it.
        engine.native_glyph_infos.insert(
            node_index,
            PortableNativeGlyphInfo {
                glyphs: Vec::from([PortableNativeGlyph {
                    glyph_id: glyph,
                    x: 0,
                    y: 0,
                    advance: metrics.width,
                    cluster_start: 0,
                    cluster_end: 0,
                    src_start: 0,
                    src_end: 0,
                }]),
            },
        );
    }

    pub(crate) unsafe fn znotaatfonterror(
        self: &mut Self,
        cmd: integer,
        c_0: integer,
        f_0: integer,
    ) -> EngineFlow<()> {
        self.znototfonterror(cmd, c_0, f_0)?;
        Ok(())
    }

    pub(crate) unsafe fn znotaatgrfonterror(
        self: &mut Self,
        cmd: integer,
        c_0: integer,
        f_0: integer,
    ) -> EngineFlow<()> {
        self.znototfonterror(cmd, c_0, f_0)?;
        Ok(())
    }

    /// XeTeX `stack_glyph_into_box`: appends a measured glyph node to box `b` and grows the box.
    unsafe fn stack_glyph_into_box(
        self: &mut Self,
        b: halfword,
        f_0: internalfontnumber,
        g_0: integer,
    ) -> EngineFlow<()> {
        let mem: *mut memoryword = self.state.zmem.as_mut_ptr();
        let p = (&mut *(self as *mut PortableTexEngine<'_>)).zgetnode(5)?;
        (*mem.offset(p as isize)).hh.u.B0 = WHATSIT_NODE as i16;
        (*mem.offset(p as isize)).hh.u.B1 = GLYPH_NODE as i16;
        (*mem.offset((p + 4) as isize)).v.QQQQ.u.B1 = (f_0 as quarterword) as u16;
        (*mem.offset((p + 4) as isize)).v.QQQQ.u.B2 = (g_0 as quarterword) as u16;
        Self::measure_native_glyph(
            self as *mut PortableTexEngine<'resources>,
            mem.offset(p as isize) as *mut memoryword as *mut (),
            1,
        );
        Ok(
            if (*mem.offset(b as isize)).hh.u.B0 as i32 == HLIST_NODE {
                let mut q = (*mem.offset((b + 5) as isize)).hh.v.RH;
                if q == TEX_NULL {
                    (*mem.offset((b + 5) as isize)).hh.v.RH = p;
                } else {
                    while (*mem.offset(q as isize)).hh.v.RH != TEX_NULL {
                        q = (*mem.offset(q as isize)).hh.v.RH;
                    }
                    (*mem.offset(q as isize)).hh.v.RH = p;
                    if (*mem.offset((b + 3) as isize)).u.CINT
                        < (*mem.offset((p + 3) as isize)).u.CINT
                    {
                        (*mem.offset((b + 3) as isize)).u.CINT = (*mem
                            .offset((p + 3) as isize))
                            .u
                            .CINT;
                    }
                    if (*mem.offset((b + 2) as isize)).u.CINT
                        < (*mem.offset((p + 2) as isize)).u.CINT
                    {
                        (*mem.offset((b + 2) as isize)).u.CINT = (*mem
                            .offset((p + 2) as isize))
                            .u
                            .CINT;
                    }
                }
            } else {
                (*mem.offset(p as isize)).hh.v.RH = (*mem.offset((b + 5) as isize))
                    .hh
                    .v
                    .RH;
                (*mem.offset((b + 5) as isize)).hh.v.RH = p;
                (*mem.offset((b + 3) as isize)).u.CINT = (*mem.offset((p + 3) as isize))
                    .u
                    .CINT;
                if (*mem.offset((b + 1) as isize)).u.CINT
                    < (*mem.offset((p + 1) as isize)).u.CINT
                {
                    (*mem.offset((b + 1) as isize)).u.CINT = (*mem
                        .offset((p + 1) as isize))
                        .u
                        .CINT;
                }
            },
        )
    }

    /// XeTeX `stack_glue_into_box`: appends glue of width `min` stretching to `max` to box `b`.
    unsafe fn stack_glue_into_box(
        self: &mut Self,
        b: halfword,
        min: scaled,
        max: scaled,
    ) -> EngineFlow<()> {
        let mem: *mut memoryword = self.state.zmem.as_mut_ptr();
        const ZERO_GLUE: halfword = 0;
        let q = (&mut *(self as *mut PortableTexEngine<'_>)).znewspec(ZERO_GLUE)?;
        (*mem.offset((q + 1) as isize)).u.CINT = min;
        (*mem.offset((q + 2) as isize)).u.CINT = max - min;
        let p = (&mut *(self as *mut PortableTexEngine<'_>)).znewglue(q)?;
        Ok(
            if (*mem.offset(b as isize)).hh.u.B0 as i32 == HLIST_NODE {
                let mut r = (*mem.offset((b + 5) as isize)).hh.v.RH;
                if r == TEX_NULL {
                    (*mem.offset((b + 5) as isize)).hh.v.RH = p;
                } else {
                    while (*mem.offset(r as isize)).hh.v.RH != TEX_NULL {
                        r = (*mem.offset(r as isize)).hh.v.RH;
                    }
                    (*mem.offset(r as isize)).hh.v.RH = p;
                }
            } else {
                (*mem.offset(p as isize)).hh.v.RH = (*mem.offset((b + 5) as isize))
                    .hh
                    .v
                    .RH;
                (*mem.offset((b + 5) as isize)).hh.v.RH = p;
                (*mem.offset((b + 3) as isize)).u.CINT = (*mem.offset((p + 3) as isize))
                    .u
                    .CINT;
                (*mem.offset((b + 1) as isize)).u.CINT = (*mem.offset((p + 1) as isize))
                    .u
                    .CINT;
            },
        )
    }

    /// XeTeX `build_opentype_assembly`: a box at least `s` long stacking `assembly` parts with overlap glue.
    pub(crate) unsafe fn zbuildopentypeassembly(
        self: &mut Self,
        f_0: internalfontnumber,
        assembly: voidpointer,
        s: scaled,
        horiz_flag: integer,
    ) -> EngineFlow<halfword> {
        let mem: *mut memoryword = self.state.zmem.as_mut_ptr();
        let horiz = horiz_flag != 0;
        let b = (&mut *(self as *mut PortableTexEngine<'_>)).newnullbox()?;
        (*mem.offset(b as isize)).hh.u.B0 = if horiz { HLIST_NODE } else { VLIST_NODE }
            as i16;
        let parts: &[PortableMathAssemblyPart] = if assembly.is_null() {
            &[]
        } else {
            &(*(assembly as *const GlyphAssembly)).parts
        };
        let min_o = Self::ot_min_connector_overlap(
            self as *mut PortableTexEngine<'resources>,
            f_0 as i32,
        );
        let mut n: integer = -1;
        let mut no_extenders = true;
        loop {
            n += 1;
            let mut s_max: scaled = 0;
            let mut prev_o: scaled = 0;
            for part in parts.iter() {
                if part.extender {
                    no_extenders = false;
                    for _ in 0..n {
                        let mut o = part.start_connector;
                        if min_o < o {
                            o = min_o;
                        }
                        if prev_o < o {
                            o = prev_o;
                        }
                        s_max = s_max - o + part.full_advance;
                        prev_o = part.end_connector;
                    }
                } else {
                    let mut o = part.start_connector;
                    if min_o < o {
                        o = min_o;
                    }
                    if prev_o < o {
                        o = prev_o;
                    }
                    s_max = s_max - o + part.full_advance;
                    prev_o = part.end_connector;
                }
            }
            if s_max >= s || no_extenders {
                break;
            }
        }
        let mut prev_o: scaled = 0;
        for &part in parts {
            let reps = if part.extender { n } else { 1 };
            for _ in 0..reps {
                let mut o = part.start_connector;
                if prev_o < o {
                    o = prev_o;
                }
                let oo = o;
                if min_o < o {
                    o = min_o;
                }
                if oo > 0 {
                    (&mut *(self as *mut PortableTexEngine<'_>))
                        .stack_glue_into_box(b, -oo, -o)?;
                }
                let g = part.glyph;
                (&mut *(self as *mut PortableTexEngine<'_>))
                    .stack_glyph_into_box(b, f_0, g)?;
                prev_o = part.end_connector;
            }
        }
        let mut p = (*mem.offset((b + 5) as isize)).hh.v.RH;
        let mut nat: scaled = 0;
        let mut str_: scaled = 0;
        while p != TEX_NULL {
            let ty = (*mem.offset(p as isize)).hh.u.B0 as i32;
            if ty == WHATSIT_NODE {
                if horiz {
                    nat += (*mem.offset((p + 1) as isize)).u.CINT;
                } else {
                    nat
                        += (*mem.offset((p + 3) as isize)).u.CINT
                            + (*mem.offset((p + 2) as isize)).u.CINT;
                }
            } else if ty == GLUE_NODE {
                let spec = (*mem.offset((p + 1) as isize)).hh.v.LH;
                nat += (*mem.offset((spec + 1) as isize)).u.CINT;
                str_ += (*mem.offset((spec + 2) as isize)).u.CINT;
            }
            p = (*mem.offset(p as isize)).hh.v.RH;
        }
        if s > nat && str_ > 0 {
            let mut o = s - nat;
            if o > str_ {
                o = str_;
            }
            (*mem.offset((b + 5) as isize)).hh.u.B1 = GLUE_ORDER_NORMAL;
            (*mem.offset((b + 5) as isize)).hh.u.B0 = GLUE_SIGN_STRETCHING;
            (*mem.offset((b + 6) as isize)).gr = o as f64 / str_ as f64;
            let stretched = nat
                + (str_ as f64 * (*mem.offset((b + 6) as isize)).gr).round() as scaled;
            if horiz {
                (*mem.offset((b + 1) as isize)).u.CINT = stretched;
            } else {
                (*mem.offset((b + 3) as isize)).u.CINT = stretched;
            }
        } else if horiz {
            (*mem.offset((b + 1) as isize)).u.CINT = nat;
        } else {
            (*mem.offset((b + 3) as isize)).u.CINT = nat;
        }
        Ok(b)
    }

}

pub(crate) unsafe fn fputs(_text: const_string, _file: NativeFileHandle) -> i32 {
    0
}

pub(crate) unsafe fn getcreationdate() {}

pub(crate) unsafe fn getfilemoddate(_s: integer) {}

pub(crate) unsafe fn getfiledump(_s: integer, _offset: i32, _length: i32) {}

pub(crate) unsafe fn getmd5sum(_s: integer, _file: i32) {}

pub(crate) unsafe fn u_close_file_or_pipe(file: *mut unicodefile) {
    if !file.is_null() {
        PortableTexEngine::boundary_close_file(*file);
        *file = core::ptr::null_mut();
    }
}

pub(crate) unsafe fn setinputfileencoding(
    file: unicodefile,
    mode: integer,
    _encoding_data: integer,
) {
    // XeTeX modes: auto 0, UTF-8 1, UTF-16 big endian 2, little endian 3, raw 4, ICU mapping 5 read as raw bytes.
    if file.is_null() {
        return;
    }
    let handle = &mut *file;
    handle.encoding = match mode {
        1 => InputEncoding::Utf8,
        2 => InputEncoding::Utf16Be,
        3 => InputEncoding::Utf16Le,
        4 => InputEncoding::Bytes,
        0 => InputEncoding::Utf8,
        _ => InputEncoding::Bytes,
    };
}

pub(crate) unsafe fn usingGraphite(_engine: FontHandle) -> boolean {
    false_0
}

pub(crate) unsafe fn aatprintfontname(
    _what: i32,
    _attrs: CFDictionaryRef,
    _param1: i32,
    _param2: i32,
) {
}

pub(crate) unsafe fn grprintfontname(
    _what: integer,
    _engine: voidpointer,
    _param1: integer,
    _param2: integer,
) {
}

pub(crate) unsafe fn printglyphname(_font: integer, _gid: integer) {}

pub(crate) unsafe fn applymapping(
    _mapping: voidpointer,
    _text: *mut uint16_t,
    text_len: i32,
) -> i32 {
    text_len
}

pub(crate) unsafe fn checkfortfmfontmapping() {}

pub(crate) unsafe fn loadtfmfontmapping() -> voidpointer {
    nullptr
}

pub(crate) unsafe fn applytfmfontmapping(_mapping: voidpointer, c_0: i32) -> i32 {
    c_0
}

pub(crate) unsafe fn countpdffilepages() -> i32 {
    0
}

pub(crate) unsafe fn aatfontget(_what: i32, _attrs: CFDictionaryRef) -> i32 {
    0
}

pub(crate) unsafe fn aatfontget1(_what: i32, _attrs: CFDictionaryRef, _param: i32) -> i32 {
    0
}

pub(crate) unsafe fn aatfontget2(
    _what: i32,
    _attrs: CFDictionaryRef,
    _param1: i32,
    _param2: i32,
) -> i32 {
    0
}

pub(crate) unsafe fn aatfontgetnamed(_what: i32, _attrs: CFDictionaryRef) -> i32 {
    0
}

pub(crate) unsafe fn aatfontgetnamed1(
    _what: i32,
    _attrs: CFDictionaryRef,
    _param: i32,
) -> i32 {
    0
}

pub(crate) unsafe fn grfontgetnamed(_what: integer, _engine: voidpointer) -> integer {
    0
}

pub(crate) unsafe fn grfontgetnamed1(
    _what: integer,
    _engine: voidpointer,
    _param: integer,
) -> integer {
    0
}

/// XeTeX `free_ot_assembly`, `assembly` must come from `get_ot_assembly_ptr` and not be freed yet.
pub(crate) unsafe fn free_ot_assembly(assembly: *mut GlyphAssembly) {
    if !assembly.is_null() {
        drop(Box::from_raw(assembly));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a text [`PortableFileHandle`] over `bytes` with the given encoding.
    fn text_handle(bytes: Vec<u8>, encoding: InputEncoding) -> PortableFileHandle {
        let mut handle = PortableFileHandle::new(
            "test.tex".to_string(),
            None,
            resource_format_tex_input,
            bytes,
        );
        handle.encoding = encoding;
        handle
    }

    /// Drain every Unicode scalar the decoder produces until EOF.
    fn decode_all(handle: &mut PortableFileHandle) -> Vec<u32> {
        let mut out = Vec::new();
        while let Some(scalar) = handle.next_input_scalar() {
            out.push(scalar);
        }
        out
    }

    #[test]
    fn utf8_decoder_reads_multibyte_scalars() {
        // "αβγ" = CE B1 CE B2 CE B3 -> U+03B1, U+03B2, U+03B3.
        let mut h = text_handle(vec![0xCE, 0xB1, 0xCE, 0xB2, 0xCE, 0xB3], InputEncoding::Utf8);
        assert_eq!(decode_all(&mut h), vec![0x3B1, 0x3B2, 0x3B3]);
        // ASCII stays one scalar per byte, three and four byte sequences round trip.
        let mut h = text_handle(
            vec![b'A', 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80],
            InputEncoding::Utf8,
        );
        assert_eq!(decode_all(&mut h), vec![0x41, 0x20AC, 0x1F600]);
    }

    #[test]
    fn utf8_decoder_replaces_bad_sequences() {
        // A lead byte then a non continuation byte gives U+FFFD and leaves the byte to decode next.
        let mut h = text_handle(vec![0xCE, b'A'], InputEncoding::Utf8);
        assert_eq!(decode_all(&mut h), vec![0xFFFD, 0x41]);
        // A lone continuation byte is a lead byte with no continuation bytes and decodes to itself.
        let mut h = text_handle(vec![0x80], InputEncoding::Utf8);
        assert_eq!(decode_all(&mut h), vec![0x80]);
    }

    #[test]
    fn utf16_decoders_read_units_and_surrogates() {
        // "αβγ" UTF-16LE: B1 03 B2 03 B3 03.
        let mut h = text_handle(
            vec![0xB1, 0x03, 0xB2, 0x03, 0xB3, 0x03],
            InputEncoding::Utf16Le,
        );
        assert_eq!(decode_all(&mut h), vec![0x3B1, 0x3B2, 0x3B3]);
        // Same in UTF-16BE: 03 B1 03 B2 03 B3.
        let mut h = text_handle(
            vec![0x03, 0xB1, 0x03, 0xB2, 0x03, 0xB3],
            InputEncoding::Utf16Be,
        );
        assert_eq!(decode_all(&mut h), vec![0x3B1, 0x3B2, 0x3B3]);
        // Surrogate pair U+1F600 in UTF-16 little endian, units `D83D DE00`.
        let mut h = text_handle(vec![0x3D, 0xD8, 0x00, 0xDE], InputEncoding::Utf16Le);
        assert_eq!(decode_all(&mut h), vec![0x1F600]);
        // A high surrogate then a non low unit gives U+FFFD and leaves the unit to decode next.
        let mut h = text_handle(vec![0x3D, 0xD8, 0x41, 0x00], InputEncoding::Utf16Le);
        assert_eq!(decode_all(&mut h), vec![0xFFFD, 0x41]);
        // Lone low surrogate -> U+FFFD.
        let mut h = text_handle(vec![0x00, 0xDC], InputEncoding::Utf16Le);
        assert_eq!(decode_all(&mut h), vec![0xFFFD]);
    }

    #[test]
    fn bytes_mode_reads_each_byte_raw() {
        // Raw bytes: every byte is its own scalar.
        let mut h = text_handle(vec![0xCE, 0xB1, 0x41], InputEncoding::Bytes);
        assert_eq!(decode_all(&mut h), vec![0xCE, 0xB1, 0x41]);
    }

    #[test]
    fn bom_sniff_selects_encoding_and_consumes_bom() {
        // A UTF-8 BOM selects UTF-8 and is skipped.
        let mut h = text_handle(vec![0xEF, 0xBB, 0xBF, b'A'], InputEncoding::Bytes);
        h.resolve_text_encoding_auto();
        assert_eq!(h.encoding, InputEncoding::Utf8);
        assert_eq!(h.cursor, 3);
        assert_eq!(decode_all(&mut h), vec![0x41]);
        // A UTF-16 big endian BOM selects that encoding and is skipped.
        let mut h = text_handle(vec![0xFE, 0xFF, 0x03, 0xB1], InputEncoding::Bytes);
        h.resolve_text_encoding_auto();
        assert_eq!(h.encoding, InputEncoding::Utf16Be);
        assert_eq!(h.cursor, 2);
        assert_eq!(decode_all(&mut h), vec![0x3B1]);
        // A UTF-16 little endian BOM selects that encoding and is skipped.
        let mut h = text_handle(vec![0xFF, 0xFE, 0xB1, 0x03], InputEncoding::Bytes);
        h.resolve_text_encoding_auto();
        assert_eq!(h.encoding, InputEncoding::Utf16Le);
        assert_eq!(h.cursor, 2);
        assert_eq!(decode_all(&mut h), vec![0x3B1]);
        // ASCII text without a BOM reads as UTF-8 from its first byte.
        let mut h = text_handle(vec![b'h', b'i'], InputEncoding::Bytes);
        h.resolve_text_encoding_auto();
        assert_eq!(h.encoding, InputEncoding::Utf8);
        assert_eq!(h.cursor, 0);
        // A zero first byte without a BOM guesses UTF-16 big endian and skips nothing.
        let mut h = text_handle(vec![0x00, 0x41], InputEncoding::Bytes);
        h.resolve_text_encoding_auto();
        assert_eq!(h.encoding, InputEncoding::Utf16Be);
        assert_eq!(h.cursor, 0);
        assert_eq!(decode_all(&mut h), vec![0x41]);
    }

    #[test]
    fn input_line_decodes_text_into_buffer() {
        // Reads one line through `boundary_input_line` after the encoding sniff files get when opened.
        fn buffer_after_input_line(bytes: Vec<u8>) -> Vec<u32> {
            let image = PortableFormatImage::empty();
            let mut engine = PortableTexEngine::from_format(&image, EmptyResourceProvider);
            assert!(engine.initialize_format_state());
            let mut handle = PortableFileHandle::new(
                "input.tex".to_string(),
                None,
                resource_format_tex_input,
                bytes,
            );
            PortableTexEngine::resolve_input_encoding(&mut handle);
            let raw = Box::into_raw(Box::new(handle));
            engine.state.first = 0;
            let read = unsafe {
                PortableTexEngine::boundary_input_line(
                    &mut engine as *mut PortableTexEngine<'_>,
                    raw as NativeFileHandle,
                )
            };
            assert!(matches!(read, Ok(line) if line != 0), "boundary_input_line should succeed");
            let first = engine.state.first.max(0) as usize;
            let last = engine.state.last.max(0) as usize;
            let out = engine.heap.buffer_storage[first..last]
                .iter()
                .map(|&scalar| scalar as u32)
                .collect();
            unsafe { drop(Box::from_raw(raw)) };
            out
        }
        // "αβγ" in UTF-8, then UTF-16LE and UTF-16BE with byte order marks.
        let utf8 = vec![0xCE, 0xB1, 0xCE, 0xB2, 0xCE, 0xB3];
        assert_eq!(buffer_after_input_line(utf8), vec![0x3B1, 0x3B2, 0x3B3]);
        let utf16le_bom = vec![0xFF, 0xFE, 0xB1, 0x03, 0xB2, 0x03, 0xB3, 0x03];
        assert_eq!(buffer_after_input_line(utf16le_bom), vec![0x3B1, 0x3B2, 0x3B3]);
        let utf16be_bom = vec![0xFE, 0xFF, 0x03, 0xB1, 0x03, 0xB2, 0x03, 0xB3];
        assert_eq!(buffer_after_input_line(utf16be_bom), vec![0x3B1, 0x3B2, 0x3B3]);
    }

    #[test]
    fn engine_abort_is_captured_at_runtime_boundary() {
        let image = PortableFormatImage::empty();
        let mut engine = PortableTexEngine::from_format(&image, EmptyResourceProvider);

        let completed = engine.catch_engine_abort(|engine| unsafe {
            PortableTexEngine::abort_engine(engine as *mut PortableTexEngine<'_>, 7)?;
            Ok(())
        });

        assert!(!completed);
        assert_eq!(engine.last_abort_status(), Some(7));
    }

    #[test]
    fn successful_engine_abort_completes_runtime_boundary() {
        let image = PortableFormatImage::empty();
        let mut engine = PortableTexEngine::from_format(&image, EmptyResourceProvider);

        let completed = engine.catch_engine_abort(|engine| unsafe {
            PortableTexEngine::abort_engine(engine as *mut PortableTexEngine<'_>, 0)?;
            Ok(())
        });

        assert!(completed);
        assert_eq!(engine.last_abort_status(), None);
    }

    fn initialized_engine() -> PortableTexEngine<'static> {
        let mut engine =
            PortableTexEngine::from_format(&PortableFormatImage::empty(), EmptyResourceProvider);
        assert!(engine.initialize_format_state());
        engine
    }

    /// Runs INITEX `input` with brace catcodes set, as no format assigns them yet.
    fn run_initex(engine: &mut PortableTexEngine<'_>, input: &str) -> bool {
        let input = format!("\\catcode`\\{{=1 \\catcode`\\}}=2 {input}");
        assert!(engine.begin_primary_input("input.tex", input.into_bytes()));
        engine.run_main_control()
    }

    /// Replaces the body checksum after a test edits an encoded image.
    fn reseal(bytes: &mut [u8]) {
        let checksum = image_checksum(&bytes[HEADER_LEN..]);
        bytes[VALIDATION_TAG_LEN + 8..HEADER_LEN].copy_from_slice(&checksum.to_le_bytes());
    }

    #[test]
    fn state_holds_no_heap_owning_field() {
        // `PortableTexState` is copied bitwise, so a `Vec` there would be freed once per copy.
        assert!(!core::mem::needs_drop::<PortableTexState>());
    }

    #[test]
    fn heap_fields_survive_empty_clone_and_image_round_trip() {
        let image = PortableFormatImage::empty();
        let mut first = PortableTexEngine::from_format(&image, EmptyResourceProvider);
        first.heap.src_grp_stack.push(7);
        first.heap.src_spans.push(RawSpan::default());
        let copy = first.snapshot_format();
        let mut second = PortableTexEngine::from_format(&copy, EmptyResourceProvider);
        second.heap.src_grp_stack.push(8);
        assert_eq!(first.heap.src_grp_stack, [7]);
        assert_eq!(second.heap.src_grp_stack, [7, 8]);
        let reloaded = PortableFormatImage::from_bytes(&copy.to_bytes()).expect("empty image reloads");
        let third = PortableTexEngine::from_format(&reloaded, EmptyResourceProvider);
        assert!(third.heap.src_grp_stack.is_empty() && third.heap.src_spans.is_empty());
    }

    #[test]
    fn snapshot_after_source_tracking_render_does_not_share_buffers() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        assert!(run_initex(&mut engine, "\\setbox0\\hbox{\\kern1pt}\\end"));
        engine.heap.src_grp_stack.push(3);
        let image = engine.snapshot_format();
        engine.heap.src_grp_stack.clear();
        engine.heap.src_grp_stack.shrink_to_fit();
        let instance = PortableTexEngine::from_format(&image, EmptyResourceProvider);
        assert_eq!(instance.heap.src_grp_stack, [3]);
        drop(engine);
        drop(image);
        assert_eq!(instance.heap.src_grp_stack, [3]);
    }

    #[test]
    fn format_dumps_are_deterministic_and_round_trip() {
        let first = initialized_engine().into_format().to_bytes();
        let second = initialized_engine().into_format().to_bytes();
        assert!(first == second, "two dumps of the same format differ");
        let reloaded = PortableFormatImage::from_bytes(&first).expect("dump reloads");
        assert!(reloaded.to_bytes() == first, "a reloaded image dumps differently");
        let mut engine = PortableTexEngine::from_format(&reloaded, EmptyResourceProvider);
        assert!(run_initex(&mut engine, "\\setbox0\\hbox{\\kern1pt}\\end"));
    }

    #[test]
    fn hostile_images_return_errors() {
        let bytes = initialized_engine().into_format().to_bytes();
        let decode = |bytes: &[u8]| PortableFormatImage::from_bytes(bytes).err();
        assert_eq!(decode(&bytes[..10]), Some(FormatImageError::Truncated));
        assert_eq!(decode(&bytes[..bytes.len() - 1]), Some(FormatImageError::Truncated));
        let mut other_build = bytes.clone();
        other_build[9] ^= 1;
        assert_eq!(decode(&other_build), Some(FormatImageError::OtherBuild));
        let mut corrupt = bytes.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        assert_eq!(decode(&corrupt), Some(FormatImageError::ChecksumMismatch));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(decode(&trailing), Some(FormatImageError::TrailingBytes));
        // `initialized` is the first field, so the first body byte is a bool.
        let mut bad_bool = bytes.clone();
        bad_bool[HEADER_LEN] = 2;
        reseal(&mut bad_bool);
        assert_eq!(decode(&bad_bool), Some(FormatImageError::InvalidField("initialized")));
        assert_eq!(
            PortableFormatImage::from_static_bytes(Box::leak(corrupt.into_boxed_slice())).err(),
            Some(FormatImageError::ChecksumMismatch)
        );
    }

    #[test]
    fn image_tags_carry_byte_order_and_no_pointer_width() {
        let bytes = initialized_engine().into_format().to_bytes();
        let mut expected = *b"MTXfmt\x07\x00\x00________";
        expected[9..].copy_from_slice(&BUILD_FINGERPRINT.to_le_bytes());
        assert_eq!(bytes[..VALIDATION_TAG_LEN], expected, "the tag must read the same on every little endian target");
        let decode = |edit: fn(&mut [u8])| {
            let mut bytes = bytes.clone();
            edit(&mut bytes);
            PortableFormatImage::from_bytes(&bytes).err()
        };
        assert_eq!(decode(|bytes| bytes[0] = b'x'), Some(FormatImageError::NotAnImage));
        // Layout 6 stored the pointer width here and let a memory word hold a pointer.
        assert_eq!(decode(|bytes| bytes[6] = 6), Some(FormatImageError::Version(6)));
        assert_eq!(decode(|bytes| bytes[8] = 1), Some(FormatImageError::ByteOrder));
    }

    #[test]
    fn native_glyph_words_never_keep_stale_bytes() {
        // A 32 bit pointer store zeroed only the low half, so a reused word kept a stale high half.
        let stale = memoryword { raw: 0xDEAD_BEEF_0000_0000 };
        let mut engine = initialized_engine();
        unsafe {
            let mem = engine.state.zmem;
            // Six words of node plus one word holding up to four UTF-16 units.
            let size = 7;
            let block = engine.zgetnode(size).expect("allocates");
            for word in block..block + size {
                *mem.offset(word as isize) = stale;
            }
            engine.zfreenode(block, size);
            engine.state.rover = block;
            let node = engine.znewnativewordnode(0, 3).expect("allocates");
            assert_eq!(node, block, "the node reuses the stale block");
            assert_eq!((*mem.offset(node as isize + 5)).raw, 0);
            *mem.offset(node as isize + 5) = stale;
            let copy = engine.zcopynodelist(node).expect("copies");
            *mem.offset(node as isize + 5) = memoryword::default();
            assert_eq!((*mem.offset(copy as isize + 5)).raw, 0);
            assert_eq!(engine.zcharpw(copy, 0), 0);
            engine.zflushnodelist(copy).expect("frees");
            engine.zflushnodelist(node).expect("frees");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "glyph info word is nonzero")]
    fn a_nonzero_glyph_info_word_is_never_read_as_a_pointer() {
        let mut engine = initialized_engine();
        unsafe {
            let node = engine.znewnativewordnode(0, 3).expect("allocates");
            (*engine.state.zmem.offset(node as isize + 5)).raw = 0xDEAD_BEEF_0000_0000;
            engine.zcharpw(node, 0);
        }
    }

    #[test]
    fn images_with_inconsistent_sizes_are_refused() {
        let image = initialized_engine().into_format();
        let encode_with = |edit: &dyn Fn(&mut PortableTexState)| {
            let mut state = boxed_state_copy(&image.state);
            edit(&mut state);
            encode_image(&state, &image.heap)
        };
        let decode = |bytes: Vec<u8>| PortableFormatImage::from_bytes(&bytes).err();
        assert_eq!(
            decode(encode_with(&|state| state.memtop = i32::MAX)),
            Some(FormatImageError::Geometry("memtop"))
        );
        assert_eq!(
            decode(encode_with(&|state| state.memend = state.memmax + 1)),
            Some(FormatImageError::Geometry("lomemmax, himemmin and memend"))
        );
        assert_eq!(
            decode(encode_with(&|state| state.poolptr = state.poolsize + 1)),
            Some(FormatImageError::Geometry("poolptr"))
        );
        assert_eq!(
            decode(encode_with(&|state| state.fmemptr = -1)),
            Some(FormatImageError::Geometry("fmemptr"))
        );
        // A huge length must be refused before anything is allocated for it.
        assert_eq!(
            decode(encode_with(&|state| state.nativetextsize = i32::MAX)),
            Some(FormatImageError::Geometry("nativetextsize"))
        );
        let empty = PortableFormatImage::empty();
        let mut state = boxed_state_copy(&empty.state);
        state.nativetextsize = i32::MAX;
        assert_eq!(
            decode(encode_image(&state, &empty.heap)),
            Some(FormatImageError::Geometry("nativetextsize"))
        );
    }

    #[test]
    fn uninitialized_engine_refuses_every_run_entry_point() {
        let mut engine =
            PortableTexEngine::from_format(&PortableFormatImage::empty(), EmptyResourceProvider);
        assert!(!engine.begin_primary_input("input.tex", b"\\end".to_vec()));
        assert_eq!(engine.last_error_message(), Some(UNINITIALIZED_STATE_MESSAGE));
        assert!(!engine.run_main_control());
        assert!(!engine.run_format_initialization());
        assert!(!engine.finalize_trie());
        assert!(engine.snapshot_node(PortableNodeHandle(0)).is_none());
        assert!(engine.native_font_spec(0).is_none());
    }

    #[test]
    fn seal_and_load_close_file_slots() {
        let mut engine = initialized_engine();
        let handle = Box::into_raw(Box::new(PortableFileHandle::new(
            "open.tex".to_string(),
            None,
            resource_format_tex_input,
            Vec::new(),
        )));
        engine.state.readfile[3] = handle;
        engine.state.readopen[3] = 1;
        engine.state.writefile[4] = handle;
        engine.state.writeopen[4] = true_0;
        let image = engine.snapshot_format();
        assert!(image.state.readfile.iter().all(|file| file.is_null()));
        assert!(image.state.writefile.iter().all(|file| file.is_null()));
        assert!(image.state.readopen.iter().all(|&state| state == 2));
        assert!(image.state.writeopen.iter().all(|&open| open == false_0));
        let mut state = boxed_state_copy(&image.state);
        state.readopen[5] = 0;
        let reloaded = PortableFormatImage::from_bytes(&encode_image(&state, &image.heap))
            .expect("image reloads");
        assert_eq!(reloaded.state.readopen[5], 2);
        engine.state.readfile[3] = core::ptr::null_mut();
        engine.state.writefile[4] = core::ptr::null_mut();
        unsafe { drop(Box::from_raw(handle)) };
    }

    #[test]
    fn public_accessors_bound_their_indices() {
        let mut engine = initialized_engine();
        assert!(engine.native_font_spec(10_000).is_none());
        assert!(engine.native_font_spec(-1).is_none());
        assert!(engine.pool_string(i32::MAX).is_none());
        assert!(engine.pool_string(65536 + MAX_STRINGS).is_none());
        assert!(engine.font_name(10_000).is_none());
        // A high memory word that is not a char node, with a font byte past every font array.
        let word = engine.state.himemmin;
        let slot = (word - engine.state.memmin) as usize;
        engine.heap.mem[slot].hh.u.B0 = i16::MAX;
        engine.heap.mem[slot].hh.u.B1 = i16::MAX;
        let snapshot = engine.snapshot_node(PortableNodeHandle(word)).expect("in memory");
        assert_eq!(snapshot.kind, PortableNodeKind::Character);
        assert_eq!(snapshot.width, 0);
        assert!(engine.snapshot_node(PortableNodeHandle(i32::MAX)).is_none());
        assert!(engine.snapshot_node(PortableNodeHandle(-5)).is_none());
    }

    #[test]
    fn pdf_error_stops_the_run() {
        let mut engine = initialized_engine();
        assert!(!run_initex(&mut engine, "\\message{\\leftmarginkern0}\\end"));
        let message = engine.last_error_message().expect("pdf_error surfaces a message");
        assert!(message.starts_with("Error"), "{message}");
        engine.state.hliststacklevel = 0;
        assert!(matches!(unsafe { engine.popnode() }, Err(EngineBreak::Error(_))));
        engine.state.hliststacklevel = 513;
        assert!(matches!(unsafe { engine.zpushnode(0) }, Err(EngineBreak::Error(_))));
    }

    #[test]
    fn finalize_trie_reports_pattern_memory_overflow() {
        let mut engine = initialized_engine();
        let input = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b c1d e1f}\\end";
        assert!(engine.begin_primary_input("input.tex", input.as_bytes().to_vec()));
        assert!(engine.run_format_initialization());
        assert_ne!(engine.state.trienotready, 0);
        // Room for the few trie nodes in the hash, too little for `first_fit` to pack them.
        engine.state.triesize = 64;
        assert!(!engine.finalize_trie());
        let message = engine.last_error_message().expect("the overflow surfaces its message");
        assert!(message.contains("pattern memory"), "{message}");
    }

    #[test]
    fn long_native_run_grows_the_native_text_buffer() {
        let mut engine = initialized_engine().with_font_platform(MockNativeFonts);
        let run = "a".repeat(300);
        let input = format!("\\font\\x=\"mock\" \\x \\setbox0\\hbox{{{run}}}\\end");
        assert!(run_initex(&mut engine, &input), "{:?}", engine.last_error_message());
        assert!(engine.state.nativetextsize > 300);
        assert_eq!(
            engine.heap.nativetext_storage.len(),
            engine.state.nativetextsize as usize
        );
        let widest = engine
            .native_glyph_infos
            .values()
            .map(|info| info.glyphs.len())
            .max();
        assert_eq!(widest, Some(300));
    }

    /// Spans of the primary input after a tracked INITEX run, tracking switched on before or after input starts.
    fn tracked_spans(bytes: &[u8], track_before_input: bool) -> Vec<(u32, u32)> {
        let mut engine = initialized_engine();
        if track_before_input {
            engine.set_source_tracking(true);
        }
        assert!(engine.begin_primary_input("input.tex", bytes.to_vec()));
        if !track_before_input {
            engine.set_source_tracking(true);
        }
        assert!(engine.run_main_control(), "{:?}", engine.last_error_message());
        engine
            .input_source_spans()
            .into_iter()
            .filter(|span| span.name == "input.tex")
            .map(|span| (span.start, span.end))
            .collect()
    }

    /// Byte range of the `nth` occurrence of `needle` in `text`, shifted by `offset` bytes.
    fn byte_range(text: &str, needle: &str, nth: usize, offset: usize) -> (u32, u32) {
        let start = text.match_indices(needle).nth(nth).expect("needle occurs").0 + offset;
        (start as u32, (start + needle.len()) as u32)
    }

    #[test]
    fn source_spans_are_byte_offsets_into_the_input() {
        let text = "\\catcode`\\{=1 \\catcode`\\}=2 \\setbox0\\hbox{\u{3b1}\\relax \u{e9}\u{2308}   \r\n\\relax \u{2308}q\r\n}\\end\r\n";
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        let spans = tracked_spans(&bytes, true);
        let bom = 3;
        for expected in [
            byte_range(text, "\\setbox", 0, bom),
            byte_range(text, "\u{3b1}", 0, bom),
            byte_range(text, "\u{e9}", 0, bom),
            byte_range(text, "\u{2308}", 0, bom),
            byte_range(text, "\\relax", 1, bom),
            byte_range(text, "\u{2308}", 1, bom),
            byte_range(text, "q", 0, bom),
            byte_range(text, "\\end", 0, bom),
        ] {
            assert!(spans.contains(&expected), "{expected:?} missing from {spans:?}");
        }
    }

    #[test]
    fn source_spans_do_not_depend_on_when_tracking_is_enabled() {
        let text = "\\catcode`\\{=1 \\catcode`\\}=2\n\\setbox0\\hbox{a\\relax\nb\\relax}\n\\end\n";
        let before = tracked_spans(text.as_bytes(), true);
        let after = tracked_spans(text.as_bytes(), false);
        assert_eq!(before, after);
        assert!(before.contains(&byte_range(text, "\\relax", 1, 0)), "{before:?}");
        assert!(before.contains(&byte_range(text, "\\end", 0, 0)), "{before:?}");
    }

    #[test]
    fn node_allocation_clears_a_stale_span() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        engine.state.cmd_span = engine.intern_span_raw(65536, 1, 2, 0);
        engine.heap.src_stack_cells.push(SrcStackCell::default());
        engine.state.cur_stack_head = 1;
        let node = unsafe { engine.zgetnode(4) }.expect("allocates");
        assert_ne!(engine.heap.node_src.get_copy(node as usize), 0);
        assert_eq!(engine.heap.node_stack.get_copy(node as usize), 1);
        unsafe { engine.zfreenode(node, 4) };
        // The allocation hook stamps a reused block, which must not keep its previous occupant's span.
        engine.state.cmd_span = 0;
        engine.state.cur_stack_head = 0;
        unsafe { PortableTexEngine::src_stamp_node_range(&mut engine, node, 4) };
        for word in node..node + 4 {
            assert_eq!(engine.heap.node_src.get_copy(word as usize), 0);
            assert_eq!(engine.heap.node_stack.get_copy(word as usize), 0);
        }
    }

    /// Nodes of the list starting at `first`, in order.
    fn list_snapshots(
        engine: &PortableTexEngine<'_>,
        first: Option<PortableNodeHandle>,
    ) -> Vec<PortableNodeSnapshot> {
        let mut nodes = Vec::new();
        let mut next = first;
        while let Some(handle) = next {
            let node = engine.snapshot_node(handle).expect("list node");
            next = node.link;
            nodes.push(node);
        }
        nodes
    }

    /// Runs INITEX `input` with fragment capture and returns the captured root.
    fn captured_root(engine: &mut PortableTexEngine<'_>, input: &str) -> PortableNodeSnapshot {
        engine.begin_fragment_capture();
        assert!(run_initex(engine, input), "{:?}", engine.last_error_message());
        engine.end_fragment_capture();
        let root = engine.captured_fragment_root().expect("a captured root");
        engine.snapshot_node(root).expect("root snapshot")
    }

    #[test]
    fn a_backed_up_macro_body_token_maps_to_the_invocation() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        let body = "\\catcode`\\#=6 \\def\\m#1{#1\\kern1pt}\\hbox{\\count1=\\m{7}}\\end";
        let root = captured_root(&mut engine, body);
        let kern = list_snapshots(&engine, root.list)
            .into_iter()
            .find(|node| node.kind == PortableNodeKind::Kern)
            .expect("the macro's kern");
        let source = kern.source.expect("the kern is stamped");
        // `run_initex` prepends the brace catcodes, which shift every offset.
        let prefix = "\\catcode`\\{=1 \\catcode`\\}=2 ".len();
        assert_eq!((source.start, source.end), byte_range(body, "\\m{7}", 0, prefix));
    }

    /// Resolves every font to handle 1, maps each character to itself and shapes one glyph per UTF-16 unit.
    struct MockNativeFonts;

    impl FontPlatform for MockNativeFonts {
        fn resolve_font_handle(self: &mut Self, _name: &[i32], _size: i32) -> Option<PortableFontHandle> {
            Some(1)
        }

        fn map_char_to_glyph(self: &mut Self, _font: PortableFontHandle, codepoint: i32) -> i32 {
            codepoint
        }

        fn shape_native_text(
            self: &mut Self,
            _font: PortableFontHandle,
            text: &[u16],
            _use_glyph_metrics: bool,
        ) -> PortableNativeTextMetrics {
            PortableNativeTextMetrics {
                width: text.len() as i32 * 65536,
                height: 65536,
                depth: 0,
                glyphs: text
                    .iter()
                    .map(|&unit| PortableNativeGlyph {
                        glyph_id: unit,
                        ..PortableNativeGlyph::default()
                    })
                    .collect(),
            }
        }

        // Glyph `g` is `g` points wide and its control box reaches one point further, a 1pt italic correction.
        fn glyph_bounds(self: &mut Self, _font: PortableFontHandle, glyph: u16) -> PortableGlyphBounds {
            let advance = f32::from(glyph);
            PortableGlyphBounds {
                advance,
                x_min: -0.5,
                y_min: -2.0,
                x_max: advance + 1.0,
                y_max: 3.0,
            }
        }

        fn char_code_range(self: &mut Self, _font: PortableFontHandle) -> Option<(i32, i32)> {
            Some((32, 126))
        }
    }

    const MOCK_FONT: &str = "\\font\\x=\"mock\" \\x ";

    #[test]
    fn native_glyph_side_table_follows_copies_and_frees() {
        let run = |input: &str| {
            let mut engine = initialized_engine().with_font_platform(MockNativeFonts);
            assert!(run_initex(&mut engine, &format!("{MOCK_FONT}{input}\\end")), "{:?}", engine.last_error_message());
            engine.native_glyph_infos.len()
        };
        let boxes = "\\setbox1\\hbox{\\XeTeXglyph5 ab}\\setbox2\\copy1 ";
        // A glyph node and a native word, each in box 1 and in its copy.
        assert_eq!(run(boxes), 4);
        assert_eq!(run(&format!("{boxes}\\setbox1\\box3 \\setbox2\\box3 ")), 0);
    }

    /// Hands every host box token `t` back as a box `t` points wide.
    struct MockHostBoxes;

    impl PortablePlatform for MockHostBoxes {
        fn host_box(self: &mut Self, request: PortableHostBoxRequest) -> Option<PortableHostBox> {
            Some(PortableHostBox {
                width: request.token * 65536,
                height: 65536,
                depth: 0,
                runs: Vec::new(),
                rules: Vec::new(),
            })
        }
    }

    /// Record ids of every host box marker under `node`.
    fn host_box_ids(engine: &PortableTexEngine<'_>, node: &PortableNodeSnapshot, ids: &mut Vec<i32>) {
        if node.kind == PortableNodeKind::HostBoxRef {
            ids.push(node.character);
        }
        for child in list_snapshots(engine, node.list) {
            host_box_ids(engine, &child, ids);
        }
    }

    #[test]
    fn a_host_box_kept_from_an_earlier_format_finds_no_record() {
        let mut first = initialized_engine().with_platform(MockHostBoxes);
        assert!(run_initex(&mut first, "\\global\\setbox1\\hbox{\\Uhostbox 5\\relax}\\end"));
        let image = first.snapshot_format();
        let mut second = PortableTexEngine::from_format(&image, EmptyResourceProvider).with_platform(MockHostBoxes);
        let root = captured_root(&mut second, "\\hbox{\\copy1 \\Uhostbox 6\\relax}\\end");
        let mut ids = Vec::new();
        host_box_ids(&second, &root, &mut ids);
        assert_eq!(ids.len(), 2, "{ids:?}");
        assert_ne!(ids[0], ids[1]);
        let records = ids
            .iter()
            .map(|&id| second.host_box_record(id as usize).map(|record| record.width))
            .collect::<Vec<_>>();
        assert_eq!(records, [None, Some(6 * 65536)]);
    }

    #[test]
    fn a_text_host_box_is_appended_like_a_box() {
        let mut engine = initialized_engine().with_platform(MockHostBoxes);
        let root = captured_root(
            &mut engine,
            "\\baselineskip=20pt \\vbox{\\hbox{}\\Uhostbox 5\\relax}\\end",
        );
        let kinds = list_snapshots(&engine, root.list)
            .iter()
            .map(|node| node.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [PortableNodeKind::HorizontalBox, PortableNodeKind::Glue, PortableNodeKind::HorizontalBox]
        );
        let mut engine = initialized_engine().with_platform(MockHostBoxes);
        let resets_space_factor =
            "\\setbox0\\hbox{\\spacefactor=3000 \\Uhostbox 5\\relax\\ifnum\\spacefactor=1000 \\else\\errmessage{space factor}\\fi}\\end";
        assert!(run_initex(&mut engine, resets_space_factor), "{:?}", engine.last_error_message());
    }

    #[test]
    fn the_sandbox_budget_ends_an_expansion_loop() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        engine.set_sandbox(true);
        engine.set_sandbox_op_budget(10_000);
        assert!(!run_initex(&mut engine, "\\def\\x{\\x}\\x\\end"));
        let message = engine.last_error_message().expect("the budget surfaces an error");
        assert!(message.contains("did not terminate"), "{message}");
        assert_eq!(engine.last_error().map(|error| error.kind), Some(PortableErrorKind::Budget));
        assert!(engine.heap.src_stack_cells.len() <= 10_000);
    }

    #[test]
    fn a_surfaced_error_reports_its_line_and_token_span() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        let input = "\\relax\n\\relax \\undefined \\end";
        assert!(!run_initex(&mut engine, input));
        let error = engine.last_error().expect("an undefined control sequence surfaces");
        assert_eq!(error.kind, PortableErrorKind::Tex);
        assert_eq!(error.line, 2);
        let span = error.span.as_ref().expect("the offending token's span");
        // run_initex prepends the brace catcodes, so offsets count from the start of the full input.
        let full = format!("\\catcode`\\{{=1 \\catcode`\\}}=2 {input}");
        assert_eq!(&full[span.start as usize..span.end as usize], "\\undefined");

        engine.set_sandbox(true);
        assert!(!run_initex(&mut engine, "\\openin1=x \\end"));
        assert_eq!(engine.last_error().map(|error| error.kind), Some(PortableErrorKind::Sandbox));
    }

    #[test]
    fn the_construct_stack_freezes_at_its_cap() {
        let mut engine = initialized_engine();
        engine.set_source_tracking(true);
        engine.heap.src_stack_cells = vec![SrcStackCell::default(); SRC_STACK_CAP];
        engine.state.cur_stack_head = 7;
        engine.src_stack_push(1, 0, 0, false);
        assert!(engine.state.src_stack_frozen);
        assert_eq!(engine.state.cur_stack_head, 0);
        assert_eq!(engine.heap.src_stack_cells.len(), SRC_STACK_CAP);
        engine.src_stack_pop();
        assert_eq!(engine.state.cur_stack_head, 0);
    }

    /// A sandboxed engine whose write stream 1 was opened before the fragment started.
    fn sandboxed_with_open_stream() -> PortableTexEngine<'static> {
        let mut engine = initialized_engine();
        let file = PortableFileHandle::new("open.tex".to_string(), None, resource_format_tex_input, Vec::new());
        engine.state.writefile[1] = Box::into_raw(Box::new(file));
        engine.state.writeopen[1] = true_0;
        engine.set_sandbox(true);
        engine
    }

    #[test]
    fn the_sandbox_refuses_file_access() {
        for (input, command) in [
            ("\\input other.tex ", "\\input"),
            ("\\openin1=other ", "\\openin"),
            ("\\openout1=other ", "\\openout"),
            ("\\immediate\\openout1=other ", "\\openout"),
            ("\\setbox0\\hbox{\\write1{x}}", "\\write"),
            ("\\immediate\\write1{x}", "\\write"),
            ("\\read16 to\\x ", "\\read"),
        ] {
            let mut engine = sandboxed_with_open_stream();
            assert!(!run_initex(&mut engine, &format!("{input}\\end")), "{input}");
            let message = engine.last_error_message().unwrap_or_default();
            assert!(message.starts_with(command), "{input}: {message}");
        }
        // Streams not open to a file write to the terminal, as LaTeX's `\typeout` does.
        let mut engine = sandboxed_with_open_stream();
        let terminal = "\\immediate\\write16{x}\\immediate\\write5{y}\\setbox0\\hbox{\\write16{x}\\write-1{y}}\\end";
        assert!(run_initex(&mut engine, terminal), "{:?}", engine.last_error_message());
    }

    #[test]
    fn a_written_file_reads_back_through_input() {
        let mut engine = initialized_engine();
        let input = concat!(
            "\\immediate\\openout3=written ",
            "\\immediate\\write3{\\noexpand\\gdef\\noexpand\\readback{ok}}",
            "\\immediate\\closeout3 ",
            "\\input written.tex ",
            "\\ifx\\readback\\undefined \\errmessage{nothing read back}\\fi\\end",
        );
        assert!(run_initex(&mut engine, input), "{:?}", engine.last_error_message());
        assert_eq!(
            engine.virtual_files.get("written.tex").map(Vec::as_slice),
            Some(&b"\\gdef \\readback {ok}\n"[..])
        );
    }

    #[test]
    fn the_transcript_records_the_log_once_and_errors_keep_their_whole_message() {
        let mut engine = initialized_engine();
        assert!(run_initex(&mut engine, "\\message{\\jobname}\\message{hello}\\end"));
        let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
        assert!(transcript.contains("hello") && !transcript.contains("hheelllloo"), "{transcript}");
        let long = "a long message that wraps past the seventy nine column limit of the transcript lines, twice over";
        let mut engine = initialized_engine();
        assert!(!run_initex(&mut engine, &format!("\\message{{\\jobname}}\\errmessage{{{long}}}\\end")));
        assert_eq!(engine.last_error_message(), Some(long));
        let mut engine = initialized_engine();
        let split = "\\catcode`\\^=7 \\newlinechar=10 \\errmessage{first^^Jsecond}\\end";
        assert!(!run_initex(&mut engine, split));
        assert_eq!(engine.last_error_message(), Some("first\nsecond"));
    }

    /// Serves one file.
    struct OneFile;

    impl ResourceProvider for OneFile {
        fn read(self: &mut Self, request: ResourceRequest<'_>) -> Option<Vec<u8>> {
            (request.name == "present.tex").then(|| b"12345".to_vec())
        }
    }

    #[test]
    fn file_size_is_empty_for_a_missing_file_and_recorded() {
        let mut engine = PortableTexEngine::from_format(&PortableFormatImage::empty(), OneFile);
        assert!(engine.initialize_format_state());
        let input = concat!(
            "\\def\\empty{}\\def\\five{5}",
            "\\edef\\a{\\filesize{missing.tex}}\\ifx\\a\\empty\\else\\errmessage{missing has a size}\\fi",
            "\\edef\\b{\\filesize{present.tex}}\\ifx\\b\\five\\else\\errmessage{wrong size}\\fi\\end",
        );
        assert!(run_initex(&mut engine, input), "{:?}", engine.last_error_message());
        let probes = engine
            .resource_request_records()
            .iter()
            .filter(|record| record.mode == FILE_SIZE_PROBE_MODE)
            .map(|record| (record.name.as_str(), record.byte_len))
            .collect::<Vec<_>>();
        assert_eq!(probes, [("missing.tex", None), ("present.tex", Some(5))]);
    }

    #[test]
    fn a_line_longer_than_the_buffer_overflows() {
        let mut engine = initialized_engine();
        let input = format!("\\relax\n\\setbox0\\hbox{{{}}}\\end\n", "a".repeat(BUF_SIZE as usize + 100));
        assert!(engine.begin_primary_input("input.tex", input.into_bytes()));
        assert!(!engine.run_main_control());
        // `overflow` opens the log, so `succumb` reports through `error` before stopping.
        let message = engine.last_error_message().unwrap_or_default();
        assert!(message.contains("buffer size=200000"), "{message}");
    }

    #[test]
    fn vsplit_prunes_the_top_of_the_remainder() {
        let mut engine = initialized_engine();
        let input = concat!(
            "\\splittopskip=10pt \\savingvdiscards=1 ",
            "\\setbox1\\vbox{\\hbox{}\\penalty-10000 \\kern5pt\\hbox{}}",
            "\\setbox2\\vsplit1 to 0pt \\setbox3\\vbox{\\splitdiscards}",
            "\\hbox{\\box1\\box3}\\end",
        );
        let root = captured_root(&mut engine, input);
        let boxes = list_snapshots(&engine, root.list);
        let kinds = |node: &PortableNodeSnapshot| {
            list_snapshots(&engine, node.list)
                .iter()
                .map(|node| (node.kind, node.width))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            kinds(&boxes[0]),
            [(PortableNodeKind::Glue, 10 * 65536), (PortableNodeKind::HorizontalBox, 0)]
        );
        // The zero lineskip glue above the second box is discarded with the penalty and kern.
        assert_eq!(
            kinds(&boxes[1]),
            [
                (PortableNodeKind::Penalty, 0),
                (PortableNodeKind::Kern, 5 * 65536),
                (PortableNodeKind::Glue, 0),
            ]
        );
    }

    #[test]
    fn native_char_metrics_come_from_the_font_platform() {
        let mut engine = initialized_engine().with_font_platform(MockNativeFonts);
        // The mock gives `d`, 100, a 100pt advance, 3pt height, 2pt depth and 1pt overhang, `D2Fix` truncates.
        let input = concat!(
            "\\font\\x=\"mock\" \\x ",
            "\\ifdim\\fontcharwd\\x`d=100pt \\else\\errmessage{width}\\fi",
            "\\ifdim\\fontcharht\\x`d=3pt \\else\\errmessage{height}\\fi",
            "\\ifdim\\fontchardp\\x`d=2pt \\else\\errmessage{depth}\\fi",
            "\\ifdim\\fontcharic\\x`d=65535sp \\else\\errmessage{italic correction}\\fi",
            "\\ifnum\\XeTeXfirstfontchar\\x=32 \\else\\errmessage{first char}\\fi",
            "\\ifnum\\XeTeXlastfontchar\\x=126 \\else\\errmessage{last char}\\fi",
            "\\ifdim\\XeTeXglyphbounds1 100=-32767sp \\else\\errmessage{left bearing}\\fi",
            "\\ifdim\\XeTeXglyphbounds3 100=-65535sp \\else\\errmessage{right bearing}\\fi",
            // The mock shapes a word one point per character and measures a lone glyph as empty.
            "\\setbox0\\hbox{d\\/}\\ifdim\\wd0=2pt \\else\\errmessage{word correction}\\fi",
            "\\setbox0\\hbox{\\XeTeXglyph100\\/}\\ifdim\\wd0=1pt \\else\\errmessage{glyph correction}\\fi\\end",
        );
        assert!(run_initex(&mut engine, input), "{:?}", engine.last_error_message());
    }

    #[test]
    fn snapshots_expose_leaders_unset_boxes_and_noad_kinds() {
        let mut engine = initialized_engine();
        let root = captured_root(&mut engine, "\\hbox{\\leaders\\hrule\\hskip 10pt}\\end");
        let glue = list_snapshots(&engine, root.list).remove(0);
        assert_eq!((glue.kind, glue.width), (PortableNodeKind::Glue, 10 * 65536));
        let leader = engine.snapshot_node(glue.leader.expect("a leader")).expect("leader node");
        assert_eq!(leader.kind, PortableNodeKind::Rule);
        let node = unsafe { engine.zgetnode(7) }.expect("allocates");
        let slot = (node - engine.state.memmin) as usize;
        let words = &mut engine.heap.mem[slot..slot + 7];
        words[0].hh.u.B0 = UNSET_NODE as i16;
        words[1].u.CINT = 11;
        words[4].u.CINT = 22;
        words[5].hh.u.B0 = 1;
        words[5].hh.u.B1 = 2;
        words[6].u.CINT = 33;
        let unset = engine.snapshot_node(PortableNodeHandle(node)).expect("unset node");
        assert_eq!(
            (unset.width, unset.shift, unset.glue_set, unset.glue_stretch, unset.glue_shrink),
            (11, 0, 0.0, 33, 22)
        );
        assert_eq!((unset.glue_stretch_order, unset.glue_shrink_order), (2, 1));
        for (noad_type, kind) in [(ORD_NOAD, PortableNoadKind::Ord), (FRACTION_NOAD, PortableNoadKind::Fraction), (RIGHT_NOAD, PortableNoadKind::Right)] {
            engine.heap.mem[slot].hh.u.B0 = noad_type as i16;
            let noad = engine.snapshot_node(PortableNodeHandle(node)).expect("noad");
            assert_eq!(noad.kind, PortableNodeKind::Noad(kind));
        }
    }
}

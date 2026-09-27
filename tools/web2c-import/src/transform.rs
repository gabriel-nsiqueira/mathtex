//! Lexical rewrites and the per function patch registry applied to translated XeTeX functions.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::model::{PatchError, PatchMismatch, RawFunction};
use crate::select::source_hash;

/// One literal replacement inside a function body, which must match exactly `count` times.
pub(crate) struct Edit {
    pub(crate) anchor: &'static str,
    pub(crate) replacement: &'static str,
    pub(crate) count: usize,
}

/// The ordered edits applied to one translated function.
struct FunctionEdits {
    function: &'static str,
    edits: &'static [Edit],
}

/// A whole function replacement, pinned to the upstream XeTeX body it replaces.
struct Template {
    function: &'static str,
    file: &'static str,
    text: &'static str,
    /// `source_hash` of the raw upstream C2Rust function, so an upstream change fails loudly.
    upstream_hash: u64,
}

/// Function whose `\futurelet` lookahead span patch is structural rather than a literal edit.
const FUTURELET_FUNCTION: &str = "prefixedcommand";

/// Function holding the `native_room` expansions, which grow `native_text` through `xrealloc`.
const NATIVE_ROOM_FUNCTION: &str = "maincontrol";

/// `native_room` expansions in `main_control`, each formatted differently by `c2rust`.
const NATIVE_ROOM_SITES: usize = 4;

/// Functions that test or store XeTeX's native glyph info pointer in memory word 5, with their site counts.
const NATIVE_GLYPH_WORD_SITES: &[(&str, usize)] = &[
    ("zcharpw", 1),
    ("zcopynodelist", 1),
    // The test, the `free` and the store that clears the word.
    ("zflushnodelist", 3),
    ("zjustcopy", 6),
    ("znewnativewordnode", 1),
];

/// Tracks every patch application so misses surface as one error after the whole run.
pub(crate) struct PatchSession {
    mismatches: Vec<PatchMismatch>,
    visited: BTreeSet<String>,
    dump: BTreeSet<String>,
}

impl PatchSession {
    /// Creates a session, reading `PATCHER_DUMP=fn1,fn2` to dump those stage sources.
    pub(crate) fn new() -> Self {
        let dump = std::env::var("PATCHER_DUMP")
            .map(|names| {
                names
                    .split(',')
                    .map(|name| name.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            mismatches: Vec::new(),
            visited: BTreeSet::new(),
            dump,
        }
    }

    /// Applies the registered template or edits for `function` to its lexically patched source.
    pub(crate) fn apply(&mut self, function: &RawFunction, mut source: String) -> String {
        let name = function.name.as_str();
        self.visited.insert(name.to_string());
        if self.dump.contains(name) {
            let path = dump_stage(name, &source);
            eprintln!(
                "patch_engine: dumped {name} stage source to {}",
                path.display()
            );
        }

        if let Some(template) = TEMPLATES.iter().find(|template| template.function == name) {
            let found = source_hash(&function.source);
            if found != template.upstream_hash {
                self.mismatch(
                    name,
                    format!(
                        "upstream body changed under template {} (hash {found:016x}, pinned {:016x})",
                        template.file, template.upstream_hash
                    ),
                );
            }
            return template.text.replace("{function_name}", name);
        }

        if let Some(entry) = FUNCTION_EDITS.iter().find(|entry| entry.function == name) {
            for edit in entry.edits {
                self.edit(&mut source, name, edit);
            }
        }
        if name == FUTURELET_FUNCTION {
            match patch_futurelet_lookahead_span(&source) {
                Some(patched) => source = patched,
                None => self.mismatch(
                    name,
                    "`\\futurelet` lookahead span markers not found".into(),
                ),
            }
        }
        if name == NATIVE_ROOM_FUNCTION {
            let (patched, sites) = patch_native_room_growth(&source);
            if sites != NATIVE_ROOM_SITES {
                self.mismatch(
                    name,
                    format!(
                        "rewrote {sites} `native_room` growth sites, expected {NATIVE_ROOM_SITES}"
                    ),
                );
            }
            source = patched;
        }
        let expected = NATIVE_GLYPH_WORD_SITES
            .iter()
            .find(|(function, _)| *function == name)
            .map_or(0, |&(_, sites)| sites);
        if expected > 0 || mentions_ptr_field(&source) {
            let (patched, sites, leftover) = crate::astpass::rewrite_native_glyph_words(&source);
            if sites != expected || leftover != 0 {
                self.mismatch(
                    name,
                    format!(
                        "rewrote {sites} native glyph word sites, expected {expected}, {leftover} `.ptr` uses left"
                    ),
                );
            }
            source = patched;
        }
        source
    }

    /// Replaces `edit.anchor` in `source`, recording a mismatch unless it matches `edit.count` times.
    pub(crate) fn edit(&mut self, source: &mut String, function: &str, edit: &Edit) {
        let found = source.matches(edit.anchor).count();
        if found == edit.count {
            *source = source.replace(edit.anchor, edit.replacement);
            return;
        }
        let path = dump_stage(function, source);
        self.mismatch(
            function,
            format!(
                "anchor matched {found} times, expected {}, stage source in {}\n    anchor: {:?}",
                edit.count,
                path.display(),
                edit.anchor
            ),
        );
    }

    fn mismatch(&mut self, function: &str, detail: String) {
        self.mismatches.push(PatchMismatch {
            function: function.to_string(),
            detail,
        });
    }

    /// Fails with every recorded mismatch, including registered patches whose function was never seen.
    pub(crate) fn finish(mut self) -> Result<(), PatchError> {
        let registered = FUNCTION_EDITS
            .iter()
            .map(|entry| entry.function)
            .chain(TEMPLATES.iter().map(|template| template.function))
            .chain([FUTURELET_FUNCTION, NATIVE_ROOM_FUNCTION])
            .chain(
                NATIVE_GLYPH_WORD_SITES
                    .iter()
                    .map(|&(function, _)| function),
            );
        for function in registered {
            if !self.visited.contains(function) {
                self.mismatch(
                    function,
                    "function was not selected, its patch never applied".into(),
                );
            }
        }
        if self.mismatches.is_empty() {
            Ok(())
        } else {
            Err(PatchError::PatchMismatches(self.mismatches))
        }
    }
}

fn dump_stage(function: &str, source: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("patcher-stage-{function}.rs"));
    if let Err(error) = std::fs::write(&path, source) {
        eprintln!("patch_engine: cannot write {}: {error}", path.display());
    }
    path
}

/// Builds a [`Template`] whose text is read from `file` under the crate root.
macro_rules! template {
    ($function:literal, $file:literal, $upstream_hash:literal) => {
        Template {
            function: $function,
            file: $file,
            text: include_str!(concat!("../", $file)),
            upstream_hash: $upstream_hash,
        }
    };
}

const TEMPLATES: &[Template] = &[
    template!(
        "appendsrcspecial",
        "patches/appendsrcspecial_or_insertsrcspecial.rs.in",
        0x8fc58490f1078c23
    ),
    template!(
        "insertsrcspecial",
        "patches/appendsrcspecial_or_insertsrcspecial.rs.in",
        0x23d6f7e65dfe6220
    ),
    template!(
        "zprintwritewhatsit",
        "patches/zprintwritewhatsit.rs.in",
        0xe43e12ccee45e5d1
    ),
    template!(
        "scanpdfexttoks",
        "patches/scanpdfexttoks.rs.in",
        0xe528ead645cf5fe0
    ),
    template!("zpdferror", "patches/zpdferror.rs.in", 0x39d6700f852910ba),
    template!(
        "openorclosein",
        "patches/openorclosein.rs.in",
        0x06282e478408de6d
    ),
    template!("startinput", "patches/startinput.rs.in", 0x98d3f2b4cff64893),
    template!(
        "zpackfilename",
        "patches/zpackfilename.rs.in",
        0xe92cfd2cf805312a
    ),
    template!("ztrienode", "patches/ztrienode.rs.in", 0x2e1641affeb9d606),
    template!(
        "zcopynativeglyphinfo",
        "patches/zcopynativeglyphinfo.rs.in",
        0xdaf13b15fa27b4cc
    ),
    template!(
        "znewnativecharacter",
        "patches/znewnativecharacter.rs.in",
        0xcd117fee4727284c
    ),
];

/// Returns true when `function` is replaced wholesale by a template.
pub(crate) fn is_template_function(function: &str) -> bool {
    template_text(function).is_some()
}

/// The template that replaces `function`, if any.
pub(crate) fn template_text(function: &str) -> Option<&'static str> {
    TEMPLATES
        .iter()
        .find(|template| template.function == function)
        .map(|template| template.text)
}

const FUNCTION_EDITS: &[FunctionEdits] = &[
    FunctionEdits {
        function: "zloadnativefont",
        edits: &[
            // The portable boundary returns a `FontHandle`, not a layout engine pointer.
            Edit {
                anchor: "let mut fontengine: voidpointer = ::core::ptr::null_mut::<()>();",
                replacement: "let mut fontengine: FontHandle = 0;",
                count: 1,
            },
            // Set `nativefonttypeflag` inline because the portable boundary cannot.
            Edit {
                anchor: "    ) as voidpointer;\n    if !fontengine.is_null() {",
                replacement: "    );\n    if fontengine != 0 {\n        self.state.nativefonttypeflag = OTGR_FONT_FLAG;",
                count: 1,
            },
            Edit {
                anchor: "*fresh88 = fontengine;",
                replacement: "*fresh88 = fontengine as voidpointer;",
                count: 1,
            },
            Edit {
                anchor: "fontengine as *mut ()",
                replacement: "fontengine",
                count: 2,
            },
            Edit {
                anchor: "*fresh91 = self.state.loadedfontmapping;",
                replacement: "*fresh91 = ::core::ptr::null_mut::<()>();",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "getstringsstarted",
        edits: &[
            Edit {
                anchor: "loadpoolstrings(self.state.poolsize - self.state.stringvacancies)",
                replacement: "Self::load_pool_strings(self, self.state.poolsize - self.state.stringvacancies)",
                count: 1,
            },
            Edit {
                anchor: "    if g == 0 as i32 {\n        fprintf(\n            self.state.terminal_output,\n            b\"%s\\n\\0\" as *const u8 as *const i8,\n            b\"! You have to increase POOLSIZE.\\0\" as *const u8 as *const i8,\n        );\n        Result = false_0 as boolean;\n        return Result;\n    }\n",
                replacement: "    if g == 0 as i32 {\n        Result = false_0 as boolean;\n        return Result;\n    }\n",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "initialize",
        edits: &[
            Edit {
                anchor: "    self.state.nativetextsize = 128 as i32 as integer;\n    self.state.nativetext = xmalloc(\n        (self.state.nativetextsize as size_t).wrapping_mul(::core::mem::size_of::<UTF16code>() as size_t),\n    ) as *mut UTF16code;\n",
                replacement: "    self.state.nativetextsize = 128 as i32 as integer;\n    Self::ensure_nativetext_capacity(self, self.state.nativetextsize);\n",
                count: 1,
            },
            Edit {
                anchor: "    initstarttime();\n",
                replacement: "",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zprintnl",
        edits: &[
            // `print_err` prints its `! ` prefix through `print_nl`, the message that follows is captured.
            Edit {
                anchor: "    (&mut *(self as *mut PortableTexEngine<'_>)).zprint(s);\n}",
                replacement: "    (&mut *(self as *mut PortableTexEngine<'_>)).zprint(s);\n    Self::begin_error_message(self, s);\n}",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "printfileline",
        edits: &[
            // In file line error style `print_err` starts the message after the `file:line: ` prefix.
            Edit {
                anchor: "        (&mut *(self as *mut PortableTexEngine<'_>)).zprint(65593 as i64 as integer);\n    };",
                replacement: "        (&mut *(self as *mut PortableTexEngine<'_>)).zprint(65593 as i64 as integer);\n        Self::begin_file_line_error_message(self);\n    };",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zprintrawchar",
        edits: &[
            // Every byte printed while an error message is open belongs to the message.
            Edit {
                anchor: "pub(crate) unsafe fn zprintrawchar(&mut self, mut s: UnicodeScalar, mut incroffset: boolean) {",
                replacement: "pub(crate) unsafe fn zprintrawchar(&mut self, mut s: UnicodeScalar, mut incroffset: boolean) {\n    Self::capture_error_byte(self, s);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zprintchar",
        edits: &[
            // `\newlinechar` in a message is a line break the message keeps, unlike a wrap at `max_print_line`.
            Edit {
                anchor: "        if (self.state.selector as i32) < 20 as i32 {\n            (&mut *(self as *mut PortableTexEngine<'_>)).println();",
                replacement: "        if (self.state.selector as i32) < 20 as i32 {\n            Self::capture_error_byte(self, b'\\n' as UnicodeScalar);\n            (&mut *(self as *mut PortableTexEngine<'_>)).println();",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "error",
        edits: &[
            // The message is complete when `error` starts, before its period and the context lines.
            Edit {
                anchor: "pub(crate) unsafe fn error(&mut self) {",
                replacement: "pub(crate) unsafe fn error(&mut self) {\n    Self::end_error_message(self);",
                count: 1,
            },
            // Surface TeX errors after `show_context` has written the transcript message.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).showcontext();\n    if self.state.haltonerrorp != 0 {",
                replacement: "(&mut *(self as *mut PortableTexEngine<'_>)).showcontext();\n    Self::surface_error(self as *mut PortableTexEngine<'_>);\n    if self.state.haltonerrorp != 0 {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "initmath",
        edits: &[
            // Reject a second math entry as a sandbox escape from the wrapper math.
            Edit {
                anchor: "pub(crate) unsafe fn initmath(&mut self) {",
                replacement: "pub(crate) unsafe fn initmath(&mut self) {\n        Self::sandbox_open_math(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "aftermath",
        edits: &[
            // Mark wrapper math closed so the next `init_math` is treated as a user breakout.
            Edit {
                anchor: "pub(crate) unsafe fn aftermath(&mut self) {",
                replacement: "pub(crate) unsafe fn aftermath(&mut self) {\n        Self::sandbox_close_math(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "getavail",
        edits: &[
            // Source tracking hook 3 stamps allocated cells with the ambient span.
            Edit {
                anchor: "self.state.dynused += 1;\n    Result = p;",
                replacement: "self.state.dynused += 1;\n    Self::src_stamp_avail(self as *mut PortableTexEngine<'_>, p);\n    Result = p;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zcharbox",
        edits: &[
            // `char_box` bypasses `new_character`, so stamp glyphs with the construct span here.
            Edit {
                anchor: "Result = b;\n    return Result;",
                replacement: "Self::src_stamp_char(self as *mut PortableTexEngine<'_>, p);\n    Result = b;\n    return Result;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zgetnode",
        edits: &[
            // Source tracking hook 4 stamps each variable size node range with the construct span.
            Edit {
                anchor: "self.state.varused = self.state.varused + s;\n            Result = r as halfword;",
                replacement: "self.state.varused = self.state.varused + s;\n            Self::src_stamp_node_range(self as *mut PortableTexEngine<'_>, r as halfword, s);\n            Result = r as halfword;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "znewcharacter",
        edits: &[
            // Source tracking hook 5 overwrites the provisional TFM glyph stamp.
            Edit {
                anchor: "(*mem.offset(p as isize)).hh.u.B1 = c_0 as i16;\n                Result = p;",
                replacement: "(*mem.offset(p as isize)).hh.u.B1 = c_0 as i16;\n                Self::src_stamp_char(self as *mut PortableTexEngine<'_>, p);\n                Result = p;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "mlisttohlist",
        edits: &[
            // Source tracking hook 6 repoints `cmd_span` to noad `q` before geometry is synthesized.
            Edit {
                anchor: "while q as i64 != -(268435455 as i64) {\n        loop {\n            delta_0 = 0 as i32 as scaled;",
                replacement: "while q as i64 != -(268435455 as i64) {\n        Self::src_mlist_repoint(self as *mut PortableTexEngine<'_>, q);\n        loop {\n            delta_0 = 0 as i32 as scaled;",
                count: 1,
            },
            // Host box hook: resolve pending \Uhostbox placeholders after each reswitch, where cur_style is known.
            Edit {
                anchor: "Self::src_mlist_repoint(self as *mut PortableTexEngine<'_>, q);\n        loop {",
                replacement: "Self::src_mlist_repoint(self as *mut PortableTexEngine<'_>, q);\n        loop {\n            Self::host_box_resolve_noad(self as *mut PortableTexEngine<'_>, q);",
                count: 1,
            },
            // Carry a field leaf char span onto the glyph node after `make_ord` attaches it.
            Edit {
                anchor: "(*mem.offset((q as i32 + 1 as i32) as isize))\n                    .u\n                    .CINT = p as integer;",
                replacement: "(*mem.offset((q as i32 + 1 as i32) as isize))\n                    .u\n                    .CINT = p as integer;\n                Self::src_carry_nucleus(self as *mut PortableTexEngine<'_>, q, p);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zcleanbox",
        edits: &[
            // Source tracking hook 7 preserves `cmd_span` across recursive sub box cleanup.
            Edit {
                anchor: "pub(crate) unsafe fn zcleanbox(&mut self, mut p: halfword, mut s: smallnumber) -> halfword {",
                replacement: "pub(crate) unsafe fn zcleanbox(&mut self, mut p: halfword, mut s: smallnumber) -> halfword {\n        let __src_saved_cmd_span: u32 = Self::src_save_cmd_span(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Host box hook: fields copied out of single atom groups resolve here with style `s`.
            Edit {
                anchor: "let __src_saved_cmd_span: u32 = Self::src_save_cmd_span(self as *mut PortableTexEngine<'_>);",
                replacement: "let __src_saved_cmd_span: u32 = Self::src_save_cmd_span(self as *mut PortableTexEngine<'_>);\n        Self::host_box_resolve_field(self as *mut PortableTexEngine<'_>, p, s as i32);",
                count: 1,
            },
            Edit {
                anchor: "Result = x;\n    return Result;",
                replacement: "Self::src_restore_cmd_span(self as *mut PortableTexEngine<'_>, __src_saved_cmd_span);\n    Result = x;\n    return Result;",
                count: 1,
            },
            // Carry a single math char field span onto the noad that `clean_box` builds.
            Edit {
                anchor: "*mem.offset((self.state.curmlist as i32 + 1 as i32) as isize) =\n                *mem.offset(p as isize);",
                replacement: "*mem.offset((self.state.curmlist as i32 + 1 as i32) as isize) =\n                *mem.offset(p as isize);\n            Self::src_carry_field(self as *mut PortableTexEngine<'_>, p, self.state.curmlist);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zscanmath",
        edits: &[
            // Record direct math char field spans so `clean_box` can carry them later.
            Edit {
                anchor: "(*mem.offset(p as isize)).hh.v.RH = 1 as i32 as halfword;",
                replacement: "(*mem.offset(p as isize)).hh.v.RH = 1 as i32 as halfword;\n    Self::src_stamp_field(self as *mut PortableTexEngine<'_>, p);",
                count: 1,
            },
            // Push a pending construct frame for a `{` delimited math field argument.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).zpushmath(9 as i32 as groupcode);",
                replacement: "(&mut *(self as *mut PortableTexEngine<'_>)).zpushmath(9 as i32 as groupcode);\n                        Self::src_scan_math_group_open(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "endtokenlist",
        edits: &[
            // Source tracking hook 8b pops the construct frame for a macro body level.
            Edit {
                anchor: "pub(crate) unsafe fn endtokenlist(&mut self) {",
                replacement: "pub(crate) unsafe fn endtokenlist(&mut self) {\n        Self::src_end_token_list(self as *mut PortableTexEngine<'_>, self.state.curinput.indexfield as i32);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "handlerightbrace",
        edits: &[
            // Carry a collapsed braced script noad span onto its saved field.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).zfreenode(p, 4 as i32);",
                replacement: "Self::src_carry_collapse(self as *mut PortableTexEngine<'_>, p, (*self.state.savestack.offset((self.state.saveptr as i32 + 0 as i32) as isize)).u.CINT);\n                                (&mut *(self as *mut PortableTexEngine<'_>)).zfreenode(p, 4 as i32);",
                count: 1,
            },
            // Pop the math group frame after extending it through the closing brace.
            Edit {
                anchor: "9 => {\n            (&mut *(self as *mut PortableTexEngine<'_>)).unsave();\n            self.state.saveptr -= 1;",
                replacement: "9 => {\n            Self::src_scan_math_group_close(self as *mut PortableTexEngine<'_>);\n            (&mut *(self as *mut PortableTexEngine<'_>)).unsave();\n            self.state.saveptr -= 1;",
                count: 1,
            },
            // Extend construct noads through the closing brace after the nucleus group is filled.
            Edit {
                anchor: "            .v\n            .LH = p;\n            if p as i64 != -(268435455 as i64) {",
                replacement: "            .v\n            .LH = p;\n            Self::src_construct_extend_to_loc(self as *mut PortableTexEngine<'_>);\n            if p as i64 != -(268435455 as i64) {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zbegintokenlist",
        edits: &[
            // Source tracking hook 8 sets the new token list baseline after the input stack push.
            Edit {
                anchor: "*self.state.inputstack.offset(self.state.inputptr as isize) = self.state.curinput;\n    self.state.inputptr += 1;",
                replacement: "*self.state.inputstack.offset(self.state.inputptr as isize) = self.state.curinput;\n    self.state.inputptr += 1;\n    Self::src_begin_token_list(self as *mut PortableTexEngine<'_>, t);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "macrocall",
        edits: &[
            // An expansion loop never returns to `main_control`, so each macro call spends sandbox budget.
            Edit {
                anchor: "pub(crate) unsafe fn macrocall(&mut self) {",
                replacement: "pub(crate) unsafe fn macrocall(&mut self) {\n    Self::sandbox_tick(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Source tracking hook 9 records the macro invocation span before body entry.
            Edit {
                anchor: "self.state.warningindex = self.state.curcs;",
                replacement: "self.state.warningindex = self.state.curcs;\n    Self::src_macro_begin(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Hook 3b stamps argument token cells that bypass `get_avail`.
            Edit {
                anchor: "(*mem.offset(q as isize)).hh.v.LH = self.state.curtok;",
                replacement: "(*mem.offset(q as isize)).hh.v.LH = self.state.curtok;\n                                    Self::src_stamp_avail(self as *mut PortableTexEngine<'_>, q);",
                count: 3,
            },
            // Mark the pending macro before its body token list is pushed.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 6 as i32 as quarterword);",
                replacement: "Self::src_macro_set_pending(self as *mut PortableTexEngine<'_>, n);\n            (&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 6 as i32 as quarterword);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "expand",
        edits: &[
            // Every primitive expansion spends sandbox budget too.
            Edit {
                anchor: "pub(crate) unsafe fn expand(&mut self) {",
                replacement: "pub(crate) unsafe fn expand(&mut self) {\n    Self::sandbox_tick(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zreadtoks",
        edits: &[
            // A sandboxed fragment reads nothing, from a file or the terminal.
            Edit {
                anchor: "pub(crate) unsafe fn zreadtoks(&mut self, mut n: integer, mut r: halfword, mut j: halfword) {",
                replacement: "pub(crate) unsafe fn zreadtoks(&mut self, mut n: integer, mut r: halfword, mut j: halfword) {\n    Self::sandbox_reject(self as *mut PortableTexEngine<'_>, \"\\\\read is not allowed inside a math expression\");",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "mathradical",
        edits: &[
            // Anchor radical noads to the user command before delimiter or nucleus scanning.
            Edit {
                anchor: "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 24 as i16;",
                replacement: "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 24 as i16;\n    Self::src_construct_anchor(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "mathac",
        edits: &[
            // Anchor accent noads to the user command before nucleus scanning.
            Edit {
                anchor: "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 28 as i16;",
                replacement: "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 28 as i16;\n    Self::src_construct_anchor(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Extend unbraced accent noads after `scan_math` fills the nucleus.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).zscanmath(self.state.curlist.tailfield as i32 + 1 as i32);",
                replacement: "let src_acc = self.state.curlist.tailfield;\n    (&mut *(self as *mut PortableTexEngine<'_>)).zscanmath(self.state.curlist.tailfield as i32 + 1 as i32);\n    Self::src_construct_extent(self as *mut PortableTexEngine<'_>, src_acc, 0 as u32);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zcopynodelist",
        edits: &[
            // Carry source spans onto copied nodes before linking them into the new list.
            Edit {
                anchor: "(*mem.offset(q as isize)).hh.v.RH = r;\n        q = r;\n        p = (*mem.offset(p as isize)).hh.v.RH;",
                replacement: "Self::src_carry_copy(self as *mut PortableTexEngine<'_>, p, r);\n        (*mem.offset(q as isize)).hh.v.RH = r;\n        q = r;\n        p = (*mem.offset(p as isize)).hh.v.RH;",
                count: 1,
            },
            // Host box markers copy as five word nodes, metrics plus payload.
            Edit {
                anchor: "2 | 5 => {",
                replacement: "HOST_BOX_PENDING_SUBTYPE | HOST_BOX_RESOLVED_SUBTYPE => {\n    r = (&mut *(self as *mut PortableTexEngine<'_>)).zgetnode(HOST_BOX_MARKER_SIZE);\n    words = HOST_BOX_MARKER_SIZE as eightbits;\n}\n2 | 5 => {",
                count: 1,
            },
            // A copied glyph node keeps its glyph in the side table, the five word node has no info pointer.
            Edit {
                anchor: "42 => {\n                        r = (&mut *(self as *mut PortableTexEngine<'_>)).zgetnode(5 as i32);\n                        words = 5 as eightbits;",
                replacement: "42 => {\n                        r = (&mut *(self as *mut PortableTexEngine<'_>)).zgetnode(5 as i32);\n                        Self::copy_native_glyph_info(self, p, r);\n                        words = 5 as eightbits;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zfreenode",
        edits: &[
            // Every freed node leaves the native glyph side table, so a reused address never inherits glyphs.
            Edit {
                anchor: "pub(crate) unsafe fn zfreenode(&mut self, mut p: halfword, mut s: halfword) {",
                replacement: "pub(crate) unsafe fn zfreenode(&mut self, mut p: halfword, mut s: halfword) {\n    Self::release_native_glyph_info(self, p);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zsrctokencopy",
        edits: &[
            // Replace the WEB `src_token_copy` marker with a real token span carry.
            Edit {
                anchor: "dest == src;",
                replacement: "Self::src_carry_token_span(self as *mut PortableTexEngine<'_>, dest, src);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "backinput",
        edits: &[
            // Source tracking hook 1d stamps backed up tokens with their own origin.
            Edit {
                anchor: "(*mem.offset(p as isize)).hh.v.LH = self.state.curtok;",
                replacement: "(*mem.offset(p as isize)).hh.v.LH = self.state.curtok;\n        Self::src_back_input_stamp(self as *mut PortableTexEngine<'_>, p);",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "mathleftright",
        edits: &[
            // Track `\left` and `\right` as one consumed source extent outside `scan_math`.
            Edit {
                anchor: ".zscandelimiter(\n            p as i32 + 1 as i32,\n            0 as i32,\n        );\n        if t as i32 == 1 as i32 {",
                replacement: ".zscandelimiter(\n            p as i32 + 1 as i32,\n            0 as i32,\n        );\n        Self::src_leftright(self as *mut PortableTexEngine<'_>, p, t as i32);\n        if t as i32 == 1 as i32 {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zboxend",
        edits: &[
            // Hand a finished fragment box to the host instead of placing it.
            Edit {
                anchor: "(*mem.offset((self.state.curbox as i32 + 4 as i32) as isize))\n                .u\n                .CINT = boxcontext;",
                replacement: "(*mem.offset((self.state.curbox as i32 + 4 as i32) as isize))\n                .u\n                .CINT = boxcontext;\n            if Self::boundary_capture_fragment_box(\n                self as *mut PortableTexEngine<'_>,\n                self.state.curbox,\n                self.state.curlist.modefield as integer,\n                boxcontext,\n            ) != 0\n            {\n                return;\n            }",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "doextension",
        edits: &[
            // A sandboxed fragment opens no output file.
            Edit {
                anchor: "        0 => {\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewwritewhatsit(3 as i32 as smallnumber);",
                replacement: "        0 => {\n            Self::sandbox_reject(self as *mut PortableTexEngine<'_>, \"\\\\openout is not allowed inside a math expression\");\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewwritewhatsit(3 as i32 as smallnumber);",
                count: 1,
            },
            // A sandboxed `\write`, immediate or not, may not name a stream open to a file.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).znewwritewhatsit(2 as i32 as smallnumber);\n            self.state.curcs = k as halfword;",
                replacement: "(&mut *(self as *mut PortableTexEngine<'_>)).znewwritewhatsit(2 as i32 as smallnumber);\n            Self::sandbox_check_write_stream(self as *mut PortableTexEngine<'_>);\n            self.state.curcs = k as halfword;",
                count: 1,
            },
            // Host box hook: \Uhostbox scans its token id.
            Edit {
                anchor: "        46 => {",
                replacement: "        HOST_BOX_EXTENSION_CODE => {\n            (&mut *(self as *mut PortableTexEngine<'_>)).scanint();\n            Self::host_box_insert(\n                self as *mut PortableTexEngine<'_>,\n                self.state.curval,\n            );\n        }\n        46 => {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zflushnodelist",
        edits: &[
            // Host box markers free as five word nodes, metrics plus payload.
            Edit {
                anchor: "2 | 5 => {",
                replacement: "HOST_BOX_PENDING_SUBTYPE | HOST_BOX_RESOLVED_SUBTYPE => {\n    (&mut *(self as *mut PortableTexEngine<'_>)).zfreenode(p, HOST_BOX_MARKER_SIZE);\n}\n2 | 5 => {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zhpack",
        edits: &[
            // Host box markers measure like glyph nodes in hpack so rebox repacks keep their width.
            Edit {
                anchor: "42 | 43 | 44 => {",
                replacement: "42 | 43 | 44 | HOST_BOX_PENDING_SUBTYPE | HOST_BOX_RESOLVED_SUBTYPE => {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zlinebreak",
        edits: &[
            // Format initialization never hyphenates, so it must not pack the trie early.
            Edit {
                anchor: "if self.state.trienotready != 0 {",
                replacement: "if self.state.trienotready != 0 && !self.format_initialization {",
                count: 1,
            },
            // Format initialization discards paragraphs instead of breaking them.
            Edit {
                anchor: "let mut mem: *mut memoryword = self.state.zmem;",
                replacement: "let mut mem: *mut memoryword = self.state.zmem;\n    if self.format_initialization {\n        let paragraph = (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH;\n        if paragraph != TEX_NULL {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(paragraph);\n        }\n        if self.state.nestptr > 0 {\n            (&mut *(self as *mut PortableTexEngine<'_>)).popnest();\n        } else {\n            (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH = TEX_NULL;\n            self.state.curlist.tailfield = self.state.curlist.headfield;\n        }\n        (&mut *(self as *mut PortableTexEngine<'_>)).normalparagraph();\n        self.state.packbeginline = 0 as integer;\n        return;\n    }",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "inittrie",
        edits: &[
            // The trie is packed lazily after format initialization.
            Edit {
                anchor: "pub(crate) unsafe fn inittrie(&mut self) {",
                replacement: "pub(crate) unsafe fn inittrie(&mut self) {\n    if self.format_initialization {\n        return;\n    }",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "newpatterns",
        edits: &[
            // Format initialization discards material left in the current list by `\patterns`.
            Edit {
                anchor: "let mut mem: *mut memoryword = self.state.zmem;",
                replacement: "let mut mem: *mut memoryword = self.state.zmem;\n    if self.format_initialization {\n        let list = (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH;\n        if list != TEX_NULL {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(list);\n        }\n        (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH = TEX_NULL;\n        self.state.curlist.tailfield = self.state.curlist.headfield;\n    }",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "maincontrol",
        edits: &[
            // Tick the sandbox budget once per main control iteration.
            Edit {
                anchor: "'_lab60: loop {\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
                replacement: "'_lab60: loop {\n        Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
                count: 1,
            },
            // Source tracking hook 2 freezes the command token span before argument scanning.
            Edit {
                anchor: "Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
                replacement: "Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();\n        Self::src_latch_cmd_span(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Once the wrapper math closes, the sandbox lets only the wrapper's own closing tokens run.
            Edit {
                anchor: "Self::src_latch_cmd_span(self as *mut PortableTexEngine<'_>);",
                replacement: "Self::src_latch_cmd_span(self as *mut PortableTexEngine<'_>);\n        Self::sandbox_check_command(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Source tracking hook 11 records a span for each XeTeX native text code unit.
            Edit {
                anchor: "self.state.ishyph = (self.state.curchr",
                replacement: "Self::src_native_run_push(self as *mut PortableTexEngine<'_>);\n                        self.state.ishyph = (self.state.curchr",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "itsallover",
        edits: &[
            // `build_page` is stripped, so pending page material is flushed, the upstream eject and retry never ends.
            Edit {
                anchor: "(&mut *(self as *mut PortableTexEngine<'_>)).backinput();\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH = (&mut *(self as *mut PortableTexEngine<'_>)).newnullbox();\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        (*mem.offset(\n            (self.state.curlist.tailfield as i32 + 1 as i32) as isize,\n        ))\n        .u\n        .CINT = (*eqtb.offset(9006722 as i64 as isize))\n            .u\n            .CINT;\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH =\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewglue(self.state.membot as i32 + 8 as i32);\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH =\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewpenalty(-(1073741824 as i64) as integer);\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        Self::boundary_build_page();",
                replacement: "let head = self.state.curlist.headfield;\n        let first = (*mem.offset(head as isize)).hh.v.RH;\n        (*mem.offset(head as isize)).hh.v.RH = TEX_NULL;\n        self.state.curlist.tailfield = head;\n        if first != TEX_NULL {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(first);\n        }\n        Result = true_0 as boolean;\n        return Result;",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "zreadtoks",
        edits: &[
            // Format initialization reads an empty line instead of prompting the terminal.
            Edit {
                anchor: "if self.state.readopen[m as usize] as i32 == 2 as i32 {\n            if self.state.interaction as i32 > 1 as i32 {",
                replacement: "if self.state.readopen[m as usize] as i32 == 2 as i32 {\n            if self.format_initialization {\n                self.state.last = self.state.first;\n            } else if self.state.interaction as i32 > 1 as i32 {",
                count: 1,
            },
        ],
    },
    FunctionEdits {
        function: "getnext",
        edits: &[
            // Abort a format run whose token list escaped memory, then surface token cell spans.
            Edit {
                anchor: "t = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.LH;\n            self.state.curinput.locfield = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.RH;",
                replacement: "if self.format_initialization\n                && self.state.inputptr == 0\n                && self.state.inopen == 0\n                && (self.state.curinput.locfield < self.state.memmin\n                    || self.state.curinput.locfield > self.state.memmax)\n            {\n                Self::abort_engine(self as *mut PortableTexEngine<'_>, 0 as integer);\n            }\n            Self::src_tokenlist_span(self as *mut PortableTexEngine<'_>);\n            t = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.LH;\n            self.state.curinput.locfield = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.RH;",
                count: 1,
            },
            // Source tracking hook 1a marks the token buffer start at each lexer loop.
            Edit {
                anchor: "'_lab20: loop {\n        self.state.curcs = 0 as i32 as halfword;",
                replacement: "'_lab20: loop {\n        Self::src_mark_token_start(self as *mut PortableTexEngine<'_>);\n        self.state.curcs = 0 as i32 as halfword;",
                count: 1,
            },
            // Source tracking hook 1a marks the start again after skipped material.
            Edit {
                anchor: "10 | 26 | 42 | 27 | 43 => {",
                replacement: "10 | 26 | 42 | 27 | 43 => {\n                                    Self::src_mark_token_start(self as *mut PortableTexEngine<'_>);",
                count: 1,
            },
            // Source tracking hook 1b records the span at the common lexer tail.
            Edit {
                anchor: "self.state.alignstate = 1000000 as i64 as integer;\n    }\n}",
                replacement: "self.state.alignstate = 1000000 as i64 as integer;\n    }\n    Self::src_record_buffer_span(self as *mut PortableTexEngine<'_>);\n}",
                count: 1,
            },
        ],
    },
];

pub(crate) fn add_self_arg(mut source: String, name: &str) -> String {
    let marker = format!("pub(crate) unsafe fn {name}(");
    let Some(start) = source.find(marker.as_str()) else {
        return source;
    };
    let args_start = start + marker.len();
    if source[args_start..].starts_with(')') {
        source.insert_str(args_start, "&mut self");
    } else {
        source.insert_str(args_start, "&mut self, ");
    }
    source
}

/// Drops the stale `*mut memoryword` type because `zeqtb` now returns a `PagedView<memoryword>`.
pub(crate) fn patch_paged_eqtb_binding(source: String) -> String {
    source.replace(
        "let mut eqtb: *mut memoryword = self.state.zeqtb.as_mut_ptr();",
        "let eqtb = self.state.zeqtb.as_mut_ptr();",
    )
}

pub(crate) fn patch_c_types(mut source: String) -> String {
    for (from, to) in crate::boundary::C_TYPE_ALIASES {
        source = source.replace(from, to);
    }
    source
}

pub(crate) fn patch_resource_search_names(mut source: String) -> String {
    for (from, to) in crate::boundary::RESOURCE_SEARCH_RENAMES {
        source = source.replace(from, to);
    }
    source
}

pub(crate) fn patch_terminal_io_names(mut source: String) -> String {
    for (from, to) in crate::boundary::TERMINAL_IO_ACCESS_RENAMES {
        source = source.replace(from, to);
    }
    source
}

pub(crate) use crate::boundary::state_field_name;

pub(crate) fn state_field_type(raw_type: &str) -> String {
    if raw_type == "*mut FILE" {
        return "NativeFileHandle".to_string();
    }

    patch_resource_search_names(raw_type.to_string())
}

pub(crate) fn is_vector_state_field(raw_type: &str) -> bool {
    raw_type.starts_with("*mut ") && raw_type != "*mut FILE"
}

/// Preserves the saved token span across the second `\futurelet` lookahead token read.
fn patch_futurelet_lookahead_span(source: &str) -> Option<String> {
    const GETTOKEN: &str = "(&mut *(self as *mut PortableTexEngine<'_>)).gettoken();";
    const BACKINPUT: &str = "(&mut *(self as *mut PortableTexEngine<'_>)).backinput();";
    const Q_ASSIGN: &str = "q = self.state.curtok;";
    const CURTOK_ASSIGN: &str = "self.state.curtok = q;";

    let save_marker_idx = source.find(Q_ASSIGN)?;
    let before_save = &source[..save_marker_idx];
    let gettoken_rel = before_save.rfind(GETTOKEN)?;
    let line_start = before_save[..gettoken_rel].rfind('\n').map_or(0, |i| i + 1);
    let indent = &before_save[line_start..gettoken_rel];

    let q_assign_end = save_marker_idx + Q_ASSIGN.len();

    let restore_marker_idx = source.find(CURTOK_ASSIGN)?;
    let after_restore_marker = restore_marker_idx + CURTOK_ASSIGN.len();
    let backinput_rel = source[after_restore_marker..].find(BACKINPUT)?;
    let second_backinput_start = after_restore_marker + backinput_rel;

    let mut out = String::with_capacity(source.len() + 512);
    out.push_str(&source[..q_assign_end]);
    out.push('\n');
    out.push_str(indent);
    out.push_str(
        "let src_tok_a_saved = Self::src_capture_tok_span(self as *mut PortableTexEngine<'_>);",
    );
    out.push_str(&source[q_assign_end..second_backinput_start]);
    out.push_str(indent);
    out.push_str(
        "Self::src_restore_tok_span(self as *mut PortableTexEngine<'_>, src_tok_a_saved);\n",
    );
    out.push_str(&source[second_backinput_start..]);
    Some(out)
}

/// Renames each `putc` writing to `__stdoutp` to `putc_terminal`, so terminal and log output route apart.
pub(crate) fn patch_terminal_writes(source: String) -> String {
    const CALL: &str = "putc(";
    let mut out = String::with_capacity(source.len());
    let mut rest = source.as_str();
    while let Some(at) = rest.find(CALL) {
        let args = at + CALL.len();
        let whole_word = !rest[..at]
            .chars()
            .next_back()
            .is_some_and(is_identifier_continue);
        let to_terminal = closing_paren(&rest[args..]).is_some_and(|close| {
            rest[args..args + close]
                .trim_end()
                .trim_end_matches(',')
                .rsplit(',')
                .next()
                .is_some_and(|stream| stream.trim() == "__stdoutp")
        });
        out.push_str(&rest[..at]);
        out.push_str(if whole_word && to_terminal {
            "putc_terminal("
        } else {
            CALL
        });
        rest = &rest[args..];
    }
    out.push_str(rest);
    out
}

/// Rewrites each `self.state.nativetext = xrealloc(..) as *mut UTF16code;` to grow the backing `Vec`.
fn patch_native_room_growth(source: &str) -> (String, usize) {
    const TARGET: &str = "self.state.nativetext";
    const CALL: &str = "xrealloc(";
    const CAST: &str = "as *mut UTF16code;";
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut sites = 0;
    while let Some(call) = rest.find(CALL) {
        let Some(before_target) = rest[..call]
            .trim_end()
            .strip_suffix('=')
            .and_then(|before| before.trim_end().strip_suffix(TARGET))
        else {
            break;
        };
        let args = call + CALL.len();
        let Some(close) = closing_paren(&rest[args..]) else {
            break;
        };
        let Some(after_cast) = rest[args + close + 1..].trim_start().strip_prefix(CAST) else {
            break;
        };
        out.push_str(before_target);
        out.push_str("Self::ensure_nativetext_capacity(self, self.state.nativetextsize);");
        rest = after_cast;
        sites += 1;
    }
    out.push_str(rest);
    (out, sites)
}

/// Whether `source` reads a `.ptr` field, the pointer view that memory words no longer have.
fn mentions_ptr_field(source: &str) -> bool {
    source.match_indices(".ptr").any(|(at, needle)| {
        !source[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(is_identifier_continue)
    })
}

/// Byte offset of the `)` closing a call whose arguments start at `source[0]`.
fn closing_paren(source: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (offset, byte) in source.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn prefix_host_calls(source: String) -> String {
    let host_functions = crate::boundary::HOST_FUNCTIONS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    rewrite_identifiers(source, |identifier, previous, next| {
        if next == Some('(') && previous != Some('.') && host_functions.contains(identifier) {
            Some(format!("Self::host_{identifier}"))
        } else {
            None
        }
    })
}

pub(crate) fn prefix_function_calls(
    source: String,
    translated_functions: &BTreeSet<String>,
) -> String {
    rewrite_identifiers(source, |identifier, previous, next| {
        if next != Some('(') || previous == Some('.') {
            return None;
        }

        if let Some(boundary) = portable_boundary_call(identifier) {
            Some(format!("Self::{boundary}"))
        } else if translated_functions.contains(identifier) {
            Some(format!(
                "(&mut *(self as *mut PortableTexEngine<'_>)).{identifier}"
            ))
        } else {
            None
        }
    })
}

pub(crate) fn portable_boundary_call(identifier: &str) -> Option<&'static str> {
    crate::boundary::PORTABLE_BOUNDARY_CALLS
        .iter()
        .find_map(|(from, to)| (*from == identifier).then_some(*to))
}

pub(crate) fn prefix_globals<'a>(
    source: String,
    globals: impl IntoIterator<Item = &'a String>,
    local_bindings: &BTreeSet<String>,
) -> String {
    let globals = globals
        .into_iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    rewrite_identifiers(source, |identifier, previous, next| {
        if previous != Some('.')
            && next != Some(':')
            && globals.contains(identifier)
            && !local_bindings.contains(identifier)
        {
            Some(format!("self.state.{identifier}"))
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_identifiers<F>(source: String, mut rewrite: F) -> String
where
    F: FnMut(&str, Option<char>, Option<char>) -> Option<String>,
{
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;

    scan_identifier_ranges(source.as_str(), |start, end, previous, next| {
        output.push_str(&source[cursor..start]);
        let identifier = &source[start..end];
        if let Some(replacement) = rewrite(identifier, previous, next) {
            output.push_str(replacement.as_str());
        } else {
            output.push_str(identifier);
        }
        cursor = end;
    });
    output.push_str(&source[cursor..]);
    output
}

pub(crate) fn scan_identifiers<F>(source: &str, mut visit: F)
where
    F: FnMut(&str, Option<char>, Option<char>),
{
    scan_identifier_ranges(source, |start, end, previous, next| {
        visit(&source[start..end], previous, next);
    });
}

pub(crate) fn scan_identifier_ranges<F>(source: &str, mut visit: F)
where
    F: FnMut(usize, usize, Option<char>, Option<char>),
{
    let mut index = 0;

    while index < source.len() {
        let current = source[index..]
            .chars()
            .next()
            .expect("index should point at a char boundary");
        if !is_identifier_start(current) {
            index += current.len_utf8();
            continue;
        }

        let start = index;
        index += current.len_utf8();
        while index < source.len() {
            let next = source[index..]
                .chars()
                .next()
                .expect("index should point at a char boundary");
            if !is_identifier_continue(next) {
                break;
            }
            index += next.len_utf8();
        }

        let previous = source[..start].chars().next_back();
        let next = source[index..].chars().next();
        visit(start, index, previous, next);
    }
}

pub(crate) fn contains_identifier(source: &str, needle: &str) -> bool {
    let mut found = false;
    scan_identifiers(source, |identifier, _previous, _next| {
        if identifier == needle {
            found = true;
        }
    });
    found
}

pub(crate) fn identifier_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    scan_identifiers(source, |identifier, _previous, _next| {
        names.insert(identifier.to_string());
    });
    names
}

pub(crate) fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

pub(crate) fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, body: &str) -> RawFunction {
        RawFunction {
            name: name.to_string(),
            source: body.to_string(),
        }
    }

    #[test]
    fn edit_count_mismatch_names_function_and_anchor() {
        let mut session = PatchSession::new();
        let mut source = String::from("a(); a();");
        let edit = Edit {
            anchor: "a();",
            replacement: "b();",
            count: 1,
        };
        session.edit(&mut source, "sample", &edit);
        assert_eq!(source, "a(); a();");
        let mismatch = &session.mismatches[0];
        assert_eq!(mismatch.function, "sample");
        assert!(mismatch.detail.contains("matched 2 times, expected 1"));
        assert!(mismatch.detail.contains("a();"));
    }

    #[test]
    fn edit_with_exact_count_replaces_every_match() {
        let mut session = PatchSession::new();
        let mut source = String::from("a(); a();");
        let edit = Edit {
            anchor: "a();",
            replacement: "b();",
            count: 2,
        };
        session.edit(&mut source, "sample", &edit);
        assert_eq!(source, "b(); b();");
        assert!(session.mismatches.is_empty());
    }

    #[test]
    fn template_upstream_drift_names_the_template() {
        let mut session = PatchSession::new();
        let changed = raw(
            "zpdferror",
            "pub unsafe extern \"C\" fn zpdferror() { changed(); }",
        );
        let out = session.apply(&changed, changed.source.clone());
        assert!(out.contains("record_stripped_pdf_extension"));
        assert_eq!(session.mismatches.len(), 1);
        assert!(session.mismatches[0]
            .detail
            .contains("patches/zpdferror.rs.in"));
    }

    #[test]
    fn unvisited_registered_function_is_a_mismatch() {
        let session = PatchSession::new();
        let Err(PatchError::PatchMismatches(mismatches)) = session.finish() else {
            panic!("patches for functions never seen must fail");
        };
        assert!(mismatches
            .iter()
            .any(|mismatch| mismatch.function == "maincontrol"
                && mismatch.detail.contains("never applied")));
    }

    #[test]
    fn native_room_growth_calls_ensure_capacity() {
        let source = "a;\n    self.state.nativetext =\n        xrealloc(\n            p as address,\n            (n as size_t).wrapping_mul(f()),\n        ) as *mut UTF16code;\n    b;";
        let (patched, sites) = patch_native_room_growth(source);
        assert_eq!(sites, 1);
        assert_eq!(
            patched,
            "a;\n    Self::ensure_nativetext_capacity(self, self.state.nativetextsize);\n    b;"
        );
        let (unchanged, sites) = patch_native_room_growth("q = xrealloc(p, n);");
        assert_eq!((unchanged.as_str(), sites), ("q = xrealloc(p, n);", 0));
    }

    #[test]
    fn terminal_putc_calls_are_renamed() {
        let source = "putc('\\n' as i32, __stdoutp);\nputc(s, logfile as *mut FILE);\nputc(\n    s,\n    __stdoutp,\n);";
        assert_eq!(
            patch_terminal_writes(source.to_string()),
            "putc_terminal('\\n' as i32, __stdoutp);\nputc(s, logfile as *mut FILE);\nputc_terminal(\n    s,\n    __stdoutp,\n);"
        );
    }

    #[test]
    fn futurelet_patch_reports_missing_markers() {
        let mut session = PatchSession::new();
        let function = raw(FUTURELET_FUNCTION, "");
        session.apply(&function, "fn prefixedcommand() {}".to_string());
        assert!(session.mismatches[0].detail.contains("futurelet"));
    }
}

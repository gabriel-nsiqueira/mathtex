//! String transforms applied to translated TeX functions.

use std::collections::BTreeSet;

use crate::model::SharedSourceOrigin;

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


pub(crate) fn patch_generic_native_font_path(mut source: String) -> String {
    // The portable boundary returns a `FontHandle`, so set `nativefonttypeflag` inline here.
    source = source.replace(
        "let mut fontengine: voidpointer = ::core::ptr::null_mut::<()>();",
        "let mut fontengine: FontHandle = 0;",
    );
    source = source.replace(
        "        ) as voidpointer;\n        if !fontengine.is_null() {",
        "        );\n        if fontengine != 0 {\n            self.state.nativefonttypeflag = 65534 as integer;",
    );
    source = source.replace(
        "    ) as voidpointer;\n    if !fontengine.is_null() {",
        "    );\n    if fontengine != 0 {\n        self.state.nativefonttypeflag = 65534 as integer;",
    );
    source = source.replace(
        "if self.state.nativefonttypeflag as i64 == 65535 as i64 {\n                            Self::measure_font_metrics(\n                                self as *mut PortableTexEngine<'_>,\n                                fontengine,\n                                &raw mut ascent,\n                                &raw mut descent,\n                                &raw mut xht,\n                                &raw mut capht,\n                                &raw mut fontslant,\n                            );\n                        } else {\n                            Self::measure_opentype_font_metrics(\n                                self as *mut PortableTexEngine<'_>,\n                                fontengine,\n                                &raw mut ascent,\n                                &raw mut descent,\n                                &raw mut xht,\n                                &raw mut capht,\n                                &raw mut fontslant,\n                            );\n                        }",
        "Self::measure_opentype_font_metrics(\n                            self as *mut PortableTexEngine<'_>,\n                            fontengine,\n                            &raw mut ascent,\n                            &raw mut descent,\n                            &raw mut xht,\n                            &raw mut capht,\n                            &raw mut fontslant,\n                        );",
    );
    source = source.replace(
        "if self.state.nativefonttypeflag as i64 == 65535 as i64 {\n                        Self::measure_font_metrics(self as *mut PortableTexEngine<'_>, \n                            fontengine,\n                            &raw mut ascent,\n                            &raw mut descent,\n                            &raw mut xht,\n                            &raw mut capht,\n                            &raw mut fontslant,\n                        );\n                    } else {\n                        Self::measure_opentype_font_metrics(self as *mut PortableTexEngine<'_>, \n                            fontengine,\n                            &raw mut ascent,\n                            &raw mut descent,\n                            &raw mut xht,\n                            &raw mut capht,\n                            &raw mut fontslant,\n                        );\n                    }",
        "Self::measure_opentype_font_metrics(self as *mut PortableTexEngine<'_>, \n                        fontengine,\n                        &raw mut ascent,\n                        &raw mut descent,\n                        &raw mut xht,\n                        &raw mut capht,\n                        &raw mut fontslant,\n                    );",
    );
    source = source.replace(
        "*fresh88 = fontengine;",
        "*fresh88 = fontengine as voidpointer;",
    );
    source = source.replace("fontengine as *mut ()", "fontengine");
    source = source.replace(
        "*fresh91 = self.state.loadedfontmapping;",
        "*fresh91 = ::core::ptr::null_mut::<()>();",
    );
    source
}


pub(crate) fn patch_initialization_function(mut source: String, function_name: &str) -> String {
    if function_name == "getstringsstarted" {
        source = source.replace(
            "loadpoolstrings(self.state.poolsize - self.state.stringvacancies)",
            "Self::load_pool_strings(self, self.state.poolsize - self.state.stringvacancies)",
        );
        return source.replace(
            "    if g == 0 as i32 {\n        fprintf(\n            self.state.terminal_output,\n            b\"%s\\n\\0\" as *const u8 as *const i8,\n            b\"! You have to increase POOLSIZE.\\0\" as *const u8 as *const i8,\n        );\n        Result = false_0 as boolean;\n        return Result;\n    }\n",
            "    if g == 0 as i32 {\n        Result = false_0 as boolean;\n        return Result;\n    }\n",
        );
    }

    if function_name != "initialize" {
        return source;
    }

    source = source.replace(
        "    if !self.state.translate_filename.is_null() {\n        readtcxfile();\n    }\n",
        "",
    );
    source = source.replace(
        "    self.state.nativetextsize = 128 as i32 as integer;\n    self.state.nativetext = xmalloc(\n        (self.state.nativetextsize as size_t).wrapping_mul(::core::mem::size_of::<UTF16code>() as size_t),\n    ) as *mut UTF16code;\n",
        "    self.state.nativetextsize = 128 as i32 as integer;\n    Self::ensure_nativetext_capacity(self, self.state.nativetextsize);\n",
    );
    source.replace("    initstarttime();\n", "")
}


/// Replaces exactly one anchor or dumps the stage source to `/tmp/patcher-stage-<fn>.rs`.
fn replace_once(source: &mut String, function_name: &str, anchor: &str, replacement: &str) {
    match source.matches(anchor).count() {
        1 => *source = source.replacen(anchor, replacement, 1),
        n => {
            let path = format!("/tmp/patcher-stage-{function_name}.rs");
            std::fs::write(&path, source.as_str()).ok();
            panic!(
                "patch_node_source_recording[{function_name}]: anchor matched {n}x (expected 1); \
                 stage source dumped to {path} -- copy the exact anchor from there.\n\
                 --- anchor ---\n{anchor}\n--------------"
            );
        }
    }
}

/// Like [`replace_once`] but accepts zero matches for anchors present only in some engine variants.
#[allow(dead_code)]
fn replace_at_most_once(source: &mut String, function_name: &str, anchor: &str, replacement: &str) {
    match source.matches(anchor).count() {
        0 => {}
        1 => *source = source.replacen(anchor, replacement, 1),
        n => {
            let path = format!("/tmp/patcher-stage-{function_name}.rs");
            std::fs::write(&path, source.as_str()).ok();
            panic!(
                "patch_node_source_recording[{function_name}]: anchor matched {n}x (expected 0 or 1); \
                 stage source dumped to {path}.\n--- anchor ---\n{anchor}\n--------------"
            );
        }
    }
}


pub(crate) fn patch_node_source_recording(mut source: String, function_name: &str) -> String {
    // `PATCHER_DUMP=fn1,fn2 cargo run --bin patch_engine` writes stage sources for anchor repair.
    if std::env::var("PATCHER_DUMP")
        .map(|v| v.split(',').any(|f| f.trim() == function_name))
        .unwrap_or(false)
    {
        std::fs::write(format!("/tmp/patcher-stage-{function_name}.rs"), &source).ok();
    }
    match function_name {
        // Surface TeX errors after `show_context` has written the transcript message.
        "error" => {
            replace_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).showcontext();\n    if self.state.haltonerrorp != 0 {",
                "(&mut *(self as *mut PortableTexEngine<'_>)).showcontext();\n    Self::surface_error(self as *mut PortableTexEngine<'_>);\n    if self.state.haltonerrorp != 0 {",
            );
        }
        // Reject a second math entry as a sandbox escape from the wrapper math.
        "initmath" => {
            replace_once(
                &mut source,
                function_name,
                "pub(crate) unsafe fn initmath(&mut self) {",
                "pub(crate) unsafe fn initmath(&mut self) {\n        Self::sandbox_open_math(self as *mut PortableTexEngine<'_>);",
            );
        }
        // Mark wrapper math closed so the next `init_math` is treated as a user breakout.
        "aftermath" => {
            replace_once(
                &mut source,
                function_name,
                "pub(crate) unsafe fn aftermath(&mut self) {",
                "pub(crate) unsafe fn aftermath(&mut self) {\n        Self::sandbox_close_math(self as *mut PortableTexEngine<'_>);",
            );
        }
        // Source tracking hook 3 stamps allocated cells with the ambient span.
        "getavail" => {
            replace_once(
                &mut source,
                function_name,
                "self.state.dynused += 1;\n    Result = p;",
                "self.state.dynused += 1;\n    Self::src_stamp_avail(self as *mut PortableTexEngine<'_>, p);\n    Result = p;",
            );
        }
        // `char_box` bypasses `new_character`, so stamp glyphs with the construct span here.
        "zcharbox" => {
            replace_once(
                &mut source,
                function_name,
                "Result = b;\n    return Result;",
                "Self::src_stamp_char(self as *mut PortableTexEngine<'_>, p);\n    Result = b;\n    return Result;",
            );
        }
        // Source tracking hook 4 stamps each variable size node range with the construct span.
        "zgetnode" => {
            replace_once(
                &mut source,
                function_name,
                "self.state.varused = self.state.varused + s;\n            Result = r as halfword;",
                "self.state.varused = self.state.varused + s;\n            Self::src_stamp_node_range(self as *mut PortableTexEngine<'_>, r as halfword, s);\n            Result = r as halfword;",
            );
        }
        // Source tracking hook 5 overwrites the provisional TFM glyph stamp.
        "znewcharacter" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(p as isize)).hh.u.B1 = c_0 as i16;\n                Result = p;",
                "(*mem.offset(p as isize)).hh.u.B1 = c_0 as i16;\n                Self::src_stamp_char(self as *mut PortableTexEngine<'_>, p);\n                Result = p;",
            );
        }
        // Source tracking hook 6 repoints `cmd_span` to noad `q` before geometry is synthesized.
        "mlisttohlist" => {
            replace_once(
                &mut source,
                function_name,
                "while q as i64 != -(268435455 as i64) {\n        loop {\n            delta_0 = 0 as i32 as scaled;",
                "while q as i64 != -(268435455 as i64) {\n        Self::src_mlist_repoint(self as *mut PortableTexEngine<'_>, q);\n        loop {\n            delta_0 = 0 as i32 as scaled;",
            );
            // Carry a field leaf char span onto the glyph node after `make_ord` attaches it.
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset((q as i32 + 1 as i32) as isize))\n                    .u\n                    .CINT = p as integer;",
                "(*mem.offset((q as i32 + 1 as i32) as isize))\n                    .u\n                    .CINT = p as integer;\n                Self::src_carry_nucleus(self as *mut PortableTexEngine<'_>, q, p);",
            );
        }
        // Source tracking hook 7 preserves `cmd_span` across recursive sub box cleanup.
        "zcleanbox" => {
            replace_once(
                &mut source,
                function_name,
                "pub(crate) unsafe fn zcleanbox(&mut self, mut p: halfword, mut s: smallnumber) -> halfword {",
                "pub(crate) unsafe fn zcleanbox(&mut self, mut p: halfword, mut s: smallnumber) -> halfword {\n        let __src_saved_cmd_span: u32 = Self::src_save_cmd_span(self as *mut PortableTexEngine<'_>);",
            );
            replace_once(
                &mut source,
                function_name,
                "Result = x;\n    return Result;",
                "Self::src_restore_cmd_span(self as *mut PortableTexEngine<'_>, __src_saved_cmd_span);\n    Result = x;\n    return Result;",
            );
            // Carry a single math char field span onto the noad that `clean_box` builds.
            replace_once(
                &mut source,
                function_name,
                "*mem.offset((self.state.curmlist as i32 + 1 as i32) as isize) =\n                *mem.offset(p as isize);",
                "*mem.offset((self.state.curmlist as i32 + 1 as i32) as isize) =\n                *mem.offset(p as isize);\n            Self::src_carry_field(self as *mut PortableTexEngine<'_>, p, self.state.curmlist);",
            );
        }
        // Record direct math char field spans so `clean_box` can carry them later.
        "zscanmath" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(p as isize)).hh.v.RH = 1 as i32 as halfword;",
                "(*mem.offset(p as isize)).hh.v.RH = 1 as i32 as halfword;\n    Self::src_stamp_field(self as *mut PortableTexEngine<'_>, p);",
            );
            // Push a pending construct frame for a `{` delimited math field argument.
            replace_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).zpushmath(9 as i32 as groupcode);",
                "(&mut *(self as *mut PortableTexEngine<'_>)).zpushmath(9 as i32 as groupcode);\n                        Self::src_scan_math_group_open(self as *mut PortableTexEngine<'_>);",
            );
        }
        // Source tracking hook 8b pops the construct frame for a macro body level.
        "endtokenlist" => {
            replace_once(
                &mut source,
                function_name,
                "pub(crate) unsafe fn endtokenlist(&mut self) {",
                "pub(crate) unsafe fn endtokenlist(&mut self) {\n        Self::src_end_token_list(self as *mut PortableTexEngine<'_>, self.state.curinput.indexfield as i32);",
            );
        }
        // Carry a collapsed braced script noad span onto its saved field.
        "handlerightbrace" => {
            replace_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).zfreenode(p, 4 as i32);",
                "Self::src_carry_collapse(self as *mut PortableTexEngine<'_>, p, (*self.state.savestack.offset((self.state.saveptr as i32 + 0 as i32) as isize)).u.CINT);\n                                (&mut *(self as *mut PortableTexEngine<'_>)).zfreenode(p, 4 as i32);",
            );
            // Pop the math group frame after extending it through the closing brace.
            replace_once(
                &mut source,
                function_name,
                "9 => {\n            (&mut *(self as *mut PortableTexEngine<'_>)).unsave();\n            self.state.saveptr -= 1;",
                "9 => {\n            Self::src_scan_math_group_close(self as *mut PortableTexEngine<'_>);\n            (&mut *(self as *mut PortableTexEngine<'_>)).unsave();\n            self.state.saveptr -= 1;",
            );
            // Extend construct noads through the closing brace after the nucleus group is filled.
            replace_once(
                &mut source,
                function_name,
                "            .v\n            .LH = p;\n            if p as i64 != -(268435455 as i64) {",
                "            .v\n            .LH = p;\n            Self::src_construct_extend_to_loc(self as *mut PortableTexEngine<'_>);\n            if p as i64 != -(268435455 as i64) {",
            );
        }
        // Source tracking hook 8 sets the new token list baseline after the input stack push.
        "zbegintokenlist" => {
            replace_once(
                &mut source,
                function_name,
                "*self.state.inputstack.offset(self.state.inputptr as isize) = self.state.curinput;\n    self.state.inputptr += 1;",
                "*self.state.inputstack.offset(self.state.inputptr as isize) = self.state.curinput;\n    self.state.inputptr += 1;\n    Self::src_begin_token_list(self as *mut PortableTexEngine<'_>, t);",
            );
        }
        // Source tracking hook 9 records the macro invocation span before body entry.
        "macrocall" => {
            replace_once(
                &mut source,
                function_name,
                "self.state.warningindex = self.state.curcs;",
                "self.state.warningindex = self.state.curcs;\n    Self::src_macro_begin(self as *mut PortableTexEngine<'_>);",
            );
            // Hook 3b stamps argument token cells that bypass `get_avail`.
            source = source.replace(
                "(*mem.offset(q as isize)).hh.v.LH = self.state.curtok;",
                "(*mem.offset(q as isize)).hh.v.LH = self.state.curtok;\n                                    Self::src_stamp_avail(self as *mut PortableTexEngine<'_>, q);",
            );
            // The macro token type differs by profile, so tolerate the absent variant.
            replace_at_most_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 6 as i32 as quarterword);",
                "Self::src_macro_set_pending(self as *mut PortableTexEngine<'_>, n);\n            (&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 6 as i32 as quarterword);",
            );
            replace_at_most_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 5 as i32 as quarterword);",
                "Self::src_macro_set_pending(self as *mut PortableTexEngine<'_>, n);\n            (&mut *(self as *mut PortableTexEngine<'_>)).zbegintokenlist(refcount, 5 as i32 as quarterword);",
            );
        }
        // Anchor radical and accent noads to the user command before delimiter or nucleus scanning.
        "mathradical" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 24 as i16;",
                "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 24 as i16;\n    Self::src_construct_anchor(self as *mut PortableTexEngine<'_>);",
            );
        }
        "mathac" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 28 as i16;",
                "(*mem.offset(self.state.curlist.tailfield as isize)).hh.u.B0 = 28 as i16;\n    Self::src_construct_anchor(self as *mut PortableTexEngine<'_>);",
            );
            // Extend unbraced accent noads after `scan_math` fills the nucleus.
            replace_once(
                &mut source,
                function_name,
                "(&mut *(self as *mut PortableTexEngine<'_>)).zscanmath(self.state.curlist.tailfield as i32 + 1 as i32);",
                "let src_acc = self.state.curlist.tailfield;\n    (&mut *(self as *mut PortableTexEngine<'_>)).zscanmath(self.state.curlist.tailfield as i32 + 1 as i32);\n    Self::src_construct_extent(self as *mut PortableTexEngine<'_>, src_acc, 0 as u32);",
            );
        }
        // Carry source spans onto copied nodes before linking them into the new list.
        "zcopynodelist" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(q as isize)).hh.v.RH = r;\n        q = r;\n        p = (*mem.offset(p as isize)).hh.v.RH;",
                "Self::src_carry_copy(self as *mut PortableTexEngine<'_>, p, r);\n        (*mem.offset(q as isize)).hh.v.RH = r;\n        q = r;\n        p = (*mem.offset(p as isize)).hh.v.RH;",
            );
        }
        // Replace the WEB `src_token_copy` marker with a real token span carry.
        "zsrctokencopy" => {
            replace_once(
                &mut source,
                function_name,
                "dest == src;",
                "Self::src_carry_token_span(self as *mut PortableTexEngine<'_>, dest, src);",
            );
        }
        // Source tracking hook 1d stamps backed up tokens with their own origin.
        "backinput" => {
            replace_once(
                &mut source,
                function_name,
                "(*mem.offset(p as isize)).hh.v.LH = self.state.curtok;",
                "(*mem.offset(p as isize)).hh.v.LH = self.state.curtok;\n        Self::src_back_input_stamp(self as *mut PortableTexEngine<'_>, p);",
            );
        }
        // Track `\left` and `\right` as one consumed source extent outside `scan_math`.
        "mathleftright" => {
            replace_once(
                &mut source,
                function_name,
                ".zscandelimiter(\n            p as i32 + 1 as i32,\n            0 as i32,\n        );\n        if t as i32 == 1 as i32 {",
                ".zscandelimiter(\n            p as i32 + 1 as i32,\n            0 as i32,\n        );\n        Self::src_leftright(self as *mut PortableTexEngine<'_>, p, t as i32);\n        if t as i32 == 1 as i32 {",
            );
        }
        "appendsrcspecial" | "insertsrcspecial" => {
            source = replacement_template("appendsrcspecial_or_insertsrcspecial")
                .replace("{function_name}", function_name);
        }
        "zprintwritewhatsit" => {
            source = replacement_template(function_name).to_string();
        }
        "scanpdfexttoks" => {
            source = replacement_template(function_name).to_string();
        }
        "zpdferror" => {
            source = replacement_template(function_name).to_string();
        }
        "openorclosein" => {
            source = replacement_template(function_name).to_string();
        }
        "startinput" => {
            source = replacement_template(function_name).to_string();
        }
        "zpackfilename" => {
            source = replacement_template(function_name).to_string();
        }
        "ztrienode" => {
            source = replacement_template(function_name).to_string();
        }
        "zcopynativeglyphinfo" => {
            source = replacement_template(function_name).to_string();
        }
        "znewnativecharacter" => {
            source = replacement_template(function_name).to_string();
        }
        "zboxend" => {
            source = source.replace(
                "(*mem.offset((self.state.curbox as i32 + 4 as i32) as isize))\n                .u\n                .CINT = boxcontext;",
                "(*mem.offset((self.state.curbox as i32 + 4 as i32) as isize))\n                .u\n                .CINT = boxcontext;\n            if Self::boundary_capture_fragment_box(\n                self as *mut PortableTexEngine<'_>,\n                self.state.curbox,\n                self.state.curlist.modefield as integer,\n                boxcontext,\n            ) != 0\n            {\n                return;\n            }",
            );
        }
        "zlinebreak" => {
            source = source.replace(
                "if self.state.trienotready != 0 {",
                "if self.state.trienotready != 0 && !self.format_initialization {",
            );
            source = source.replace(
                "let mut mem: *mut memoryword = self.state.zmem;",
                "let mut mem: *mut memoryword = self.state.zmem;\n    if self.format_initialization {\n        let paragraph = (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH;\n        if paragraph as i64 != -(268435455 as i64) {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(paragraph);\n        }\n        if self.state.nestptr > 0 {\n            (&mut *(self as *mut PortableTexEngine<'_>)).popnest();\n        } else {\n            (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH = -(268435455 as i64) as halfword;\n            self.state.curlist.tailfield = self.state.curlist.headfield;\n        }\n        (&mut *(self as *mut PortableTexEngine<'_>)).normalparagraph();\n        self.state.packbeginline = 0 as integer;\n        return;\n    }",
            );
        }
        "inittrie" => {
            source = source.replace(
                "pub(crate) unsafe fn inittrie(&mut self) {",
                "pub(crate) unsafe fn inittrie(&mut self) {\n    if self.format_initialization {\n        return;\n    }",
            );
        }
        "newpatterns" => {
            source = source.replace(
                "let mut mem: *mut memoryword = self.state.zmem;",
                "let mut mem: *mut memoryword = self.state.zmem;\n    if self.format_initialization {\n        let list = (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH;\n        if list as i64 != -(268435455 as i64) {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(list);\n        }\n        (*mem.offset(self.state.curlist.headfield as isize)).hh.v.RH = -(268435455 as i64) as halfword;\n        self.state.curlist.tailfield = self.state.curlist.headfield;\n    }",
            );
        }
        "maincontrol" => {
            // Tick the sandbox budget once per main control iteration.
            replace_once(
                &mut source,
                function_name,
                "'_lab60: loop {\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
                "'_lab60: loop {\n        Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
            );
            // Source tracking hook 2 freezes the command token span before argument scanning.
            replace_once(
                &mut source,
                function_name,
                "Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();",
                "Self::sandbox_tick(self as *mut PortableTexEngine<'_>);\n        (&mut *(self as *mut PortableTexEngine<'_>)).getxtoken();\n        Self::src_latch_cmd_span(self as *mut PortableTexEngine<'_>);",
            );
            // Source tracking hook 11 records a span for each XeTeX native text code unit.
            replace_at_most_once(
                &mut source,
                function_name,
                "self.state.ishyph = (self.state.curchr",
                "Self::src_native_run_push(self as *mut PortableTexEngine<'_>);\n                        self.state.ishyph = (self.state.curchr",
            );
            source = source.replace(
                "15 => {\n                        if (&mut *(self as *mut PortableTexEngine<'_>)).itsallover() != 0 {",
                "15 => {\n                        if self.format_initialization && self.state.curchr == 1 {\n                            Self::abort_engine(self as *mut PortableTexEngine<'_>, 0 as integer);\n                        }\n                        if (&mut *(self as *mut PortableTexEngine<'_>)).itsallover() != 0 {",
            );
        }
        "itsallover" => {
            // Flush pending main vertical material instead of retrying a stripped page ship path.
            source = source.replace(
                "(&mut *(self as *mut PortableTexEngine<'_>)).backinput();\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH = (&mut *(self as *mut PortableTexEngine<'_>)).newnullbox();\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        (*mem.offset(\n            (self.state.curlist.tailfield as i32 + 1 as i32) as isize,\n        ))\n        .u\n        .CINT = (*eqtb.offset(9006722 as i64 as isize))\n            .u\n            .CINT;\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH =\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewglue(self.state.membot as i32 + 8 as i32);\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH =\n            (&mut *(self as *mut PortableTexEngine<'_>)).znewpenalty(-(1073741824 as i64) as integer);\n        self.state.curlist.tailfield = (*mem.offset(self.state.curlist.tailfield as isize)).hh.v.RH;\n        Self::boundary_build_page();",
                "// Output drivers are stripped from this engine, so a page that\n        // cannot be shipped is DISCARDED, not forced out. The upstream TeX\n        // path appends `\\hbox{}\\vfill\\penalty-2^30` and calls `build_page` to\n        // eject residual material, then retries; but `build_page` is a no-op\n        // here, so the page never clears and the retry loop spins forever and\n        // exhausts main memory (e.g. the page LaTeX's `\\begin{document}`\n        // leaves pending). Flush the pending main vertical list and finish.\n        let head = self.state.curlist.headfield;\n        let first = (*mem.offset(head as isize)).hh.v.RH;\n        (*mem.offset(head as isize)).hh.v.RH = -(268435455 as i64) as halfword;\n        self.state.curlist.tailfield = head;\n        if first != -(268435455 as i64) as halfword {\n            (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(first);\n        }\n        Result = true_0 as boolean;\n        return Result;",
            );
        }
        "headforvmode" => {
            // Discard pending paragraph material before forcing `\par` for `\end` or `\dump`.
            source = source.replace(
                "} else {\n        (&mut *(self as *mut PortableTexEngine<'_>)).backinput();\n        self.state.curtok = self.state.partoken;",
                "} else {\n        if self.state.curcmd as i32 == 15 as i32 {\n            let mem: *mut memoryword = self.state.zmem.as_mut_ptr();\n            let head = self.state.curlist.headfield;\n            let first = (*mem.offset(head as isize)).hh.v.RH;\n            (*mem.offset(head as isize)).hh.v.RH = -(268435455 as i64) as halfword;\n            self.state.curlist.tailfield = head;\n            if first != -(268435455 as i64) as halfword {\n                (&mut *(self as *mut PortableTexEngine<'_>)).zflushnodelist(first);\n            }\n        }\n        (&mut *(self as *mut PortableTexEngine<'_>)).backinput();\n        self.state.curtok = self.state.partoken;",
            );
        }
        "prefixedcommand" => {
            source = patch_font_int_assignment_arm(source);
            source = patch_futurelet_lookahead_span(source);
        }
        "zscansomethinginternal" => {
            source = patch_font_int_query_arm(source);
        }
        "zreadtoks" => {
            source = source.replace(
                "if self.state.readopen[m as usize] as i32 == 2 as i32 {\n            if self.state.interaction as i32 > 1 as i32 {",
                "if self.state.readopen[m as usize] as i32 == 2 as i32 {\n            if self.format_initialization {\n                self.state.last = self.state.first;\n            } else if self.state.interaction as i32 > 1 as i32 {",
            );
        }
        "getnext" => {
            // Source tracking hook 1c surfaces token cell spans when rereading arguments.
            source = source.replace(
                "t = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.LH;\n            self.state.curinput.locfield = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.RH;",
                "if self.format_initialization\n                && self.state.inputptr == 0\n                && self.state.inopen == 0\n                && (self.state.curinput.locfield < self.state.memmin\n                    || self.state.curinput.locfield > self.state.memmax)\n            {\n                Self::abort_engine(self as *mut PortableTexEngine<'_>, 0 as integer);\n            }\n            Self::src_tokenlist_span(self as *mut PortableTexEngine<'_>);\n            t = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.LH;\n            self.state.curinput.locfield = (*mem.offset(self.state.curinput.locfield as isize)).hh.v.RH;",
            );
            // Source tracking hook 1a marks the token buffer start at each lexer loop.
            replace_once(
                &mut source,
                function_name,
                "'_lab20: loop {\n        self.state.curcs = 0 as i32 as halfword;",
                "'_lab20: loop {\n        Self::src_mark_token_start(self as *mut PortableTexEngine<'_>);\n        self.state.curcs = 0 as i32 as halfword;",
            );
            // Source tracking hook 1a marks the start again after skipped material.
            replace_once(
                &mut source,
                function_name,
                "10 | 26 | 42 | 27 | 43 => {",
                "10 | 26 | 42 | 27 | 43 => {\n                                    Self::src_mark_token_start(self as *mut PortableTexEngine<'_>);",
            );
            // Source tracking hook 1b records the span at the common lexer tail.
            replace_once(
                &mut source,
                function_name,
                "self.state.alignstate = 1000000 as i64 as integer;\n    }\n}",
                "self.state.alignstate = 1000000 as i64 as integer;\n    }\n    Self::src_record_buffer_span(self as *mut PortableTexEngine<'_>);\n}",
            );
        }
        _ => {}
    }
    source
}


pub(crate) fn replacement_template(function_name: &str) -> &'static str {
    match function_name {
        "appendsrcspecial_or_insertsrcspecial" => {
            include_str!("bin/patches/appendsrcspecial_or_insertsrcspecial.rs.in")
        }
        "zprintwritewhatsit" => include_str!("bin/patches/zprintwritewhatsit.rs"),
        "scanpdfexttoks" => include_str!("bin/patches/scanpdfexttoks.rs"),
        "zpdferror" => include_str!("bin/patches/zpdferror.rs"),
        "openorclosein" => include_str!("bin/patches/openorclosein.rs"),
        "startinput" => include_str!("bin/patches/startinput.rs"),
        "ztrienode" => include_str!("bin/patches/ztrienode.rs"),
        "zpackfilename" => include_str!("bin/patches/zpackfilename.rs"),
        "zcopynativeglyphinfo" => include_str!("bin/patches/zcopynativeglyphinfo.rs"),
        "znewnativecharacter" => include_str!("bin/patches/znewnativecharacter.rs"),
        _ => unreachable!("missing replacement template for {function_name}"),
    }
}


pub(crate) fn replace_match_arm(
    mut source: String,
    arm_marker: &str,
    next_arm_marker: &str,
    replacement: &str,
) -> String {
    let Some(start) = source.find(arm_marker) else {
        return source;
    };
    let Some(relative_end) = source[start..].find(next_arm_marker) else {
        return source;
    };
    let end = start + relative_end;
    source.replace_range(start..end, replacement);
    source
}


/// Preserves the saved token span across the second `\futurelet` lookahead token read.
pub(crate) fn patch_futurelet_lookahead_span(source: String) -> String {
    const GETTOKEN: &str = "(&mut *(self as *mut PortableTexEngine<'_>)).gettoken();";
    const BACKINPUT: &str = "(&mut *(self as *mut PortableTexEngine<'_>)).backinput();";
    const Q_ASSIGN: &str = "q = self.state.curtok;";
    const CURTOK_ASSIGN: &str = "self.state.curtok = q;";

    let Some(save_marker_idx) = source.find(Q_ASSIGN) else {
        return source;
    };
    let before_save = &source[..save_marker_idx];
    let Some(gettoken_rel) = before_save.rfind(GETTOKEN) else {
        return source;
    };
    let line_start = before_save[..gettoken_rel].rfind('\n').map_or(0, |i| i + 1);
    let indent = before_save[line_start..gettoken_rel].to_string();

    let q_assign_end = save_marker_idx + Q_ASSIGN.len();

    let Some(restore_marker_idx) = source.find(CURTOK_ASSIGN) else {
        return source;
    };
    let after_restore_marker = restore_marker_idx + CURTOK_ASSIGN.len();
    let Some(backinput_rel) = source[after_restore_marker..].find(BACKINPUT) else {
        return source;
    };
    let second_backinput_start = after_restore_marker + backinput_rel;

    let mut out = String::with_capacity(source.len() + 512);
    out.push_str(&source[..q_assign_end]);
    out.push('\n');
    out.push_str(&indent);
    out.push_str(
        "let src_tok_a_saved = Self::src_capture_tok_span(self as *mut PortableTexEngine<'_>);",
    );
    out.push_str(&source[q_assign_end..second_backinput_start]);
    out.push_str(&indent);
    out.push_str(
        "Self::src_restore_tok_span(self as *mut PortableTexEngine<'_>, src_tok_a_saved);\n",
    );
    out.push_str(&source[second_backinput_start..]);
    out
}

pub(crate) fn patch_font_int_assignment_arm(source: String) -> String {
    replace_match_arm(
        source,
        "        78 => {",
        "        88 => {",
        "        78 => {\n            n = self.state.curchr as integer;\n            (&mut *(self as *mut PortableTexEngine<'_>)).scanfontident();\n            f = self.state.curval as internalfontnumber;\n            if n < 2 as i32 {\n                (&mut *(self as *mut PortableTexEngine<'_>)).scanoptionalequals();\n                (&mut *(self as *mut PortableTexEngine<'_>)).scanint();\n                if n == 0 as i32 {\n                    *self.state.hyphenchar.offset(f as isize) = self.state.curval;\n                } else {\n                    *self.state.skewchar.offset(f as isize) = self.state.curval;\n                }\n            } else {\n                if self.supports_native_fonts()\n                    && (*self.state.fontarea.offset(f as isize) as i64 == 65535 as i64\n                        || *self.state.fontarea.offset(f as isize) as i64 == 65534 as i64)\n                {\n                    (&mut *(self as *mut PortableTexEngine<'_>)).zscanglyphnumber(f);\n                } else {\n                    (&mut *(self as *mut PortableTexEngine<'_>)).scancharnum();\n                }\n                p = self.state.curval;\n                (&mut *(self as *mut PortableTexEngine<'_>)).scanoptionalequals();\n                (&mut *(self as *mut PortableTexEngine<'_>)).scanint();\n                let side = if n == 2 as i32 { 0 as integer } else { 1 as integer };\n                Self::set_character_protrusion(\n                    self,\n                    f as integer,\n                    p as u32,\n                    side,\n                    self.state.curval,\n                );\n            }\n        }\n",
    )
}


pub(crate) fn patch_font_int_query_arm(source: String) -> String {
    let source = source.replace(
        "let mut p: integer = 0;\n    m = self.state.curchr;",
        "let mut p: integer = 0;\n    let mut n: integer = 0;\n    let mut k: integer = 0;\n    m = self.state.curchr;",
    );
    replace_match_arm(
        source,
        "        78 => {",
        "        89 => {",
        "        78 => {\n            (&mut *(self as *mut PortableTexEngine<'_>)).scanfontident();\n            if m == 0 as i32 {\n                self.state.curval = *self.state.hyphenchar.offset(self.state.curval as isize);\n                self.state.curvallevel = 0 as eightbits;\n            } else if m == 1 as i32 {\n                self.state.curval = *self.state.skewchar.offset(self.state.curval as isize);\n                self.state.curvallevel = 0 as eightbits;\n            } else {\n                n = self.state.curval;\n                if self.supports_native_fonts()\n                    && (*self.state.fontarea.offset(n as isize) as i64 == 65535 as i64\n                        || *self.state.fontarea.offset(n as isize) as i64 == 65534 as i64)\n                {\n                    (&mut *(self as *mut PortableTexEngine<'_>)).zscanglyphnumber(n);\n                } else {\n                    (&mut *(self as *mut PortableTexEngine<'_>)).scancharnum();\n                }\n                k = self.state.curval;\n                let side = if m == 2 as i32 { 0 as integer } else { 1 as integer };\n                self.state.curval = Self::get_character_protrusion(\n                    self as *mut PortableTexEngine<'_>,\n                    n as integer,\n                    k as u32,\n                    side,\n                );\n                self.state.curvallevel = 0 as eightbits;\n            }\n        }\n",
    )
}


pub(crate) fn guard_xetex_patch_function(_function_name: &str, source: String) -> String {
    // Reachability from the TeX core decides whether a XeTeX body is shared or profile gated.
    source
}


/// Classify a XeTeX only shared symbol by provenance and reachability from the TeX core.
pub(crate) fn profile_gated_origin(
    function_name: &str,
    reachable_from_tex_core: &BTreeSet<String>,
) -> SharedSourceOrigin {
    if matches!(function_name, "scanpdfexttoks" | "zpdferror") {
        SharedSourceOrigin::StrippedPdfExtension
    } else if reachable_from_tex_core.contains(function_name) {
        SharedSourceOrigin::XetexWidenedDuplicate
    } else {
        SharedSourceOrigin::XetexOnlyProfileGated
    }
}


pub(crate) fn patched_shared_origin(function_name: &str, fallback: SharedSourceOrigin) -> SharedSourceOrigin {
    match function_name {
        "appendsrcspecial" | "insertsrcspecial" => SharedSourceOrigin::StrippedSourceSpecial,
        "zprintwritewhatsit" => SharedSourceOrigin::StrippedWriteWhatsitDiagnostic,
        "scanpdfexttoks" | "zpdferror" => SharedSourceOrigin::StrippedPdfExtension,
        "openorclosein" | "startinput" => SharedSourceOrigin::AdaptedNativeIo,
        _ => fallback,
    }
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


pub(crate) fn prefix_function_calls(source: String, translated_functions: &BTreeSet<String>) -> String {
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


/// Repairs single character literals that the lexical [`prefix_globals`] scanner mangled.
pub(crate) fn repair_mangled_char_literals(mut source: String) -> String {
    for letter in ['c', 'f', 'g', 'l'] {
        source = source.replace(
            &format!("'self.state.{letter}'"),
            &format!("'{letter}'"),
        );
    }
    source
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

        let identifier = &source[start..index];
        let previous = source[..start].chars().next_back();
        let next = source[index..].chars().next();
        let _ = identifier;
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

//! Registry for every rename and boundary rewrite the patcher applies to translated TeX functions.

/// C type aliases from `c2rust`, rewritten to engine scalar type names.
pub(crate) const C_TYPE_ALIASES: &[(&str, &str)] = &[
    ("::core::ffi::c_int", "i32"),
    ("::core::ffi::c_long", "i64"),
    ("::core::ffi::c_short", "i16"),
    ("::core::ffi::c_uchar", "eightbits"),
    ("::core::ffi::c_schar", "i8"),
    ("::core::ffi::c_ushort", "u16"),
    ("::core::ffi::c_uint", "u32"),
    ("::core::ffi::c_char", "i8"),
    ("::core::ffi::c_double", "f64"),
    ("::core::ffi::c_void", "()"),
    ("XeTeXLayoutEngine", "FontHandle"),
];

/// Upstream resource search renames applied as ordered substring replaces.
pub(crate) const RESOURCE_SEARCH_RENAMES: &[(&str, &str)] = &[
    (
        "self.state.kpse_def_inst",
        "self.state.resource_search_state",
    ),
    ("kpse_tex_format", "resource_format_tex_input"),
    ("kpse_tfm_format", "resource_format_tfm"),
    ("kpse_fmt_format", "resource_format_format_image"),
    ("kpse_cnf_format", "resource_format_config"),
    ("kpse_fontmap_format", "resource_format_font_map"),
    ("kpse_enc_format", "resource_format_encoding"),
    ("kpse_cmap_format", "resource_format_encoding"),
    ("kpse_type1_format", "resource_format_font"),
    ("kpse_truetype_format", "resource_format_font"),
    ("kpse_opentype_format", "resource_format_font"),
    ("kpse_pdftex_config_format", "resource_format_config"),
    ("kpse_dvips_config_format", "resource_format_config"),
    ("kpathsea_instance", "ResourceSearchState"),
    ("kpathsea", "ResourceSearchHandle"),
];

/// State field renames keyed by the bare translated global name.
const STATE_FIELD_RENAMES: &[(&str, &str)] = &[
    ("__stdinp", "terminal_input"),
    ("__stdoutp", "terminal_output"),
    ("kpse_def_inst", "resource_search_state"),
];

/// Terminal field access renames applied as substring replaces over function bodies.
pub(crate) const TERMINAL_IO_ACCESS_RENAMES: &[(&str, &str)] = &[
    ("self.state.__stdinp", "self.state.terminal_input"),
    ("self.state.__stdoutp", "self.state.terminal_output"),
];

/// Host service call rewrites applied after `Self::host_` prefixing.
pub(crate) const HOST_SERVICE_CALL_RENAMES: &[(&str, &str)] = &[
    (
        "Self::host_aatgetfontmetrics(",
        "Self::measure_font_metrics(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_findnativefont(",
        "Self::resolve_font_handle(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_get_cp_code(",
        "Self::get_character_protrusion(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_get_native_glyph(",
        "Self::get_native_glyph(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_native_mathex_param(",
        "Self::get_native_mathex_parameter(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_native_mathsy_param(",
        "Self::get_native_mathsy_parameter(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_native_word_cp(",
        "Self::get_native_word_cp(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_ot_math_ital_corr(",
        "Self::get_ot_math_ital_corr(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_ot_math_variant(",
        "Self::get_ot_math_variant(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_ot_math_kern(",
        "Self::get_ot_math_kern(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_ot_assembly_ptr(",
        "Self::get_ot_assembly_ptr(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_getencodingmodeandinfo(",
        "Self::get_encoding_mode_and_info(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_get_ot_math_accent_pos(",
        "Self::get_opentype_math_accent_position(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_get_ot_math_constant(",
        "Self::get_opentype_math_constant(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_get_seconds_and_micros(",
        "Self::get_seconds_and_micros(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_isOpenTypeMathFont(",
        "Self::is_opentype_math_font(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_usingOpenType(",
        "Self::using_opentype(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_linebreaknext(",
        "Self::linebreak_next(self as *mut PortableTexEngine<'_>",
    ),
    (
        "Self::host_linebreakstart(",
        "Self::linebreak_start(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_mapchartoglyph(",
        "Self::map_char_to_glyph(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_mapglyphtoindex(",
        "Self::map_glyph_to_index(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_measure_native_glyph(",
        "Self::measure_native_glyph(self as *mut PortableTexEngine<'resources>, ",
    ),
    (
        "Self::host_measure_native_node(",
        "Self::measure_native_node(self as *mut PortableTexEngine<'resources>,",
    ),
    (
        "Self::host_ot_min_connector_overlap(",
        "Self::opentype_min_connector_overlap(",
    ),
    ("Self::host_ot_part_count(", "Self::opentype_part_count("),
    (
        "Self::host_ot_part_end_connector(",
        "Self::opentype_part_end_connector(",
    ),
    (
        "Self::host_ot_part_full_advance(",
        "Self::opentype_part_full_advance(",
    ),
    ("Self::host_ot_part_glyph(", "Self::opentype_part_glyph("),
    (
        "Self::host_ot_part_is_extender(",
        "Self::opentype_part_is_extender(",
    ),
    (
        "Self::host_ot_part_start_connector(",
        "Self::opentype_part_start_connector(",
    ),
    (
        "Self::host_otgetfontmetrics(",
        "Self::measure_opentype_font_metrics(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_releasefontengine(",
        "Self::release_font_engine(self as *mut PortableTexEngine<'_>, ",
    ),
    (
        "Self::host_uexit(",
        "Self::abort_engine(self as *mut PortableTexEngine<'_>, ",
    ),
];

/// Translated functions resolved against host provided services.
pub(crate) const HOST_FUNCTIONS: &[&str] = &[
    "aatgetfontmetrics",
    "findnativefont",
    "get_cp_code",
    "get_native_glyph",
    "get_native_mathex_param",
    "get_native_mathsy_param",
    "get_native_word_cp",
    "get_ot_math_ital_corr",
    "get_ot_math_variant",
    "get_ot_math_kern",
    "get_ot_assembly_ptr",
    "get_seconds_and_micros",
    "getencodingmodeandinfo",
    "get_ot_math_accent_pos",
    "get_ot_math_constant",
    "isOpenTypeMathFont",
    "usingOpenType",
    "linebreaknext",
    "linebreakstart",
    "mapchartoglyph",
    "mapglyphtoindex",
    "measure_native_glyph",
    "measure_native_node",
    "ot_min_connector_overlap",
    "ot_part_count",
    "ot_part_end_connector",
    "ot_part_full_advance",
    "ot_part_glyph",
    "ot_part_is_extender",
    "ot_part_start_connector",
    "otgetfontmetrics",
    "releasefontengine",
    "uexit",
];

pub(crate) const PORTABLE_BOUNDARY_CALLS: &[(&str, &str)] = &[
    ("openlogfile", "boundary_open_log_file"),
    ("jumpout", "boundary_jump_out"),
    ("zshipout", "boundary_shipout"),
    ("buildpage", "boundary_build_page"),
    ("zprunepagetop", "boundary_prune_page_top"),
    ("zoutwhat", "boundary_special_out"),
    ("zloadpicture", "boundary_load_picture"),
    ("open_input", "boundary_open_input"),
    ("input_line", "boundary_input_line"),
    ("getc", "boundary_read_byte"),
    ("feof", "boundary_end_of_file"),
    ("fflush", "boundary_flush_file"),
    ("putc", "boundary_write_byte"),
    ("close_file", "boundary_close_file"),
    ("close_file_or_pipe", "boundary_close_file"),
    ("getfilesize", "boundary_get_file_size"),
];

/// Boundary calls that take a leading `self as *mut PortableTexEngine<'_>` argument.
pub(crate) const RECEIVER_BOUNDARY_CALLS: &[&str] = &[
    "boundary_open_input",
    "boundary_input_line",
    "boundary_shipout",
    "boundary_build_page",
    "boundary_prune_page_top",
    "boundary_special_out",
    "boundary_load_picture",
    "boundary_open_log_file",
    "boundary_jump_out",
    "boundary_write_byte",
    "boundary_get_file_size",
];

/// Returns the renamed `PortableTexState` field for a translated global, or the original name.
pub(crate) fn state_field_name(name: &str) -> &str {
    for (from, to) in STATE_FIELD_RENAMES {
        if name == *from {
            return to;
        }
    }
    name
}

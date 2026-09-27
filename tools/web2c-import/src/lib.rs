//! Imports the Web2C/C2Rust translation of XeTeX into the portable engine crate.

mod astpass;
mod boundary;
mod codegen;
mod extract;
mod flow;
mod inputs;
mod model;
mod orchestrate;
mod pin;
mod select;
mod synpass;
mod transform;

pub use model::PatchError;
pub use orchestrate::run;

/// Core TeX functions the selection closure starts from.
const CORE_SELECTION_ROOTS: &[&str] = &[
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

/// XeTeX extension functions the selection closure starts from.
const XETEX_SELECTION_ROOTS: &[&str] = &[
    "maxhyphenatablelength",
    "scanusvnum",
    "zbuildopentypeassembly",
    "zloadnativefont",
    "zmathxheight",
    "znewnativecharacter",
    "znewnativewordnode",
];

/// Translated functions that must never be linked, with the reason printed in the patch report.
const FORBIDDEN_SYMBOLS: &[(&str, &str)] = &[
    ("lua_initialize", "LuaRuntime"),
    ("lua_close", "LuaRuntime"),
    ("luaopen_tex", "LuaRuntime"),
    ("callback_register", "CallbackOrNodeFilter"),
    ("run_callback", "CallbackOrNodeFilter"),
    ("node_filter", "CallbackOrNodeFilter"),
    ("ship_out", "ShipoutOrDriver"),
    ("shipout", "ShipoutOrDriver"),
    ("dvi_out", "ShipoutOrDriver"),
    ("pdf_ship_out", "ShipoutOrDriver"),
    ("zshipout", "ShipoutOrDriver"),
    ("hlistout", "ShipoutOrDriver"),
    ("vlistout", "ShipoutOrDriver"),
    ("dviswap", "ShipoutOrDriver"),
    ("zdvifour", "ShipoutOrDriver"),
    ("zdvitwo", "ShipoutOrDriver"),
    ("zdvipop", "ShipoutOrDriver"),
    ("zdvifontdef", "ShipoutOrDriver"),
    ("zdvinativefontdef", "ShipoutOrDriver"),
    ("zmovement", "ShipoutOrDriver"),
    ("zprunemovements", "ShipoutOrDriver"),
    ("zspecialout", "ShipoutOrDriver"),
    ("zwriteout", "ShipoutOrDriver"),
    ("zpicout", "ShipoutOrDriver"),
    ("zloadpicture", "ShipoutOrDriver"),
    ("zoutwhat", "ShipoutOrDriver"),
    ("zfireup", "ShipoutOrDriver"),
    ("buildpage", "ShipoutOrDriver"),
    ("zprunepagetop", "ShipoutOrDriver"),
    ("zfreezepagespecs", "ShipoutOrDriver"),
    ("kpse_find_file", "NativeIo"),
    ("kpse_in_name_ok", "NativeIo"),
    ("kpse_out_name_ok", "NativeIo"),
    ("fopen", "NativeIo"),
    ("open_input", "NativeIo"),
    ("open_output", "NativeIo"),
    ("open_out_or_pipe", "NativeIo"),
    ("close_file", "NativeIo"),
    ("close_file_or_pipe", "NativeIo"),
    ("openlogfile", "NativeIo"),
    ("startinput", "NativeIo"),
    ("openorclosein", "NativeIo"),
    ("openfmtfile", "NativeIo"),
    ("loadfmtfile", "NativeIo"),
    ("storefmtfile", "NativeIo"),
    ("closefilesandterminate", "NativeIo"),
    ("jumpout", "NativeIo"),
    ("uexit", "NativeIo"),
    ("exit", "NativeIo"),
    ("runsystem", "NativeIo"),
    ("system", "NativeIo"),
];

fn forbidden_reason(symbol: &str) -> Option<&'static str> {
    FORBIDDEN_SYMBOLS
        .iter()
        .find_map(|(forbidden, reason)| (*forbidden == symbol).then_some(*reason))
}

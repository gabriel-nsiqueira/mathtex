#!/usr/bin/env bash
set -eu

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
generated_root="$repo_root/generated/c2rust"
tmp_dir="${TMPDIR:-/tmp}/mathtex-translated-audit.$$"
inventory_output=""

if [ "${1:-}" = "--write-symbol-inventory" ]; then
    if [ "$#" -ne 2 ]; then
        echo "usage: $0 [--write-symbol-inventory path]" >&2
        exit 1
    fi
    inventory_output="$2"
elif [ "$#" -ne 0 ]; then
    echo "usage: $0 [--write-symbol-inventory path]" >&2
    exit 1
fi

trap 'rm -rf "$tmp_dir"' EXIT
mkdir -p "$tmp_dir"

extract_symbols() {
    engine="$1"
    output="$2"

    if [ ! -d "$generated_root/$engine/src" ]; then
        echo "missing generated C2Rust source for $engine at $generated_root/$engine/src" >&2
        exit 1
    fi

    rg -N '^(pub unsafe extern "C" fn|pub static mut) ' "$generated_root/$engine/src" |
        sed -E 's#^.*:(pub unsafe extern "C" fn|pub static mut) ([A-Za-z_][A-Za-z0-9_]*).*$#\2#' |
        sort -u >"$output"
}

count_lines() {
    wc -l <"$1" | tr -d ' '
}

write_symbol_inventory() {
    output="$1"
    mkdir -p "$(dirname "$output")"
    {
        printf 'engine\tsymbol\n'
        awk '{ printf "tex\t%s\n", $0 }' "$tmp_dir/tex.symbols"
        awk '{ printf "xetex\t%s\n", $0 }' "$tmp_dir/xetex.symbols"
    } >"$output"
}

extract_symbols tex "$tmp_dir/tex.symbols"
extract_symbols xetex "$tmp_dir/xetex.symbols"
comm -12 "$tmp_dir/tex.symbols" "$tmp_dir/xetex.symbols" >"$tmp_dir/duplicate.symbols"
comm -23 "$tmp_dir/xetex.symbols" "$tmp_dir/tex.symbols" >"$tmp_dir/xetex-only.symbols"

cat >"$tmp_dir/forbidden-patterns" <<'PATTERNS'
callback
close_file
close_file_or_pipe
closefilesandterminate
dvi
fopen
hlistout
jumpout
kpse_
loadfmtfile
lua
openfmtfile
open_input
open_output
open_out_or_pipe
openlogfile
openorclosein
pdf_ship
run_callback
runsystem
shipout
ship_out
startinput
storefmtfile
system
vlistout
zdvi
zfireup
zfreezepagespecs
zoutwhat
zpicout
zprunemovements
zprunepagetop
zspecialout
zwriteout
zshipout
PATTERNS

printf 'translated bootstrap symbol audit\n'
printf 'tex symbols: %s\n' "$(count_lines "$tmp_dir/tex.symbols")"
printf 'xetex symbols: %s\n' "$(count_lines "$tmp_dir/xetex.symbols")"
printf 'duplicate tex/xetex symbols: %s\n' "$(count_lines "$tmp_dir/duplicate.symbols")"
printf 'xetex-only symbols: %s\n' "$(count_lines "$tmp_dir/xetex-only.symbols")"

if [ -n "$inventory_output" ]; then
    write_symbol_inventory "$inventory_output"
    printf 'symbol inventory: %s\n' "$inventory_output"
fi

printf '\nfirst duplicate symbols that must stay shared/profile-gated:\n'
sed -n '1,25p' "$tmp_dir/duplicate.symbols"

printf '\nfirst xetex-only symbols to audit as profile-gated patches:\n'
sed -n '1,25p' "$tmp_dir/xetex-only.symbols"

printf '\nraw bootstrap forbidden-symbol matches:\n'
if rg -n -f "$tmp_dir/forbidden-patterns" "$generated_root/tex/src" "$generated_root/xetex/src" \
    >"$tmp_dir/forbidden.matches"; then
    sed -n '1,60p' "$tmp_dir/forbidden.matches"
    printf 'forbidden matches total: %s\n' "$(count_lines "$tmp_dir/forbidden.matches")"
else
    printf 'none\n'
fi

if [ "$(count_lines "$tmp_dir/duplicate.symbols")" -eq 0 ]; then
    echo "expected repeated translated TeX symbols between raw TeX and XeTeX outputs" >&2
    exit 1
fi

if ! rg -q 'BootstrapOnly' "$repo_root/tools/web2c-import/src/lib.rs"; then
    echo "raw generated XeTeX outputs are not marked BootstrapOnly in the import tooling recipe" >&2
    exit 1
fi

printf '\naudit result: raw XeTeX output overlaps TeX and must remain bootstrap-only until ported into shared profile-gated code\n'

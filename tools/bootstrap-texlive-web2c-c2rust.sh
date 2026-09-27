#!/usr/bin/env bash
# Rebuilds the XeTeX web2c and C2Rust inputs in the web2c cache, pins them, and regenerates the engine.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
texlive_root="$repo_root/vendor/texlive-source"
web2c_root="$texlive_root/texk/web2c"
pin_file="$repo_root/tools/web2c-import/pin.toml"

# Same resolution as tools/web2c-import/src/inputs.rs, gitignored under cache/.
cache="${MATHTEX_WEB2C_CACHE:-$repo_root/cache/web2c}"

fail() {
    echo "bootstrap: $*" >&2
    exit 1
}

cc="${CC:-cc}"
missing=()
for tool in tie otangle c2rust pkg-config cargo python3 make git "$cc"; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
[ "${#missing[@]}" -eq 0 ] || fail "missing required tools: ${missing[*]}"
cargo +nightly --version >/dev/null 2>&1 || fail "cargo +nightly is required, install it with: rustup toolchain install nightly"

xetex_packages=(freetype2 harfbuzz graphite2 icu-uc icu-io)
missing=()
for package in "${xetex_packages[@]}"; do
    pkg-config --exists "$package" || missing+=("$package")
done
[ "${#missing[@]}" -eq 0 ] ||
    fail "pkg-config cannot find: ${missing[*]}, add their .pc directories to PKG_CONFIG_PATH"
xetex_cflags="$(pkg-config --cflags "${xetex_packages[@]}")"

[ -d "$web2c_root" ] ||
    fail "missing TeX Live source at $web2c_root, clone https://github.com/TeX-Live/texlive-source.git into vendor/texlive-source"

# TIE exits 1 after printing its banner for `--version`, so only the first output line is used.
version_line() {
    local output
    output="$("$@" 2>&1)" || [ -n "$output" ] || fail "cannot read the version of $1"
    printf '%s\n' "${output%%$'\n'*}"
}

if [ ! -x "$web2c_root/web2c/web2c" ]; then
    (
        cd "$web2c_root/web2c"
        ./configure --disable-shared
        make web2c fixwrites splitup makecpool
    )
fi

web2c_out="$cache/web2c"
c2rust_out="$cache/c2rust/xetex"
mkdir -p "$web2c_out/include/teckit" "$web2c_out/include/w2c" "$web2c_out/xetex" "$c2rust_out"

cp "$web2c_root/web2c/kpathsea/c-auto.h" "$web2c_out/include/w2c/c-auto.h"
cp "$texlive_root/libs/teckit/TECkit-src/source/Public-headers/TECkit_Common.h" \
    "$web2c_out/include/teckit/TECkit_Common.h"

(
    cd "$web2c_root"
    tie -c "$web2c_out/xetex/xetex-final.ch" \
        xetexdir/xetex.web \
        xetexdir/tex.ch0 \
        tex.ch \
        tracingstacklevels.ch \
        partoken-102.ch \
        partoken.ch \
        locnull-optimize.ch \
        unbalanced-braces.ch \
        showstream.ch \
        xetexdir/xetex.ch \
        xetexdir/char-warning-xetex.ch \
        tex-binpool.ch \
        "$repo_root/tools/web2c-import/changes/src-tracking.ch"
    otangle xetexdir/xetex.web "$web2c_out/xetex/xetex-final.ch"
    cp xetex.p xetex.pool "$web2c_out/xetex/"
    srcdir=. ./web2c/convert xetex
    cp xetex0.c xetexini.c xetexcoerce.h xetexd.h "$web2c_out/xetex/"
)

# Keep XETEX_MAC undefined so XeTeX uses the generic FreeType, fontconfig and HarfBuzz path.
read -r -a xetex_cflag_words <<<"$xetex_cflags"
c2rust transpile --emit-build-files --overwrite-existing \
    -o "$c2rust_out" \
    "$web2c_out/xetex/xetex0.c" \
    "$web2c_out/xetex/xetexini.c" \
    -- \
    -I"$web2c_out/include" \
    -I"$texlive_root/texk" \
    -I"$texlive_root/texk/kpathsea" \
    -I"$web2c_root" \
    -I"$web2c_root/web2c" \
    -I"$web2c_root/web2c/kpathsea" \
    -I"$web2c_out/xetex" \
    -UXETEX_MAC \
    ${xetex_cflag_words[@]+"${xetex_cflag_words[@]}"}

# XeTeX_ext.h supplies generic CFDictionaryRef and Apple font stubs when XETEX_MAC is unset.
macos_symbols='CoreFoundation|ApplicationServices|CoreText|Cocoa|CTFontRef|CTFontDescriptorRef|CFArrayRef|CFStringRef|CFNumberRef|CFBooleanRef|XeTeXFontMgr_Mac|XeTeXFontInst_Mac'
if grep -R -n -E "$macos_symbols" "$web2c_out/xetex" "$c2rust_out/src"; then
    fail "the XeTeX bootstrap imported the XETEX_MAC CoreText profile, keep XETEX_MAC undefined"
fi

cargo +nightly check --manifest-path "$c2rust_out/Cargo.toml"

texlive_commit="$(git -C "$texlive_root" rev-parse HEAD)"
c2rust_version="$(version_line c2rust --version)"
tie_version="$(version_line tie --version)"
otangle_version="$(version_line otangle --version)"
cat >"$pin_file" <<PIN
# Upstream inputs of the portable engine. The bootstrap script writes this file and the patcher checks it.
[upstream]
texlive_source = "$texlive_commit"
c2rust = "$c2rust_version"
tie = "$tie_version"
otangle = "$otangle_version"
PIN

MATHTEX_WEB2C_CACHE="$cache" MATHTEX_WEB2C_UPDATE_PIN=1 "$repo_root/tools/patch-translated-engine.sh"

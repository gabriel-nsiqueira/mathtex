#!/usr/bin/env bash
set -eu

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web2c_root="$repo_root/vendor/texlive-source/texk/web2c"
generated_root="$repo_root/generated"

if [ ! -d "$web2c_root" ]; then
    echo "missing TeX Live source at $web2c_root" >&2
    echo "clone https://github.com/TeX-Live/texlive-source.git into vendor/texlive-source first" >&2
    exit 1
fi

if [ ! -x "$web2c_root/web2c/web2c" ]; then
    (
        cd "$web2c_root/web2c"
        ./configure --disable-shared
        make web2c fixwrites splitup makecpool
    )
fi

mkdir -p \
    "$generated_root/web2c/include/teckit" \
    "$generated_root/web2c/include/w2c" \
    "$generated_root/web2c/etex" \
    "$generated_root/web2c/tex" \
    "$generated_root/web2c/xetex" \
    "$generated_root/c2rust"

cp "$web2c_root/web2c/kpathsea/c-auto.h" "$generated_root/web2c/include/w2c/c-auto.h"
cp "$repo_root/vendor/texlive-source/libs/teckit/TECkit-src/source/Public-headers/TECkit_Common.h" \
    "$generated_root/web2c/include/teckit/TECkit_Common.h"

(
    cd "$web2c_root"

    tie -c "$generated_root/web2c/tex/tex-final.ch" \
        tex.web \
        tex.ch \
        tex-binpool.ch \
        "$repo_root/tools/web2c-import/changes/src-tracking.ch"
    tangle tex.web "$generated_root/web2c/tex/tex-final.ch"
    cp tex.p tex.pool "$generated_root/web2c/tex/"
    srcdir=. ./web2c/convert tex
    cp tex0.c texini.c texcoerce.h texd.h "$generated_root/web2c/tex/"

    tie -m etex.web \
        tex.web \
        etexdir/etex.ch
    tie -c etex.ch \
        etex.web \
        etexdir/tex.ch0 \
        tex.ch \
        zlib-fmt.ch \
        enctexdir/enctex1.ch \
        enctexdir/enctex-tex.ch \
        enctexdir/enctex2.ch \
        etexdir/tex.ch1 \
        etexdir/tex.ech \
        tex-binpool.ch \
        "$repo_root/tools/web2c-import/changes/src-tracking.ch"
    tangle etex.web etex.ch
    cp etex.web etex.ch etex.p etex.pool "$generated_root/web2c/etex/"
    srcdir=. ./web2c/convert etex
    cp etex0.c etexini.c etexcoerce.h etexd.h "$generated_root/web2c/etex/"

    tie -c "$generated_root/web2c/xetex/xetex-final.ch" \
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
    otangle xetexdir/xetex.web "$generated_root/web2c/xetex/xetex-final.ch"
    cp xetex.p xetex.pool "$generated_root/web2c/xetex/"
    srcdir=. ./web2c/convert xetex
    cp xetex0.c xetexini.c xetexcoerce.h xetexd.h "$generated_root/web2c/xetex/"
)

common_includes=(
    -I"$generated_root/web2c/include"
    -I"$repo_root/vendor/texlive-source/texk"
    -I"$repo_root/vendor/texlive-source/texk/kpathsea"
    -I"$web2c_root"
    -I"$web2c_root/web2c"
    -I"$web2c_root/web2c/kpathsea"
)

c2rust transpile --emit-build-files --overwrite-existing \
    -o "$generated_root/c2rust/tex" \
    "$generated_root/web2c/tex/tex0.c" \
    "$generated_root/web2c/tex/texini.c" \
    -- \
    "${common_includes[@]}" \
    -I"$generated_root/web2c/tex"

c2rust transpile --emit-build-files --overwrite-existing \
    -o "$generated_root/c2rust/etex" \
    "$generated_root/web2c/etex/etex0.c" \
    "$generated_root/web2c/etex/etexini.c" \
    -- \
    "${common_includes[@]}" \
    -I"$generated_root/web2c/etex"

xetex_cflags="$(PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}:/opt/homebrew/opt/icu4c@78/lib/pkgconfig" \
    pkg-config --cflags freetype2 harfbuzz graphite2 icu-uc icu-io 2>/dev/null || true)"

# Keep XETEX_MAC undefined so XeTeX uses the generic FreeType/fontconfig/HarfBuzz path.
c2rust transpile --emit-build-files --overwrite-existing \
    -o "$generated_root/c2rust/xetex" \
    "$generated_root/web2c/xetex/xetex0.c" \
    "$generated_root/web2c/xetex/xetexini.c" \
    -- \
    "${common_includes[@]}" \
    -I"$generated_root/web2c/xetex" \
    -UXETEX_MAC \
    $xetex_cflags

# XeTeX_ext.h supplies generic CFDictionaryRef and Apple font stubs when XETEX_MAC is unset.
xetex_macos_profile_symbols='CoreFoundation|ApplicationServices|CoreText|Cocoa|CTFontRef|CTFontDescriptorRef|CFArrayRef|CFStringRef|CFNumberRef|CFBooleanRef|XeTeXFontMgr_Mac|XeTeXFontInst_Mac'
if grep -R -n -E "$xetex_macos_profile_symbols" \
    "$generated_root/web2c/xetex" \
    "$generated_root/c2rust/xetex/src"
then
    echo "XeTeX bootstrap imported the XETEX_MAC/CoreText profile; keep XETEX_MAC undefined for portable c2rust input" >&2
    exit 1
fi

cargo +nightly check --manifest-path "$generated_root/c2rust/tex/Cargo.toml"
cargo +nightly check --manifest-path "$generated_root/c2rust/etex/Cargo.toml"
cargo +nightly check --manifest-path "$generated_root/c2rust/xetex/Cargo.toml"
"$repo_root/tools/audit-translated-bootstrap.sh"
"$repo_root/tools/patch-translated-engine.sh"

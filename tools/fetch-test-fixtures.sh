#!/usr/bin/env bash
# Fetches the texlive-source files the tests read, at the commit tools/web2c-import/pin.toml pins.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin_file="$repo_root/tools/web2c-import/pin.toml"
remote="https://github.com/TeX-Live/texlive-source.git"

# Same resolution as tools/test-fixtures, gitignored under cache/.
dest="${MATHTEX_TEXLIVE_SOURCE:-$repo_root/cache/texlive-source}"

# Keep in sync with FILES in tools/test-fixtures, a test there checks it.
fixtures=(
    texk/dvipsk/testdata/cmr10.tfm
    texk/dvipsk/testdata/cmmi10.tfm
    texk/dvipsk/testdata/cmsy10.tfm
    texk/dvipsk/testdata/cmex10.tfm
    libs/gd/libgd-src/tests/freetype/DejaVuSans.ttf
    texk/dvisvgm/dvisvgm-src/tests/data/lmmono12-regular.otf
)

fail() {
    echo "fetch-test-fixtures: $*" >&2
    exit 1
}

command -v git >/dev/null 2>&1 || fail "git is required"
[ -f "$pin_file" ] || fail "missing $pin_file"
commit="$(sed -n 's/^texlive_source = "\([0-9a-f]\{40\}\)"$/\1/p' "$pin_file")"
[ -n "$commit" ] || fail "$pin_file has no texlive_source commit"

missing_fixtures() {
    local path
    for path in "${fixtures[@]}"; do
        [ -f "$dest/$path" ] || echo "$path"
    done
}

head_commit() {
    git -C "$dest" rev-parse --verify --quiet HEAD || true
}

write_sparse_paths() {
    local git_dir
    git_dir="$(git -C "$dest" rev-parse --absolute-git-dir)"
    mkdir -p "$git_dir/info"
    printf '/%s\n' "${fixtures[@]}" >"$git_dir/info/sparse-checkout"
}

checkout_pinned() {
    git -C "$dest" fetch --quiet --depth 1 --filter=blob:none origin "$commit"
    git -C "$dest" -c advice.detachedHead=false checkout --quiet --detach "$commit"
}

if [ -e "$dest" ] && [ ! -d "$dest" ]; then
    fail "$dest exists and is not a directory"
fi

if [ -d "$dest" ] && [ "$(git -C "$dest" rev-parse --show-toplevel 2>/dev/null || true)" = "$(cd "$dest" && pwd -P)" ]; then
    if [ "$(git -C "$dest" config --get mathtex.fixtures || true)" != "true" ]; then
        # A clone this script did not make, such as vendor/texlive-source, is only checked.
        [ "$(head_commit)" = "$commit" ] ||
            fail "$dest is a git checkout at $(head_commit), not $commit, and this script only changes clones it made"
        missing="$(missing_fixtures)"
        [ -z "$missing" ] || fail "$dest lacks fixtures, check them out there:"$'\n'"$missing"
        echo "fetch-test-fixtures: $dest is at $commit with every fixture"
        exit 0
    fi
    write_sparse_paths
    if [ "$(head_commit)" = "$commit" ]; then
        git -C "$dest" sparse-checkout reapply
    else
        echo "fetch-test-fixtures: moving $dest to $commit"
        checkout_pinned
    fi
elif [ -d "$dest" ] && [ -n "$(ls -A "$dest")" ]; then
    fail "$dest is not empty and not a git clone, remove it or set MATHTEX_TEXLIVE_SOURCE elsewhere"
else
    echo "fetch-test-fixtures: cloning $commit into $dest"
    mkdir -p "$dest"
    git -C "$dest" init --quiet
    git -C "$dest" config mathtex.fixtures true
    git -C "$dest" remote add origin "$remote"
    git -C "$dest" config remote.origin.promisor true
    git -C "$dest" config remote.origin.partialclonefilter blob:none
    git -C "$dest" config core.sparseCheckout true
    git -C "$dest" config core.sparseCheckoutCone false
    write_sparse_paths
    checkout_pinned
fi

[ "$(head_commit)" = "$commit" ] || fail "$dest ended at $(head_commit), not $commit"
missing="$(missing_fixtures)"
[ -z "$missing" ] || fail "the checkout at $dest lacks:"$'\n'"$missing"
echo "fetch-test-fixtures: $dest is at $commit with every fixture"

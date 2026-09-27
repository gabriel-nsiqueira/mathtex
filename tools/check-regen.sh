#!/usr/bin/env bash
# Regenerates the engine into a scratch tree and exits 1 if it differs from generated/portable-engine.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
engine="$repo_root/generated/portable-engine"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/mathtex-check-regen.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

# The patcher reads only the crate version from the existing manifest, so every other file is regenerated.
mkdir -p "$scratch/generated/portable-engine"
cp "$engine/Cargo.toml" "$scratch/generated/portable-engine/Cargo.toml"

cargo run --quiet --manifest-path "$repo_root/tools/web2c-import/Cargo.toml" --bin patch_engine -- "$scratch"

# The license files come from tools/generate_third_party_licenses.py, not the patcher.
if diff -ru --exclude target --exclude LICENSE --exclude THIRD-PARTY-LICENSES.json \
    "$engine" "$scratch/generated/portable-engine"; then
    echo "check-regen: generated/portable-engine matches a fresh regeneration"
else
    echo "check-regen: generated/portable-engine differs from a fresh regeneration" >&2
    exit 1
fi

#!/usr/bin/env bash
# Regenerates generated/portable-engine from the web2c cache, then refreshes its license files.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo run --quiet --manifest-path "$repo_root/tools/web2c-import/Cargo.toml" --bin patch_engine -- "$repo_root"

# The license generator copies upstream notices out of vendor/texlive-source, so it needs the clone.
if [ -d "$repo_root/vendor/texlive-source/texk/web2c" ]; then
    python3 "$repo_root/tools/generate_third_party_licenses.py"
else
    echo "patch: vendor/texlive-source is missing, LICENSE and THIRD-PARTY-LICENSES.json were not refreshed" >&2
fi

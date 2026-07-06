#!/usr/bin/env bash
set -eu

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo run -p mathtex-web2c-import --bin patch_engine -- "$repo_root"
python3 "$repo_root/tools/generate_third_party_licenses.py"

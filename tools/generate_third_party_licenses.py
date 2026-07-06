#!/usr/bin/env python3
"""Extract upstream TeX Live WEB license headers into THIRD-PARTY-LICENSES.json."""

import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WEB2C_ROOT = REPO_ROOT / "vendor/texlive-source/texk/web2c"
OUT_FILE = REPO_ROOT / "generated/portable-engine/THIRD-PARTY-LICENSES.json"
LICENSE_FILE = REPO_ROOT / "generated/portable-engine/LICENSE"

# WEB and change file inputs per profile, with src-tracking.ch excluded as project code.
PROFILE_SOURCES = {
    "tex": [
        "tex.web",
        "tex.ch",
        "tex-binpool.ch",
    ],
    "etex": [
        "tex.web",
        "etexdir/etex.ch",
        "etexdir/tex.ch0",
        "tex.ch",
        "zlib-fmt.ch",
        "enctexdir/enctex1.ch",
        "enctexdir/enctex-tex.ch",
        "enctexdir/enctex2.ch",
        "etexdir/tex.ch1",
        "etexdir/tex.ech",
        "tex-binpool.ch",
    ],
    "xetex": [
        "xetexdir/xetex.web",
        "xetexdir/tex.ch0",
        "tex.ch",
        "tracingstacklevels.ch",
        "partoken-102.ch",
        "partoken.ch",
        "locnull-optimize.ch",
        "unbalanced-braces.ch",
        "showstream.ch",
        "xetexdir/xetex.ch",
        "xetexdir/char-warning-xetex.ch",
        "tex-binpool.ch",
    ],
}

# Subset of PROFILE_SOURCES with a copyright or license notice.
LICENSED_SOURCES = {
    "tex.web",
    "tex.ch",
    "etexdir/etex.ch",
    "xetexdir/xetex.web",
    "xetexdir/xetex.ch",
}

# Capture stops at the WEB front matter sentinel, then trims $Id$ and flanking blank lines.
ID_KEYWORD_RE = re.compile(r"^%[ \t]*\$Id.*\$[ \t]*$")
FRONT_MATTER_END_RE = re.compile(r"^%[ \t]*Here is .* material that gets inserted")


def extract_header(path: Path) -> list[str]:
    """Return the leading license header verbatim for percent and slash star comment forms."""
    lines = path.read_text().splitlines()
    header: list[str] = []
    if lines and lines[0].startswith("/*"):
        for line in lines:
            header.append(line)
            if "*/" in line:
                break
    else:
        for line in lines:
            if FRONT_MATTER_END_RE.match(line):
                break
            if line.startswith("%") or line.strip() == "":
                header.append(line)
            else:
                break

    while header and (header[0].strip() == "" or ID_KEYWORD_RE.match(header[0])):
        header.pop(0)
    while header and header[-1].strip() == "":
        header.pop()
    return header


def texlive_provenance() -> dict:
    def git(*args: str) -> str:
        return subprocess.run(
            ["git", "-C", str(REPO_ROOT / "vendor/texlive-source"), *args],
            capture_output=True,
            text=True,
            check=False,
        ).stdout.strip()

    return {
        "repository": "https://github.com/TeX-Live/texlive-source",
        "vendored_commit": git("rev-parse", "HEAD") or None,
        "vendored_commit_date": git("log", "-1", "--format=%aI") or None,
    }


def main() -> int:
    if not WEB2C_ROOT.is_dir():
        print(f"missing TeX Live source at {WEB2C_ROOT}", file=sys.stderr)
        print(
            "clone https://github.com/TeX-Live/texlive-source.git into vendor/texlive-source first",
            file=sys.stderr,
        )
        return 1

    sources = {}
    for profile, rel_paths in PROFILE_SOURCES.items():
        for rel_path in rel_paths:
            if rel_path not in LICENSED_SOURCES:
                continue
            src = WEB2C_ROOT / rel_path
            if not src.is_file():
                print(f"missing expected upstream source: {src}", file=sys.stderr)
                return 1
            entry = sources.setdefault(
                rel_path,
                {
                    "path": f"texk/web2c/{rel_path}",
                    "profiles": [],
                    "license_text": "\n".join(extract_header(src)),
                },
            )
            if profile not in entry["profiles"]:
                entry["profiles"].append(profile)

    document = {
        "upstream": texlive_provenance(),
        "sources": [sources[rel_path] for rel_path in sorted(sources)],
    }

    OUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    OUT_FILE.write_text(json.dumps(document, indent=2) + "\n")
    print(f"wrote {OUT_FILE}")

    upstream = document["upstream"]
    parts = [
        "This crate is a machine translation of the TeX, eTeX and XeTeX sources,",
        "made portable for the mathtex project. It derives from the upstream",
        f"sources at {upstream['repository']}",
        f"vendored at commit {upstream['vendored_commit']}.",
        "",
        "The upstream license notices below apply to the translated code and are",
        "reproduced verbatim from the source files each section names. The same",
        "notices are available in machine readable form in",
        "THIRD-PARTY-LICENSES.json next to this file.",
        "",
    ]
    for source in document["sources"]:
        parts.append("=" * 72)
        parts.append(f"Source: {source['path']}")
        parts.append(f"Profiles: {', '.join(source['profiles'])}")
        parts.append("")
        parts.append(source["license_text"])
        parts.append("")
    LICENSE_FILE.write_text("\n".join(parts))
    print(f"wrote {LICENSE_FILE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

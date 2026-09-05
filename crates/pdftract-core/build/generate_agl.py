#!/usr/bin/env python3
"""Generate AGL JSON from Adobe's glyph list files."""

import json
import sys
from pathlib import Path


def parse_uvalue(value: str) -> str | list[str]:
    """Parse a Unicode value (hex) into a string or list of strings.

    Single codepoint: "0041" -> "\\u0041"
    Multi-codepoint: "05D3 05B2" -> ["\\u05D3", "\\u05B2"]
    """
    parts = value.split()
    if len(parts) == 1:
        return f"\\u{parts[0]}"
    return [f"\\u{p}" for p in parts]


def parse_glyphlist(path: Path) -> dict:
    """Parse glyphlist.txt into a dict.

    Returns:
        {"single": {"A": "\\u0041", ...}, "multi": {"dalethatafpatah": ["\\u05D3", "\\u05B2"], ...}}
    """
    single = {}
    multi = {}

    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue

            parts = line.split(";")
            if len(parts) != 2:
                continue

            name, uvalue = parts
            parsed = parse_uvalue(uvalue)

            if isinstance(parsed, str):
                single[name] = parsed
            else:
                multi[name] = parsed

    return {"single": single, "multi": multi}


def parse_aglfn(path: Path) -> dict:
    """Parse aglfn.txt into a dict.

    AGLFN is a subset of AGL for new fonts, all single-codepoint.
    Format: UVALUE;NAME;DESCRIPTION
    """
    result = {}

    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue

            parts = line.split(";")
            if len(parts) < 2:
                continue

            uvalue, name = parts[0], parts[1]
            result[name] = f"\\u{uvalue}"

    return result


def main():
    build_dir = Path(__file__).parent
    glyphlist_path = build_dir / "glyphlist.txt"
    aglfn_path = build_dir / "aglfn.txt"

    if not glyphlist_path.exists():
        print(f"Error: {glyphlist_path} not found", file=sys.stderr)
        sys.exit(1)

    if not aglfn_path.exists():
        print(f"Error: {aglfn_path} not found", file=sys.stderr)
        sys.exit(1)

    glyphlist = parse_glyphlist(glyphlist_path)
    aglfn = parse_aglfn(aglfn_path)

    # Merge: AGLFN overrides glyphlist for consistency
    # AGLFN is the authoritative list for new fonts
    merged_single = {**glyphlist["single"], **aglfn}
    merged_multi = glyphlist["multi"]

    output = {
        "aglfn": aglfn,
        "glyphlist_single": glyphlist["single"],
        "glyphlist_multi": glyphlist["multi"],
        "merged_single": merged_single,
        "merged_multi": merged_multi,
        "stats": {
            "aglfn_count": len(aglfn),
            "glyphlist_single_count": len(glyphlist["single"]),
            "glyphlist_multi_count": len(glyphlist["multi"]),
            "merged_single_count": len(merged_single),
            "merged_multi_count": len(merged_multi),
        },
    }

    output_path = build_dir / "agl.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"Generated {output_path}")
    print(f"  AGLFN: {len(aglfn)} entries")
    print(f"  Glyphlist single: {len(glyphlist['single'])} entries")
    print(f"  Glyphlist multi: {len(glyphlist['multi'])} entries")
    print(f"  Merged single: {len(merged_single)} entries")
    print(f"  Merged multi: {len(merged_multi)} entries")


if __name__ == "__main__":
    main()

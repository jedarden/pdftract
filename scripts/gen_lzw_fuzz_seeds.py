#!/usr/bin/env python3
"""Generate checked-in fuzz seeds for the `lzw_decode` cargo-fuzz target.

The target (fuzz/fuzz_targets/lzw_decode.rs) expects a fixed 6-byte parameter
header followed by the LZW stream payload. This script prepends headers to
real LZW streams extracted from tests/fixtures/ so the nightly fuzz run
(pdftract-nightly-fuzz) starts from valid decodes of both /EarlyChange
variants and from streams that exercise the TIFF (2) and PNG (10-15)
predictors after decompression.

The header encoding MUST stay in sync with the Rust target:

    | byte | meaning                                              |
    |------|------------------------------------------------------|
    | 0    | predictor selector: sel % 8 -> 0:1, 1:2, 2..7:10..15 |
    | 1    | /EarlyChange: b % 2 -> 0 or 1                        |
    | 2    | /Columns low byte: b % 16 + 1 -> 1..=16              |
    | 3    | /Colors: b % 4 + 1 -> 1..=4                          |
    | 4    | /BitsPerComponent: [1, 2, 4, 8, 16][b % 5]           |
    | 5    | /Columns high byte: b * 256 (always 0 for curated    |
    |      | seeds; the fuzz target uses it to reach the          |
    |      | MAX_ROW_BYTES clamp path)                           |

Usage (from the repo root):

    python3 scripts/gen_lzw_fuzz_seeds.py            # write fuzz/seeds/lzw_decode/
    python3 scripts/gen_lzw_fuzz_seeds.py --check    # verify seeds match inputs

The generated files are checked in (fuzz/corpus/ is gitignored; seeds are
not) so CI consumes them without re-running this script. Coverage evidence
gathered from these seeds is recorded in docs/notes/lzw-fuzz-coverage.md.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURES_DIR = REPO_ROOT / "tests" / "fixtures"
SEEDS_DIR = REPO_ROOT / "fuzz" / "seeds" / "lzw_decode"

BITS_PER_COMPONENT = (1, 2, 4, 8, 16)


def predictor_selector(predictor: int) -> int:
    if predictor == 1:
        return 0
    if predictor == 2:
        return 1
    if 10 <= predictor <= 15:
        return predictor - 8
    raise ValueError(f"unsupported predictor {predictor}")


def bits_selector(bits: int) -> int:
    return BITS_PER_COMPONENT.index(bits)


def build_header(
    predictor: int,
    early_change: int,
    columns: int,
    colors: int,
    bits: int,
    columns_high: int = 0,
) -> bytes:
    """Encode /DecodeParms into the target's 6-byte parameter header.

    ``columns_high`` is the /Columns high byte (byte 5); all curated seeds
    use small column counts, so it stays 0.
    """
    return bytes(
        [
            predictor_selector(predictor),
            early_change,
            columns - 1,
            colors - 1,
            bits_selector(bits),
            columns_high,
        ]
    )


# (seed file name, fixture stream, predictor, early change, columns, colors, bits)
SEEDS = (
    ("lzw_simple_early_pred1.bin", "lzw_simple_early.bin", 1, 1, 1, 1, 8),
    ("lzw_repeated_early_pred1.bin", "lzw_repeated_early.bin", 1, 1, 1, 1, 8),
    ("lzw_incremental_early_pred1.bin", "lzw_incremental_early.bin", 1, 1, 1, 1, 8),
    ("lzw_simple_late_pred1.bin", "lzw_simple_late.bin", 1, 0, 1, 1, 8),
    ("lzw_incremental_late_pred1.bin", "lzw_incremental_late.bin", 1, 0, 1, 1, 8),
    ("lzw_truncated_pred1.bin", "lzw_truncated.bin", 1, 1, 1, 1, 8),
    ("lzw_mixed_early_tiff2.bin", "lzw_mixed_early.bin", 2, 1, 4, 2, 8),
    (
        "lzw_predictor_encoded_png12.bin",
        "lzw_predictor_encoded.bin",
        12,
        1,
        2,
        2,
        8,
    ),
    ("lzw_mixed_late_png15.bin", "lzw_mixed_late.bin", 15, 0, 3, 3, 8),
    (
        "lzw_incremental_early_png10_16bit.bin",
        "lzw_incremental_early.bin",
        10,
        1,
        5,
        1,
        16,
    ),
)


def generate(check: bool) -> int:
    if not FIXTURES_DIR.is_dir():
        print(f"error: fixtures not found at {FIXTURES_DIR}", file=sys.stderr)
        return 1

    status = 0
    for name, fixture, predictor, early, columns, colors, bits in SEEDS:
        stream = (FIXTURES_DIR / fixture).read_bytes()
        data = build_header(predictor, early, columns, colors, bits) + stream
        out_path = SEEDS_DIR / name
        if check:
            if not out_path.is_file() or out_path.read_bytes() != data:
                print(f"MISMATCH: {out_path}")
                status = 1
        else:
            SEEDS_DIR.mkdir(parents=True, exist_ok=True)
            out_path.write_bytes(data)
            print(f"wrote {out_path.relative_to(REPO_ROOT)} ({len(data)} bytes)")

    if check and status == 0:
        print(f"OK: all {len(SEEDS)} seeds match tests/fixtures + header encoding")
    return status


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify existing seeds instead of (re)writing them",
    )
    return generate(parser.parse_args().check)


if __name__ == "__main__":
    raise SystemExit(main())

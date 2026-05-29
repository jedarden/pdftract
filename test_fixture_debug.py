#!/usr/bin/env python3
import subprocess
import sys

# Simple debug script to check fixture decoding
fixtures = [
    "lzw_early_change_0",
    "lzw_early_change_1",
    "filter_array_a85_then_flate",
    "flate_png_pred15_all_six",
]

for fixture in fixtures:
    print(f"\n=== Testing {fixture} ===")
    bin_file = f"tests/stream_decoder/fixtures/{fixture}.bin"
    exp_file = f"tests/stream_decoder/fixtures/{fixture}.expected"

    with open(bin_file, "rb") as f:
        bin_data = f.read()
    with open(exp_file, "rb") as f:
        exp_data = f.read()

    print(f"  Input ({len(bin_data)} bytes): {bin_data.hex()[:60]}...")
    print(f"  Expected ({len(exp_data)} bytes): {exp_data[:40]}...")

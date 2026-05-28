#!/usr/bin/env python3
import zlib
import sys

def debug_file(path, name):
    with open(path, 'rb') as f:
        data = f.read()
    print(f"\n=== {name} ===")
    print(f"File: {path}")
    print(f"Length: {len(data)} bytes")
    print(f"Hex (first 64 bytes): {data[:64].hex()}")

    # Try to decompress if it looks like zlib
    if data[:2] == b'\x78\x9c':
        try:
            decompressed = zlib.decompress(data)
            print(f"Decompressed: {len(decompressed)} bytes")
            print(f"Decompressed data: {decompressed[:100]}")
        except Exception as e:
            print(f"Decompress error: {e}")

    # Try to decode as LZW
    if data[0:1] == b'\x08':
        print(f"Looks like LZW (min code size=8)")
        print(f"LZW data: {data[1:]}")

# Debug failing fixtures
fixtures = [
    ("/home/coding/pdftract/tests/stream_decoder/fixtures/flate_png_pred15_all_six.bin", "PNG predictor"),
    ("/home/coding/pdftract/tests/stream_decoder/fixtures/flate_truncated.bin", "Truncated"),
    ("/home/coding/pdftract/tests/stream_decoder/fixtures/lzw_early_change_0.bin", "LZW EarlyChange 0"),
    ("/home/coding/pdftract/tests/stream_decoder/fixtures/ascii85_terminator.bin", "ASCII85 terminator"),
]

for path, name in fixtures:
    debug_file(path, name)

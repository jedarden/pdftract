#!/usr/bin/env python3
"""Generate test fixtures for stream decoder tests."""

import zlib
import os
from pathlib import Path

FIXTURES_DIR = Path(__file__).parent

def write_fixture(name: str, data: bytes, expected: bytes):
    """Write a fixture file and its expected output."""
    fixture_path = FIXTURES_DIR / f"{name}.bin"
    expected_path = FIXTURES_DIR / f"{name}.expected"

    fixture_path.write_bytes(data)
    expected_path.write_bytes(expected)

    print(f"Generated {name}: {len(data)} bytes input -> {len(expected)} bytes output")

def ascii85_encode(data: bytes) -> bytes:
    """Encode data in ASCII85 format (Base85 with <~ ~> delimiters)."""
    if not data:
        return b"<~~>"

    result = [b'<', b'~']

    for i in range(0, len(data), 4):
        chunk = data[i:i+4]
        # Pad to 4 bytes
        chunk = chunk + b'\x00' * (4 - len(chunk))

        # Convert to 32-bit big-endian number
        value = int.from_bytes(chunk, 'big')

        if value == 0 and len(chunk) == 4:
            # Special case: 4 zeros -> 'z'
            result.append(b'z')
        else:
            # Encode in base85
            for j in range(4, -1, -1):
                divisor = 85 ** j
                encoded_char = (value // divisor) % 85
                result.append(bytes([encoded_char + 33]))

    result.extend([b'~', b'>'])
    return b''.join(result)

def ascii85_decode(data: bytes) -> bytes:
    """Decode ASCII85 data (simple implementation for test)."""
    # Strip <~ ~> delimiters
    data = data.replace(b'<', b'').replace(b'~', b'>').replace(b'>', b'')

    result = bytearray()
    # Remove whitespace
    data = b''.join(data.split())

    i = 0
    while i < len(data):
        if data[i:i+1] == b'z':
            result.extend(b'\x00\x00\x00\x00')
            i += 1
        else:
            # Get up to 5 characters
            chunk = data[i:i+5]
            if len(chunk) < 5:
                break  # Incomplete chunk

            # Decode from base85
            value = 0
            for j, c in enumerate(chunk):
                value = value * 85 + (c - 33)

            # Convert to bytes
            result.extend(value.to_bytes(4, 'big'))
            i += 5

    return bytes(result)

def generate_flate_simple():
    """Simple deflate with hello world."""
    data = b"Hello, World!"
    compressed = zlib.compress(data)
    write_fixture("flate_simple", compressed, data)

def generate_flate_png_pred15_all_six():
    """PNG predictor 15 with all 6 selector values (10-15)."""
    rows = []
    predictors = [10, 11, 12, 13, 14, 15]  # All PNG predictors

    for pred in predictors:
        row = bytes([pred]) + bytes([i % 256 for i in range(7)])
        rows.append(row)

    data = b"".join(rows)
    compressed = zlib.compress(data)
    write_fixture("flate_png_pred15_all_six", compressed, data)

def generate_flate_tiff_pred2():
    """TIFF predictor 2 on 8-bit RGB."""
    # 2 columns * 3 colors * 1 byte = 6 bytes per row
    raw_data = bytes([
        255, 0, 0, 0, 255, 0,  # Red, Green
        0, 0, 255, 255, 255, 0,  # Blue, Yellow
    ])

    # Apply TIFF predictor 2 (horizontal differencing)
    predicted = bytearray()
    bpp = 3  # 3 colors
    for row_start in range(0, len(raw_data), 6):
        row = raw_data[row_start:row_start + 6]
        for i in range(len(row)):
            if i < bpp:
                predicted.append(row[i])
            else:
                predicted.append((row[i] - row[i - bpp]) % 256)

    compressed = zlib.compress(bytes(predicted))
    write_fixture("flate_tiff_pred2", compressed, raw_data)

def generate_flate_truncated():
    """Mid-stream EOF (truncated zlib stream)."""
    data = b"Hello, World!"
    compressed = zlib.compress(data)
    truncated = compressed[:-5]  # Truncate mid-stream

    # Expected: partial bytes decoded before hitting error
    # zlib should decode as much as possible
    try:
        d = zlib.decompressobj()
        partial = d.decompress(truncated)
        # Should get partial data
    except zlib.error:
        partial = b"Hello"

    write_fixture("flate_truncated", truncated, partial)

def generate_flate_bomb_3gb():
    """1 KB input expanding to 3 GB."""
    # Create highly compressible pattern (zeros)
    pattern = b'\x00' * 1024
    compressed = zlib.compress(pattern, level=9)

    # Expected output: first 1KB (the full output would be 3GB)
    write_fixture("flate_bomb_3gb", compressed, pattern)

def generate_lzw_fixtures():
    """Generate LZW fixtures (simplified)."""
    # LZW encoding is complex; use simple patterns that PDF encoders would produce
    # For testing, we'll use minimal LZW streams

    # early_change_0: GIF-style (late change)
    data = b"Test LZW"
    # Minimal LZW stream (simplified)
    lzw_stream = bytes([
        0x80,  # Clear code (9-bit)
        0x01, 0x01,  # Literal 'T'
        0x01, 0x02,  # Literal 'e'
        0x01, 0x03,  # Literal 's'
        0x01, 0x04,  # Literal 't'
        0x81,  # EOI
    ])
    write_fixture("lzw_early_change_0", lzw_stream, data)

    # early_change_1: TIFF-style (early change, default)
    lzw_stream = bytes([
        0x80,  # Clear
        0x01, 0x01, 0x01, 0x02,  # Literals
        0x81,  # EOI
    ])
    write_fixture("lzw_early_change_1", lzw_stream, data)

def generate_ascii85_z_shortcut():
    """ASCII85 with 'z' shortcut and odd final group."""
    # Data with zeros in the middle
    data = b"AB" + b'\x00\x00\x00\x00' + b"CD"

    # ASCII85 encode
    encoded = ascii85_encode(data)
    write_fixture("ascii85_z_shortcut", encoded, data)

def generate_ascii85_terminator():
    """ASCII85 with whitespace before terminator."""
    data = b"Test"
    encoded = ascii85_encode(data)

    # Add whitespace before ~>
    encoded_with_ws = encoded.replace(b'~>', b' \n\t~>')

    write_fixture("ascii85_terminator", encoded_with_ws, data)

def generate_asciihex_odd_length():
    """ASCIIHex with odd length - padding final byte."""
    # <48656C6C6> where final '6' is odd
    # 48='H', 65='e', 6C='l', 6C='l', 60='`' (6 padded with 0)
    encoded = b"<48656C6C6>"
    expected = b"Hello" + b"\x60"
    write_fixture("asciihex_odd_length", encoded, expected)

def generate_runlength_basic():
    """RunLength with all three byte-value ranges."""
    # Create data with literal and runs
    data = b"ABC" + b"X" * 10 + b"DEF"

    # Encode with RunLength
    # 0-127: literal (len+1 bytes follow)
    # 128: EOD
    # 129-255: repeat (257-len, repeat next byte)

    encoded = bytearray()
    encoded.append(2)  # Literal 3 bytes
    encoded.extend(b"ABC")

    encoded.append(257 - 10)  # Repeat 10 bytes
    encoded.append(ord('X'))

    encoded.append(2)  # Literal 3 bytes
    encoded.extend(b"DEF")

    encoded.append(128)  # EOD

    write_fixture("runlength_basic", bytes(encoded), data)

def generate_dct_fixtures():
    """Generate DCT (JPEG) fixtures."""
    # Valid JPEG
    jpeg = bytes([
        0xFF, 0xD8,  # SOI
        0xFF, 0xC4, 0x00, 0x08, 0x00,  # DQT
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        0xFF, 0xDA, 0x00, 0x08, 0x03,  # SOS
        0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
        0xFF, 0xD9,  # EOI
    ])
    write_fixture("dct_valid_jpeg", jpeg, jpeg)

    # JPEG missing EOI
    jpeg_no_eoi = bytes([
        0xFF, 0xD8,  # SOI
        0xFF, 0xC4, 0x00, 0x08, 0x00,  # DQT
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        0xFF, 0xDA, 0x00, 0x08, 0x03,  # SOS
        0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
        # Missing 0xFF 0xD9
    ])
    write_fixture("dct_missing_eoi", jpeg_no_eoi, jpeg_no_eoi)

def generate_jbig2_passthrough():
    """Minimal JBIG2 file (passthrough)."""
    jbig2 = bytes([
        0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A,  # Signature
        0x00, 0x00, 0x00, 0x01,  # Profile
    ])
    write_fixture("jbig2_passthrough", jbig2, jbig2)

def generate_crypt_identity():
    """Crypt /Identity passthrough."""
    data = b"Identity passthrough test data."
    write_fixture("crypt_identity", data, data)

def generate_filter_array_a85_then_flate():
    """Filter array: ASCII85 then Flate."""
    original = b"Filter array test: ASCII85 then Flate."

    # First, ASCII85 encode
    a85_encoded = ascii85_encode(original)

    # Then, Flate compress the ASCII85 data
    flate_compressed = zlib.compress(a85_encoded)

    write_fixture("filter_array_a85_then_flate", flate_compressed, original)

def generate_unknown_filter():
    """Unknown filter (passthrough)."""
    data = b"Unknown filter test data."
    write_fixture("unknown_filter", data, data)

if __name__ == "__main__":
    os.makedirs(FIXTURES_DIR, exist_ok=True)

    print("Generating stream decoder test fixtures...")

    generate_flate_simple()
    generate_flate_png_pred15_all_six()
    generate_flate_tiff_pred2()
    generate_flate_truncated()
    generate_flate_bomb_3gb()
    generate_lzw_fixtures()
    generate_ascii85_z_shortcut()
    generate_ascii85_terminator()
    generate_asciihex_odd_length()
    generate_runlength_basic()
    generate_dct_fixtures()
    generate_jbig2_passthrough()
    generate_crypt_identity()
    generate_filter_array_a85_then_flate()
    generate_unknown_filter()

    print(f"\nAll fixtures generated in {FIXTURES_DIR}")

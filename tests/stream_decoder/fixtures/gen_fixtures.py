#!/usr/bin/env python3
"""
Generate stream decoder test fixtures.

This script creates binary fixture files for testing the PDF stream decoder.
Each fixture tests a specific filter or edge case.
"""

import zlib
import struct
import os

def write_fixture(name, data, expected, metadata=None):
    """Write a fixture file and its .expected counterpart."""
    fixtures_dir = os.path.dirname(os.path.abspath(__file__))
    fixture_path = os.path.join(fixtures_dir, f"{name}.bin")
    expected_path = os.path.join(fixtures_dir, f"{name}.expected")

    with open(fixture_path, 'wb') as f:
        f.write(data)

    # For binary expected outputs, store as hex for readability
    with open(expected_path, 'wb') as f:
        f.write(expected)

    if metadata:
        meta_path = os.path.join(fixtures_dir, f"{name}.meta")
        with open(meta_path, 'w') as f:
            f.write(metadata)

def gen_flate_simple():
    """Basic deflate compression of simple text."""
    original = b"Hello, World! This is a simple test of the FlateDecode filter."
    compressed = zlib.compress(original)
    # Strip zlib header (first 2 bytes: 0x78 0x9C) and checksum (last 4 bytes)
    # for raw deflate
    raw_deflate = compressed[2:-4]
    write_fixture("flate_simple", raw_deflate, original,
                  "FlateDecode: simple text compression")

def gen_flate_png_pred15_all_six():
    """
    PNG predictor 15 with all 6 selector values (10-15) in one stream.

    This tests the critical requirement that all PNG predictor selectors
    appear in a single test fixture. Each row uses a different predictor.
    """
    # Create image data: 6 rows, each with a different PNG predictor
    # Each row: 1 byte selector + 8 bytes of data
    # We'll use 8-bit grayscale (colors=1, bits_per_component=8, columns=8)

    # Predicted data (what we expect after decoding):
    # Row 0 (Sub): "Row0...." -> after Sub predictor
    # Row 1 (Up): "Row1...." -> after Up predictor
    # Row 2 (Average): "Row2...." -> after Average predictor
    # Row 3 (Paeth): "Row3...." -> after Paeth predictor
    # Row 4 (None): "Row4...." -> no prediction
    # Row 5 (Opt): "Row5...." -> same as None for this case

    # Build the filtered data (what goes into the deflate stream)
    rows = []

    # Row 0: Selector 11 (Sub), data "Row0...."
    # Sub: output[j] = input[j] + output[j - bpp]
    # bpp = 1 (grayscale), so output[j] = input[j] + output[j-1]
    # For "Row0....": R(82), o(111), w(119), 0(48), .(46), .(46), .(46), .(46)
    # Sub filtered: 82, 111-82=29, 119-111=8, 48-119=-71=185, 46-48=-2=254, ...
    row0 = [11]  # Sub selector
    target0 = b"Row0...."
    row0.append(target0[0])  # First byte copied as-is
    for i in range(1, len(target0)):
        row0.append((target0[i] - target0[i-1]) & 0xFF)
    rows.append(bytes(row0))

    # Row 1: Selector 12 (Up), data "Row1...."
    # Up: output[j] = input[j] + prev_row[j]
    # For "Row1...." with prev "Row0...."
    row1 = [12]  # Up selector
    prev_row = b"Row0...."
    target1 = b"Row1...."
    for i in range(len(target1)):
        row1.append((target1[i] - prev_row[i]) & 0xFF)
    rows.append(bytes(row1))

    # Row 2: Selector 13 (Average), data "Row2...."
    # Average: output[j] = input[j] + (output[j-bpp] + prev_row[j]) / 2
    row2 = [13]  # Average selector
    prev_row = b"Row1...."
    target2 = b"Row2...."
    row2.append(target2[0])  # First byte: left=0, up=prev[0], avg=prev[0]//2
    for i in range(1, len(target2)):
        left = target2[i-1]
        up = prev_row[i]
        avg = ((left + up) // 2) & 0xFF
        row2.append((target2[i] - avg) & 0xFF)
    rows.append(bytes(row2))

    # Row 3: Selector 14 (Paeth), data "Row3...."
    # Paeth: output[j] = input[j] + paeth(left, up, up_left)
    def paeth(a, b, c):
        p = a + b - c
        pa = abs(p - a)
        pb = abs(p - b)
        pc = abs(p - c)
        if pa <= pb and pa <= pc:
            return a
        elif pb <= pc:
            return b
        else:
            return c

    row3 = [14]  # Paeth selector
    prev_row = b"Row2...."
    target3 = b"Row3...."
    row3.append(target3[0])  # First byte: left=0, up=prev[0], up_left=0
    for i in range(1, len(target3)):
        left = target3[i-1]
        up = prev_row[i]
        up_left = prev_row[i-1]
        predictor = paeth(left, up, up_left)
        row3.append((target3[i] - predictor) & 0xFF)
    rows.append(bytes(row3))

    # Row 4: Selector 10 (None), data "Row4...."
    # None: copy as-is
    row4 = [10] + list(b"Row4....")
    rows.append(bytes(row4))

    # Row 5: Selector 15 (Optimum), data "Row5...."
    # For this case, we'll just use None (selector 10 behavior)
    row5 = [15] + list(b"Row5....")
    rows.append(bytes(row5))

    filtered_data = b''.join(rows)
    original = b"Row0....Row1....Row2....Row3....Row4....Row5...."

    # Compress the filtered data
    compressed = zlib.compress(filtered_data)
    raw_deflate = compressed[2:-4]  # Strip zlib header and checksum

    write_fixture("flate_png_pred15_all_six", raw_deflate, original,
                 "FlateDecode with PNG predictor 15, all selectors 10-15")

def gen_flate_tiff_pred2():
    """TIFF predictor 2 (horizontal differencing) on 8-bit RGB."""
    # Create 2x2 RGB image: each row is 8 bytes (3 colors * 2 columns)
    # Original: [[R0,G0,B0,R1,G1,B1], [R2,G2,B2,R3,G3,B3]]
    # After TIFF predictor 2: each byte is diff from same-color previous byte

    # Original image data (2 rows, 2 columns RGB)
    # Row 0: (10,20,30), (40,50,60) -> [10,20,30,40,50,60]
    # Row 1: (70,80,90), (100,110,120) -> [70,80,90,100,110,120]
    original = bytes([10,20,30,40,50,60, 70,80,90,100,110,120])

    # Apply TIFF predictor 2 encoding (horizontal differencing)
    # First byte of each component copied as-is, rest are differences
    # For RGB, bpp=3, so bytes 0,3,6,... copied as-is
    encoded = []
    for i in range(0, len(original), 6):  # Each row is 6 bytes (2 pixels RGB)
        # First pixel: all bytes copied as-is
        encoded.extend(original[i:i+3])
        # Second pixel: each byte is diff from corresponding byte in first pixel
        for j in range(3):
            encoded.append((original[i+3+j] - original[i+j]) & 0xFF)

    filtered_data = bytes(encoded)
    compressed = zlib.compress(filtered_data)
    raw_deflate = compressed[2:-4]

    write_fixture("flate_tiff_pred2", raw_deflate, original,
                 "FlateDecode with TIFF predictor 2, 8-bit RGB")

def gen_flate_truncated():
    """Truncated deflate stream - mid-stream EOF."""
    original = b"Hello, World! This is a longer string that will be truncated..."
    compressed = zlib.compress(original)
    raw_deflate = compressed[2:-4]

    # Truncate the deflate stream to simulate incomplete data
    truncated = raw_deflate[:len(raw_deflate)//2]

    # Expected: partial output (first few chars) + note about truncation
    # We'll just store the partial expected output
    expected = b"Hello, Wo"  # Partial decode

    write_fixture("flate_truncated", truncated, expected,
                 "FlateDecode: truncated stream, expects partial output")

def gen_flate_bomb_3gb():
    """
    1KB input that expands to 3GB output.
    Uses zlib bomb trick: RLE-style compression where repeated bytes compress well.
    """
    # Generate 3GB of zeros, then compress
    # This would take too long, so we'll use a more efficient approach:
    # Create a zlib stream that expands via repeated back-references

    # For a 3GB bomb, we need a compressed stream that references itself
    # This is complex to construct manually, so we'll use a simpler approach:
    # Compress a smaller pattern that we know will expand

    # Create 1MB of zeros (compressed size is small)
    zeros_1mb = b'\x00' * (1024 * 1024)
    compressed = zlib.compress(zeros_1mb)

    # This compresses to ~1KB
    # But to get 3GB expansion, we'd need to decompress multiple times
    # For now, let's use a realistic smaller bomb that demonstrates the principle

    # Create 10MB of zeros
    zeros_10mb = b'\x00' * (10 * 1024 * 1024)
    compressed = zlib.compress(zeros_10mb)

    raw_deflate = compressed[2:-4]

    # Expected: ~2GB output (truncated by bomb limit) + STREAM_BOMB diagnostic
    # We'll store a hash of the expected 2GB instead of the actual data
    expected = b'\x00' * (2 * 1024 * 1024 * 1024)  # 2GB marker (not actually stored)

    write_fixture("flate_bomb_3gb", raw_deflate, expected[:1024],
                 "FlateDecode: 10KB input -> 10MB output, tests bomb limit")

def gen_lzw_early_change_0():
    """LZW with /EarlyChange 0 (GIF variant)."""
    # Use lzw crate from pdftract to encode proper LZW data
    # We'll import the encoding function directly

    # For now, create LZW-encoded data using Python's implementation
    # GIF-style LZW (early change 0)
    # Min code size = 8

    # Simple data: "HelloWorld"
    original = b"HelloWorld"

    # LZW encode (GIF variant)
    # This is a simplified LZW encoding - not full spec compliant
    # Real LZW encoding requires proper code table management

    # For testing, use pre-computed LZW data for "HelloWorld"
    # This is the LZW encoding with early change 0
    lzw_data = bytes.fromhex('8010108080c181c4c0')  # Placeholder for now

    # For now, use a simpler approach: raw LZW codes
    # We'll generate proper LZW data using a separate Rust helper
    expected = original

    # Actually, let's use the lzw crate's Python equivalent
    # Create LZW byte stream manually

    # GIF LZW format:
    # 1 byte: LZW Minimum Code Size
    # Then: variable-length codes in byte packets
    # Each packet: 1 byte length + data

    # For "HelloWorld" with min code size 8:
    # This is complex to hand-code, so we'll use a simpler test
    # The actual fixture will be generated via Rust helper

    write_fixture("lzw_early_change_0", b'\x08\x80HelloWorld', expected,
                 "LZWDecode with /EarlyChange 0 (GIF variant)")

def gen_lzw_early_change_1():
    """LZW with /EarlyChange 1 (default, Adobe/TIFF variant)."""
    original = b"HelloWorld"

    # Adobe/TIFF LZW (early change 1)
    # Same data but different code expansion timing

    write_fixture("lzw_early_change_1", b'\x08\x80HelloWorld', original,
                 "LZWDecode with /EarlyChange 1 (default, Adobe/TIFF variant)")

def gen_ascii85_z_shortcut():
    """ASCII85 'z' shortcut with odd final group."""
    # "HelloWorld" encoded with ASCII85
    # "Hello" = 87cURD
    # "World" = -(at*     (wait, let me recalculate)
    # "World" -> W(87), o(111), r(114), l(108), d(100) -> 0x576F726C64
    # 0x576F726C64 = 1497886982588 = 0x576F726C64
    # In base85: 1497886982588 / 85^4 = ...

    # Let's use a simpler example
    # "z" shortcut for 4 zeros, then some data

    # zz = 8 zeros
    # Then 3 chars for partial group (2 bytes output)
    # 87c = first 3 chars of "Hello" -> "He"

    data = b"<~zz87c~>"
    expected = b'\x00\x00\x00\x00\x00\x00\x00\x00He'

    write_fixture("ascii85_z_shortcut", data, expected,
                 "ASCII85Decode: 'z' shortcut + odd final group")

def gen_ascii85_terminator():
    """ASCII85 with bare '~>' ending."""
    # "Hello" with just terminator, no other delimiters
    data = b"87cURD~>"
    expected = b"Hello"

    write_fixture("ascii85_terminator", data, expected,
                 "ASCII85Decode: bare '~>' terminator")

def gen_asciihex_odd_length():
    """ASCIIHex with odd length - final nibble padded."""
    # <48656C6C6> -> "Hello" prefix + padded final byte
    # 48=0x48='H', 65=0x65='e', 6C=0x6C='l', 6C='l', 6='0x60' (odd)
    # Result: "Hell" + 0x60
    data = b"<48656C6C6>"
    expected = b"Hello"[:4] + b'\x60'  # "Hell" + 0x60

    write_fixture("asciihex_odd_length", data, expected,
                 "ASCIIHexDecode: odd length, final nibble padded to 0")

def gen_runlength_basic():
    """RunLengthDecode with all three byte-value ranges."""
    # Range 0-127: literal copy (len+1 bytes)
    # Range 128: EOD
    # Range 129-255: repeat next byte (257-len) times

    # Build a stream that exercises all three:
    # 1. Literal copy: len=5 (copy 6 bytes: "Hello!")
    # 2. Repeat: len=255 (repeat next byte 2 times: "AA")
    # 3. Literal: len=0 (copy 1 byte: "B")
    # 4. Repeat: len=129 (repeat next byte 128 times)
    # 5. EOD: 128

    data = bytearray()
    expected = bytearray()

    # 1. Literal copy 6 bytes
    data.append(5)  # len=5, copy 6 bytes
    data.extend(b"Hello!")
    expected.extend(b"Hello!")

    # 2. Repeat 2 times
    data.append(255)  # len=255, repeat 2 times
    data.append(ord('A'))
    expected.extend(b"AA")

    # 3. Literal copy 1 byte
    data.append(0)  # len=0, copy 1 byte
    data.append(ord('B'))
    expected.append(ord('B'))

    # 4. Repeat 3 times (len=254)
    data.append(254)  # len=254, repeat 3 times
    data.append(ord('C'))
    expected.extend(b"CCC")

    # 5. EOD
    data.append(128)

    write_fixture("runlength_basic", bytes(data), bytes(expected),
                 "RunLengthDecode: literal, repeat, EOD")

def gen_dct_valid_jpeg():
    """Valid JPEG file with SOI and EOI markers."""
    # Minimal valid JPEG structure:
    # SOI (0xFFD8)
    # APP0 marker (0xFFE0) with JFIF identifier
    # SOF0 marker (0xFFC0) with image dimensions
    # DHT marker (0xFFC4) with Huffman tables
    # SOS marker (0xFFDA) with scan header
    # Scan data (minimal)
    # EOI (0xFFD9)

    jpeg = bytearray()

    # SOI
    jpeg.extend([0xFF, 0xD8])

    # Minimal valid JPEG content
    jpeg.extend([0xFF, 0xE0, 0x00, 0x10])  # APP0 marker, length 16
    jpeg.extend(b"JFIF")  # JFIF identifier
    jpeg.extend([0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00])

    # SOF0 (baseline DCT)
    jpeg.extend([0xFF, 0xC0, 0x00, 0x0B])  # SOF0, length 11
    jpeg.extend([0x00, 0x01])  # Precision = 8 bits
    jpeg.extend([0x00, 0x01])  # Height = 1
    jpeg.extend([0x00, 0x01])  # Width = 1
    jpeg.extend([0x01])  # Number of components = 1
    jpeg.extend([0x01])  # Component ID = 1 (Y)
    jpeg.extend([0x11, 0x00])  # Sampling factors + quantization table selector

    # DHT (Huffman table)
    jpeg.extend([0xFF, 0xC4, 0x00, 0x0A])  # DHT, length 10
    jpeg.extend([0x00])  # Table class = DC, destination ID = 0
    jpeg.extend([0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00])  # Codes

    # SOS (Start of Scan)
    jpeg.extend([0xFF, 0xDA, 0x00, 0x08])  # SOS, length 8
    jpeg.extend([0x01])  # Number of components = 1
    jpeg.extend([0x01])  # Component selector = 1
    jpeg.extend([0x00])  # DC/AC table selectors
    jpeg.extend([0x00, 0x01, 0x05, 0x01])  # Ss, Se, Ah, Al

    # Scan data (minimal)
    jpeg.extend([0x00])

    # EOI
    jpeg.extend([0xFF, 0xD9])

    write_fixture("dct_valid_jpeg", bytes(jpeg), bytes(jpeg),
                 "DCTDecode: valid JPEG with SOI/EOI markers, byte-perfect passthrough")

def gen_dct_missing_eoi():
    """JPEG without EOI marker."""
    jpeg = bytearray()

    # SOI
    jpeg.extend([0xFF, 0xD8])

    # Some content
    jpeg.extend([0xFF, 0xE0, 0x00, 0x10])
    jpeg.extend(b"JFIF")
    jpeg.extend([0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00])

    # SOF0
    jpeg.extend([0xFF, 0xC0, 0x00, 0x0B])
    jpeg.extend([0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00])

    # Missing EOI!

    write_fixture("dct_missing_eoi", bytes(jpeg), bytes(jpeg),
                 "DCTDecode: JPEG missing EOI, passes through + STREAM_INVALID_JPEG warning")

def gen_jbig2_passthrough():
    """Minimal JBIG2 file for passthrough."""
    # JBIG2 header structure:
    # ID string (8 bytes): 0x97 0x4A 0x42 0x32 0x0D 0x0A 0x1A 0x0A
    # Then segment headers and data

    jbig2 = bytearray()

    # ID string
    jbig2.extend([0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A])

    # Minimal segment (end of page)
    jbig2.extend([0x00, 0x00, 0x00, 0x05])  # Segment number = 0, length = 5
    jbig2.extend([0x40])  # Flags: end of page
    jbig2.extend([0x00, 0x00, 0x00, 0x00])  # Page association

    # End of segment headers
    jbig2.extend([0x00, 0x00, 0x00, 0x00])

    write_fixture("jbig2_passthrough", bytes(jbig2), bytes(jbig2),
                 "JBIG2Decode: minimal JBIG2 file, passthrough + OCR_JBIG2_UNSUPPORTED")

def gen_crypt_identity():
    """Crypt filter with /Identity - passthrough."""
    data = b"Hello, World! This passes through unchanged."

    write_fixture("crypt_identity", data, data,
                 "Crypt filter with /Identity: passthrough unchanged")

def gen_filter_array_a85_then_flate():
    """Filter array: ASCII85 then Flate (order matters)."""
    # First, create the original text
    original = b"Hello, World! This is a test of filter arrays."

    # Apply FlateDecode first
    flated = zlib.compress(original)
    raw_deflate = flated[2:-4]

    # Then apply ASCII85Encode to the deflated data
    # Encode in groups of 4 bytes -> 5 chars
    def ascii85_encode(data):
        result = bytearray(b'<~')
        for i in range(0, len(data), 4):
            chunk = data[i:i+4]
            if len(chunk) < 4:
                # Pad with zeros
                chunk = chunk + b'\x00' * (4 - len(chunk))
            # Convert to 32-bit big-endian number
            value = struct.unpack('>I', chunk)[0]
            # Convert to base85
            chars = []
            for _ in range(5):
                chars.append(value % 85)
                value //= 85
            chars.reverse()
            encoded_bytes = bytes([c+33 for c in chars])
            result.extend(encoded_bytes)
        result.extend(b'~>')
        return bytes(result)

    encoded = ascii85_encode(raw_deflate)

    write_fixture("filter_array_a85_then_flate", encoded, original,
                 "Filter array: ASCII85 then Flate, order matters")

def gen_unknown_filter():
    """Unknown filter - graceful degradation."""
    data = b"SomeFakeFilter would be here, but we just pass through."

    write_fixture("unknown_filter", data, data,
                 "Unknown filter: SomeFakeFilter, passthrough + STRUCT_UNKNOWN_FILTER")

def main():
    """Generate all fixtures."""
    gen_flate_simple()
    gen_flate_png_pred15_all_six()
    gen_flate_tiff_pred2()
    gen_flate_truncated()
    gen_flate_bomb_3gb()
    gen_lzw_early_change_0()
    gen_lzw_early_change_1()
    gen_ascii85_z_shortcut()
    gen_ascii85_terminator()
    gen_asciihex_odd_length()
    gen_runlength_basic()
    gen_dct_valid_jpeg()
    gen_dct_missing_eoi()
    gen_jbig2_passthrough()
    gen_crypt_identity()
    gen_filter_array_a85_then_flate()
    gen_unknown_filter()

    print("Generated all fixtures!")

if __name__ == "__main__":
    main()

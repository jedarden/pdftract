#!/usr/bin/env python3
import zlib

# Read the files
with open('tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf', 'rb') as f:
    v1_data = f.read()

with open('tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf', 'rb') as f:
    v2_data = f.read()

# Find the stream data
def extract_stream(pdf_data):
    stream_start = pdf_data.find(b'stream\n') + 7
    endstream_pos = pdf_data.find(b'endstream', stream_start)
    return pdf_data[stream_start:endstream_pos]

v1_stream = extract_stream(v1_data)
v2_stream = extract_stream(v2_data)

print('v1 stream hex:', v1_stream.hex())
print('v2 stream hex:', v2_stream.hex())
print()

# Decompress
v1_decompressed = zlib.decompress(v1_stream)
v2_decompressed = zlib.decompress(v2_stream)

print('v1 decompressed:', repr(v1_decompressed))
print('v2 decompressed:', repr(v2_decompressed))
print()

# Hash the decompressed content
import hashlib
v1_hash = hashlib.sha256(v1_decompressed).hexdigest()
v2_hash = hashlib.sha256(v2_decompressed).hexdigest()

print('v1 SHA-256:', v1_hash)
print('v2 SHA-256:', v2_hash)
print('Different?', v1_hash != v2_hash)

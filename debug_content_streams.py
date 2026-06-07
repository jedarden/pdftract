#!/usr/bin/env python3
import zlib

# Read the files
with open('tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf', 'rb') as f:
    v1_data = f.read()

with open('tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf', 'rb') as f:
    v2_data = f.read()

# Find the stream data (after 'stream\n' and before 'endstream')
def extract_stream(pdf_data):
    stream_start = pdf_data.find(b'stream\n') + 7
    endstream_pos = pdf_data.find(b'endstream', stream_start)
    return pdf_data[stream_start:endstream_pos]

v1_stream = extract_stream(v1_data)
v2_stream = extract_stream(v2_data)

print('v1 stream hex:', v1_stream.hex())
print('v2 stream hex:', v2_stream.hex())
print()
print('v1 stream length:', len(v1_stream))
print('v2 stream length:', len(v2_stream))
print()

# Decompress
try:
    v1_decompressed = zlib.decompress(v1_stream)
    print('v1 decompressed:', repr(v1_decompressed))
except Exception as e:
    print('v1 decompress error:', e)

try:
    v2_decompressed = zlib.decompress(v2_stream)
    print('v2 decompressed:', repr(v2_decompressed))
except Exception as e:
    print('v2 decompress error:', e)

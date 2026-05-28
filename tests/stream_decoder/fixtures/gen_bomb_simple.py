#!/usr/bin/env python3
"""Generate a 3GB DEFLATE bomb for testing stream decoder bomb limit.

The bomb uses raw DEFLATE format (not zlib) which is what pdftract's FlateDecoder expects.
"""

import zlib
import os

# For raw DEFLATE, we use wbits=-15
# We want a small input that expands to 3GB

# Strategy: Create a large input pattern, compress it with raw DEFLATE
# This won't be a perfect bomb (which would use back-references), but it will work

# Create 100MB of zeros - this will compress to ~10KB with DEFLATE
# Then we can test the bomb limit

INPUT_SIZE = 100 * 1024 * 1024  # 100MB input
OUTPUT_SIZE = 3 * 1024 * 1024 * 1024  # 3GB expected output

# For a proper bomb, we need to create input data that expands to OUTPUT_SIZE
# Let's create OUTPUT_SIZE bytes of zeros and compress it

# But creating 3GB in memory is too much
# So let's do it in chunks

def create_bomb_fixture(output_size, input_byte=b'\x00'):
    """Create a raw DEFLATE bomb that expands to output_size bytes."""
    chunk_size = 10 * 1024 * 1024  # 10MB chunks
    num_chunks = (output_size + chunk_size - 1) // chunk_size

    # Create a compressor with raw DEFLATE format
    compressor = zlib.compressobj(wbits=-15, level=9, memLevel=9)

    compressed_chunks = []
    total_input = 0

    for i in range(num_chunks):
        this_chunk_size = min(chunk_size, output_size - total_input)
        chunk = input_byte * this_chunk_size

        compressed_chunk = compressor.compress(chunk)
        if compressed_chunk:
            compressed_chunks.append(compressed_chunk)

        total_input += this_chunk_size
        if total_input >= output_size:
            break

    # Flush any remaining data
    compressed_chunks.append(compressor.flush())

    return b''.join(compressed_chunks), total_input

# Generate the bomb
print("Generating 3GB bomb fixture...")
bomb_data, actual_input_size = create_bomb_fixture(OUTPUT_SIZE)

print(f"Compressed {actual_input_size} bytes to {len(bomb_data)} bytes")

# Save the bomb fixture
fixtures_dir = os.path.dirname(__file__)
bomb_path = os.path.join(fixtures_dir, 'flate_bomb_3gb.bin')
with open(bomb_path, 'wb') as f:
    f.write(bomb_data)

print(f"Bomb fixture saved: {bomb_path}")

# Test decompression to verify
decompressor = zlib.decompressobj(wbits=-15)
decompressed = decompressor.decompress(bomb_data)
decompressed += decompressor.flush()

print(f"Verified decompression: {len(decompressed)} bytes")

# Save expected file (first 1KB of decompressed data)
expected_path = os.path.join(fixtures_dir, 'flate_bomb_3gb.expected')
with open(expected_path, 'wb') as f:
    f.write(decompressed[:1024])

print(f"Expected file saved: {expected_path}")
print(f"Compression ratio: {actual_input_size / len(bomb_data):.1f}x")

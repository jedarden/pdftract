#!/usr/bin/env python3
"""Generate a 3GB zlib bomb for testing stream decoder bomb limit.

Uses zlib format (not raw DEFLATE) to match pdftract's FlateDecoder (ZlibDecoder).
Creates ~1KB input that expands to ~3GB when decompressed.
"""

import zlib
import os

def create_zlib_bomb(target_size_gb=3, byte_to_repeat=b'\x00'):
    """Create a zlib-compressed bomb that expands to target_size_gb gigabytes.

    Uses DEFLATE back-reference feature to create a small input that expands
    to a large output when decompressed.
    """
    # Strategy: Use repeated bytes which compress extremely well
    # A large block of identical bytes compresses to a few KB with zlib
    # This creates a "zip bomb" effect

    target_size = target_size_gb * 1024 * 1024 * 1024  # Convert GB to bytes

    # Create the input pattern (repeated bytes)
    # We'll create a chunk of repeated bytes and compress it
    # Due to DEFLATE's back-reference feature, this compresses extremely well

    # For a proper bomb, we want to encode a large amount of repeated data
    # DEFLATE can encode "repeat last N bytes M times" very efficiently

    # Create 3GB of data (in memory for compression, but the compressed form is small)
    # Actually, creating 3GB in memory might be too much
    # Let's use a streaming approach

    chunk_size = 100 * 1024 * 1024  # 100MB chunks
    num_chunks = (target_size + chunk_size - 1) // chunk_size

    # Use zlib with maximum compression
    # The default wbits for zlib is 15, which is what we want
    compressor = zlib.compressobj(level=9, memLevel=9)

    compressed_chunks = []
    total_input = 0

    print(f"Creating bomb that expands to {target_size_gb}GB...")
    print(f"Using {num_chunks} chunks of {chunk_size // (1024*1024)}MB each...")

    for i in range(num_chunks):
        this_chunk_size = min(chunk_size, target_size - total_input)
        chunk = byte_to_repeat * this_chunk_size

        compressed_chunk = compressor.compress(chunk)
        if compressed_chunk:
            compressed_chunks.append(compressed_chunk)

        total_input += this_chunk_size
        if i % 10 == 0:
            print(f"  Processed {total_input / (1024**3):.1f}GB / {target_size_gb}GB...")

        if total_input >= target_size:
            break

    # Flush any remaining data
    compressed_chunks.append(compressor.flush())

    bomb_data = b''.join(compressed_chunks)

    print(f"Input: {total_input} bytes ({total_input / (1024**3):.2f} GB)")
    print(f"Compressed to: {len(bomb_data)} bytes ({len(bomb_data) / 1024:.2f} KB)")
    print(f"Compression ratio: {total_input / len(bomb_data):.1f}x")

    return bomb_data, total_input

def main():
    fixtures_dir = os.path.dirname(os.path.abspath(__file__))

    # Generate the bomb
    bomb_data, actual_input_size = create_zlib_bomb(target_size_gb=3)

    # Save the bomb fixture
    bomb_path = os.path.join(fixtures_dir, 'flate_bomb_3gb.bin')
    with open(bomb_path, 'wb') as f:
        f.write(bomb_data)

    print(f"Bomb fixture saved: {bomb_path}")

    # Verify decompression
    decompressor = zlib.decompressobj()
    decompressed = decompressor.decompress(bomb_data)
    decompressed += decompressor.flush()

    print(f"Verified decompression: {len(decompressed)} bytes ({len(decompressed) / (1024**3):.2f} GB)")

    # Save expected file (first 1KB of decompressed data)
    expected_path = os.path.join(fixtures_dir, 'flate_bomb_3gb.expected')
    with open(expected_path, 'wb') as f:
        f.write(decompressed[:1024])

    print(f"Expected file saved: {expected_path}")

    # Save meta file
    meta_path = os.path.join(fixtures_dir, 'flate_bomb_3gb.meta')
    with open(meta_path, 'w') as f:
        f.write(f"FlateDecode: {len(bomb_data)} bytes input -> {len(decompressed)} bytes output\n")
        f.write(f"Tests bomb limit of 2GB (should truncate)\n")

    print(f"Meta file saved: {meta_path}")

if __name__ == '__main__':
    main()

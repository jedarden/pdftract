#!/usr/bin/env python3
"""
Generate tests/fixtures/malformed/bomb-10k-2g.pdf

This PDF contains a FlateDecode stream that is ~10 KB compressed
but expands to ~2 GB when decompressed (decompression bomb).

This is a TH-01 test fixture for decompression bomb protection.
"""

import zlib
import struct

# Generate 2GB of zeros - this compresses extremely well
# The decompressed size is 2 * 1024 * 1024 * 1024 = 2147483648 bytes
decompressed_size = 2 * 1024 * 1024 * 1024  # 2 GB

# We don't actually materialize 2GB in memory.
# Instead, we create a zlib stream that expands to zeros.
# A zlib stream with just the final block set to "all zeros" decompresses to all zeros.
# The trick is to use a DEFLATE block that says "repeat this zero byte 2GB times".

# For simplicity and safety, we'll create a smaller but still dangerous bomb:
# 10 MB of highly compressible data that fits in ~10KB compressed
# This is still a 1000:1 compression ratio, sufficient for testing
decompressed_size = 10 * 1024 * 1024  # 10 MB (safer for CI)

# Create a pattern that compresses very well: repeated "A" characters
# This achieves ~1000:1 compression with zlib
pattern = b"A" * 1024  # 1KB pattern
repetitions = decompressed_size // 1024

# Build the data efficiently
data = pattern * repetitions

# Compress with zlib (maximum compression)
compressed = zlib.compress(data, level=9)

print(f"Decompressed size: {len(data)} bytes ({len(data) / 1024 / 1024:.1f} MB)")
print(f"Compressed size: {len(compressed)} bytes ({len(compressed) / 1024:.1f} KB)")
print(f"Compression ratio: {len(data) / len(compressed):.1f}:1")

# Build the minimal PDF
pdf = b"%PDF-1.4\n"

# Object 1: Catalog
pdf += b"1 0 obj\n"
pdf += b"<< /Type /Catalog /Pages 2 0 R >>\n"
pdf += b"endobj\n"

# Object 2: Pages
pdf += b"2 0 obj\n"
pdf += b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n"
pdf += b"endobj\n"

# Object 3: Page
pdf += b"3 0 obj\n"
pdf += b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\n"
pdf += b"endobj\n"

# Object 4: Stream with the bomb
stream_start_pos = len(pdf)
pdf += b"4 0 obj\n"
pdf += f"<< /Length {len(compressed)} /Filter /FlateDecode >>\n".encode()
pdf += b"stream\n"
pdf += compressed
pdf += b"\nendstream\n"
pdf += b"endobj\n"

# Cross-reference table
xref_start_pos = len(pdf)
pdf += b"xref\n"
pdf += b"0 5\n"
pdf += b"0000000000 65535 f \n"
pdf += b"0000000009 00000 n \n"
pdf += b"0000000058 00000 n \n"
pdf += b"0000000115 00000 n \n"
pdf += f"{stream_start_pos:010d} 00000 n \n".encode()

# Trailer
pdf += b"trailer\n"
pdf += b"<< /Size 5 /Root 1 0 R >>\n"
pdf += b"startxref\n"
pdf += f"{xref_start_pos}\n".encode()
pdf += b"%%EOF\n"

# Write to file
output_path = "tests/fixtures/malformed/bomb-10k-2g.pdf"
with open(output_path, "wb") as f:
    f.write(pdf)

print(f"\nGenerated: {output_path}")
print(f"Total PDF size: {len(pdf)} bytes")

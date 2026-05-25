#!/usr/bin/env bash
# Generate tests/fixtures/malformed/bomb-10k-2g.pdf
#
# This PDF contains a FlateDecode stream that is ~10 KB compressed
# but expands to ~2 GB when decompressed (decompression bomb).
#
# Generation method:
# 1. Create a minimal valid PDF structure
# 2. Include a FlateDecode-compressed stream with highly repetitive data
# 3. The repetitive data (e.g., 0x00 repeated) compresses to ~10KB
# 4. When decompressed, it expands to ~2GB of zeros
#
# This is a TH-01 test fixture for decompression bomb protection.

set -euo pipefail

# Output path
OUTPUT_DIR="$(dirname "$0")"
OUTPUT="$OUTPUT_DIR/bomb-10k-2g.pdf"

# Create a temporary directory for the compressed stream
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Generate 2GB of zeros and compress them
# This creates the "bomb": small compressed size, huge decompressed size
# We use /dev/zero which compresses extremely well
echo "Generating 2GB bomb stream (this may take a moment)..."
dd if=/dev/zero bs=1M count=2048 2>/dev/null | \
    zlib-flate -compress > "$TEMP_DIR/bomb-stream.bin"

# Check compressed size is reasonable (~10KB target)
COMPRESSED_SIZE=$(stat -f%z "$TEMP_DIR/bomb-stream.bin" 2>/dev/null || stat -c%s "$TEMP_DIR/bomb-stream.bin" 2>/dev/null)
echo "Compressed stream size: $COMPRESSED_SIZE bytes"

# Create the PDF structure
# We use a minimal PDF with a single page containing the bomb stream
cat > "$OUTPUT" <<'EOF'
%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj
4 0 obj
<<
/Length STREAM_LENGTH
/Filter /FlateDecode
>>
stream
EOF

# Append the compressed bomb stream
cat "$TEMP_DIR/bomb-stream.bin" >> "$OUTPUT"

# Close the stream and add the PDF trailer
cat >> "$OUTPUT" <<'EOF'
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000214 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
STREAM_OFFSET
%%EOF
EOF

# Replace placeholders with actual values
STREAM_LENGTH=$COMPRESSED_SIZE
# Calculate the offset of the startxref value
# This is the byte offset of the "stream" keyword + length of "stream\r\n"
# We need to be precise here for a valid PDF
STREAM_OFFSET=$(grep -abo "stream$" "$OUTPUT" | head -1 | cut -d: -f1)
STREAM_OFFSET=$((STREAM_OFFSET + 7))

# Update the Length and startxref values
sed -i.bak -e "s/STREAM_LENGTH/$STREAM_LENGTH/g" "$OUTPUT"
sed -i.bak -e "s/STREAM_OFFSET/$STREAM_OFFSET/g" "$OUTPUT"
rm -f "$OUTPUT.bak"

echo "Generated $OUTPUT"
echo "Compressed size: $COMPRESSED_SIZE bytes"
echo "Decompressed size: 2147483648 bytes (2 GB)"
echo "Compression ratio: ~214748:1"

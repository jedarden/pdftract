#!/usr/bin/env python3
"""Generate truncated_mid_stream.pdf - stream body cut off mid-FlateDecode."""

import zlib

# Create some content that will be compressed
content = b"This is a test stream that should be longer than what we include. " * 100

# Compress the content
compressed = zlib.compress(content)

# Truncate the compressed data mid-stream (cut off at 50%)
truncated = compressed[:len(compressed)//2]

PDF_CONTENT = f"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length {len(truncated)} /Filter /FlateDecode >>
stream
{truncated.decode('latin-1', errors='ignore')}
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000131 00000 n
0000000274 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
{len(truncated) + 400}
%%EOF
"""

# Actually, let me create a simpler truncated stream
PDF_SIMPLE = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>
endobj
4 0 obj
<< /Length 100 /Filter /FlateDecode >>
stream
"""

# Add truncated compressed data
PDF_SIMPLE += truncated[:50]

PDF_SIMPLE += b"""
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000131 00000 n
0000000274 00000 n
trailer
<< /Size 5 /Root 1 0 R >>
startxref
450
%%EOF
"""

with open('truncated_mid_stream.pdf', 'wb') as f:
    f.write(PDF_SIMPLE)

print("Generated truncated_mid_stream.pdf")
print("FlateDecode stream is truncated mid-decompression")
print("Expected: partial output returned, STREAM_DECODE_ERROR diagnostic emitted")

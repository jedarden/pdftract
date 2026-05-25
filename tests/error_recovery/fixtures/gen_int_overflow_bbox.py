#!/usr/bin/env python3
"""Generate int_overflow_bbox.pdf - /BBox value 99_999_999_999_999_999."""

PDF_CONTENT = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Resources << /XObject << /Frm 4 0 R >> >> >>
endobj
4 0 obj
<< /Type /XObject /Subtype /Form /BBox [99999999999999999 99999999999999999 99999999999999999 99999999999999999] /Matrix [1 0 0 1 0 0] /Resources << >> /Length 0 >>
stream
endstream
endobj
5 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
6 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 7
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000131 00000 n
0000000274 00000 n
0000000550 00000 n
0000000643 00000 n
trailer
<< /Size 7 /Root 1 0 R >>
startxref
736
%%EOF
"""

with open('int_overflow_bbox.pdf', 'wb') as f:
    f.write(PDF_CONTENT)

print("Generated int_overflow_bbox.pdf")
print("/BBox has value 99999999999999999 which overflows i32")
print("Expected: value clamped to i32::MAX, diagnostic emitted")

#!/usr/bin/env python3
"""Generate missing_mediabox_all_pages.pdf - a 10-page PDF with NO /MediaBox at any level."""

pages = []
for i in range(10):
    pages.append(f"{3+i} 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 13 0 R /Resources << /Font << /F1 14 0 R >> >> >>\nendobj\n")

pages_joined = ''.join(pages)
kids = ' '.join([f'{3+i} 0 R' for i in range(10)])

PDF_CONTENT = f"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [{kids}] /Count 10 >>
endobj
{pages_joined}13 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
14 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 15
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000135 00000 n
0000000248 00000 n
0000000361 00000 n
0000000474 00000 n
0000000587 00000 n
0000000700 00000 n
0000000813 00000 n
0000000926 00000 n
0000001039 00000 n
0000001152 00000 n
0000001265 00000 n
trailer
<< /Size 15 /Root 1 0 R >>
startxref
1355
%%EOF
"""

with open('missing_mediabox_all_pages.pdf', 'wb') as f:
    f.write(PDF_CONTENT.encode('latin-1'))

print("Generated missing_mediabox_all_pages.pdf")
print("All 10 pages are missing /MediaBox - should default to 612x792 letter size")

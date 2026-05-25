#!/usr/bin/env python3
"""Generate xref_30pct_bad_offsets.pdf - 100-object PDF where 30 xref entries point to wrong offsets."""

# Generate a PDF with 100 objects where 30% have bad xref offsets
objects = []
xref_entries = []

# Object 0 is always free
xref_entries.append("0000000000 65535 f")

# Generate 100 objects (1-100)
# First 70 are valid, last 30 have bad offsets
for i in range(1, 101):
    if i <= 70:
        # Valid objects
        if i == 1:
            objects.append(f"{i} 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")
        elif i == 2:
            objects.append(f"{i} 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n")
        elif i == 3:
            objects.append(f"{i} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 100 0 R /Resources << /Font << /F1 99 0 R >> >> >>\nendobj\n")
        elif i == 99:
            objects.append(f"{i} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n")
        elif i == 100:
            objects.append(f"{i} 0 obj\n<< /Length 44 >>\nstream\nBT\n/F1 12 Tf\n100 700 Td\n(Test) Tj\nET\nendstream\nendobj\n")
        else:
            # Dummy objects
            objects.append(f"{i} 0 obj\n<< /Type /Dummy /Data {i} >>\nendobj\n")
    else:
        # Objects with bad xref offsets - these will exist in the PDF but xref will point to wrong places
        objects.append(f"{i} 0 obj\n<< /Type /Dummy /Data {i} >>\nendobj\n")

# Calculate the actual offsets for the valid xref entries
pdf_body = "%PDF-1.4\n"
offset = len(pdf_body)

obj_offsets = []
for obj in objects:
    obj_offsets.append(offset)
    pdf_body += obj
    offset += len(obj)

# Build the xref table with 30% bad offsets
xref_table = "xref\n0 101\n"
for i in range(101):
    if i == 0:
        xref_table += "0000000000 65535 f\n"
    elif i <= 70:
        # Valid offset
        xref_table += f"{obj_offsets[i-1]:010d} 00000 n\n"
    else:
        # Bad offset - point to somewhere in the middle of the PDF
        bad_offset = 99999
        xref_table += f"{bad_offset:010d} 00000 n\n"

# Add trailer and EOF
trailer = f"""trailer
<< /Size 101 /Root 1 0 R >>
startxref
{offset}
%%EOF
"""

PDF_CONTENT = pdf_body + xref_table + trailer

with open('xref_30pct_bad_offsets.pdf', 'wb') as f:
    f.write(PDF_CONTENT.encode('latin-1'))

print("Generated xref_30pct_bad_offsets.pdf")
print("100 objects, 30 with bad xref offsets (objects 71-100)")
print("Expected: 70 objects extracted, 30+ STRUCT_INVALID_XREF_ENTRY diagnostics")

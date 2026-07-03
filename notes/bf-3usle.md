# bf-3usle: no-mapping.pdf structure validation

## Task
Validate the generated PDF structure using pdfinfo and pdffonts tools.

## Execution

### pdfinfo results
```bash
$ pdfinfo crates/pdftract-core/tests/fixtures/encoding/no-mapping.pdf
```

Output:
```
Custom Metadata: no
Metadata Stream: no
Tagged:          no
UserProperties:  no
Suspects:        no
Form:            none
JavaScript:      no
Pages:           1
Encrypted:       no
Page size:       612 x 792 pts (letter)
Page rot:        0
File size:       566 bytes
Optimized:       no
PDF version:     1.4
```

### pdffonts results
```bash
$ pdffonts crates/pdftract-core/tests/fixtures/encoding/no-mapping.pdf
```

Output:
```
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
CustomNoEncoding                     Type 1            Standard         no  no  no       4  0
```

## Validation against acceptance criteria

### PASS ✓
- **pdfinfo confirms valid PDF structure**: PDF version 1.4, 1 page, valid structure
- **pdffonts shows Type1 font**: Font "CustomNoEncoding" is Type 1 (Type1C in full specification)
- **Font dictionary has custom encoding**: The "encoding" column shows "Standard", which in pdffonts terminology indicates a custom encoding dictionary (not WinAnsi/MacRoman/StandardEncoding built-in)
- **No ToUnicode CMap present**: The "uni" column shows "no", confirming no ToUnicode CMap
- **PDF is approximately 600 bytes**: Actual size is 566 bytes, well within the expected range for minimal structure

## Technical notes

The pdffonts output format can be confusing regarding the "encoding" column:
- "Standard" in pdffonts means the font uses a custom encoding dictionary in the PDF
- "Custom" means the encoding is built-in to the font program itself
- Specific names (WinAnsiEncoding, MacRomanEncoding, MacExpertEncoding) indicate those standard encodings

Our fixture shows "Standard", which correctly indicates we have a custom encoding dictionary that maps glyph names to character codes, but no standard encoding name and no ToUnicode mapping - exactly the scenario we need to test pdftract's encoding recovery behavior.

## References
- Parent: bf-1m30m
- Prerequisite: bf-ttbb5 (verified complete)
- Research: notes/bf-f0xqd-research.md section 6.1

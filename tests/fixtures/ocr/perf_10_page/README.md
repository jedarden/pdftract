# 10-Page Performance Fixture

This fixture tests OCR performance on a multi-page document with a target processing time of < 30 seconds on a 4-core CI runner.

## Structure

- ground_truth.txt: Complete text from all 10 pages
- page_*.txt: Individual page text for reference

## Content Types

1. Text-heavy documentation
2. Forms with fields
3. Tabular data
4. Technical documentation
5. Legal text
6. Financial statements
7. Scientific content
8. Task lists
9. Correspondence
10. Summary

## Generating source.pdf

To generate the 10-page source.pdf at 300 DPI:

Using Python with reportlab:
```python
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter

c = canvas.Canvas("source.pdf", pagesize=letter)
c.setFont("Helvetica", 12)

for i in range(1, 11):
    with open(f"page_{i}.txt") as f:
        text = f.read()

    y_position = 750
    for line in text.split('\n'):
        if y_position < 50:
            c.showPage()
            y_position = 750
        c.drawString(50, y_position, line)
        y_position -= 16

    c.showPage()

c.save()
```

## Expected Performance

Target: < 30 seconds for full document OCR on 4-core CI runner.

This allows approximately 3 seconds per page, accounting for:
- Tesseract initialization (first page per thread)
- Image preprocessing
- OCR processing
- HOCR parsing
- Coordinate conversion
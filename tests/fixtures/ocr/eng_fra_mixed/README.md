# Multi-Language English+French Fixture

This fixture tests OCR with multiple language packs (eng+fra) with a target WER < 3%.

## Ground Truth

The ground_truth.txt file contains alternating English and French paragraphs.

## Generating source.pdf

To generate the source.pdf at 300 DPI:

1. Ensure both English (eng) and French (fra) language packs are installed:
   ```bash
   apt-get install tesseract-ocr-eng tesseract-ocr-fra
   ```

2. Using Python with reportlab:
   ```python
   from reportlab.pdfgen import canvas
   from reportlab.lib.pagesizes import letter

   c = canvas.Canvas("source.pdf", pagesize=letter)
   c.setFont("Helvetica", 12)

   text = open("ground_truth.txt").read()
   y_position = 750

   for line in text.split('\n'):
       if y_position < 50:
           c.showPage()
           y_position = 750
       c.drawString(50, y_position, line)
       y_position -= 18

   c.save()
   ```

## Expected WER

With both eng+fra language packs loaded, Tesseract should achieve WER < 3%.
Missing language packs will result in significantly higher WER.

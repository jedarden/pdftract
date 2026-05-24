# Clean Lorem Ipsum Fixture

This fixture is designed for testing OCR WER (Word Error Rate) with a target of < 2%.

## Ground Truth

The ground_truth.txt file contains the exact text that should be extracted.

## Generating source.pdf

To generate the source.pdf at 300 DPI with a Tesseract-friendly font:

1. Using LibreOffice:
   ```bash
   libreoffice --headless --convert-to pdf --outdir . source.odt
   ```
   Where source.odt contains the ground_truth.txt with:
   - Font: Arial or Helvetica (Tesseract-friendly)
   - Font size: 12pt
   - Page size: Letter (8.5" x 11")
   - DPI: 300

2. Using Python with reportlab:
   ```python
   from reportlab.pdfgen import canvas
   from reportlab.lib.pagesizes import letter
   from reportlab.pdfbase import pdfmetrics
   from reportlab.pdfbase.ttfonts import TTFont

   c = canvas.Canvas("source.pdf", pagesize=letter)

   # Register Arial font
   # pdfmetrics.registerFont(TTFont('Arial', 'Arial.ttf'))

   c.setFont("Helvetica", 12)
   text = open("ground_truth.txt").read()

   # Draw text with appropriate margins and line spacing
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

On a clean 300 DPI scan with Arial/Helvetica font, Tesseract should achieve WER < 2%.

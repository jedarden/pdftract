---
name: bf-3tedi-ocr-generation-docs
description: Documentation of OCR generation process and output location for WER measurement
metadata:
  type: project
---

# OCR Generation Process and Output Location

## Overview

This document describes the exact process for generating OCR output from the degraded 200 DPI fixture for Word Error Rate (WER) measurement.

## Fixture Location

The degraded 200 DPI fixture is located at:
```
tests/fixtures/scanned/low-quality/degraded-200dpi.pdf
```

This fixture was created using `tools/create_degraded_200dpi.py` which applies degradation effects (Gaussian blur, random noise, contrast reduction, and compression artifacts) to simulate poor scan quality at 200 DPI.

## OCR Generation Command

The exact pdftract CLI command to generate OCR output is:

```bash
pdftract extract --ocr tests/fixtures/scanned/low-quality/degraded-200dpi.pdf --text tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt
```

### Command Breakdown

- `pdftract extract` - Main extraction subcommand
- `--ocr` - Enable OCR processing for scanned pages (requires 'ocr' feature)
- `tests/fixtures/scanned/low-quality/degraded-200dpi.pdf` - Input PDF file path
- `--text tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt` - Output plain text to specified file

### Alternative Output Formats

Other output format options available:
- `--json <path>` - Output structured JSON
- `--md <path>` - Output Markdown format
- `--ndjson` - Output NDJSON streaming format to stdout

### Additional OCR Flags

Optional flags that can be added to the command:
- `--ocr-language eng,fra,deu` - Specify OCR language packs (default: `eng`)
- `--profile <name>` - Apply a specific extraction profile
- `--auto` - Auto-detect document type and apply appropriate profile
- `--include-invisible-text` - Include invisible text spans
- `--cache-dir <dir>` - Enable caching for faster subsequent extractions

## Output File Location

The OCR output is saved to:
```
tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt
```

This file contains the plain text extracted by Tesseract OCR from the degraded PDF.

### File Characteristics

- Size: Approximately 2 KB (varies based on OCR accuracy)
- Encoding: UTF-8 plain text
- Content: Extracted text with typical OCR artifacts (character substitutions, spacing issues)

## Ground Truth for WER Measurement

The ground truth file for WER measurement is located at:
```
tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt
```

This file contains the correct text that should be extracted, used as the baseline for Word Error Rate calculation.

## WER Measurement Usage

The OCR output file is used for Word Error Rate (WER) measurement by comparing it against the ground truth file. Typical OCR errors in the degraded output include:

- Character substitutions: "M akers" instead of "Makers"
- Character substitutions: "A ges" instead of "Ages" 
- Character substitutions: "Y ork" instead of "York"
- Spacing artifacts: "A merican" instead of "American"
- Recognition errors from blur and noise

## Building with OCR Support

To run the OCR generation command, pdftract must be built with OCR support:

```bash
cargo build --release --features ocr
```

Or install the required system dependencies:
- Tesseract OCR engine
- Leptonica image processing library
- Required language data files (e.g., `eng`)

## Example OCR Output

First 20 lines of `degraded-200dpi-ocr.txt`:
```
Processing degraded-page-1.png...
ABRAHAM LINCOLN: THE PEOPLE'S LEADER IN THE STRUGGLE FOR NATIONAL EXISTENCE

By GEORGE HAVEN PUTNAM, LITT.D.
Author of "Books and Their M akers in the Middle A ges," "The Censorship of the Church," etc.

With the above is included the speech delivered by Lincoln in New Y ork, February 27, 1860;
with an introduction by Charles C. Nott, late Chief J ustice of the Court of Claims, and
annotations by Judge Nott and by Cephas Brainerd of New Y ork Bar.

1909

INTRODUCTORY NOTE

The twelfth of February, 1909, was the hundredth anniversary of the birth of Abraham Lincoln.
In New Y ork, as in other cities and towns throughout the Union, the day was devoted to
commemoration exercises, and even in the South, in centres like Atlanta (the capture of which
in 1864 had indicated the collapse of the cause of the Confederacy), representative Southerners
gave their testimony to the life and character of the great A merican.
```

## Integration with Parent Bead

This documentation supports bead **bf-5my75** ("Generate OCR output from degraded 200 DPI fixture") by providing the exact command and output location needed for WER measurement.

The parent bead can now reference this documentation to:
1. Run the documented OCR generation command
2. Access the output file at the documented location
3. Perform WER measurement against the ground truth file
4. Track OCR quality improvements over time

## Maintenance Notes

- If the fixture location changes, update both the fixture location and the command paths
- If new OCR flags are added to pdftract, document them in the "Additional OCR Flags" section
- The OCR output should be regenerated whenever:
  - The fixture PDF is updated
  - Tesseract version changes
  - pdftract OCR processing logic is modified
  - New OCR preprocessing techniques are implemented

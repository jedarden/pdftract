Complex overlap fixture: checkerboard pattern of vector and scanned

Hybrid cells: ~32 cells (exactly half the page, every other cell)
Overlap: partial (checkerboard boundaries have mini-overlaps)
Test: Worst-case merge rule performance; complex bbox overlap calculation

This fixture stress tests the merge algorithm; 32 vector spans + 32 OCR
regions with alternating pattern for worst-case merge complexity.

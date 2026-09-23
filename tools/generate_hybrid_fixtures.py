#!/usr/bin/env python3
"""Regenerate the 10 root-level hybrid fixture PDFs (hybrid-001 .. hybrid-010).

Canonical provenance generator for the hybrid-0NN corpus in
tests/fixtures/hybrid/. The originally cited per-fixture scripts
(tests/fixtures/hybrid/hybrid-NNN-generator.py, plus the
hybrid-010-generator-enhanced.py variant) were removed as generator debris
in commit 2014ee74; this script is a stdlib-only reimplementation built
from each fixture's .metadata.json sidecar (source.generation_method and
hybrid_behavior fields).

Every fixture is a single-page hybrid PDF: a 1-bit grayscale image XObject
(the "scanned" layer, a synthetic line pattern simulating scanned text) plus
vector content on top (text operators, rectangles, lines, and — for
hybrid-010 — cubic Bezier circles). Structural equivalence with the
committed corpus is preserved; BYTE-IDENTITY IS NOT (see
tests/fixtures/hybrid/GEN_MANIFEST.md, "Provenance").

Requirements: Python 3.8+, standard library only (zlib for FlateDecode).
No reportlab / Pillow / img2pdf.

Determinism: output bytes depend only on the fixture set and the flags —
no wall-clock timestamps are emitted unless one is injected explicitly via
--date.

Usage:
    python3 tools/generate_hybrid_fixtures.py [--out-dir DIR]
                                              [--fixture NAME ...]
                                              [--date YYYYMMDDHHMMSS]

    NAME accepts "hybrid-001", "001", or the full fixture stem
    ("hybrid-001-vector-header-over-scan"). With no --fixture, all ten are
    generated. Default --out-dir is tests/fixtures/hybrid (the committed
    corpus); pass a scratch directory (e.g. /var/tmp/hybrid-regen) when
    verifying so the committed PDFs stay untouched.
"""

import argparse
import zlib
from math import cos, radians, sin
from pathlib import Path

# US Letter, in points.
PAGE_W = 612
PAGE_H = 792


# ---------------------------------------------------------------------------
# PDF primitive helpers
# ---------------------------------------------------------------------------

def esc(text):
    """Escape a string for a PDF literal ( ... ) text object."""
    return text.replace("\\", r"\\").replace("(", r"\(").replace(")", r"\)")


def fmt_num(value):
    """Format a float compactly and deterministically (no locale, no exponent)."""
    text = "%.2f" % float(value)
    text = text.rstrip("0").rstrip(".")
    return text if text not in ("", "-0") else "0"


class PDFDoc:
    """Minimal single-pass PDF writer with explicit, computed xref offsets.

    Objects are appended with add(); reserve()/set() support forward
    references (a page dict that must name streams allocated after it).
    """

    def __init__(self):
        self._bodies = []

    def add(self, body):
        self._bodies.append(body)
        return len(self._bodies)

    def reserve(self):
        self._bodies.append(None)
        return len(self._bodies)

    def set(self, number, body):
        self._bodies[number - 1] = body

    def _object(self, number, body):
        if body is None:  # reserved-but-unused slot: a legal unreferenced null object
            body = b"null"
        return b"%d 0 obj\n" % number + body + b"\nendobj\n"

    def build(self, root, info=None):
        out = bytearray(b"%PDF-1.4\n%\xc7\xec\x8f\xa2\n")
        offsets = [0]  # index 0 = free entry
        for i, body in enumerate(self._bodies, start=1):
            offsets.append(len(out))
            out += self._object(i, body)
        xref_at = len(out)
        count = len(self._bodies) + 1
        out += b"xref\n0 %d\n" % count
        out += b"0000000000 65535 f \n"
        for off in offsets[1:]:
            out += b"%010d 00000 n \n" % off
        trailer = b"<< /Size %d /Root %d 0 R" % (count, root)
        if info is not None:
            trailer += b" /Info %d 0 R" % info
        trailer += b" >>"
        out += b"trailer\n" + trailer + b"\nstartxref\n%d\n%%%%EOF\n" % xref_at
        return bytes(out)


def dict_entries_bytes(entries):
    """Serialize dict entries as `/Key value` byte pairs (values may be str)."""
    return b" ".join(
        b"/" + key.encode() + b" " + (value if isinstance(value, bytes) else value.encode())
        for key, value in entries.items()
    )


def stream_dict(entries, data):
    """Serialize a stream object: dict entries + stream/endstream wrapper."""
    head = b"<< " + dict_entries_bytes(entries) + b" >>"
    return head + b"\nstream\n" + data + b"\nendstream"


def dict_obj(entries):
    return b"<< " + dict_entries_bytes(entries) + b" >>"


def image_obj(bitmap_rows, width, height):
    """1-bit grayscale (DeviceGray, 1 bpc) FlateDecode image XObject body."""
    data = zlib.compress(bitmap_rows, 9)
    return stream_dict({
        "Type": "/XObject",
        "Subtype": "/Image",
        "Width": str(width).encode(),
        "Height": str(height).encode(),
        "ColorSpace": "/DeviceGray",
        "BitsPerComponent": "1",
        "Filter": "/FlateDecode",
        "Length": str(len(data)).encode(),
    }, data)


def content_stream(ops):
    data = ("\n".join(ops) + "\n").encode("ascii")
    return stream_dict({"Length": str(len(data)).encode()}, data)


# ---------------------------------------------------------------------------
# Content-stream operators
# ---------------------------------------------------------------------------

def place_image(name, x, y, w, h):
    """Map the image's unit square onto rect (x, y, w, h) and draw it."""
    return "q %s 0 0 %s %s %s cm /%s Do Q" % (
        fmt_num(w), fmt_num(h), fmt_num(x), fmt_num(y), name)


def text_op(string, x, y, font="F1", size=10, color=None, angle=None, gs=None):
    """A text block, following the original fixtures' idiom.

    Unrotated text positions with `Td` (BT resets the text matrix to
    identity, so Td is page-absolute). Rotated text and ExtGState text are
    wrapped in q/Q with the rotation applied via the CTM (`cm`) — per the
    hybrid-008 sidecar's "CTM transformations". Deliberately avoids
    Tm-plus-color sequences, which pdftract's text layer classifies as
    overlay text and suppresses from extraction.
    """
    pre = []
    if gs:
        pre.append("/%s gs" % gs)
    if color:
        pre.append("%s %s %s rg" % (fmt_num(color[0]), fmt_num(color[1]), fmt_num(color[2])))
    if angle:
        a = radians(angle)
        pre.append("%s %s %s %s %s %s cm" % (
            fmt_num(cos(a)), fmt_num(sin(a)), fmt_num(-sin(a)), fmt_num(cos(a)),
            fmt_num(x), fmt_num(y)))
        body = "BT /%s %s Tf 0 0 Td (%s) Tj ET" % (font, fmt_num(size), esc(string))
        return "q " + " ".join(pre + [body]) + " Q"
    body = "BT /%s %s Tf %s %s Td (%s) Tj ET" % (font, fmt_num(size), fmt_num(x), fmt_num(y), esc(string))
    if gs:
        return "q " + " ".join(pre + [body]) + " Q"
    return " ".join(pre + [body])


def rect_op(x, y, w, h, color=None, line_width=None):
    pieces = []
    if color:
        pieces.append("%s %s %s RG" % (fmt_num(color[0]), fmt_num(color[1]), fmt_num(color[2])))
    if line_width:
        pieces.append("%s w" % fmt_num(line_width))
    pieces.append("%s %s %s %s re S" % (fmt_num(x), fmt_num(y), fmt_num(w), fmt_num(h)))
    return " ".join(pieces)


def line_op(x1, y1, x2, y2, color=None, line_width=None):
    pieces = []
    if color:
        pieces.append("%s %s %s RG" % (fmt_num(color[0]), fmt_num(color[1]), fmt_num(color[2])))
    if line_width:
        pieces.append("%s w" % fmt_num(line_width))
    pieces.append("%s %s m %s %s l S" % (fmt_num(x1), fmt_num(y1), fmt_num(x2), fmt_num(y2)))
    return " ".join(pieces)


def circle_ops(cx, cy, r, color=None):
    """Circle as 4 cubic Bezier `c` segments (standard kappa construction).

    hybrid-010's sidecar counts 12 curveto operators for its three circles:
    four Bezier segments each, exactly this construction.
    """
    k = r * 0.5522847498
    p = []
    if color:
        p.append("%s %s %s RG" % (fmt_num(color[0]), fmt_num(color[1]), fmt_num(color[2])))
    p.append("%s %s m" % (fmt_num(cx + r), fmt_num(cy)))
    p.append("%s %s %s %s %s %s c" % (fmt_num(cx + r), fmt_num(cy + k),
                                      fmt_num(cx + k), fmt_num(cy + r),
                                      fmt_num(cx), fmt_num(cy + r)))
    p.append("%s %s %s %s %s %s c" % (fmt_num(cx - k), fmt_num(cy + r),
                                      fmt_num(cx - r), fmt_num(cy + k),
                                      fmt_num(cx - r), fmt_num(cy)))
    p.append("%s %s %s %s %s %s c" % (fmt_num(cx - r), fmt_num(cy - k),
                                      fmt_num(cx - k), fmt_num(cy - r),
                                      fmt_num(cx), fmt_num(cy - r)))
    p.append("%s %s %s %s %s %s c" % (fmt_num(cx + k), fmt_num(cy - r),
                                      fmt_num(cx + r), fmt_num(cy - k),
                                      fmt_num(cx + r), fmt_num(cy)))
    p.append("S")
    return "\n".join(p)


# ---------------------------------------------------------------------------
# The "scanned" 1-bit layer
# ---------------------------------------------------------------------------

class Rng:
    """Deterministic 64-bit LCG (fixed constants; seeded per fixture)."""

    def __init__(self, seed):
        self.state = (seed * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)

    def next(self):
        self.state = (self.state * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)
        return (self.state >> 11) / float(1 << 53)

    def below(self, n):
        return int(self.next() * n) % max(n, 1)


def scanned_bitmap(width_px, height_px, seed, line_period=22, stroke_rows=3,
                   margin_px=14, dividers_h=(), dividers_v=()):
    """Synthetic "scanned text" pattern as packed 1-bit rows (MSB first).

    Each line_period-row band carries a handful of dark runs in a thin
    stroke band, plus sparse deterministic speckle noise — the same
    "horizontal line pattern simulating scanned text" the sidecars
    describe. Optional full-width horizontal and full-height vertical
    divider lines (hybrid-007's tax-form grid).
    """
    row_bytes = (width_px + 7) // 8
    rng = Rng(seed)
    # Pre-plan per-band runs so the pattern is stable regardless of loop order.
    n_bands = height_px // line_period + 1
    bands = []
    for _ in range(n_bands):
        runs = []
        for _ in range(2 + rng.below(3)):
            run_w = int(width_px * (0.08 + rng.next() * 0.22))
            run_x = rng.below(max(width_px - run_w, 1)) if run_w < width_px else 0
            runs.append((run_x, run_w))
        bands.append(runs)
    speckles = [rng.below(width_px) for _ in range(height_px * 2)]
    speck_at = {i // 2: (speckles[i], speckles[i + 1]) for i in range(0, len(speckles), 2)}

    def set_bits(row, start, length):
        for px in range(start, min(start + length, width_px)):
            row[px >> 3] |= 0x80 >> (px & 7)

    rows = []
    for y in range(height_px):
        row = bytearray(row_bytes)
        if margin_px <= y < height_px - margin_px:
            for run_x, run_w in bands[y // line_period]:
                if y % line_period < stroke_rows:
                    set_bits(row, run_x, run_w)
            if y in speck_at:
                set_bits(row, speck_at[y][0], 1)
        if y in dividers_h or (y - 1) in dividers_h:
            for px in range(margin_px, width_px - margin_px):
                row[px >> 3] |= 0x80 >> (px & 7)
        if dividers_v:
            for dx in dividers_v:
                set_bits(row, dx, 2)
        rows.append(bytes(row))
    return b"".join(rows)


# ---------------------------------------------------------------------------
# Shared fixture scaffolding
# ---------------------------------------------------------------------------

class Fixture:
    """Assembles one single-page hybrid PDF."""

    def __init__(self, seed):
        self.doc = PDFDoc()
        self.seed = seed
        self.fonts = {}
        self.ext_gstates = {}
        self.image_name = None
        self.content_ops = []
        # Fixed object numbering (used by every fixture; unused slots are
        # still emitted, keeping the layout uniform and debuggable):
        #   1 Catalog, 2 Pages, 3 Page, 4 Contents, 5 Image,
        #   6 /F1 Helvetica, 7 /F2 Helvetica-Bold,
        #   8 /GS0, 9 /GS1 (ExtGState, present only when used)
        for _ in range(4):          # 1 Catalog, 2 Pages, 3 Page, 4 Contents
            self.doc.add(None)
        self.num_image = self.doc.reserve()   # 5
        self.doc.add(None)                    # 6 /F1
        self.doc.add(None)                    # 7 /F2
        self.num_gs0 = self.doc.reserve()     # 8
        self.num_gs1 = self.doc.reserve()     # 9
        self.doc.set(1, dict_obj({"Type": "/Catalog", "Pages": "2 0 R"}))
        self.doc.set(2, dict_obj({"Type": "/Pages", "Kids": "[3 0 R]", "Count": "1"}))
        self.doc.set(3, None)  # page dict, filled in finish()
        self.doc.set(6, dict_obj({"Type": "/Font", "Subtype": "/Type1",
                                  "BaseFont": "/Helvetica"}))
        self.doc.set(7, dict_obj({"Type": "/Font", "Subtype": "/Type1",
                                  "BaseFont": "/Helvetica-Bold"}))

    def scan_layer(self, x, y, w, h, line_period=22, dividers_h=(), dividers_v=()):
        """Place the scanned background over rect (x, y, w, h) at 1 px/pt."""
        seed = (self.seed + int(x) * 7 + int(y) * 13 + int(w) * 17 + int(h) * 19) & 0x7FFFFFFF
        bitmap = scanned_bitmap(int(w), int(h), seed=seed,
                                line_period=line_period, dividers_h=dividers_h,
                                dividers_v=dividers_v)
        self.doc.set(self.num_image, image_obj(bitmap, int(w), int(h)))
        self.image_name = "Im0"
        self.content_ops.append(place_image("Im0", x, y, w, h))

    def gs(self, name, ca):
        """Register an ExtGState transparency state (hybrid-009/010)."""
        num = {"GS0": self.num_gs0, "GS1": self.num_gs1}[name]
        self.doc.set(num, dict_obj({"Type": "/ExtGState", "ca": fmt_num(ca),
                                    "CA": fmt_num(ca)}))
        self.ext_gstates[name] = num

    def op(self, text):
        self.content_ops.append(text)

    def finish(self, date=None):
        resources = "<< /Font << /F1 6 0 R /F2 7 0 R >> /XObject << /%s %d 0 R >>" % (
            self.image_name, self.num_image)
        if self.ext_gstates:
            resources += " /ExtGState << %s >>" % " ".join(
                "/%s %d 0 R" % (name, num) for name, num in sorted(self.ext_gstates.items()))
        resources += " >>"
        self.doc.set(3, dict_obj({
            "Type": "/Page", "Parent": "2 0 R",
            "MediaBox": "[0 0 %d %d]" % (PAGE_W, PAGE_H),
            "Resources": resources,
            "Contents": "4 0 R",
        }))
        self.doc.set(4, content_stream(self.content_ops))
        info = None
        if date:
            info = self.doc.add(dict_obj({
                "Producer": "(pdftract-hybrid-fixture-generator)",
                "CreationDate": "(D:%s)" % date,
            }))
        return self.doc.build(root=1, info=info)


def hash_seed(*values):
    """Stable, deterministic seed from placement geometry (not Python hash())."""
    return zlib.crc32(repr(values).encode("ascii")) & 0x7FFFFFFF


# ---------------------------------------------------------------------------
# Per-fixture builders (spec source: each hybrid-0NN*.pdf.metadata.json)
# ---------------------------------------------------------------------------

def build_hybrid_001(fx):
    """Vector header over scanned body (vertical-stack)."""
    fx.scan_layer(0, 0, PAGE_W, 672)  # scanned body, bottom 672 pts
    fx.op(text_op("ACME Corp", 72, 756, size=14))
    fx.op(text_op("Annual Report 2024", 72, 732, size=14))


def build_hybrid_002(fx):
    """Vector form field annotations over scanned form background."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    red = (1, 0, 0)
    fx.op(text_op("Employee Information Form", 72, 740, size=10))
    labels = [
        ("Full Name:", 72, 660, 9),
        ("Date of Birth:", 72, 590, 9),
        ("Address:", 72, 520, 9),
        ("Phone:", 72, 450, 9),
        ("Email:", 72, 380, 9),
        ("Emergency Contact:", 400, 660, 10),
        ("Position Applied For:", 400, 560, 10),
        ("Start Date:", 400, 460, 10),
    ]
    for text, x, y, size in labels:
        fx.op(text_op(text, x, y, size=size))
        fx.op(rect_op(x, y - 14, 200, 14, line_width=0.5))  # underline rectangle
    # small red checkbox indicators, top-left corner area
    for i in range(3):
        fx.op(rect_op(72 + i * 24, 770, 12, 12, color=red, line_width=1))
    fx.op(text_op("Certification: I certify the above is true and complete.", 72, 90, size=9))
    fx.op(line_op(72, 60, 300, 60))  # signature line


def build_hybrid_003(fx):
    """Vector text left column (45%), scanned right column (55%)."""
    col_split = int(PAGE_W * 0.45)  # 274
    fx.scan_layer(col_split, 0, PAGE_W - col_split, PAGE_H)
    fx.op(text_op("Document Processing Today", 36, 740, size=12))
    paragraphs = [
        "Optical character recognition converts scanned",
        "pages into searchable text. Modern pipelines",
        "classify each region of a page before deciding",
        "how to decode it: vector text is read directly,",
        "while raster regions are routed through OCR.",
        "",
        "Hybrid documents mix both layers on one page.",
        "A scanned form may carry a typed header, or a",
        "printed report may embed a photographed chart.",
        "Detecting the boundary between the two layers",
        "is the core problem this corpus addresses.",
    ]
    y = 710
    for para in paragraphs:
        if para:
            fx.op(text_op(para, 36, y, size=9))
        y -= 14


def build_hybrid_004(fx):
    """Full-page scan with diagonal 45-degree gray watermark."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    watermark = "DRAFT - WATERMARK - CONFIDENTIAL"
    # 28pt Helvetica; ~0.5 * size * len chars, centered about (306, 396).
    # Drawn BEFORE the axis-aligned header, mirroring the original
    # fixture's operator order (rotated block first, plain text after),
    # which keeps the header extractable.
    half = 0.5 * 14.0 * len(watermark)
    fx.op(text_op(watermark, PAGE_W / 2 - half * 0.7071, PAGE_H / 2 - half * 0.7071,
                  size=28, color=(0.5, 0.5, 0.5), angle=45))
    fx.op(text_op("Quarterly Business Review", 72, 730, size=10))


def build_hybrid_005(fx):
    """Scanned body (top 90%) with vector footer and divider line."""
    body_bottom = int(PAGE_H * 0.10)  # 79
    fx.scan_layer(0, body_bottom, PAGE_W, PAGE_H - body_bottom)
    fx.op(text_op("QUARTERLY REPORT", 72, 745, size=28))
    fx.op(line_op(72, 62, 540, 62))  # divider between body and footer
    fx.op(text_op("Page 1 of 12", 72, 40, size=9))
    fx.op(text_op("CONFIDENTIAL - For internal use only", 300, 40, size=9))


def build_hybrid_006(fx):
    """Scanned contract with circular APPROVED stamp, bottom right."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    fx.op(text_op("SERVICE AGREEMENT", 72, 730, size=12))
    red = (1, 0, 0)
    cx, cy, r = 500, 100, 60
    fx.op(circle_ops(cx, cy, r, color=red))
    fx.op(text_op("APPROVED", cx - 26, cy + 4, font="F2", size=10, color=red))
    fx.op(text_op("Official Seal", cx - 22, cy - 12, font="F2", size=8, color=red))


def build_hybrid_007(fx):
    """Scanned tax form with vector fillable textbox overlays."""
    # scanned layer: horizontal dividers every 60 pts, vertical dividers
    fx.scan_layer(0, 0, PAGE_W, PAGE_H,
                  dividers_h=tuple(range(60, PAGE_H, 60)),
                  dividers_v=(200, 400))
    fx.op(text_op("Form HT-1040 Income Tax Return", 72, 750, size=10))
    gray = (0.45, 0.45, 0.45)
    boxes = [
        ("First name", 90, 640, 180, 30),
        ("Last name", 320, 640, 180, 30),
        ("SSN", 90, 520, 200, 30),
        ("Home address", 90, 400, 340, 30),
        ("Total income", 90, 240, 200, 30),
        ("Tax withheld", 320, 240, 200, 30),
    ]
    for label, x, y, w, h in boxes:
        fx.op(rect_op(x, y, w, h, color=gray, line_width=1))
        fx.op(text_op(label, x + 6, y + h + 10, size=9, color=gray))


def build_hybrid_008(fx):
    """Full-page scan with vector text rotated at multiple angles."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    fx.op(text_op("Rotated Vector Layer Test", 72, 730, size=10))
    overlays = [
        ("REVIEW COPY 15", 100, 620, 15),
        ("DRAFT COPY 30", 300, 560, 30),
        ("SPECIMEN 45", 130, 420, 45),
        ("SAMPLE COPY -15", 330, 340, -15),
        ("ARCHIVE COPY -30", 160, 220, -30),
    ]
    for text, x, y, angle in overlays:
        fx.op(text_op(text, x, y, size=14, color=(0.2, 0.2, 0.7), angle=angle))


def build_hybrid_009(fx):
    """Full-page scan with ExtGState semi-transparent text (ca/CA 0.5, 0.7)."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    fx.op(text_op("Transparent Overlay Test", 72, 730, size=10))
    fx.gs("GS0", 0.5)
    fx.gs("GS1", 0.7)
    fx.op(text_op("50% TRANSPARENT OVERLAY TEXT", 90, 600, size=16, gs="GS0"))
    fx.op(text_op("70% OPAQUE OVERLAY TEXT", 200, 480, size=14, gs="GS1"))
    fx.op(text_op("50% TRANSPARENT NOTICE", 120, 300, size=12, gs="GS0"))
    fx.op(text_op("70% OPAQUE NOTICE", 260, 180, size=12, gs="GS1"))


def build_hybrid_010(fx):
    """Complex layered page: header, footer, sidebar, main column,
    annotation box, and vector shapes (3 Bezier circles, 2 rectangles,
    3 lines) with an ExtGState transparency layer."""
    fx.scan_layer(0, 0, PAGE_W, PAGE_H)
    fx.op(text_op("ACME Holdings - Consolidated Statement", 72, 745, size=12))
    fx.op(text_op("Prepared 2024 - Page 1", 72, 40, size=9))
    # sidebar column (left ~25%)
    for i, line_text in enumerate(["Item", "Assets", "Liabilities", "Equity",
                                   "Revenue", "Expenses"]):
        fx.op(text_op(line_text, 36, 640 - i * 60, size=9))
    # main column (right ~60%)
    for i, line_text in enumerate(["Consolidated figures in thousands.",
                                   "All values audited 2024-03.",
                                   "Currency: USD.",
                                   "See notes 1 through 9.",
                                   "Restatement: none."]):
        fx.op(text_op(line_text, 200, 660 - i * 60, size=9))
    # annotation box, bottom right corner
    fx.op(rect_op(380, 100, 200, 60, color=(0, 0, 0.6), line_width=1))
    fx.op(text_op("Note: preliminary figures.", 392, 138, size=9, color=(0, 0, 0.6)))
    # vector shapes: three circles = 12 curveto operators (4 per circle)
    fx.op(circle_ops(200, 400, 40))
    fx.op(circle_ops(400, 500, 30))
    fx.op(circle_ops(300, 600, 20))
    # rectangles
    fx.op(rect_op(150, 550, 80, 40))
    fx.op(rect_op(380, 450, 90, 30))
    # dividers: two horizontal (rows 1 and 7), one vertical separator
    fx.op(line_op(36, 693, 576, 693))
    fx.op(line_op(36, 99, 576, 99))
    fx.op(line_op(180, 99, 180, 693))
    # transparency layer over the shapes (PDF transparency model demo)
    fx.gs("GS0", 0.7)
    fx.op(text_op("AUDITED", 60, 200, size=18, color=(0.2, 0.2, 0.2), gs="GS0"))


# name -> (filename, builder). Order matters: it is generation order.
FIXTURES = [
    ("hybrid-001", "hybrid-001-vector-header-over-scan.pdf", build_hybrid_001),
    ("hybrid-002", "hybrid-002-vector-form-over-scan.pdf", build_hybrid_002),
    ("hybrid-003", "hybrid-003-mixed-column-layout.pdf", build_hybrid_003),
    ("hybrid-004", "hybrid-004-watermark-over-scan.pdf", build_hybrid_004),
    ("hybrid-005", "hybrid-005-vector-footer-over-scan.pdf", build_hybrid_005),
    ("hybrid-006", "hybrid-006-stamp-annotation.pdf", build_hybrid_006),
    ("hybrid-007", "hybrid-007-textbox-overlay.pdf", build_hybrid_007),
    ("hybrid-008", "hybrid-008-rotated-vector.pdf", build_hybrid_008),
    ("hybrid-009", "hybrid-009-transparent-vector.pdf", build_hybrid_009),
    ("hybrid-010", "hybrid-010-complex-layered.pdf", build_hybrid_010),
]


def resolve_names(requested):
    """Map CLI --fixture names ("001", "hybrid-001", or full stem) to keys."""
    if not requested:
        return [key for key, _, _ in FIXTURES]
    keys = []
    for request in requested:
        matches = [key for key, fname, _ in FIXTURES
                   if request in (key, key.replace("hybrid-", ""), fname.rsplit(".", 1)[0])]
        if not matches:
            raise SystemExit("unknown fixture %r (known: %s)"
                             % (request, ", ".join(key for key, _, _ in FIXTURES)))
        keys.append(matches[0])
    return keys


def main(argv=None):
    repo_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(
        description="Regenerate the hybrid-0NN hybrid fixture PDFs (stdlib only).")
    parser.add_argument("--out-dir", default=str(repo_root / "tests" / "fixtures" / "hybrid"),
                        help="output directory (default: tests/fixtures/hybrid; "
                             "use a scratch dir when verifying)")
    parser.add_argument("--fixture", action="append", default=[],
                        help="fixture to generate: 001, hybrid-001, or full stem "
                             "(repeatable; default: all ten)")
    parser.add_argument("--date", default=None, metavar="YYYYMMDDHHMMSS",
                        help="optional /CreationDate for the Info dictionary; "
                             "without it no timestamp is emitted at all")
    args = parser.parse_args(argv)
    if args.date is not None and (len(args.date) != 14 or not args.date.isdigit()):
        parser.error("--date must be YYYYMMDDHHMMSS")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    wanted = set(resolve_names(args.fixture))
    for key, filename, builder in FIXTURES:
        if key not in wanted:
            continue
        fixture = Fixture(seed=hash_seed(*[ord(c) for c in key]))
        builder(fixture)
        pdf_bytes = fixture.finish(date=args.date)
        target = out_dir / filename
        target.write_bytes(pdf_bytes)
        print("wrote %s (%d bytes)" % (target, len(pdf_bytes)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

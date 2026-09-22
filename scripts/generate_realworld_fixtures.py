#!/usr/bin/env python3
"""Generate deterministic synthetic fixtures for the real-world extraction
failure classes tracked by bead pdftract-7ec0f722 (parent pdftract-257d92c3).

Each fixture is small, ASCII-only, uses fixed object IDs, fixed timestamps and
no randomness, so the bytes are reproducible run-to-run (the SHA-256 values in
tests/fixtures/profiles/PROVENANCE.md stay valid). The generated .pdf files are
COMMITTED -- the baseline harness (crates/pdftract-core/tests/
realworld_extraction_baseline.rs) must never depend on this script at test time.

Fixture -> failure class -> owning fix bead:

  realworld/incremental-updates-offer-letter.pdf
      Classic xref tables chained by /Prev, 3 %%EOF markers, /Root still
      pointing at the base revision's catalog in the LAST trailer.
      Class: Docusign-signed offer letter (dogfood F2: 4 pp / 12,418 chars,
      failed "Failed to resolve /Root: object 1 0 R not found").
      Owning fix bead: pdftract-4196ae99 (multi-section xref resolution).

  realworld/xref-stream-only-report.pdf
      No classic xref table anywhere; a single /Type /XRef cross-reference
      stream (PDF 1.5). Class: business report (dogfood F2: 31 pp /
      39,018 chars, failed "object 456 0 R not found").
      Owning fix bead: pdftract-4196ae99.

  realworld/startxref-offset-edge.pdf
      Valid classic-xref document whose startxref VALUE is deliberately 6
      bytes short of the real xref keyword, landing inside the preceding
      object's "endobj". A strict offset->keyword read fails; a bounded
      search must recover. Class: trailer-loss / "No trailer in xref
      section" (54 corpus files, 2026-09-21 re-check).
      Owning fix bead: pdftract-0df07688 (startxref/trailer detection).

  realworld/dense-one-page-agreement.pdf
      Plain valid document, single dense page, classic xref. Control for the
      resume / legal-agreement class (dogfood F2: resume 4 pp / 5,958 chars,
      legal agreement 23 pp / 77,733 chars -- both failed "Document contains
      no pages"). Owning fix bead: pdftract-bcf935ec (page-tree resolution).

Usage: python3 scripts/generate_realworld_fixtures.py
Writes into tests/fixtures/realworld/ and prints each file's SHA-256.
"""

import hashlib
import pathlib
import struct

OUT_DIR = pathlib.Path(__file__).resolve().parent.parent / "tests" / "fixtures" / "realworld"

CREATION_DATE = "D:20260101000000Z"


def esc(text: str) -> str:
    return text.replace("\\", r"\\").replace("(", r"\(").replace(")", r"\)")


def content_stream(lines) -> bytes:
    """A simple text content stream: one Tj per line, leading via TL/T*."""
    body = ["BT", "/F1 11 Tf", "72 720 Td", "14 TL"]
    for i, line in enumerate(lines):
        if i == 0:
            body.append(f"({esc(line)}) Tj")
        else:
            body.append("T*")
            body.append(f"({esc(line)}) Tj")
    body.append("ET")
    return ("\n".join(body) + "\n").encode("ascii")


class Builder:
    """Accumulates a PDF body and records object offsets."""

    def __init__(self) -> None:
        self.buf = bytearray()
        self.offsets = {}

    def raw(self, data: bytes) -> None:
        self.buf += data

    def obj(self, num: int, body: str) -> None:
        self.offsets[num] = len(self.buf)
        self.buf += f"{num} 0 obj\n{body}\nendobj\n".encode("ascii")

    def stream_obj(self, num: int, dict_entries: str, data: bytes) -> None:
        self.offsets[num] = len(self.buf)
        head = f"{num} 0 obj\n<< {dict_entries} /Length {len(data)} >>\n".encode("ascii")
        self.buf += head + b"stream\n" + data + b"\nendstream\nendobj\n"

    def xref_and_trailer(self, max_obj: int, trailer_entries: str) -> int:
        """Classic xref table for objects 0..max_obj + trailer + startxref + %%EOF.

        Returns the byte offset of the `xref` keyword (the value a correct
        startxref would carry for this revision).
        """
        xref_off = len(self.buf)
        entries = ["xref", f"0 {max_obj + 1}"]
        entries.append(f"{0:010d} {65535:05d} f \n")
        for num in range(1, max_obj + 1):
            entries.append(f"{self.offsets[num]:010d} {0:05d} n \n")
        self.buf += ("\n".join(entries[:2]) + "\n" + "".join(entries[2:])).encode("ascii")
        self.buf += (
            f"trailer\n<< {trailer_entries} >>\nstartxref\n{xref_off}\n%%EOF\n"
        ).encode("ascii")
        return xref_off

    def update_xref_and_trailer(self, new_objs, trailer_entries: str) -> int:
        """Incremental-update xref: one subsection for this revision's objects.

        `new_objs` is an iterable of (num, prev_xref_off) -- only the added or
        replaced objects appear, and the trailer chains back via /Prev.
        Returns this revision's xref keyword offset.
        """
        xref_off = len(self.buf)
        nums = sorted(num for num, _ in new_objs)
        lo, hi = nums[0], nums[-1]
        self.buf += f"xref\n{lo} {hi - lo + 1}\n".encode("ascii")
        by_num = dict(new_objs)
        for num in range(lo, hi + 1):
            if num in by_num:
                self.buf += f"{by_num[num]:010d} {0:05d} n \n".encode("ascii")
            else:
                # Free placeholder for gaps inside the subsection range.
                self.buf += f"{0:010d} {0:05d} f \n".encode("ascii")
        self.buf += (
            f"trailer\n<< {trailer_entries} >>\nstartxref\n{xref_off}\n%%EOF\n"
        ).encode("ascii")
        return xref_off

    def bytes(self) -> bytes:
        return bytes(self.buf)


def font_obj() -> str:
    return "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"


def page_obj(pages_ref: int, contents_ref: int, res_font_ref: int) -> str:
    return (
        f"<< /Type /Page /Parent {pages_ref} 0 R /MediaBox [0 0 612 792] "
        f"/Resources << /Font << /F1 {res_font_ref} 0 R >> >> "
        f"/Contents {contents_ref} 0 R >>"
    )


def offer_letter_lines(page_idx: int) -> list:
    return [
        "OFFER OF EMPLOYMENT",
        f"Paragraph {page_idx:02d} of the offer letter.",
        "This document carries three incremental updates, as produced by",
        "electronic-signature services: each revision appends an xref",
        "section chained by /Prev and the latest trailer keeps /Root on the",
        "base revision's catalog object.",
        "The extraction pipeline must walk the full /Prev chain to resolve",
        "objects defined in earlier revisions.",
        "Base salary, start date, and reporting line appear on page 1.",
        "Benefits summary appears on page 2.",
        "Equity grant terms appear on page 3.",
        "Signature blocks appear on page 4.",
        "CONFIDENTIAL - generated synthetic fixture, not a real letter.",
    ]


def generate_incremental_updates() -> bytes:
    """Base revision (4 pages) + 2 incremental updates, 3 %%EOF markers.

    The LAST trailer's /Root stays 1 0 R -- the base revision's catalog --
    exactly like the Docusign class document from the dogfood pilot.
    """
    b = Builder()
    b.raw(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")

    b.obj(1, "<< /Type /Catalog /Pages 2 0 R >>")
    b.obj(
        2,
        "<< /Type /Pages /Count 4 /Kids [3 0 R 5 0 R 7 0 R 9 0 R] >>",
    )
    font_ref = 11
    page_nums = [3, 5, 7, 9]
    content_nums = [4, 6, 8, 10]
    for page_num, content_num, idx in zip(page_nums, content_nums, range(4)):
        b.obj(page_num, page_obj(2, content_num, font_ref))
        b.stream_obj(content_num, "", content_stream(offer_letter_lines(idx + 1)))
    b.obj(font_ref, font_obj())

    base_xref = b.xref_and_trailer(
        font_ref,
        f"/Size {font_ref + 1} /Root 1 0 R /CreationDate ({CREATION_DATE})",
    )

    # Update 1: an /Info dictionary revision (signature-service metadata).
    b.obj(
        12,
        f"<< /Producer (SyntheticFixtureGenerator) /ModDate ({CREATION_DATE}) "
        f"/Title (Offer of Employment) >>",
    )
    upd1_xref = b.update_xref_and_trailer(
        [(12, base_xref)],
        f"/Size 13 /Root 1 0 R /Info 12 0 R /Prev {base_xref}",
    )

    # Update 2: document-history metadata stream; /Root still 1 0 R.
    b.stream_obj(
        13,
        "/Type /Metadata /Subtype /XML",
        b"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>"
        b"<history><revision n='1'/><revision n='2'/></history>"
        b"<?xpacket end='w'?>",
    )
    b.update_xref_and_trailer(
        [(13, upd1_xref)],
        f"/Size 14 /Root 1 0 R /Info 12 0 R /Prev {upd1_xref}",
    )
    return b.bytes()


def generate_xref_stream_only() -> bytes:
    """PDF 1.5 document whose ONLY cross-reference is a /Type /XRef stream."""
    pages = 6
    b = Builder()
    b.raw(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")

    b.obj(1, "<< /Type /Catalog /Pages 2 0 R >>")
    kids = " ".join(f"{3 + 2 * i} 0 R" for i in range(pages))
    b.obj(2, f"<< /Type /Pages /Count {pages} /Kids [{kids}] >>")
    font_num = 3 + 2 * pages  # 15
    for i in range(pages):
        page_num = 3 + 2 * i
        content_num = page_num + 1
        b.obj(page_num, page_obj(2, content_num, font_num))
        b.stream_obj(
            content_num,
            "",
            content_stream(
                [
                    "QUARTERLY BUSINESS REPORT",
                    f"Section {i + 1} of {pages}: revenue, retention, and headcount.",
                    "This fixture has no classic xref table; the single",
                    "cross-reference section is a PDF 1.5 /Type /XRef stream.",
                    f"Synthetic page {i + 1}.",
                ]
            ),
        )
    b.obj(font_num, font_obj())

    xref_num = font_num + 1  # 16
    size = xref_num + 1  # 17

    def w_row(entry_type: int, field2: int, field3: int) -> bytes:
        # /W [1 4 2]: 1-byte type, 4-byte offset/next-free, 2-byte generation.
        return struct.pack(">BIH", entry_type, field2, field3)

    # The stream's own xref entry must carry the byte offset of its object
    # header. That offset is knowable before the data exists (the header sits
    # at the current write position), so record it, then emit the object.
    header = (
        f"{xref_num} 0 obj\n<< /Type /XRef /Size {size} /W [1 4 2] "
        f"/Index [0 {size}] /Root 1 0 R /Length {7 * size} >>\nstream\n"
    ).encode("ascii")
    b.offsets[xref_num] = len(b.buf)
    rows = b"".join(
        [w_row(0, 0, 65535)] + [w_row(1, b.offsets[n], 0) for n in range(1, xref_num + 1)]
    )
    assert len(rows) == 7 * size
    b.buf += header + rows + b"\nendstream\nendobj\n"

    startxref_off = b.offsets[xref_num]
    b.raw(f"startxref\n{startxref_off}\n%%EOF\n".encode("ascii"))
    return b.bytes()


def agreement_lines() -> list:
    return [
        "MUTUAL NON-DISCLOSURE AGREEMENT (startxref offset edge fixture)",
        "1. Purpose. The parties wish to explore a business relationship",
        "and may disclose confidential information to each other.",
        "2. Confidentiality. Each party shall protect the other party's",
        "confidential information with at least the same degree of care",
        "it uses for its own similar information, and no less than",
        "reasonable care.",
        "3. Term. This agreement runs for three years from the effective",
        "date, and the confidentiality obligation survives five years.",
        "4. Exclusions. Information that is or becomes public through no",
        "fault of the receiving party is not confidential.",
        "5. Remedies. The parties agree that breach may cause irreparable",
        "harm for which damages are an inadequate remedy.",
        "6. Governing law. This agreement is governed by the laws of the",
        "State of Delaware without regard to conflicts-of-law rules.",
        "7. Entire agreement. This document is the entire agreement of the",
        "parties concerning its subject matter.",
        "Signed by the authorized representatives of each party.",
    ]


def generate_startxref_offset_edge() -> bytes:
    """Valid classic-xref doc; the startxref VALUE is 6 bytes short.

    The recorded offset lands inside the final object's `endobj` keyword, so
    a strict read at that offset sees neither `xref` nor leading whitespace.
    Nothing in the file recovers it except a bounded search FORWARD from the
    recorded offset. Mirrors the 54-file corpus class failing with
    "No trailer in xref section" (2026-09-21 re-check).
    """
    b = Builder()
    b.raw(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")

    b.obj(1, "<< /Type /Catalog /Pages 2 0 R >>")
    b.obj(2, "<< /Type /Pages /Count 2 /Kids [3 0 R 5 0 R] >>")
    for page_num, content_num, idx in ((3, 4, 0), (5, 6, 1)):
        b.obj(page_num, page_obj(2, content_num, 7))
        b.stream_obj(content_num, "", content_stream(agreement_lines()))
    b.obj(7, font_obj())

    true_xref_off = b.xref_and_trailer(
        7, f"/Size 8 /Root 1 0 R /CreationDate ({CREATION_DATE})"
    )

    # Rewind the last revision's startxref by 6 bytes. The bytes at that
    # offset are "ndobj\nxref..." (the tail of obj 7's endobj), so the offset
    # points 2 bytes into "endobj" and 6 bytes before the real keyword.
    # NOTE: the trailer's startxref line was already written with the true
    # offset; rewrite exactly that occurrence (the only one in the file).
    data = b.bytes()
    marker = f"startxref\n{true_xref_off}\n%%EOF\n".encode("ascii")
    assert data.count(marker) == 1
    bogus_off = true_xref_off - 6
    assert data[bogus_off:bogus_off + 6] == b"ndobj\n", data[bogus_off:bogus_off + 10]
    data = data.replace(marker, f"startxref\n{bogus_off}\n%%EOF\n".encode("ascii"))
    return data


def generate_dense_one_page() -> bytes:
    """Plain valid single-page document with a dense page tree (control)."""
    b = Builder()
    b.raw(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")

    b.obj(1, "<< /Type /Catalog /Pages 2 0 R >>")
    b.obj(2, "<< /Type /Pages /Count 1 /Kids [3 0 R] >>")
    b.obj(3, page_obj(2, 4, 5))
    dense = agreement_lines() * 2  # dense single page
    b.stream_obj(4, "", content_stream(dense))
    b.obj(5, font_obj())

    b.xref_and_trailer(5, f"/Size 6 /Root 1 0 R /CreationDate ({CREATION_DATE})")
    return b.bytes()


GENERATORS = {
    "incremental-updates-offer-letter.pdf": generate_incremental_updates,
    "xref-stream-only-report.pdf": generate_xref_stream_only,
    "startxref-offset-edge.pdf": generate_startxref_offset_edge,
    "dense-one-page-agreement.pdf": generate_dense_one_page,
}


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, gen in GENERATORS.items():
        data = gen()
        assert b"%%EOF" in data, name
        path = OUT_DIR / name
        path.write_bytes(data)
        sha = hashlib.sha256(data).hexdigest()
        print(f"{path}: {len(data)} bytes  sha256={sha}")


if __name__ == "__main__":
    main()

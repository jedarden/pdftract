"""Generate the small PDFs used by auxiliary_surfaces.rs.

The fixtures deliberately use ordinary PDF names (``/Subtype`` becomes the
parser key ``Subtype``) and a traditional xref table so they exercise the
same paths as user-produced PDFs.
"""

from pathlib import Path


ROOT = Path(__file__).parent


def stream(data: bytes, dictionary: str = "") -> bytes:
    extra = f" {dictionary}" if dictionary else ""
    return (
        f"<< /Length {len(data)}{extra} >>\nstream\n".encode()
        + data
        + b"\nendstream"
    )


def pdf(objects: dict[int, bytes], root: int = 1) -> bytes:
    output = bytearray(b"%PDF-1.4\n")
    offsets = {0: 0}
    for number in sorted(objects):
        offsets[number] = len(output)
        output.extend(f"{number} 0 obj\n".encode())
        output.extend(objects[number])
        output.extend(b"\nendobj\n")

    xref_offset = len(output)
    size = max(objects) + 1
    output.extend(f"xref\n0 {size}\n".encode())
    output.extend(b"0000000000 65535 f\n")
    for number in range(1, size):
        if number in offsets:
            output.extend(f"{offsets[number]:010d} 00000 n\n".encode())
        else:
            output.extend(b"0000000000 65535 f\n")
    output.extend(
        f"trailer\n<< /Size {size} /Root {root} 0 R >>\n"
        f"startxref\n{xref_offset}\n%%EOF\n".encode()
    )
    return bytes(output)


def base_page(page_count: int = 1, *, annotations: str = "") -> dict[int, bytes]:
    kids = " ".join(f"{number} 0 R" for number in range(3, 3 + page_count))
    font_ref = 20
    content_start = 3 + page_count
    objects: dict[int, bytes] = {
        2: f"<< /Type /Pages /Kids [ {kids} ] /Count {page_count} >>".encode(),
        font_ref: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    }
    for index in range(page_count):
        page = 3 + index
        content = content_start + index
        page_annotations = f" /Annots [ {annotations} ]" if annotations else ""
        objects[page] = (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] "
            f"/Contents {content} 0 R /Resources << /Font << /F1 {font_ref} 0 R >> >>"
            f"{page_annotations} >>"
        ).encode()
        objects[content] = stream(
            f"BT /F1 12 Tf 72 {720 - index * 24} Td (Auxiliary fixture) Tj ET\n".encode()
        )
    return objects


def write(name: str, objects: dict[int, bytes]) -> None:
    (ROOT / name).write_bytes(pdf(objects))


def make_plain() -> None:
    objects = base_page()
    objects[1] = b"<< /Type /Catalog /Pages 2 0 R >>"
    write("plain.pdf", objects)


def make_form_and_javascript() -> None:
    objects = base_page()
    objects[1] = (
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R "
        b"/OpenAction << /S /JavaScript /JS (app.alert('fixture')) >> >>"
    )
    objects[6] = b"<< /Fields [ 7 0 R ] >>"
    objects[7] = b"<< /FT /Tx /T (customer_name) /V (Ada Lovelace) >>"
    write("form-and-javascript.pdf", objects)


def make_signature() -> None:
    objects = base_page()
    objects[1] = b"<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>"
    objects[6] = b"<< /Fields [ 7 0 R ] >>"
    objects[7] = b"<< /FT /Sig /T (approval_signature) /V 8 0 R >>"
    objects[8] = (
        b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached "
        b"/Name (Ada Lovelace) /M (D:20260102112233Z) /Reason (Approved) "
        b"/Location (New York) /ByteRange [ 0 100 200 300 ] >>"
    )
    write("signature.pdf", objects)


def make_links() -> None:
    objects = base_page(annotations="6 0 R 7 0 R")
    objects[1] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[6] = (
        b"<< /Type /Annot /Subtype /Link /Rect [ 72 700 220 720 ] "
        b"/A << /S /URI /URI (https://example.com/docs) >> >>"
    )
    objects[7] = (
        b"<< /Type /Annot /Subtype /Link /Rect [ 72 650 220 670 ] "
        b"/Dest (chapter-one) >>"
    )
    write("links.pdf", objects)


def make_attachment() -> None:
    objects = base_page()
    objects[1] = b"<< /Type /Catalog /Pages 2 0 R /Names 6 0 R >>"
    objects[6] = b"<< /EmbeddedFiles 7 0 R >>"
    objects[7] = b"<< /Names [ (notes.txt) 8 0 R ] >>"
    objects[8] = (
        b"<< /Type /Filespec /F (notes.txt) /UF (notes.txt) "
        b"/Desc (Fixture notes) /EF << /F 9 0 R >> >>"
    )
    objects[9] = stream(
        b"Hello attachment!",
        "/Subtype /text#2Fplain /Params << /Size 17 >>",
    )
    write("attachment.pdf", objects)


def make_article_thread() -> None:
    objects = base_page(2)
    objects[1] = b"<< /Type /Catalog /Pages 2 0 R /Threads 10 0 R >>"
    objects[10] = b"[ 13 0 R ]"
    objects[13] = (
        b"<< /F 11 0 R /I << /Title (Fixture thread) /Author (Ada) "
        b"/Subject (Reading order) /Keywords (pdf,thread) >> >>"
    )
    objects[11] = (
        b"<< /Type /Bead /T 13 0 R /N 12 0 R /R 3 0 R "
        b"/V [ 50 600 250 720 ] >>"
    )
    objects[12] = (
        b"<< /Type /Bead /T 13 0 R /N 11 0 R /R 4 0 R "
        b"/V [ 300 100 550 220 ] >>"
    )
    write("article-thread.pdf", objects)


if __name__ == "__main__":
    ROOT.mkdir(parents=True, exist_ok=True)
    make_plain()
    make_form_and_javascript()
    make_signature()
    make_links()
    make_attachment()
    make_article_thread()

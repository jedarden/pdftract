from pdftract.types import Diagnostic, DiagnosticLocation, Document, Metadata


def test_structured_diagnostic_preserves_canonical_fields_and_legacy_messages():
    raw = {
        "code": "STREAM_DECODE_ERROR",
        "message": "zlib stream truncated mid-inflation",
        "severity": "warning",
        "page_index": 3,
        "location": {"object_number": 42, "generation_number": 7},
        "hint": "Inspect the source PDF for corrupt stream data",
    }

    diagnostic = Diagnostic.from_native(raw)
    metadata = Metadata.from_native(
        {
            "page_count": 1,
            "diagnostics": [raw["message"]],
            "diagnostics_detailed": [raw],
        }
    )
    document = Document.from_native(
        {"pages": [], "metadata": {"page_count": 1}, "errors": [raw]}
    )

    assert diagnostic == Diagnostic(
        code=raw["code"],
        message=raw["message"],
        severity=raw["severity"],
        page_index=3,
        location=DiagnosticLocation(object_number=42, generation_number=7),
        hint=raw["hint"],
    )
    assert metadata.diagnostics == [raw["message"]]
    assert metadata.diagnostics_detailed == [diagnostic]
    assert document.errors == [diagnostic]

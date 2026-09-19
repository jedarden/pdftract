"""Regression tests for the native/Python exception hierarchy boundary."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

# Add the Python package to the path when this file is run directly from the
# repository rather than through an installed wheel.
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import pdftract


EXCEPTION_NAMES = (
    "PdftractError",
    "CorruptPdfError",
    "EncryptionError",
    "SourceUnreachableError",
    "RemoteFetchInterruptedError",
    "TlsError",
    "ReceiptVerifyError",
    "UnsupportedOperationError",
)


def _require_native() -> None:
    if not pdftract._native_available:
        pytest.skip("native extension is not available")


def test_native_reexports_the_python_exception_classes_by_identity() -> None:
    """The package and native module must expose one class per exception."""
    _require_native()

    native = pdftract._native
    for name in EXCEPTION_NAMES:
        public_type = getattr(pdftract, name)
        native_type = getattr(native, name)
        assert native_type is public_type, f"{name} was registered twice"
        assert issubclass(public_type, pdftract.PdftractError)


@pytest.mark.parametrize("name", EXCEPTION_NAMES)
def test_each_native_exception_type_is_catchable_as_its_public_type(name: str) -> None:
    """An exception constructed through the native export matches its API name."""
    _require_native()

    public_type = getattr(pdftract, name)
    native_type = getattr(pdftract._native, name)
    with pytest.raises(public_type) as caught:
        raise native_type("native exception regression probe")
    assert type(caught.value) is public_type


def test_native_extraction_failure_is_catchable_by_public_base(tmp_path: Path) -> None:
    """A failure raised by the native extractor reaches pdftract.PdftractError."""
    _require_native()

    invalid_pdf = tmp_path / "invalid.pdf"
    invalid_pdf.write_bytes(b"not a PDF")

    with pytest.raises(pdftract.PdftractError) as caught:
        pdftract.extract(str(invalid_pdf))

    assert type(caught.value) in {
        getattr(pdftract, name) for name in EXCEPTION_NAMES
    }

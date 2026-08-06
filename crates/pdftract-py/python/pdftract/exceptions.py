"""Exception hierarchy for pdftract.

All pdftract exceptions inherit from PdftractError.
"""

from __future__ import annotations


class PdftractError(Exception):
    """Base exception for all pdftract errors.

    This is raised when extraction fails for reasons not covered
    by more specific exception types.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class CorruptPdfError(PdftractError):
    """Raised when the PDF file is corrupted or malformed.

    This indicates the PDF structure is invalid or the file
    is not a valid PDF document.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class EncryptionError(PdftractError):
    """Raised when a PDF is encrypted and no password was provided,
    or the provided password is incorrect.

    Supply the correct password via the `password` option:
        pdftract.extract("encrypted.pdf", password="secret")
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class SourceUnreachableError(PdftractError):
    """Raised when the PDF source (file or URL) cannot be accessed.

    For files: check the path and file permissions.
    For URLs: check network connectivity and URL validity.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class RemoteFetchInterruptedError(PdftractError):
    """Raised when a remote fetch is interrupted.

    This can happen due to network timeouts, connection drops,
    or server issues during URL fetching.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class TlsError(PdftractError):
    """Raised when TLS/SSL certificate validation fails.

    This indicates a problem with the HTTPS connection,
    such as an invalid certificate or TLS protocol mismatch.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class ReceiptVerifyError(PdftractError):
    """Raised when receipt verification fails.

    This can happen when:
    - The PDF fingerprint doesn't match
    - No span has sufficient bbox overlap
    - The content hash doesn't match
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message


class UnsupportedOperationError(PdftractError):
    """Raised when calling a method not supported by the binary version.

    This can happen when using features added in newer binary versions
    with an older binary.
    """

    def __init__(self, message: str | None = None):
        """Initialize the exception.

        Args:
            message: Optional error message
        """
        super().__init__(message)
        self.message = message

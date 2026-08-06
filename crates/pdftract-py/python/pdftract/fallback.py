"""Subprocess fallback for when the native module is unavailable.

This module provides a subprocess-based implementation that calls
the pdftract CLI binary. It is used automatically when the native
PyO3 module fails to import.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterator, List, Optional

from pdftract.exceptions import (
    CorruptPdfError,
    EncryptionError,
    PdftractError,
    ReceiptVerifyError,
    RemoteFetchInterruptedError,
    SourceUnreachableError,
    TlsError,
    UnsupportedOperationError,
)
from pdftract.types import (
    Block,
    Document,
    Fingerprint,
    Match,
    Metadata,
    Page,
    Span,
    Table,
)


class SubprocessExtractor:
    """Subprocess-based extractor using the pdftract CLI binary."""

    def __init__(self, cli_path: Optional[str] = None):
        """Initialize the subprocess extractor.

        Args:
            cli_path: Path to the pdftract binary. If None, searches PATH.
        """
        if cli_path is None:
            cli_path = self._find_cli()
        self.cli_path = cli_path

    def _find_cli(self) -> str:
        """Find the pdftract binary in PATH."""
        # Try to find pdftract in PATH
        for name in ["pdftract", "pdftract.exe"]:
            try:
                result = subprocess.run(
                    ["which", name],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if result.returncode == 0 and result.stdout.strip():
                    return result.stdout.strip()
            except FileNotFoundError:
                pass

        # Try common installation paths
        for path in [
            "/usr/local/bin/pdftract",
            "/usr/bin/pdftract",
            os.path.expanduser("~/.local/bin/pdftract"),
            os.path.join(sys.prefix, "bin", "pdftract"),
        ]:
            if os.path.exists(path):
                return path

        raise PdftractError(
            "pdftract CLI binary not found. Install pdftract from "
            "https://github.com/jedarden/pdftract or set PDFTRACT_CLI_PATH."
        )

    def _run(
        self,
        args: List[str],
        capture: bool = True,
        input_data: Optional[str] = None,
    ) -> subprocess.CompletedProcess[str]:
        """Run the pdftract CLI.

        Args:
            args: Command-line arguments
            capture: Whether to capture stdout/stderr
            input_data: Optional stdin data

        Returns:
            Completed process result

        Raises:
            PdftractError: If the binary fails to run
        """
        cmd = [self.cli_path] + args

        try:
            result = subprocess.run(
                cmd,
                capture_output=capture,
                text=True,
                check=False,
                input=input_data,
            )
        except FileNotFoundError:
            raise PdftractError(f"pdftract binary not found: {self.cli_path}")
        except Exception as e:
            raise PdftractError(f"Failed to run pdftract: {e}")

        return result

    def _map_exit_code_to_exception(self, exit_code: int, stderr: str) -> PdftractError:
        """Map pdftract exit codes to Python exceptions."""
        # Exit codes from plan line 3529-3536
        # 2: Corrupt PDF
        # 3: Encrypted, password missing or wrong
        # 4: Source unreadable
        # 5: Network interrupted
        # 6: TLS or certificate failure
        # 10: Receipt verification failed
        # any other non-zero: Internal error
        if exit_code == 2:
            return CorruptPdfError(stderr or "PDF file is corrupted")
        elif exit_code == 3:
            return EncryptionError(stderr or "PDF is encrypted and password is missing or wrong")
        elif exit_code == 4:
            return SourceUnreachableError(stderr or "Source (file or URL) is unreachable")
        elif exit_code == 5:
            return RemoteFetchInterruptedError(stderr or "Network interrupted")
        elif exit_code == 6:
            return TlsError(stderr or "TLS or certificate failure")
        elif exit_code == 10:
            return ReceiptVerifyError(stderr or "Receipt verification failed")
        else:
            return PdftractError(stderr or f"pdftract failed with exit code {exit_code}")

    def extract(self, source: str, **options) -> Document:
        """Extract a PDF document.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Document: Extracted document

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("extract", source, options)
        args.append("--json")  # Always request JSON output

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        try:
            data = json.loads(result.stdout)
            return Document.from_dict(data)
        except json.JSONDecodeError as e:
            raise PdftractError(f"Failed to parse JSON output: {e}")

    def extract_text(self, source: str, **options) -> str:
        """Extract plain text from a PDF.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            str: Extracted text

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("extract", source, options)
        args.append("--text")

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        return result.stdout

    def extract_markdown(self, source: str, **options) -> str:
        """Extract Markdown from a PDF.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            str: Extracted Markdown

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("extract", source, options)
        args.append("--md")

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        return result.stdout

    def extract_stream(self, source: str, **options) -> Iterator[Page]:
        """Extract pages from a PDF as a streaming iterator.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Iterator[Page]: Iterator yielding pages

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("extract", source, options)
        args.append("--ndjson")  # Use NDJSON for streaming

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        for line in result.stdout.splitlines():
            if not line.strip():
                continue
            try:
                data = json.loads(line)
                yield Page.from_dict(data)
            except json.JSONDecodeError as e:
                raise PdftractError(f"Failed to parse NDJSON line: {e}")

    def search(self, source: str, pattern: str, **options) -> Iterator[Match]:
        """Search for a pattern in a PDF.

        Args:
            source: Path to PDF file or URL
            pattern: Regex pattern to search for
            **options: Extraction options

        Returns:
            Iterator[Match]: Iterator yielding matches

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("grep", source, options)
        args.extend(["--pattern", pattern, "--json"])

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        data = json.loads(result.stdout)
        for match_data in data.get("matches", []):
            yield Match(
                text=match_data["text"],
                page_index=match_data["page_index"],
                span_index=match_data["span_index"],
                bbox=match_data["bbox"],
                match_start=match_data.get("match_start", 0),
                match_end=match_data.get("match_end", len(match_data["text"])),
            )

    def get_metadata(self, source: str, **options) -> Metadata:
        """Get metadata from a PDF.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Metadata: Document metadata

        Raises:
            PdftractError: If extraction fails
        """
        args = self._build_args("extract", source, options)
        args.append("--metadata-only")

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        try:
            data = json.loads(result.stdout)
            return Metadata(
                page_count=data.get("page_count", 0),
                title=data.get("title"),
                author=data.get("author"),
                subject=data.get("subject"),
                keywords=data.get("keywords"),
                creator=data.get("creator"),
                producer=data.get("producer"),
                creation_date=data.get("creation_date"),
                mod_date=data.get("mod_date"),
                fingerprint=data.get("fingerprint"),
                outline=data.get("outline"),
            )
        except json.JSONDecodeError as e:
            raise PdftractError(f"Failed to parse JSON output: {e}")

    def hash(self, source: str, **options) -> Fingerprint:
        """Compute fingerprint of a PDF.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Fingerprint: Document fingerprint

        Raises:
            PdftractError: If extraction fails
        """
        args = [self.cli_path, "hash", source]

        # Add password option if provided
        if password := options.get("password"):
            args.extend(["--password", password])

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        value = result.stdout.strip()
        return Fingerprint.from_string(value)

    def classify(self, source: str) -> Any:
        """Classify a PDF page type.

        Args:
            source: Path to PDF file or URL

        Returns:
            Classification result

        Raises:
            PdftractError: If extraction fails
        """
        args = [self.cli_path, "classify", source, "--json"]

        result = self._run(args)

        if result.returncode != 0:
            raise self._map_exit_code_to_exception(result.returncode, result.stderr)

        try:
            data = json.loads(result.stdout)
            # Return a simple dict with class info
            return {
                "class_name": data.get("class", "Unknown"),
                "confidence": data.get("confidence", 0.0),
                "hybrid_cells": data.get("hybrid_cells"),
            }
        except json.JSONDecodeError as e:
            raise PdftractError(f"Failed to parse JSON output: {e}")

    def verify_receipt(self, path: str, receipt: dict) -> bool:
        """Verify a receipt against a PDF.

        Args:
            path: Path to PDF file
            receipt: Receipt dict

        Returns:
            bool: True if receipt verifies

        Raises:
            ReceiptVerifyError: If verification fails
            PdftractError: Other errors
        """
        import tempfile

        # Write receipt to a temp file
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(receipt, f)
            receipt_path = f.name

        try:
            args = [self.cli_path, "verify-receipt", path, receipt_path]
            result = self._run(args)

            if result.returncode == 0:
                return True
            elif result.returncode == 10:
                raise ReceiptVerifyError("Receipt verification failed: fingerprint mismatch")
            elif result.returncode == 11:
                raise ReceiptVerifyError("Receipt verification failed: bbox mismatch")
            elif result.returncode == 12:
                raise ReceiptVerifyError("Receipt verification failed: content hash mismatch")
            else:
                raise self._map_exit_code_to_exception(result.returncode, result.stderr)
        finally:
            os.unlink(receipt_path)

    def _build_args(self, command: str, source: str, options: dict) -> List[str]:
        """Build CLI argument list from options.

        Args:
            command: Subcommand name
            source: PDF path or URL
            options: Python-style options (snake_case)

        Returns:
            List of CLI arguments
        """
        args = [self.cli_path, command, source]

        # Map Python options to CLI flags
        option_map = {
            "ocr": "--ocr",
            "ocr_language": "--ocr-language",
            "include_invisible": "--include-invisible",
            "extract_forms": "--extract-forms",
            "extract_attachments": "--extract-attachments",
            "readability_threshold": "--readability-threshold",
            "password": "--password",
            "max_decompress_gb": "--max-decompress-gb",
            "full_render": "--full-render",
            "anchors": "--anchors",
        }

        for key, value in options.items():
            if key not in option_map:
                continue

            flag = option_map[key]

            # Boolean flags
            if isinstance(value, bool):
                if value:
                    args.append(flag)
            # List flags (repeatable)
            elif isinstance(value, list):
                for item in value:
                    args.extend([flag, str(item)])
            # String/number flags
            elif value is not None:
                args.extend([flag, str(value)])

        return args

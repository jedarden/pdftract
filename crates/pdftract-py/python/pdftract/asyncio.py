"""Asyncio wrappers for pdftract.

This module provides async versions of the long-running pdftract methods
using asyncio.to_thread to offload work to a thread pool.
"""

from __future__ import annotations

import asyncio
from typing import Any, Iterator, Optional

from pdftract.types import Document, Fingerprint, Match, Metadata, Page


class AsyncExtractor:
    """Async wrapper for pdftract extraction methods.

    This class provides async versions of the long-running extraction
    methods that block on I/O or CPU-intensive work.
    """

    def __init__(self):
        """Initialize the async extractor."""
        import pdftract

        self._pdftract = pdftract

    async def extract(self, source: str, **options) -> Document:
        """Async version of pdftract.extract.

        Offloads extraction to a thread pool to avoid blocking the event loop.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Document: Extracted document
        """
        return await asyncio.to_thread(self._pdftract.extract, source, **options)

    async def extract_text(self, source: str, **options) -> str:
        """Async version of pdftract.extract_text.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            str: Extracted text
        """
        return await asyncio.to_thread(self._pdftract.extract_text, source, **options)

    async def extract_markdown(self, source: str, **options) -> str:
        """Async version of pdftract.extract_markdown.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            str: Extracted Markdown
        """
        return await asyncio.to_thread(
            self._pdftract.extract_markdown, source, **options
        )

    async def extract_stream(self, source: str, **options) -> AsyncPageIterator:
        """Async version of pdftract.extract_stream.

        Returns an async iterator that yields pages.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            AsyncPageIterator: Async iterator yielding pages
        """
        sync_iterator = self._pdftract.extract_stream(source, **options)
        return AsyncPageIterator(sync_iterator)

    async def search(self, source: str, pattern: str, **options) -> AsyncMatchIterator:
        """Async version of pdftract.search.

        Returns an async iterator that yields matches.

        Args:
            source: Path to PDF file or URL
            pattern: Regex pattern to search for
            **options: Extraction options

        Returns:
            AsyncMatchIterator: Async iterator yielding matches
        """
        sync_iterator = self._pdftract.search(source, pattern, **options)
        return AsyncMatchIterator(sync_iterator)

    async def get_metadata(self, source: str, **options) -> Metadata:
        """Async version of pdftract.get_metadata.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Metadata: Document metadata
        """
        return await asyncio.to_thread(self._pdftract.get_metadata, source, **options)

    async def hash(self, source: str, **options) -> Fingerprint:
        """Async version of pdftract.hash.

        Args:
            source: Path to PDF file or URL
            **options: Extraction options

        Returns:
            Fingerprint: Document fingerprint
        """
        return await asyncio.to_thread(self._pdftract.hash, source, **options)

    async def classify(self, source: str) -> Any:
        """Async version of pdftract.classify.

        Args:
            source: Path to PDF file or URL

        Returns:
            Classification result
        """
        return await asyncio.to_thread(self._pdftract.classify, source)

    async def verify_receipt(self, path: str, receipt: dict) -> bool:
        """Async version of pdftract.verify_receipt.

        Args:
            path: Path to PDF file
            receipt: Receipt dict

        Returns:
            bool: True if receipt verifies
        """
        return await asyncio.to_thread(self._pdftract.verify_receipt, path, receipt)


class AsyncPageIterator:
    """Async iterator wrapper for sync page iterators."""

    def __init__(self, sync_iterator: Iterator[Page]):
        """Initialize the async iterator.

        Args:
            sync_iterator: Synchronous page iterator
        """
        self._sync_iterator = sync_iterator

    def __aiter__(self) -> "AsyncPageIterator":
        """Return self as async iterator."""
        return self

    async def __anext__(self) -> Page:
        """Get the next page asynchronously."""
        try:
            return await asyncio.to_thread(next, self._sync_iterator)
        except StopIteration:
            raise StopAsyncIteration


class AsyncMatchIterator:
    """Async iterator wrapper for sync match iterators."""

    def __init__(self, sync_iterator: Iterator[Match]):
        """Initialize the async iterator.

        Args:
            sync_iterator: Synchronous match iterator
        """
        self._sync_iterator = sync_iterator

    def __aiter__(self) -> "AsyncMatchIterator":
        """Return self as async iterator."""
        return self

    async def __anext__(self) -> Match:
        """Get the next match asynchronously."""
        try:
            return await asyncio.to_thread(next, self._sync_iterator)
        except StopIteration:
            raise StopAsyncIteration


# Module-level async extractor instance
_extractor: Optional[AsyncExtractor] = None


def _get_async_extractor() -> AsyncExtractor:
    """Get or create the module-level async extractor."""
    global _extractor
    if _extractor is None:
        _extractor = AsyncExtractor()
    return _extractor


# Export async functions
async def extract(source: str, **options) -> Document:
    """Async version of pdftract.extract."""
    return await _get_async_extractor().extract(source, **options)


async def extract_text(source: str, **options) -> str:
    """Async version of pdftract.extract_text."""
    return await _get_async_extractor().extract_text(source, **options)


async def extract_markdown(source: str, **options) -> str:
    """Async version of pdftract.extract_markdown."""
    return await _get_async_extractor().extract_markdown(source, **options)


async def extract_stream(source: str, **options) -> AsyncPageIterator:
    """Async version of pdftract.extract_stream."""
    return await _get_async_extractor().extract_stream(source, **options)


async def search(source: str, pattern: str, **options) -> AsyncMatchIterator:
    """Async version of pdftract.search."""
    return await _get_async_extractor().search(source, pattern, **options)


async def get_metadata(source: str, **options) -> Metadata:
    """Async version of pdftract.get_metadata."""
    return await _get_async_extractor().get_metadata(source, **options)


async def hash(source: str, **options) -> Fingerprint:
    """Async version of pdftract.hash."""
    return await _get_async_extractor().hash(source, **options)


async def classify(source: str) -> Any:
    """Async version of pdftract.classify."""
    return await _get_async_extractor().classify(source)


async def verify_receipt(path: str, receipt: dict) -> bool:
    """Async version of pdftract.verify_receipt."""
    return await _get_async_extractor().verify_receipt(path, receipt)


__all__ = [
    "AsyncExtractor",
    "AsyncPageIterator",
    "AsyncMatchIterator",
    "extract",
    "extract_text",
    "extract_markdown",
    "extract_stream",
    "search",
    "get_metadata",
    "hash",
    "classify",
    "verify_receipt",
]

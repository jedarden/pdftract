//! Memory-backed PDF source for testing.
//!
//! This module provides `MemorySource`, a simple in-memory `PdfSource`
//! implementation used primarily in tests. It wraps a `Vec<u8>` and
//! provides zero-copy access via `Bytes`.

use crate::source::PdfSource;
use bytes::Bytes;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

/// A memory-backed PDF source.
///
/// This is primarily used in tests where a PDF document is provided
/// as a byte array or `Vec<u8>`. It provides cheap cloning and
/// zero-copy reads via `Bytes`.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::MemorySource;
///
/// let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";
/// let source = MemorySource::new(pdf_data.to_vec());
///
/// assert_eq!(source.len(), 48);
/// let data = source.read_range(0, 10).unwrap();
/// assert_eq!(&data[..], b"%PDF-1.4\n");
/// ```
pub struct MemorySource {
    data: Bytes,
    cursor: Cursor<u64>,
}

impl MemorySource {
    /// Create a new memory-backed source from a `Vec<u8>`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::MemorySource;
    ///
    /// let data = vec![0, 1, 2, 3, 4];
    /// let source = MemorySource::new(data);
    /// ```
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
            cursor: Cursor::new(0),
        }
    }

    /// Create a new memory-backed source from a byte slice.
    ///
    /// This copies the slice into a new `Vec<u8>`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::MemorySource;
    ///
    /// let data: &[u8] = b"test data";
    /// let source = MemorySource::from_slice(data);
    /// ```
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl PdfSource for MemorySource {
    fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes> {
        let start = offset as usize;
        let end = start
            .checked_add(length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "overflow"))?;

        if start > self.data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "offset exceeds length",
            ));
        }

        let end = end.min(self.data.len());

        // Zero-copy slice into Bytes
        Ok(self.data.slice(start..end))
    }
}

impl Read for MemorySource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = self.cursor.position() as usize;
        if pos >= self.data.len() {
            return Ok(0);
        }

        let remaining = self.data.len() - pos;
        let to_read = buf.len().min(remaining);
        buf[..to_read].copy_from_slice(&self.data[pos..pos + to_read]);

        self.cursor.set_position((pos + to_read) as u64);
        Ok(to_read)
    }
}

impl Seek for MemorySource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => self.data.len() as i64 + n,
            SeekFrom::Current(n) => self.cursor.position() as i64 + n,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }

        self.cursor.set_position(new_pos as u64);
        Ok(new_pos as u64)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self.cursor.position())
    }
}

// SAFETY: Bytes is Send + Sync, Cursor<u64> is Send + Sync
unsafe impl Send for MemorySource {}
unsafe impl Sync for MemorySource {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let data = vec![0, 1, 2, 3, 4];
        let source = MemorySource::new(data);
        assert_eq!(source.len(), 5);
    }

    #[test]
    fn test_from_slice() {
        let data: &[u8] = b"test";
        let source = MemorySource::from_slice(data);
        assert_eq!(source.len(), 4);
    }

    #[test]
    fn test_read_range() {
        let data = b"Hello, World!".to_vec();
        let source = MemorySource::new(data);

        let bytes = source.read_range(0, 5).unwrap();
        assert_eq!(&bytes[..], b"Hello");

        let bytes = source.read_range(7, 5).unwrap();
        assert_eq!(&bytes[..], b"World");
    }

    #[test]
    fn test_read_range_past_end() {
        let data = b"Hello".to_vec();
        let source = MemorySource::new(data);

        // Read past end should truncate
        let bytes = source.read_range(3, 10).unwrap();
        assert_eq!(&bytes[..], b"lo");
    }

    #[test]
    fn test_read_range_offset_past_end() {
        let data = b"Hello".to_vec();
        let source = MemorySource::new(data);

        let result = source.read_range(100, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_trait() {
        let data = b"Hello, World!".to_vec();
        let mut source = MemorySource::new(data);

        let mut buf = [0u8; 5];
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello");

        let mut buf = [0u8; 2];
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b", ");
    }

    #[test]
    fn test_seek_trait() {
        let data = b"0123456789".to_vec();
        let mut source = MemorySource::new(data);

        source.seek(SeekFrom::Start(5)).unwrap();
        let mut buf = [0u8; 2];
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"56");
    }

    #[test]
    fn test_seek_from_end() {
        let data = b"Hello".to_vec();
        let mut source = MemorySource::new(data);

        source.seek(SeekFrom::End(-2)).unwrap();
        let mut buf = [0u8; 2];
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"lo");
    }

    #[test]
    fn test_empty() {
        let source = MemorySource::new(vec![]);
        assert_eq!(source.len(), 0);

        let data = source.read_range(0, 10).unwrap();
        assert_eq!(data.len(), 0);
    }
}

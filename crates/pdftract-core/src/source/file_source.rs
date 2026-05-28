//! File-backed PDF source implementation.
//!
//! FileSource provides Read+Seek access to PDF files using standard file I/O.
//! This is a fallback for when memory-mapping is not available or desired.

use crate::source::PdfSource;
use bytes::Bytes;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A file-backed PDF source using standard I/O.
///
/// This implementation uses `std::fs::File` with Read+Seek to access PDF data.
/// It's less efficient than MmapSource for random access but works on all
/// platforms and filesystems.
///
/// # Advantages
///
/// - Works on all platforms and filesystems (including network mounts, FUSE)
/// - No mmap limitations (address space, kernel restrictions)
/// - Simpler error handling (no unsafe mmap calls)
///
/// # Disadvantages
///
/// - Higher overhead for random access (each `read_range` is a separate seek+read)
/// - No zero-copy reads (data is copied into a new buffer)
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::FileSource;
/// use std::io::{Read, Seek, SeekFrom};
///
/// let mut source = FileSource::open("document.pdf")?;
///
/// // Read using Read+Seek
/// source.seek(SeekFrom::Start(1000))?;
/// let mut buffer = vec![0u8; 4096];
/// source.read_exact(&mut buffer)?;
///
/// // Or using read_range
/// let data = source.read_range(1000, 4096)?;
/// ```
pub struct FileSource {
    /// The underlying file
    file: File,
    /// Cached file length
    len: u64,
}

impl FileSource {
    /// Open a PDF file using standard I/O.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be found or opened
    /// - Permission is denied
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(&path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    /// Create a FileSource from an already-opened File.
    ///
    /// This is useful when you need to perform additional operations on the
    /// file before passing it to the parser.
    pub fn from_file(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl PdfSource for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes> {
        // Bounds check
        if offset > self.len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("offset {} exceeds source length {}", offset, self.len),
            ));
        }

        let max_read = (self.len - offset).min(length as u64) as usize;

        if max_read == 0 {
            return Ok(Bytes::new());
        }

        // Allocate buffer and read data
        let mut buffer = vec![0u8; max_read];

        // Seek and read (clone file handle to avoid mutating &self)
        let mut file = self.file.try_clone()?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buffer)?;

        Ok(Bytes::from(buffer))
    }
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_range() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello World!").unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();
        assert_eq!(source.len(), 12);

        let data = source.read_range(0, 5).unwrap();
        assert_eq!(&data[..], b"Hello");

        let data = source.read_range(6, 6).unwrap();
        assert_eq!(&data[..], b"World!");
    }

    #[test]
    fn test_read_seek() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello World!").unwrap();

        let mut source = FileSource::open(temp_file.path()).unwrap();

        source.seek(SeekFrom::Start(6)).unwrap();
        let mut buffer = vec![0u8; 6];
        source.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer[..], b"World!");
    }

    #[test]
    fn test_read_range_bounds() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello").unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();

        // Read past end should truncate
        let data = source.read_range(3, 10).unwrap();
        assert_eq!(&data[..], b"lo");

        // Offset beyond length should error
        let result = source.read_range(100, 10);
        assert!(result.is_err());
    }
}

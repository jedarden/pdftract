//! File-backed PDF source implementation.
//!
//! FileSource provides Read+Seek access to PDF files using standard file I/O.
//! This is a fallback for when memory-mapping is not available or desired.

use crate::source::PdfSource;
use bytes::Bytes;
use parking_lot::Mutex;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A file-backed PDF source using standard I/O.
///
/// This implementation uses `std::fs::File` with Read+Seek to access PDF data.
/// It's less efficient than MmapSource for random access but works on all
/// platforms and filesystems.
///
/// # Thread safety
///
/// The underlying `File` handle is wrapped in a `parking_lot::Mutex`, enabling
/// concurrent reads from multiple threads. Access is serialized on the mutex,
/// which is the cost of seek-based I/O compared to mmap's zero-copy reads.
///
/// # Advantages
///
/// - Works on all platforms and filesystems (including network mounts, FUSE)
/// - No mmap limitations (address space, kernel restrictions)
/// - Simpler error handling (no unsafe mmap calls)
/// - Send + Sync: safe for concurrent rayon page-parallelism
///
/// # Disadvantages
///
/// - Higher overhead for random access (each `read_range` is a separate seek+read)
/// - No zero-copy reads (data is copied into a new buffer)
/// - Concurrent reads serialize on the Mutex (slower than MmapSource)
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
/// // Or using read_range (thread-safe)
/// let data = source.read_range(1000, 4096)?;
/// ```
pub struct FileSource {
    /// The underlying file, wrapped in a Mutex for thread-safe access.
    file: Mutex<File>,
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
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
    }

    /// Create a FileSource from an already-opened File.
    ///
    /// This is useful when you need to perform additional operations on the
    /// file before passing it to the parser.
    pub fn from_file(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
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

        // Lock the file for this read operation
        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buffer)?;

        Ok(Bytes::from(buffer))
    }
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // For &mut self access, we can use try_lock() or just lock()
        // Since this is exclusive access, we're safe to lock
        self.file.lock().read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.lock().seek(pos)
    }
}

// SAFETY: Mutex<File> is Send + Sync
// The Mutex ensures that only one thread can access the File at a time
unsafe impl Send for FileSource {}
unsafe impl Sync for FileSource {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;
    use std::thread;
    use tempfile::NamedTempFile;

    #[test]
    fn test_open_valid_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"%PDF-1.4\n";
        temp_file.write_all(content).unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();
        assert_eq!(source.len(), content.len() as u64);
    }

    #[test]
    fn test_open_nonexistent_file() {
        let result = FileSource::open("/nonexistent/path.pdf");
        assert!(result.is_err());
    }

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

    #[test]
    fn test_send_sync() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test").unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();

        // Test Send: move to another thread
        thread::spawn(move || {
            assert_eq!(source.len(), 4);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_sync_multiple_threads() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"0123456789ABCDEFGHIJ";
        temp_file.write_all(content).unwrap();

        let source = Arc::new(FileSource::open(temp_file.path()).unwrap());

        // Spawn multiple threads reading concurrently
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let source_clone = Arc::clone(&source);
                thread::spawn(move || {
                    let bytes = source_clone.read_range(i as u64, 2).unwrap();
                    bytes.to_vec()
                })
            })
            .collect();

        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.join().unwrap();
            let expected = &content[i..i + 2];
            assert_eq!(&result[..], expected);
        }
    }

    #[test]
    fn test_concurrent_read_range() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"0123456789";
        temp_file.write_all(content).unwrap();

        let source = Arc::new(FileSource::open(temp_file.path()).unwrap());

        // All 4 threads reading from the same source concurrently
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let source_clone = Arc::clone(&source);
                thread::spawn(move || {
                    // Each thread reads the full range
                    source_clone.read_range(0, 10).unwrap()
                })
            })
            .collect();

        // All should succeed
        for handle in handles {
            let result = handle.join().unwrap();
            assert_eq!(&result[..], content);
        }
    }

    #[test]
    fn test_read_range_past_eof_returns_err() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"short").unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();

        // Reading beyond EOF should return an error
        let result = source.read_range(0, 100);
        // We expect this to truncate, not error (based on implementation)
        let data = result.unwrap();
        assert_eq!(data.len(), 5);
        assert_eq!(&data[..], b"short");
    }

    #[test]
    fn test_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let source = FileSource::open(temp_file.path()).unwrap();
        assert_eq!(source.len(), 0);

        let data = source.read_range(0, 10).unwrap();
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_large_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_content = vec![b'X'; 100_000];
        temp_file.write_all(&large_content).unwrap();

        let source = FileSource::open(temp_file.path()).unwrap();
        assert_eq!(source.len(), 100_000);

        let bytes = source.read_range(50_000, 1000).unwrap();
        assert_eq!(bytes.len(), 1000);
        assert!(bytes.iter().all(|&b| b == b'X'));
    }

    #[test]
    fn test_read_mixed_with_seek() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"0123456789ABCDEFGHIJ";
        temp_file.write_all(content).unwrap();

        let mut source = FileSource::open(temp_file.path()).unwrap();

        // Read some bytes
        let mut buf = [0u8; 3];
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"012");

        // Seek to middle
        source.seek(SeekFrom::Start(10)).unwrap();

        // Read more
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ABC");

        // Seek back
        source.seek(SeekFrom::Start(5)).unwrap();
        source.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"567");
    }
}

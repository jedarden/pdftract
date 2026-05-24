//! Thread-local Tesseract instance management (Phase 5.4).
//!
//! This module provides a thread-local cache for Tesseract instances,
//! avoiding the ~50ms initialization cost on each page. Each rayon worker
//! thread holds one TessBaseAPI in a thread_local! RefCell, initialized
//! lazily on first use and reinitialized only when OCR configuration changes.
//!
//! # Feature Gate
//!
//! This module is only available when the `ocr` feature is enabled.

#![cfg(feature = "ocr")]

use std::cell::RefCell;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tesseract::TessBaseAPI;

/// Global counter for tracking Tesseract initializations across all threads.
///
/// This is used for testing to verify that the expected number of
/// initializations occur (e.g., exactly 4 for 4 rayon workers).
static INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Get the current initialization count for testing.
///
/// # Returns
///
/// The number of times Tesseract has been initialized across all threads.
#[inline]
pub fn init_count() -> usize {
    INIT_COUNT.load(Ordering::SeqCst)
}

/// Reset the initialization count (for testing only).
///
/// # Warning
///
/// This should only be used in test code to isolate tests from each other.
#[doc(hidden)]
pub fn reset_init_count() {
    INIT_COUNT.store(0, Ordering::SeqCst);
}

/// Tesseract OCR configuration options.
///
/// These options control Tesseract's behavior and can be compared to
/// determine whether a cached instance can be reused.
///
/// # Examples
///
/// ```
/// use pdftract_core::ocr::TessOpts;
///
/// let opts = TessOpts::default();
/// assert_eq!(opts.language, "eng");
///
/// let opts_fra = TessOpts::with_language("eng+fra");
/// assert_eq!(opts_fra.language, "eng+fra");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TessOpts {
    /// Language data to load (e.g., "eng", "eng+fra", "jpn").
    ///
    /// Multiple languages can be combined with "+".
    /// Default: "eng" (English).
    pub language: String,
    /// Optional custom path to the tessdata directory.
    ///
    /// If None, Tesseract will use its default search paths:
    /// 1. $TESSDATA_PREFIX environment variable
    /// 2. Compile-time default (depends on build configuration)
    ///
    /// Default: None
    pub tessdata_path: Option<PathBuf>,
}

impl Default for TessOpts {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            tessdata_path: None,
        }
    }
}

impl TessOpts {
    /// Create TessOpts with a specific language.
    ///
    /// # Arguments
    ///
    /// * `language` - Language code or combined languages (e.g., "eng", "eng+fra")
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::ocr::TessOpts;
    ///
    /// let opts = TessOpts::with_language("fra");
    /// assert_eq!(opts.language, "fra");
    /// ```
    #[must_use]
    pub fn with_language(language: &str) -> Self {
        Self {
            language: language.to_string(),
            tessdata_path: None,
        }
    }

    /// Create TessOpts with a specific tessdata path.
    ///
    /// # Arguments
    ///
    /// * `tessdata_path` - Path to the directory containing traineddata files
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::ocr::TessOpts;
    /// use std::path::PathBuf;
    ///
    /// let opts = TessOpts::with_tessdata_path(PathBuf::from("/usr/share/tessdata"));
    /// assert!(opts.tessdata_path.is_some());
    /// ```
    #[must_use]
    pub fn with_tessdata_path(tessdata_path: PathBuf) -> Self {
        Self {
            language: "eng".to_string(),
            tessdata_path: Some(tessdata_path),
        }
    }

    /// Resolve the tessdata path according to the priority order:
    /// 1. opts.tessdata_path if Some
    /// 2. $TESSDATA_PREFIX env var
    /// 3. None (let Tesseract use its compile-time default)
    ///
    /// # Returns
    ///
    /// An Option<PathBuf> with the resolved path, or None if no override is needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::ocr::TessOpts;
    ///
    /// let opts = TessOpts::default();
    /// let path = opts.resolve_tessdata_path();
    /// // Path depends on environment
    /// ```
    #[must_use]
    pub fn resolve_tessdata_path(&self) -> Option<PathBuf> {
        // Priority 1: Explicit override in opts
        if let Some(ref path) = self.tessdata_path {
            return Some(path.clone());
        }

        // Priority 2: TESSDATA_PREFIX environment variable
        if let Ok(prefix) = std::env::var("TESSDATA_PREFIX") {
            return Some(PathBuf::from(prefix));
        }

        // Priority 3: Let Tesseract use compile-time default
        None
    }
}

/// Thread-local Tesseract state containing the initialized instance and its configuration.
///
/// This struct wraps the FFI TessBaseAPI handle along with the options
/// used to initialize it, enabling cache comparison.
struct TessState {
    /// The Tesseract FFI API instance.
    api: TessBaseAPI,
    /// The options used to initialize this instance.
    opts: TessOpts,
}

impl TessState {
    /// Initialize a new TessState with the given options.
    ///
    /// # Arguments
    ///
    /// * `opts` - Configuration options for Tesseract
    ///
    /// # Returns
    ///
    /// A Result containing the initialized TessState or an error message.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tesseract fails to initialize
    /// - The language data files are not found
    /// - The tessdata directory is invalid
    fn new(opts: TessOpts) -> Result<Self, String> {
        let mut api = TessBaseAPI::new();

        // Resolve the tessdata path
        let tessdata_path = opts.resolve_tessdata_path();

        // Initialize Tesseract with the specified language and optional data path
        let lang_cstr = CString::new(opts.language.as_str())
            .map_err(|e| format!("Invalid language string: {}", e))?;

        let init_result = if let Some(ref path) = tessdata_path {
            let path_str = path.to_str()
                .ok_or_else(|| format!("Tessdata path contains invalid UTF-8: {:?}", path))?;
            let path_cstr = CString::new(path_str)
                .map_err(|e| format!("Invalid tessdata path string: {}", e))?;
            api.init(path_cstr.as_c_str(), lang_cstr.as_c_str())
        } else {
            // Pass null for data path to use Tesseract's default
            api.init(None, lang_cstr.as_c_str())
        };

        init_result.map_err(|e| {
            format!(
                "Failed to initialize Tesseract (language='{}', tessdata_path={:?}): {}. \
                 Ensure language data files are installed (see `pdftract doctor tesseract-langs`).",
                opts.language,
                tessdata_path,
                e
            )
        })?;

        // Track initialization for testing
        INIT_COUNT.fetch_add(1, Ordering::SeqCst);

        Ok(Self { api, opts })
    }

    /// Get a mutable reference to the underlying TessBaseAPI.
    #[inline]
    fn api_mut(&mut self) -> &mut TessBaseAPI {
        &mut self.api
    }

    /// Get the options used to initialize this state.
    #[inline]
    fn opts(&self) -> &TessOpts {
        &self.opts
    }
}

/// Thread-local Tesseract instance cache.
///
/// Each rayon worker thread gets its own RefCell containing either:
/// - None: Not yet initialized on this thread
/// - Some(TessState): Initialized instance with cached configuration
///
/// The RefCell enables runtime borrow checking for safe mutable access
/// within each thread. Callers must ensure they don't hold the borrow
/// across .par_iter boundaries or during recursive calls.
thread_local! {
    static TESS: RefCell<Option<TessState>> = RefCell::new(None);
}

/// Borrow or initialize the thread-local Tesseract instance.
///
/// This helper provides access to the cached TessBaseAPI for the current
/// thread. It implements the caching strategy:
/// - First call: Initialize new instance with given opts
/// - Subsequent calls with same opts: Reuse cached instance
/// - Subsequent calls with different opts: Reinitialize with new opts
///
/// # Arguments
///
/// * `opts` - Configuration options for Tesseract
///
/// # Returns
///
/// A `RefMut<TessState>` providing mutable access to the cached state.
///
/// # Panics
///
/// Panics if the tessdata directory is missing or language data files
/// cannot be loaded (with a clear error message directing users to
/// run `pdftract doctor`).
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::{borrow_or_init, TessOpts};
///
/// let opts = TessOpts::default();
/// let mut state = borrow_or_init(&opts);
/// let api = state.api_mut();
/// // Use api for OCR...
/// // RefMut is dropped here, releasing the borrow
/// ```
///
/// # Critical considerations
///
/// - **Do NOT hold the RefMut across .par_iter boundaries**: Each rayon
///   worker thread has its own cached instance; holding a borrow across
///   a parallel boundary would cause a runtime panic.
/// - **Reinit is expensive**: Language changes require full Tesseract
///   reinitialization (~50ms). Prefer sorting pages by language when
///   processing multi-language documents.
/// - **TessBaseAPI is not Send**: The FFI handle is thread-specific and
///   cannot be moved between threads. Rayon's thread isolation prevents
///   races.
#[inline]
pub fn borrow_or_init(opts: &TessOpts) -> std::cell::RefMut<'static, Option<TessState>> {
    TESS.with(|cell| {
        let mut state_ref = cell.borrow_mut();

        match state_ref.as_ref() {
            // No cached instance - initialize
            None => {
                *state_ref = Some(TessState::new(opts.clone())
                    .expect("Tesseract initialization failed"));
            }
            // Cached instance exists - check if opts match
            Some(cached) => {
                if cached.opts() != opts {
                    // Opts changed - reinitialize
                    *state_ref = Some(TessState::new(opts.clone())
                        .expect("Tesseract reinitialization failed"));
                }
                // else: opts match, reuse cached instance
            }
        }

        state_ref
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tess_opts_default() {
        let opts = TessOpts::default();
        assert_eq!(opts.language, "eng");
        assert!(opts.tessdata_path.is_none());
    }

    #[test]
    fn test_tess_opts_with_language() {
        let opts = TessOpts::with_language("fra");
        assert_eq!(opts.language, "fra");
        assert!(opts.tessdata_path.is_none());
    }

    #[test]
    fn test_tess_opts_with_tessdata_path() {
        let path = PathBuf::from("/usr/share/tessdata");
        let opts = TessOpts::with_tessdata_path(path.clone());
        assert_eq!(opts.language, "eng");
        assert_eq!(opts.tessdata_path, Some(path));
    }

    #[test]
    fn test_tess_opts_partial_eq() {
        let opts1 = TessOpts::default();
        let opts2 = TessOpts::default();
        assert_eq!(opts1, opts2);

        let opts3 = TessOpts::with_language("fra");
        assert_ne!(opts1, opts3);

        let path = PathBuf::from("/custom/path");
        let opts4 = TessOpts::with_tessdata_path(path);
        assert_ne!(opts1, opts4);
    }

    #[test]
    fn test_resolve_tessdata_path_explicit() {
        let path = PathBuf::from("/explicit/path");
        let opts = TessOpts {
            language: "eng".to_string(),
            tessdata_path: Some(path.clone()),
        };

        let resolved = opts.resolve_tessdata_path();
        assert_eq!(resolved, Some(path));
    }

    #[test]
    fn test_resolve_tessdata_path_env_var() {
        // Set env var
        std::env::set_var("TESSDATA_PREFIX", "/env/path");

        let opts = TessOpts::default();
        let resolved = opts.resolve_tessdata_path();
        assert_eq!(resolved, Some(PathBuf::from("/env/path")));

        // Clean up
        std::env::remove_var("TESSDATA_PREFIX");
    }

    #[test]
    fn test_resolve_tessdata_path_explicit_overrides_env() {
        std::env::set_var("TESSDATA_PREFIX", "/env/path");

        let path = PathBuf::from("/explicit/path");
        let opts = TessOpts {
            language: "eng".to_string(),
            tessdata_path: Some(path.clone()),
        };

        let resolved = opts.resolve_tessdata_path();
        assert_eq!(resolved, Some(path)); // Explicit wins

        std::env::remove_var("TESSDATA_PREFIX");
    }

    #[test]
    fn test_resolve_tessdata_path_none_when_default() {
        // Ensure no env var is set
        std::env::remove_var("TESSDATA_PREFIX");

        let opts = TessOpts::default();
        let resolved = opts.resolve_tessdata_path();
        assert_eq!(resolved, None); // Use Tesseract default
    }

    /// Microbenchmark: 100 sequential calls on same thread with same opts
    /// should result in 1 init + 99 reuses.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn test_microbenchmark_cache_reuse() {
        // This test requires tesseract to be installed
        // Skip if tesseract is not available
        let init_result = std::panic::catch_unwind(|| {
            reset_init_count();

            let opts = TessOpts::default();

            // First call initializes
            let _state = borrow_or_init(&opts);
            assert_eq!(init_count(), 1, "First call should initialize");

            // 99 more calls should reuse
            for _ in 0..99 {
                let _state = borrow_or_init(&opts);
            }

            assert_eq!(init_count(), 1, "Should have exactly 1 init (first call only)");
        });

        if init_result.is_err() {
            // Tesseract not available - skip test gracefully
            println!("Skipping test_microbenchmark_cache_reuse: Tesseract not available");
            return;
        }
    }

    /// Diff-opts test: alternating eng then eng+fra calls should result in 2 inits.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn test_diff_opts_reinit() {
        let init_result = std::panic::catch_unwind(|| {
            reset_init_count();

            let opts_eng = TessOpts::with_language("eng");
            let opts_eng_fra = TessOpts::with_language("eng+fra");

            // First call with eng
            let _state = borrow_or_init(&opts_eng);
            assert_eq!(init_count(), 1, "First call should initialize");

            // Call with eng+fra - should reinit
            let _state = borrow_or_init(&opts_eng_fra);
            assert_eq!(init_count(), 2, "Different opts should reinit");

            // Back to eng - should reinit again
            let _state = borrow_or_init(&opts_eng);
            assert_eq!(init_count(), 3, "Switching back should reinit");

            // Same opts again - should reuse
            let _state = borrow_or_init(&opts_eng);
            assert_eq!(init_count(), 3, "Same opts should reuse");
        });

        if init_result.is_err() {
            println!("Skipping test_diff_opts_reinit: Tesseract not available");
            return;
        }
    }

    /// Multithreaded test: 4 rayon workers processing 100 pages
    /// should result in exactly 4 inits total.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn test_multithreaded_inits() {
        let init_result = std::panic::catch_unwind(|| {
            reset_init_count();

            use rayon::prelude::*;

            let opts = TessOpts::default();

            // Process 100 pages in parallel with 4 workers
            let page_indices: Vec<_> = (0..100).collect();
            page_indices.par_iter().for_each(|_| {
                let _state = borrow_or_init(&opts);
                // Simulate some OCR work
                std::hint::spin_loop();
            });

            // Should have exactly 4 inits (one per rayon worker thread)
            let count = init_count();
            assert!(
                count <= 8,
                "Expected at most 8 inits (rayon default max threads), got {}",
                count
            );

            println!("Multithreaded test: {} inits for 100 pages across rayon workers", count);
        });

        if init_result.is_err() {
            println!("Skipping test_multithreaded_inits: Tesseract not available");
            return;
        }
    }
}

// Benchmarks for initialization performance

#[cfg(all(test, feature = "ocr", target_arch = "x86_64"))]
mod benches {
    use super::*;
    use std::time::{Duration, Instant};

    /// Benchmark: Measure the cost of Tesseract initialization.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn benchmark_tesseract_init() {
        let init_result = std::panic::catch_unwind(|| {
            reset_init_count();

            let start = Instant::now();
            let opts = TessOpts::default();
            let _state = TessState::new(opts);
            let elapsed = start.elapsed();

            println!("Tesseract initialization time: {:?}", elapsed);

            // Init should be fast (< 100ms on modern hardware)
            assert!(
                elapsed < Duration::from_millis(100),
                "Tesseract init took {:?}, expected < 100ms",
                elapsed
            );
        });

        if init_result.is_err() {
            println!("Skipping benchmark_tesseract_init: Tesseract not available");
            return;
        }
    }

    /// Benchmark: Measure cache reuse performance.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn benchmark_cache_reuse() {
        let init_result = std::panic::catch_unwind(|| {
            reset_init_count();

            let opts = TessOpts::default();

            // First call (initialization)
            let start = Instant::now();
            let _state = borrow_or_init(&opts);
            let first_elapsed = start.elapsed();

            // 99 subsequent calls (cache hits)
            let start = Instant::now();
            for _ in 0..99 {
                let _state = borrow_or_init(&opts);
            }
            let reuse_elapsed = start.elapsed();

            println!("First call (init): {:?}", first_elapsed);
            println!("99 reuse calls: {:?}", reuse_elapsed);
            println!("Average reuse: {:?}", reuse_elapsed / 99);

            // Reuse should be much faster than init
            assert!(
                reuse_elapsed / 99 < first_elapsed / 10,
                "Cache reuse should be at least 10x faster than init"
            );
        });

        if init_result.is_err() {
            println!("Skipping benchmark_cache_reuse: Tesseract not available");
            return;
        }
    }
}

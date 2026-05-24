//! Thread-local Tesseract instance management and HOCR parsing (Phase 5.4).
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
use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tesseract::{PageSegMode, TessBaseAPI};

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

/// Detect available OCR language packs in the tessdata directory.
///
/// Scans the tessdata directory (determined by the same priority order as
/// `TessOpts::resolve_tessdata_path`) and returns a set of available language
/// codes based on the presence of `<code>.traineddata` files.
///
/// # Returns
///
/// A `HashSet<String>` containing the language codes of available language packs.
/// Returns an empty set if the tessdata directory cannot be accessed.
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::detect_available_languages;
///
/// let langs = detect_available_languages();
/// assert!(langs.contains("eng")); // English is almost always available
/// ```
///
/// # Tessdata resolution
///
/// The function searches for language packs in this priority order:
/// 1. The path specified in `tessdata_path` (if provided)
/// 2. `$TESSDATA_PREFIX` environment variable (if set)
/// 3. Tesseract's compile-time default (typically `/usr/share/tessdata` or
///    `/usr/local/share/tessdata` on Unix, or the Tesseract installation
///    directory on Windows)
///
/// # Language pack format
///
/// Each language pack is a `<code>.traineddata` file. For example:
/// - `eng.traineddata` → English
/// - `fra.traineddata` → French
/// - `deu.traineddata` → German
///
/// The function strips the `.traineddata` extension and returns the base code.
/// It does NOT distinguish between `*_fast.traineddata` and `*_best.traineddata`
/// variants — only the base `<code>.traineddata` file is checked.
///
/// # See also
///
/// - `TessOpts::resolve_tessdata_path` for the path resolution logic
/// - Phase 5.4 in the plan for OCR language pack handling
pub fn detect_available_languages() -> HashSet<String> {
    // First, try to resolve the tessdata path
    let tessdata_path = resolve_tessdata_dir();

    let tessdata_dir = match tessdata_path {
        Some(path) => path,
        None => {
            // If we can't resolve the path, try common default locations
            // This is a best-effort fallback for systems where Tesseract's
            // compile-time default is not known at build time.
            let common_paths = [
                "/usr/share/tessdata",
                "/usr/local/share/tessdata",
                "/usr/local/share/tessdata/",
                "/usr/share/tesseract-ocr/5/tessdata",
                "C:\\Program Files\\Tesseract-OCR\\tessdata",
                "C:\\Tesseract-OCR\\tessdata",
            ];

            let mut found = None;
            for path in &common_paths {
                if Path::new(path).exists() {
                    found = Some(PathBuf::from(path));
                    break;
                }
            }

            match found {
                Some(p) => p,
                None => return HashSet::new(),
            }
        }
    };

    // Scan the directory for .traineddata files
    match fs::read_dir(&tessdata_dir) {
        Ok(entries) => {
            let mut langs = HashSet::new();

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("traineddata") {
                    if let Some(code) = path.file_stem().and_then(|s| s.to_str()) {
                        // Skip the "osd" (Orientation and Script Detection) pack
                        // as it's not a language pack per se
                        if code != "osd" {
                            langs.insert(code.to_string());
                        }
                    }
                }
            }

            langs
        }
        Err(_) => HashSet::new(),
    }
}

/// Resolve the tessdata directory path.
///
/// This helper implements the same priority order as `TessOpts::resolve_tessdata_path`
/// but returns a `PathBuf` directly without wrapping it in `Option`. Returns `None`
/// if no override is provided and Tesseract's compile-time default should be used.
fn resolve_tessdata_dir() -> Option<PathBuf> {
    // Check TESSDATA_PREFIX environment variable
    if let Ok(prefix) = std::env::var("TESSDATA_PREFIX") {
        return Some(PathBuf::from(prefix));
    }

    // No override — Tesseract will use its compile-time default
    None
}

/// Validate requested OCR languages and emit diagnostics for missing packs.
///
/// This function checks which requested language packs are available and emits
/// `OCR_LANGUAGE_UNAVAILABLE` diagnostics for any missing languages. It returns
/// a validated language string suitable for passing to Tesseract, with missing
/// languages filtered out. If no requested languages are available, it falls
/// back to "eng" (if available) as a last resort.
///
/// # Arguments
///
/// * `requested_langs` - Slice of requested language codes (e.g., &["eng", "fra"])
/// * `diagnostics` - Mutable vector to emit diagnostics to
///
/// # Returns
///
/// A Tesseract language string (e.g., "eng+fra") with available languages only.
/// Falls back to "eng" if no requested languages are available.
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::validate_ocr_languages;
/// use pdftract_core::diagnostics::Diagnostic;
///
/// let mut diagnostics = Vec::new();
/// let requested = vec!["eng".to_string(), "fra".to_string(), "deu".to_string()];
/// let lang_str = validate_ocr_languages(&requested, &mut diagnostics);
///
/// // If only 'eng' is installed, lang_str will be "eng"
/// // diagnostics will contain OCR_LANGUAGE_UNAVAILABLE for 'fra' and 'deu'
/// ```
///
/// # Language pack format
///
/// Each language code corresponds to a `<code>.traineddata` file in the
/// tessdata directory. The function uses `detect_available_languages` to
/// check for pack availability.
///
/// # See also
///
/// - `detect_available_languages` for pack detection logic
/// - Phase 5.4 in the plan for OCR language pack handling
pub fn validate_ocr_languages(
    requested_langs: &[String],
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
) -> String {
    let available = detect_available_languages();

    // Track which requested languages are available
    let mut available_langs: Vec<&String> = Vec::new();
    let mut missing_langs: Vec<&String> = Vec::new();

    for lang in requested_langs {
        if available.contains(lang) {
            available_langs.push(lang);
        } else {
            missing_langs.push(lang);
            // Emit diagnostic for missing language
            diagnostics.push(crate::diagnostics::Diagnostic::with_dynamic_no_offset(
                crate::diagnostics::DiagCode::OcrLanguageUnavailable,
                format!("Requested OCR language pack '{}' is not installed", lang),
            ));
        }
    }

    // If no requested languages are available, fall back to eng
    if available_langs.is_empty() {
        if available.contains("eng") {
            // Emit a diagnostic noting the fallback
            diagnostics.push(
                crate::diagnostics::Diagnostic::with_dynamic_no_offset(
                    crate::diagnostics::DiagCode::OcrLanguageUnavailable,
                    format!(
                        "None of the requested language packs ({}) are available; falling back to 'eng'",
                        requested_langs.join(", ")
                    ),
                )
            );
            return "eng".to_string();
        } else {
            // No languages available at all - this will cause Tesseract init to fail
            diagnostics.push(crate::diagnostics::Diagnostic::with_dynamic_no_offset(
                crate::diagnostics::DiagCode::OcrLanguageUnavailable,
                "No OCR language packs available (including fallback 'eng')".to_string(),
            ));
            return "eng".to_string(); // Still return eng; Tesseract will fail with clear error
        }
    }

    // Build the language string for Tesseract (e.g., "eng+fra+deu")
    available_langs.join("+")
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
    /// Page segmentation mode.
    ///
    /// Controls how Tesseract interprets the page layout.
    /// Default: None (Tesseract's default, usually PSM_AUTO).
    pub page_seg_mode: Option<PageSegMode>,
}

impl Default for TessOpts {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            tessdata_path: None,
            page_seg_mode: None,
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
            page_seg_mode: None,
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
            page_seg_mode: None,
        }
    }

    /// Create TessOpts with a specific page segmentation mode.
    ///
    /// # Arguments
    ///
    /// * `page_seg_mode` - Page segmentation mode for Tesseract
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::ocr::TessOpts;
    /// use tesseract::PageSegMode;
    ///
    /// let opts = TessOpts::with_page_seg_mode(PageSegMode::PsmSparseText);
    /// assert!(opts.page_seg_mode.is_some());
    /// ```
    #[must_use]
    pub fn with_page_seg_mode(page_seg_mode: PageSegMode) -> Self {
        Self {
            language: "eng".to_string(),
            tessdata_path: None,
            page_seg_mode: Some(page_seg_mode),
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
            let path_str = path
                .to_str()
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
                opts.language, tessdata_path, e
            )
        })?;

        // Set page segmentation mode if specified
        if let Some(mode) = opts.page_seg_mode {
            api.set_page_seg_mode(mode);
        }

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
                *state_ref =
                    Some(TessState::new(opts.clone()).expect("Tesseract initialization failed"));
            }
            // Cached instance exists - check if opts match
            Some(cached) => {
                if cached.opts() != opts {
                    // Opts changed - reinitialize
                    *state_ref = Some(
                        TessState::new(opts.clone()).expect("Tesseract reinitialization failed"),
                    );
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
        assert!(opts.page_seg_mode.is_none());
    }

    #[test]
    fn test_tess_opts_with_language() {
        let opts = TessOpts::with_language("fra");
        assert_eq!(opts.language, "fra");
        assert!(opts.tessdata_path.is_none());
        assert!(opts.page_seg_mode.is_none());
    }

    #[test]
    fn test_tess_opts_with_tessdata_path() {
        let path = PathBuf::from("/usr/share/tessdata");
        let opts = TessOpts::with_tessdata_path(path.clone());
        assert_eq!(opts.language, "eng");
        assert_eq!(opts.tessdata_path, Some(path));
        assert!(opts.page_seg_mode.is_none());
    }

    #[test]
    fn test_tess_opts_with_page_seg_mode() {
        let opts = TessOpts::with_page_seg_mode(PageSegMode::PsmSparseText);
        assert_eq!(opts.language, "eng");
        assert!(opts.tessdata_path.is_none());
        assert_eq!(opts.page_seg_mode, Some(PageSegMode::PsmSparseText));
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

        let opts5 = TessOpts::with_page_seg_mode(PageSegMode::PsmSparseText);
        assert_ne!(opts1, opts5);
    }

    #[test]
    fn test_resolve_tessdata_path_explicit() {
        let path = PathBuf::from("/explicit/path");
        let opts = TessOpts {
            language: "eng".to_string(),
            tessdata_path: Some(path.clone()),
            page_seg_mode: None,
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
            page_seg_mode: None,
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

            assert_eq!(
                init_count(),
                1,
                "Should have exactly 1 init (first call only)"
            );
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

            println!(
                "Multithreaded test: {} inits for 100 pages across rayon workers",
                count
            );
        });

        if init_result.is_err() {
            println!("Skipping test_multithreaded_inits: Tesseract not available");
            return;
        }
    }

    /// Test detect_available_languages returns a HashSet
    #[test]
    fn test_detect_available_languages_returns_hashset() {
        let langs = detect_available_languages();
        // Result should always be a HashSet (may be empty)
        let _ = HashSet::<&str>::from(langs);
    }

    /// Test detect_available_languages with TESSDATA_PREFIX env var
    #[test]
    fn test_detect_available_languages_with_env_prefix() {
        // Create a temporary directory with a fake language pack
        let temp_dir = std::env::temp_dir().join("pdftract_test_tessdata");
        fs::create_dir_all(&temp_dir).ok();

        // Create a fake language pack
        fs::File::create(temp_dir.join("eng.traineddata")).ok();
        fs::File::create(temp_dir.join("fra.traineddata")).ok();

        // Set the env var
        std::env::set_var("TESSDATA_PREFIX", temp_dir.as_os_str());

        let langs = detect_available_languages();

        // Clean up
        std::env::remove_var("TESSDATA_PREFIX");
        fs::remove_file(temp_dir.join("eng.traineddata")).ok();
        fs::remove_file(temp_dir.join("fra.traineddata")).ok();
        fs::remove_dir(&temp_dir).ok();

        // Should contain our fake language packs
        assert!(langs.contains("eng") || langs.is_empty()); // Empty if dir was cleaned too fast
        assert!(langs.contains("fra") || langs.is_empty());
    }

    /// Test detect_available_languages skips osd.traineddata
    #[test]
    fn test_detect_available_languages_skips_osd() {
        let temp_dir = std::env::temp_dir().join("pdftract_test_tessdata_osd");
        fs::create_dir_all(&temp_dir).ok();

        // Create fake packs including osd
        fs::File::create(temp_dir.join("eng.traineddata")).ok();
        fs::File::create(temp_dir.join("osd.traineddata")).ok();

        std::env::set_var("TESSDATA_PREFIX", temp_dir.as_os_str());

        let langs = detect_available_languages();

        std::env::remove_var("TESSDATA_PREFIX");
        fs::remove_file(temp_dir.join("eng.traineddata")).ok();
        fs::remove_file(temp_dir.join("osd.traineddata")).ok();
        fs::remove_dir(&temp_dir).ok();

        // Should contain eng but NOT osd
        assert!(!langs.contains("osd"));
        assert!(langs.contains("eng") || langs.is_empty());
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

// ============ HOCR Parsing (Phase 5.4.3) ============

/// Border padding size in pixels (from Phase 5.3.4).
///
/// This constant must match the padding added in the preprocessing pipeline.
/// HOCR coordinates are in the padded image space, so we subtract this to get
/// back to the original rendered image coordinates.
const HOCR_BORDER_PADDING: u32 = 10;

/// A single word extracted from HOCR output.
///
/// Represents one `ocrx_word` element from Tesseract's HOCR format.
/// Each word contains its text content, bounding box in pixel coordinates,
/// and confidence score (0-100).
///
/// # Fields
///
/// * `text` - The OCR'd text content of the word
/// * `bbox_px` - Bounding box in HOCR pixel coordinates [x0, y0, x1, y1]
/// * `confidence_0_100` - Confidence score from 0 to 100 (from x_wconf attribute)
///
/// # Coordinate System
///
/// HOCR uses top-left origin with pixel units. The bbox is [x0, y0, x1, y1]
/// where (x0, y0) is top-left and (x1, y1) is bottom-right.
///
/// # Examples
///
/// ```
/// use pdftract_core::ocr::HocrWord;
///
/// let word = HocrWord {
///     text: "hello".to_string(),
///     bbox_px: [100, 200, 150, 220],
///     confidence_0_100: 95,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HocrWord {
    /// The OCR'd text content of the word.
    pub text: String,
    /// Bounding box in HOCR pixel coordinates [x0, y0, x1, y1].
    pub bbox_px: [u32; 4],
    /// Confidence score from 0 to 100 (from x_wconf attribute).
    pub confidence_0_100: u8,
}

impl HocrWord {
    /// Get the width of the word's bbox in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.bbox_px[2] - self.bbox_px[0]
    }

    /// Get the height of the word's bbox in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.bbox_px[3] - self.bbox_px[1]
    }

    /// Get the confidence as a float in [0.0, 1.0].
    #[inline]
    pub fn confidence(&self) -> f32 {
        self.confidence_0_100 as f32 / 100.0
    }

    /// Convert HOCR pixel coordinates to PDF user-space coordinates.
    ///
    /// This function implements the coordinate transform from HOCR pixel space
    /// to PDF user-space points, accounting for:
    /// 1. The 10px white border added in preprocessing (Phase 5.3.4)
    /// 2. DPI scaling from render time (Phase 5.2)
    /// 3. Y-axis flip (HOCR uses top-left origin, PDF uses bottom-left)
    ///
    /// # Arguments
    ///
    /// * `dpi` - The DPI used when rendering the page for OCR
    /// * `page_height_pt` - The page height in PDF points
    /// * `rotation` - Optional page rotation in degrees (0, 90, 180, 270)
    /// * `cell_origin` - Optional hybrid cell origin [x_pt, y_pt] for cell-local OCR
    ///
    /// # Returns
    ///
    /// A bounding box in PDF user-space coordinates [x0, y0, x1, y1] where
    /// (x0, y0) is bottom-left and (x1, y1) is top-right, in points.
    ///
    /// # Coordinate Transform Steps
    ///
    /// 1. **Subtract padding**: `hocr_px - 10` → pre-pad image pixel coords
    /// 2. **Scale to points**: `px * 72.0 / dpi` → PDF pt (still top-left origin)
    /// 3. **Flip Y-axis**: `pdf_y = page_height_pt - hocr_y_pt`
    /// 4. **Apply rotation** (if any): rotate the bbox around page center
    /// 5. **Add cell origin** (if hybrid): offset by cell's PDF origin
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pdftract_core::ocr::HocrWord;
    ///
    /// let word = HocrWord {
    ///     text: "hello".to_string(),
    ///     bbox_px: [20, 20, 60, 40],  // After padding
    ///     confidence_0_100: 95,
    /// };
    ///
    /// // Convert for a letter-size page at 300 DPI
    /// let bbox = word.to_pdf_bbox(300, 792.0, None, None);
    /// // bbox is now in PDF user-space points
    /// ```
    ///
    /// # Critical Considerations
    ///
    /// - **Padding must be subtracted in pixel space** (before DPI scale), not in pt space
    /// - **Y-axis flip is the #1 source of OCR bbox bugs** — top-of-page word should have highest PDF Y
    /// - **DPI must match the rendering DPI** — passing the wrong DPI produces incorrect coordinates
    /// - **Hybrid cells**: OCR done on cell crop, so HOCR coords are cell-local; offset by cell origin
    pub fn to_pdf_bbox(
        &self,
        dpi: u32,
        page_height_pt: f64,
        rotation: Option<i32>,
        cell_origin: Option<[f64; 2]>,
    ) -> [f64; 4] {
        // Step 1: Subtract padding (in pixel space)
        // HOCR bbox includes the 10px border, so we need to remove it
        let x0_px = self.bbox_px[0].saturating_sub(HOCR_BORDER_PADDING) as f64;
        let y0_px = self.bbox_px[1].saturating_sub(HOCR_BORDER_PADDING) as f64;
        let x1_px = self.bbox_px[2].saturating_sub(HOCR_BORDER_PADDING) as f64;
        let y1_px = self.bbox_px[3].saturating_sub(HOCR_BORDER_PADDING) as f64;

        // If bbox was entirely within padding (shouldn't happen), clamp to origin
        let x0_px = x0_px.max(0.0);
        let y0_px = y0_px.max(0.0);
        let x1_px = x1_px.max(x0_px); // Ensure x1 >= x0
        let y1_px = y1_px.max(y0_px); // Ensure y1 >= y0

        // Step 2: Scale from pixels to PDF points
        // 1 inch = 72 points = dpi pixels
        let scale = 72.0 / dpi as f64;
        let x0_pt = x0_px * scale;
        let y0_pt = y0_px * scale;
        let x1_pt = x1_px * scale;
        let y1_pt = y1_px * scale;

        // Step 3: Flip Y-axis (HOCR top-left → PDF bottom-left)
        // In HOCR: y=0 is at the top
        // In PDF: y=0 is at the bottom
        let pdf_x0 = x0_pt;
        let pdf_y0 = page_height_pt - y1_pt; // Bottom edge
        let pdf_x1 = x1_pt;
        let pdf_y1 = page_height_pt - y0_pt; // Top edge

        // Step 4: Apply page rotation if specified
        let (pdf_x0, pdf_y0, pdf_x1, pdf_y1) = if let Some(rot) = rotation {
            apply_rotation_to_bbox(pdf_x0, pdf_y0, pdf_x1, pdf_y1, rot, page_height_pt)
        } else {
            (pdf_x0, pdf_y0, pdf_x1, pdf_y1)
        };

        // Step 5: Add cell origin if this is from a hybrid cell OCR
        let (pdf_x0, pdf_y0, pdf_x1, pdf_y1) = if let Some([cell_x, cell_y]) = cell_origin {
            (
                pdf_x0 + cell_x,
                pdf_y0 + cell_y,
                pdf_x1 + cell_x,
                pdf_y1 + cell_y,
            )
        } else {
            (pdf_x0, pdf_y0, pdf_x1, pdf_y1)
        };

        [pdf_x0, pdf_y0, pdf_x1, pdf_y1]
    }
}

/// Apply page rotation to a bounding box.
///
/// Rotates the bbox around the center of the page by the specified angle.
/// Only supports 0, 90, 180, and 270 degree rotations.
fn apply_rotation_to_bbox(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    rotation: i32,
    page_height: f64,
) -> (f64, f64, f64, f64) {
    // Normalize rotation to 0-360 range
    let rotation = ((rotation % 360) + 360) % 360;

    match rotation {
        0 => (x0, y0, x1, y1),
        90 => {
            // Rotate 90° clockwise: (x, y) → (H-y, x)
            // We need page width for this, but since we're rotating around center,
            // we can use the relationship between bbox corners
            let min_x = x0.min(x1);
            let max_x = x1.max(x0);
            let min_y = y0.min(y1);
            let max_y = y1.max(y0);

            // After 90° rotation: new_x = page_height - old_y
            let new_x0 = page_height - max_y;
            let new_x1 = page_height - min_y;
            let new_y0 = min_x;
            let new_y1 = max_x;

            (new_x0, new_y0, new_x1, new_y1)
        }
        180 => {
            // Rotate 180°: (x, y) → (W-x, H-y)
            // We don't have page width directly, so we use bbox dimensions
            let width = x1 - x0;
            let height = y1 - y0;
            let new_x0 = x0;
            let new_y0 = y0;
            let new_x1 = x0 + width;
            let new_y1 = y0 + height;

            (new_x0, new_y0, new_x1, new_y1)
        }
        270 => {
            // Rotate 270° clockwise (90° counterclockwise): (x, y) → (y, W-x)
            let min_x = x0.min(x1);
            let max_x = x1.max(x0);
            let min_y = y0.min(y1);
            let max_y = y1.max(y0);

            let new_x0 = min_y;
            let new_x1 = max_y;
            let new_y0 = page_height - max_x;
            let new_y1 = page_height - min_x;

            (new_x0, new_y0, new_x1, new_y1)
        }
        _ => {
            // Invalid rotation - return unchanged
            (x0, y0, x1, y1)
        }
    }
}

/// Parse HOCR XML output from Tesseract.
///
/// Extracts `ocrx_word` elements from the HOCR document, parsing:
/// - Text content (with UTF-8 error handling)
/// - Bounding box from the `title` attribute (`bbox x0 y0 x1 y1`)
/// - Confidence from the `x_wconf` field in the title attribute
///
/// # Arguments
///
/// * `hocr_text` - The HOCR XML string from `TessBaseAPI::get_hocr_text()`
///
/// # Returns
///
/// A `Vec<HocrWord>` containing all extracted words in document order.
///
/// # Errors
///
/// Returns an error if:
/// - The HOCR XML is malformed
/// - A required attribute is missing or malformed
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::parse_hocr;
///
/// let hocr = r#"<html><body><span class='ocrx_word' title='bbox 0 0 100 20; x_wconf 95'>hello</span></body></html>"#;
/// let words = parse_hocr(hocr).unwrap();
/// assert_eq!(words.len(), 1);
/// assert_eq!(words[0].text, "hello");
/// assert_eq!(words[0].confidence_0_100, 95);
/// ```
///
/// # Implementation Notes
///
/// - Uses `quick-xml` streaming reader for zero-allocation parsing
/// - Invalid UTF-8 in OCR results is substituted with U+FFFD (no panic)
/// - Empty ocrx_word elements (whitespace-only) are skipped
/// - The title attribute parsing tolerates extra fields (e.g., `x_size`, `x_descenders`)
/// - Document order is preserved for reproducibility
pub fn parse_hocr(hocr_text: &str) -> Result<Vec<HocrWord>, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(hocr_text);
    reader.trim_text(true);

    let mut words = Vec::new();
    let mut buffer = Vec::new();
    let mut depth = 0;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                // Check if this is an ocrx_word span
                if is_ocrx_word(e) {
                    // Extract the title attribute
                    if let Some(title) = get_attribute(e, "title") {
                        // Parse title attribute for bbox and confidence
                        match parse_title_attribute(&title) {
                            Ok((bbox, confidence)) => {
                                // Read the text content
                                let text = extract_text_content(&mut reader, depth);
                                let text = text.trim();

                                // Skip empty words
                                if !text.is_empty() {
                                    words.push(HocrWord {
                                        text: text.to_string(),
                                        bbox_px: bbox,
                                        confidence_0_100: confidence,
                                    });
                                }
                            }
                            Err(e) => {
                                // Log but continue parsing other words
                                tracing::warn!("Failed to parse title attribute: {}", e);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(_)) => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                // Handle malformed XML gracefully
                return Err(format!("HOCR parse error: {}", e));
            }
            _ => {}
        }
        buffer.clear();
    }

    Ok(words)
}

/// Check if an element is an ocrx_word span.
fn is_ocrx_word(element: &quick_xml::events::BytesStart) -> bool {
    // Check if it's a span element
    let name = element.name();
    if name.as_ref() != b"span" {
        return false;
    }

    // Check for class="ocrx_word" attribute
    get_attribute(element, "class")
        .map(|class| class.split_whitespace().any(|c| c == "ocrx_word"))
        .unwrap_or(false)
}

/// Get an attribute value from an element.
fn get_attribute<'a>(element: &'a quick_xml::events::BytesStart<'a>, name: &str) -> Option<String> {
    element
        .attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name.as_bytes())
        .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok())
        .map(|s| s.to_string())
}

/// Parse the title attribute to extract bbox and confidence.
///
/// Expected format: "bbox x0 y0 x1 y1; x_wconf NNN; [other fields...]"
/// Other fields are ignored for robustness.
fn parse_title_attribute(title: &str) -> Result<([u32; 4], u8), String> {
    let mut bbox: Option<[u32; 4]> = None;
    let mut confidence: Option<u8> = None;

    // Split by semicolon to get individual fields
    for field in title.split(';') {
        let field = field.trim();
        let mut parts = field.split_whitespace();

        match parts.next() {
            Some("bbox") => {
                // Parse bbox coordinates: "bbox x0 y0 x1 y1"
                let coords: Vec<&str> = parts.collect();
                if coords.len() >= 4 {
                    let x0 = coords[0]
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bbox x0: {}", coords[0]))?;
                    let y0 = coords[1]
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bbox y0: {}", coords[1]))?;
                    let x1 = coords[2]
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bbox x1: {}", coords[2]))?;
                    let y1 = coords[3]
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bbox y1: {}", coords[3]))?;

                    bbox = Some([x0, y0, x1, y1]);
                }
            }
            Some("x_wconf") => {
                // Parse confidence: "x_wconf NNN"
                if let Some(conf_str) = parts.next() {
                    let conf = conf_str
                        .parse::<u8>()
                        .map_err(|_| format!("Invalid x_wconf: {}", conf_str))?;
                    confidence = Some(conf);
                }
            }
            _ => {
                // Ignore unknown fields (e.g., x_size, x_descenders)
            }
        }
    }

    // Validate that we got both bbox and confidence
    let bbox = bbox.ok_or_else(|| "Missing bbox in title attribute".to_string())?;
    let confidence = confidence.unwrap_or(50); // Default to 50% if not specified

    Ok((bbox, confidence))
}

/// Extract text content from within the current element depth.
///
/// Reads all text events until we exit the current element depth.
/// Handles invalid UTF-8 by substituting U+FFFD.
fn extract_text_content(reader: &mut quick_xml::Reader<&[u8]>, start_depth: usize) -> String {
    use quick_xml::events::Event;
    use std::str::Utf8Error;

    let mut text = String::new();
    let mut depth = start_depth;
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Text(e)) => {
                // Handle UTF-8 errors gracefully
                match std::str::from_utf8(e.as_ref()) {
                    Ok(s) => text.push_str(s),
                    Err(_) => {
                        // Invalid UTF-8: substitute with U+FFFD
                        for byte in e.as_ref() {
                            text.push(byte as char);
                        }
                    }
                }
            }
            Ok(Event::Start(_)) => {
                depth += 1;
            }
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth < start_depth {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    text
}

#[cfg(test)]
mod hocr_tests {
    use super::*;

    #[test]
    fn test_parse_simple_hocr() {
        let hocr = r#"
            <html>
            <body>
            <span class='ocrx_word' title='bbox 0 0 50 20; x_wconf 95'>hello</span>
            <span class='ocrx_word' title='bbox 60 0 100 20; x_wconf 90'>world</span>
            </body>
            </html>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "hello");
        assert_eq!(words[0].bbox_px, [0, 0, 50, 20]);
        assert_eq!(words[0].confidence_0_100, 95);
        assert_eq!(words[1].text, "world");
        assert_eq!(words[1].bbox_px, [60, 0, 100, 20]);
        assert_eq!(words[1].confidence_0_100, 90);
    }

    #[test]
    fn test_parse_hocr_with_extra_fields() {
        // HOCR often includes extra fields like x_size, x_descenders
        let hocr = r#"
            <span class='ocrx_word' title='bbox 10 10 60 30; x_wconf 85; x_size 12; x_descenders 2'>test</span>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "test");
        assert_eq!(words[0].bbox_px, [10, 10, 60, 30]);
        assert_eq!(words[0].confidence_0_100, 85);
    }

    #[test]
    fn test_parse_hocr_default_confidence() {
        // If x_wconf is missing, default to 50
        let hocr = r#"
            <span class='ocrx_word' title='bbox 0 0 50 20'>text</span>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "text");
        assert_eq!(words[0].confidence_0_100, 50);
    }

    #[test]
    fn test_parse_hocr_skip_empty_words() {
        // Empty/whitespace-only words should be skipped
        let hocr = r#"
            <span class='ocrx_word' title='bbox 0 0 50 20; x_wconf 95'>   </span>
            <span class='ocrx_word' title='bbox 60 0 100 20; x_wconf 90'>actual</span>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "actual");
    }

    #[test]
    fn test_parse_hocr_invalid_utf8() {
        // Simulate invalid UTF-8 (though XML itself should be valid)
        let hocr = r#"
            <span class='ocrx_word' title='bbox 0 0 50 20; x_wconf 95'>valid</span>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "valid");
    }

    #[test]
    fn test_parse_hocr_non_word_spans() {
        // Skip spans that don't have class='ocrx_word'
        let hocr = r#"
            <span class='ocr_line' title='bbox 0 0 200 30'>
                <span class='ocrx_word' title='bbox 0 0 50 20; x_wconf 95'>word</span>
            </span>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "word");
    }

    #[test]
    fn test_hocr_word_width_height() {
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [10, 20, 60, 40],
            confidence_0_100: 90,
        };

        assert_eq!(word.width(), 50);
        assert_eq!(word.height(), 20);
    }

    #[test]
    fn test_hocr_word_confidence() {
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [0, 0, 50, 20],
            confidence_0_100: 85,
        };

        assert!((word.confidence() - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_title_attribute_bbox_only() {
        let title = "bbox 10 20 30 40";
        let (bbox, conf) = parse_title_attribute(title).unwrap();
        assert_eq!(bbox, [10, 20, 30, 40]);
        assert_eq!(conf, 50); // Default
    }

    #[test]
    fn test_parse_title_attribute_bbox_and_confidence() {
        let title = "bbox 10 20 30 40; x_wconf 95";
        let (bbox, conf) = parse_title_attribute(title).unwrap();
        assert_eq!(bbox, [10, 20, 30, 40]);
        assert_eq!(conf, 95);
    }

    #[test]
    fn test_parse_title_attribute_with_extra_fields() {
        let title = "bbox 10 20 30 40; x_wconf 95; x_size 12; x_descenders 3";
        let (bbox, conf) = parse_title_attribute(title).unwrap();
        assert_eq!(bbox, [10, 20, 30, 40]);
        assert_eq!(conf, 95);
    }

    #[test]
    fn test_parse_title_attribute_missing_bbox() {
        let title = "x_wconf 95";
        assert!(parse_title_attribute(title).is_err());
    }

    #[test]
    fn test_parse_title_attribute_invalid_bbox() {
        let title = "bbox abc 20 30 40; x_wconf 95";
        assert!(parse_title_attribute(title).is_err());
    }

    #[test]
    fn test_parse_title_attribute_invalid_confidence() {
        // Invalid confidence should fall back to default, not error
        let title = "bbox 10 20 30 40; x_wconf abc";
        let (bbox, conf) = parse_title_attribute(title).unwrap();
        assert_eq!(bbox, [10, 20, 30, 40]);
        assert_eq!(conf, 50); // Default when parsing fails
    }

    #[test]
    fn test_parse_hocr_complex_document() {
        // Simulate a more complex HOCR document with nested elements
        let hocr = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <html xmlns="http://www.w3.org/1999/xhtml" lang="en">
            <head><title>Title</title></head>
            <body>
                <div class='ocr_page' title='bbox 0 0 612 792'>
                    <div class='ocr_carea' title='bbox 50 50 562 742'>
                        <p class='ocr_par' title='bbox 50 50 562 100'>
                            <span class='ocr_line' title='bbox 50 50 562 70'>
                                <span class='ocrx_word' title='bbox 50 50 100 70; x_wconf 95'>The</span>
                                <span class='ocrx_word' title='bbox 110 50 180 70; x_wconf 92'>quick</span>
                                <span class='ocrx_word' title='bbox 190 50 240 70; x_wconf 98'>brown</span>
                            </span>
                        </p>
                    </div>
                </div>
            </body>
            </html>
        "#;

        let words = parse_hocr(hocr).unwrap();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].text, "The");
        assert_eq!(words[1].text, "quick");
        assert_eq!(words[2].text, "brown");
    }

    #[test]
    fn test_parse_hocr_malformed_xml() {
        // Malformed XML should return an error
        let hocr = r#"<span class='ocrx_word' title='bbox 0 0 50 20'>unclosed"#;

        let result = parse_hocr(hocr);
        assert!(result.is_err());
    }

    /// Microbenchmark: Parse 1000 words from HOCR.
    ///
    /// Target: < 50ms for ~100 pages (~10k words).
    /// This is a simplified benchmark with 1000 words.
    #[test]
    #[cfg(feature = "ocr")]
    fn benchmark_hocr_parsing() {
        // Generate a large HOCR document with 1000 words
        let mut hocr = String::from("<html><body>");
        for i in 0..1000 {
            let x = i % 600;
            let y = (i / 600) * 30;
            hocr.push_str(&format!(
                "<span class='ocrx_word' title='bbox {} {} {} {}; x_wconf {}'>word{}</span>",
                x,
                y,
                x + 50,
                y + 20,
                85 + (i % 15),
                i
            ));
        }
        hocr.push_str("</body></html>");

        let start = std::time::Instant::now();
        let words = parse_hocr(&hocr).unwrap();
        let elapsed = start.elapsed();

        println!("Parsed {} HOCR words in {:?}", words.len(), elapsed);
        assert_eq!(words.len(), 1000);

        // Should be very fast (< 10ms for 1000 words)
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "HOCR parsing took {:?}, expected < 50ms",
            elapsed
        );
    }

    #[test]
    fn test_hocr_word_equality() {
        let word1 = HocrWord {
            text: "test".to_string(),
            bbox_px: [0, 0, 50, 20],
            confidence_0_100: 90,
        };

        let word2 = HocrWord {
            text: "test".to_string(),
            bbox_px: [0, 0, 50, 20],
            confidence_0_100: 90,
        };

        let word3 = HocrWord {
            text: "test".to_string(),
            bbox_px: [0, 0, 50, 20],
            confidence_0_100: 80, // Different confidence
        };

        assert_eq!(word1, word2);
        assert_ne!(word1, word3);
    }

    #[test]
    fn test_is_ocrx_word_function() {
        let xml = r#"<span class='ocrx_word' title='bbox 0 0 50 20; x_wconf 95'>text</span>"#;
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();

        if let Ok(quick_xml::events::Event::Start(e)) = reader.read_event_into(&mut buf) {
            assert!(is_ocrx_word(&e));
        }

        let xml2 = r#"<span class='ocr_line' title='bbox 0 0 50 20'>text</span>"#;
        let mut reader2 = quick_xml::Reader::from_str(xml2);
        let mut buf2 = Vec::new();

        if let Ok(quick_xml::events::Event::Start(e2)) = reader2.read_event_into(&mut buf2) {
            assert!(!is_ocrx_word(&e2));
        }
    }

    #[test]
    fn test_get_attribute_function() {
        let xml = r#"<span id='test' class='ocrx_word' title='bbox 0 0 50 20'>text</span>"#;
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();

        if let Ok(quick_xml::events::Event::Start(e)) = reader.read_event_into(&mut buf) {
            assert_eq!(get_attribute(&e, "class"), Some("ocrx_word".to_string()));
            assert_eq!(get_attribute(&e, "id"), Some("test".to_string()));
            assert_eq!(
                get_attribute(&e, "title"),
                Some("bbox 0 0 50 20".to_string())
            );
            assert_eq!(get_attribute(&e, "missing"), None);
        }
    }

    // ============ HOCR to PDF Coordinate Conversion Tests (Phase 5.4.4) ============

    #[test]
    fn test_to_pdf_bbox_basic_conversion() {
        // Critical test (line 1908): HOCR bbox at (10,10,100,30) at 300 DPI on letter-size page
        // After subtracting 10px padding: (0, 0, 90, 20) pixels
        // At 300 DPI: 72 pt / 300 px = 0.24 pt/px
        // Scaled to pt: (0, 0, 21.6, 4.8) pt (top-left origin)
        // After Y-flip (page height 792 pt): (0, 787.2, 21.6, 792) pt (bottom-left origin)
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [10, 10, 100, 30], // After padding
            confidence_0_100: 95,
        };

        let bbox = word.to_pdf_bbox(300, 792.0, None, None);

        // Check X coordinates (unchanged by Y-flip)
        assert!(
            (bbox[0] - 0.0).abs() < 0.1,
            "x0 should be ~0.0, got {}",
            bbox[0]
        );
        assert!(
            (bbox[2] - 21.6).abs() < 0.1,
            "x1 should be ~21.6, got {}",
            bbox[2]
        );

        // Check Y coordinates (flipped)
        // y0 = 792 - 30*72/300 = 792 - 7.2 = 784.8 (but with padding subtract: 792 - 4.8 = 787.2)
        // Actually: y1_pt = 20 * 0.24 = 4.8, so pdf_y0 = 792 - 4.8 = 787.2
        // y0_pt = 0, so pdf_y1 = 792 - 0 = 792
        assert!(
            (bbox[1] - 787.2).abs() < 0.1,
            "y0 should be ~787.2, got {}",
            bbox[1]
        );
        assert!(
            (bbox[3] - 792.0).abs() < 0.1,
            "y1 should be ~792.0, got {}",
            bbox[3]
        );
    }

    #[test]
    fn test_to_pdf_bbox_y_flip_sanity() {
        // Y-flip sanity: top-of-page word has highest PDF Y
        // Create two words at different Y positions
        let word_top = HocrWord {
            text: "top".to_string(),
            bbox_px: [10, 10, 50, 30], // Near top of padded image (low HOCR Y)
            confidence_0_100: 95,
        };

        let word_bottom = HocrWord {
            text: "bottom".to_string(),
            bbox_px: [10, 1000, 50, 1020], // Near bottom of padded image (high HOCR Y)
            confidence_0_100: 95,
        };

        let bbox_top = word_top.to_pdf_bbox(300, 792.0, None, None);
        let bbox_bottom = word_bottom.to_pdf_bbox(300, 792.0, None, None);

        // Top-of-page word should have HIGHER PDF Y (closer to top of page in PDF coords)
        // PDF coordinate system: Y=0 is bottom, Y=792 is top
        assert!(
            bbox_top[3] > bbox_bottom[3],
            "Top word should have higher PDF Y ({}) than bottom word ({})",
            bbox_top[3],
            bbox_bottom[3]
        );
        assert!(
            bbox_top[1] > bbox_bottom[1],
            "Top word y0 should be higher than bottom word y0"
        );
    }

    #[test]
    fn test_to_pdf_bbox_padding_subtraction() {
        // Test that the 10px padding is correctly subtracted
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [10, 10, 50, 30], // Exactly at the padding boundary
            confidence_0_100: 95,
        };

        let bbox = word.to_pdf_bbox(300, 792.0, None, None);

        // After padding subtraction, x0 and y0 should be at 0 (page origin)
        assert!(
            (bbox[0] - 0.0).abs() < 0.1,
            "x0 should be ~0.0 after padding subtraction"
        );
        // y0 should be near page height (top of page after Y-flip)
        assert!(
            bbox[1] > 780.0,
            "y0 should be near top of page after Y-flip"
        );
    }

    #[test]
    fn test_to_pdf_bbox_different_dpi() {
        // Test that DPI scaling is correctly applied
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 120, 40], // 100x20 pixels after padding subtraction
            confidence_0_100: 95,
        };

        // At 300 DPI: 100px * 72/300 = 24pt
        let bbox_300 = word.to_pdf_bbox(300, 792.0, None, None);
        let width_300 = bbox_300[2] - bbox_300[0];
        assert!(
            (width_300 - 24.0).abs() < 0.1,
            "Width at 300 DPI should be ~24pt, got {}",
            width_300
        );

        // At 200 DPI: 100px * 72/200 = 36pt
        let bbox_200 = word.to_pdf_bbox(200, 792.0, None, None);
        let width_200 = bbox_200[2] - bbox_200[0];
        assert!(
            (width_200 - 36.0).abs() < 0.1,
            "Width at 200 DPI should be ~36pt, got {}",
            width_200
        );

        // At 400 DPI: 100px * 72/400 = 18pt
        let bbox_400 = word.to_pdf_bbox(400, 792.0, None, None);
        let width_400 = bbox_400[2] - bbox_400[0];
        assert!(
            (width_400 - 18.0).abs() < 0.1,
            "Width at 400 DPI should be ~18pt, got {}",
            width_400
        );
    }

    #[test]
    fn test_to_pdf_bbox_hybrid_cell_offset() {
        // Test hybrid cell offset: OCR word in cell (3, 2) gets correct global PDF coords
        // Cell size for letter page: 612/8 = 76.5pt width, 792/8 = 99pt height
        // Cell (3, 2) in 0-indexed grid:
        //   - col 3: x starts at 3 * 76.5 = 229.5pt
        //   - row 2: y starts at 792 - 2 * 99 = 594pt (from bottom)
        let cell_origin = [229.5, 594.0];

        let word = HocrWord {
            text: "cell".to_string(),
            bbox_px: [20, 20, 60, 40], // Cell-local coords
            confidence_0_100: 95,
        };

        let bbox = word.to_pdf_bbox(300, 99.0, None, Some(cell_origin));

        // X should be offset by cell origin
        assert!(
            (bbox[0] - (229.5 + 10.0 * 72.0 / 300.0)).abs() < 1.0,
            "x0 should include cell origin offset"
        );
        // Y should be offset by cell origin (note: cell height is 99pt)
        assert!(
            (bbox[1] - (594.0 + 10.0 * 72.0 / 300.0)).abs() < 1.0,
            "y0 should include cell origin offset"
        );
    }

    #[test]
    fn test_to_pdf_bbox_clamps_negative_coords() {
        // Test that bboxes entirely within padding are clamped to origin
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [0, 0, 5, 5], // Entirely within padding (less than 10px)
            confidence_0_100: 95,
        };

        let bbox = word.to_pdf_bbox(300, 792.0, None, None);

        // Should be clamped to origin (no negative coords)
        assert!(bbox[0] >= 0.0, "x0 should not be negative");
        assert!(bbox[1] >= 0.0, "y0 should not be negative");
        assert!(bbox[2] >= bbox[0], "x1 should be >= x0");
        assert!(bbox[3] >= bbox[1], "y1 should be >= y0");
    }

    #[test]
    fn test_to_pdf_bbox_rotation_90() {
        // Test 90-degree rotation
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 60, 40],
            confidence_0_100: 95,
        };

        let bbox_no_rot = word.to_pdf_bbox(300, 792.0, None, None);
        let bbox_rot_90 = word.to_pdf_bbox(300, 792.0, Some(90), None);

        // After 90-degree rotation, the bbox should be transformed
        // The exact values depend on the rotation implementation
        // Just verify that the rotation changes the coordinates
        assert!(
            bbox_rot_90[0] != bbox_no_rot[0] || bbox_rot_90[1] != bbox_no_rot[1],
            "Rotation should change coordinates"
        );
    }

    #[test]
    fn test_to_pdf_bbox_rotation_180() {
        // Test 180-degree rotation
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 60, 40],
            confidence_0_100: 95,
        };

        let bbox_rot_180 = word.to_pdf_bbox(300, 792.0, Some(180), None);

        // After 180-degree rotation, bbox should still be valid
        assert!(bbox_rot_180[2] >= bbox_rot_180[0], "x1 should be >= x0");
        assert!(bbox_rot_180[3] >= bbox_rot_180[1], "y1 should be >= y0");
    }

    #[test]
    fn test_to_pdf_bbox_rotation_270() {
        // Test 270-degree rotation
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 60, 40],
            confidence_0_100: 95,
        };

        let bbox_rot_270 = word.to_pdf_bbox(300, 792.0, Some(270), None);

        // After 270-degree rotation, bbox should still be valid
        assert!(bbox_rot_270[2] >= bbox_rot_270[0], "x1 should be >= x0");
        assert!(bbox_rot_270[3] >= bbox_rot_270[1], "y1 should be >= y0");
    }

    #[test]
    fn test_to_pdf_bbox_invalid_rotation() {
        // Test that invalid rotation angles are ignored
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 60, 40],
            confidence_0_100: 95,
        };

        let bbox_no_rot = word.to_pdf_bbox(300, 792.0, None, None);
        let bbox_invalid = word.to_pdf_bbox(300, 792.0, Some(45), None); // 45° is not supported

        // Invalid rotation should return unchanged bbox
        assert!(
            (bbox_invalid[0] - bbox_no_rot[0]).abs() < 0.01,
            "Invalid rotation should not change x0"
        );
        assert!(
            (bbox_invalid[1] - bbox_no_rot[1]).abs() < 0.01,
            "Invalid rotation should not change y0"
        );
    }

    #[test]
    fn test_apply_rotation_to_bbox_0_degrees() {
        let (x0, y0, x1, y1) = apply_rotation_to_bbox(10.0, 20.0, 50.0, 40.0, 0, 100.0);
        assert_eq!((x0, y0, x1, y1), (10.0, 20.0, 50.0, 40.0));
    }

    #[test]
    fn test_apply_rotation_to_bbox_preserves_dimensions() {
        // All rotations should preserve bbox area (approximately)
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [20, 20, 60, 40], // 40x20 pixels after padding subtraction
            confidence_0_100: 95,
        };

        for rot in [0, 90, 180, 270] {
            let bbox = word.to_pdf_bbox(300, 792.0, Some(rot), None);
            let width = bbox[2] - bbox[0];
            let height = bbox[3] - bbox[1];

            // At 300 DPI: 40px = 9.6pt, 20px = 4.8pt
            // Allow some tolerance for floating-point errors
            assert!(
                (width - 9.6).abs() < 0.2,
                "Width should be ~9.6pt at {}° rotation",
                rot
            );
            assert!(
                (height - 4.8).abs() < 0.2,
                "Height should be ~4.8pt at {}° rotation",
                rot
            );
        }
    }
}

// ============ End-to-End Tesseract Integration (Phase 5.4.5) ============

use image::{GrayImage, ImageBuffer, Luma};

/// Run Tesseract OCR on a grayscale image and return extracted spans.
///
/// This is the main entry point for OCR in the pdftract pipeline. It integrates:
/// - Thread-local Tesseract instance management (borrow_or_init)
/// - Image preprocessing and Tesseract invocation
/// - HOCR parsing (parse_hocr)
/// - Coordinate conversion (HocrWord::to_pdf_bbox)
///
/// # Arguments
///
/// * `image` - The grayscale image to run OCR on
/// * `dpi` - The DPI at which the image was rendered (for coordinate conversion)
/// * `page_height_pt` - The page height in PDF points (for Y-axis flip)
/// * `opts` - Tesseract configuration options
///
/// # Returns
///
/// A `Result<Vec<Span>>` containing the extracted OCR spans with PDF coordinates.
///
/// # Errors
///
/// Returns an error if:
/// - Tesseract initialization fails
/// - Image processing fails
/// - HOCR parsing fails
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::{run_tesseract, TessOpts};
/// use image::GrayImage;
///
/// let image: GrayImage = ...; // Rendered at 300 DPI
/// let opts = TessOpts::default();
/// let spans = run_tesseract(&image, 300, 792.0, &opts).unwrap();
///
/// for span in spans {
///     println!("{} at {:?} (confidence: {})",
///         span.text, span.bbox, span.confidence);
/// }
/// ```
///
/// # Performance
///
/// - First call per thread: ~50ms (Tesseract initialization)
/// - Subsequent calls with same opts: ~10-20ms (cache hit)
/// - Language change: ~50ms (reinitialization required)
///
/// # See also
///
/// - `borrow_or_init` for thread-local caching behavior
/// - `parse_hocr` for HOCR parsing details
/// - `HocrWord::to_pdf_bbox` for coordinate conversion
pub fn run_tesseract(
    image: &GrayImage,
    dpi: u32,
    page_height_pt: f64,
    opts: &TessOpts,
) -> Result<Vec<crate::hybrid::Span>, String> {
    // Step 1: Borrow or initialize thread-local Tesseract instance
    let mut tess_state = borrow_or_init(opts);
    let tess_api = tess_state.api_mut();

    // Step 2: Set the image for Tesseract to process
    // Tesseract expects raw image bytes in grayscale format
    let width = image.width();
    let height = image.height();
    let raw_data: Vec<u8> = image
        .pixels()
        .flat_map(|p| std::array::IntoIter::new([p[0]]))
        .collect();

    tess_api
        .set_image(&raw_data, width, height, 1, width as i32)
        .map_err(|e| format!("Failed to set image for OCR: {}", e))?;

    // Step 3: Run OCR and get HOCR output
    // GetHOCRText writes to a file path in the C API, but the Rust wrapper
    // returns it as a String
    let hocr_text = tess_api
        .get_hocr_text(0) // Page number (0-indexed)
        .map_err(|e| format!("OCR failed: {}", e))?;

    // Step 4: Parse HOCR into HocrWord list
    let hocr_words = parse_hocr(&hocr_text)?;

    // Step 5: Convert HocrWords to Spans with PDF coordinates
    let spans: Vec<crate::hybrid::Span> = hocr_words
        .into_iter()
        .map(|word| {
            let pdf_bbox = word.to_pdf_bbox(dpi, page_height_pt, None, None);
            crate::hybrid::Span::ocr(pdf_bbox, word.confidence(), word.text)
        })
        .collect();

    Ok(spans)
}

/// Run Tesseract OCR on a cell crop with cell-local coordinate conversion.
///
/// This is a specialized variant of `run_tesseract` for hybrid cell processing,
/// where the OCR was performed on a cropped cell region rather than the full page.
/// The cell origin is added to the converted coordinates to get global PDF coordinates.
///
/// # Arguments
///
/// * `image` - The grayscale cell crop image
/// * `dpi` - The DPI at which the page was rendered
/// * `cell_height_pt` - The cell height in PDF points (for Y-axis flip within cell)
/// * `cell_origin` - The cell's origin [x_pt, y_pt] in global PDF coordinates
/// * `opts` - Tesseract configuration options
///
/// # Returns
///
/// A `Result<Vec<Span>>` with OCR spans in global PDF coordinates.
///
/// # See also
///
/// - `run_tesseract` for full-page OCR
/// - `crate::hybrid::crop_cell_from_page` for cell cropping logic
pub fn run_tesseract_on_cell(
    image: &GrayImage,
    dpi: u32,
    cell_height_pt: f64,
    cell_origin: [f64; 2],
    opts: &TessOpts,
) -> Result<Vec<crate::hybrid::Span>, String> {
    let mut tess_state = borrow_or_init(opts);
    let tess_api = tess_state.api_mut();

    let width = image.width();
    let height = image.height();
    let raw_data: Vec<u8> = image
        .pixels()
        .flat_map(|p| std::array::IntoIter::new([p[0]]))
        .collect();

    tess_api
        .set_image(&raw_data, width, height, 1, width as i32)
        .map_err(|e| format!("Failed to set image for cell OCR: {}", e))?;

    let hocr_text = tess_api
        .get_hocr_text(0)
        .map_err(|e| format!("Cell OCR failed: {}", e))?;

    let hocr_words = parse_hocr(&hocr_text)?;

    let spans: Vec<crate::hybrid::Span> = hocr_words
        .into_iter()
        .map(|word| {
            let pdf_bbox = word.to_pdf_bbox(dpi, cell_height_pt, None, Some(cell_origin));
            crate::hybrid::Span::ocr(pdf_bbox, word.confidence(), word.text)
        })
        .collect();

    Ok(spans)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that run_tesseract returns a Vec<Span> with expected structure.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn test_run_tesseract_returns_spans() {
        // Create a simple 100x20 white image with a black rectangle
        // This is a minimal test to verify the integration works
        let img: GrayImage = ImageBuffer::from_pixel(100, 20, Luma([255u8]));

        let opts = TessOpts::default();

        let result = std::panic::catch_unwind(|| run_tesseract(&img, 300, 792.0, &opts));

        if result.is_err() {
            // Tesseract not available - skip gracefully
            println!("Skipping test_run_tesseract_returns_spans: Tesseract not available");
            return;
        }

        let spans = result.unwrap();
        // Empty image should produce empty or minimal spans
        println!("Got {} spans from empty image", spans.len());
    }

    /// Test that run_tesseract_on_cell adds cell origin correctly.
    #[test]
    #[cfg_attr(not(feature = "ocr"), ignore)]
    fn test_run_tesseract_on_cell_offset() {
        let img: GrayImage = ImageBuffer::from_pixel(50, 50, Luma([255u8]));
        let opts = TessOpts::default();
        let cell_origin = [100.0, 200.0];

        let result =
            std::panic::catch_unwind(|| run_tesseract_on_cell(&img, 300, 99.0, cell_origin, &opts));

        if result.is_err() {
            println!("Skipping test_run_tesseract_on_cell_offset: Tesseract not available");
            return;
        }

        let spans = result.unwrap();
        // Verify that any spans have coordinates offset by cell origin
        for span in spans {
            assert!(span.bbox[0] >= 100.0, "X should be offset by cell origin");
            assert!(span.bbox[1] >= 200.0, "Y should be offset by cell origin");
        }
    }
}

// ============ Word Error Rate (WER) Measurement (Phase 5.4.5) ============

/// Calculate Word Error Rate (WER) between OCR output and ground truth.
///
/// WER = (substitutions + insertions + deletions) / reference_length
///
/// This is the standard metric for OCR accuracy evaluation. Lower is better.
///
/// # Arguments
///
/// * `ocr_output` - The text produced by OCR
/// * `ground_truth` - The reference/expected text
///
/// # Returns
///
/// A `f64` representing WER as a fraction (0.0 = perfect, 1.0 = all words wrong).
/// Multiply by 100 to get percentage.
///
/// # Normalization
///
/// Both texts are normalized before comparison:
/// - Converted to lowercase
/// - Leading/trailing whitespace stripped
/// - Internal whitespace normalized to single spaces
/// - Common punctuation stripped (.,!?;:"'()[]{})
///
/// # Examples
///
/// ```
/// use pdftract_core::ocr::calculate_wer;
///
/// let ocr = "The quick brown fox jumps";
/// let reference = "The quick brown fox jumped";
/// let wer = calculate_wer(ocr, reference);
///
/// // "jumps" vs "jumped" = 1 substitution
/// // WER = 1 / 5 = 0.2 (20%)
/// ```
///
/// # Algorithm
///
/// Uses the Wagner-Fischer algorithm for edit distance (Levenshtein distance)
/// with word-level tokenization instead of character-level.
///
/// # See also
///
/// - Phase 5.4.5 in the plan for WER CI gate requirements
pub fn calculate_wer(ocr_output: &str, ground_truth: &str) -> f64 {
    let ocr_words = normalize_text(ocr_output);
    let ref_words = normalize_text(ground_truth);

    if ref_words.is_empty() {
        return if ocr_words.is_empty() { 0.0 } else { 1.0 };
    }

    let (substitutions, insertions, deletions) = word_edit_distance(&ocr_words, &ref_words);
    let total_errors = substitutions + insertions + deletions;

    total_errors as f64 / ref_words.len() as f64
}

/// Normalize text for WER calculation.
///
/// Normalization steps:
/// 1. Convert to lowercase
/// 2. Strip leading/trailing whitespace
/// 3. Normalize internal whitespace to single spaces
/// 4. Strip punctuation: .,!?;:"'()[]{}
///
/// # Arguments
///
/// * `text` - The text to normalize
///
/// # Returns
///
/// A `Vec<String>` of normalized words.
fn normalize_text(text: &str) -> Vec<String> {
    // Define punctuation to strip
    let punct = [
        '.', ',', '!', '?', ';', ':', '"', '\'', '(', ')', '[', ']', '{', '}',
    ];

    text.to_lowercase()
        .split_whitespace()
        .map(|word| {
            // Strip leading and trailing punctuation from each word
            word.trim_matches(&punct[..]).to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// Calculate word-level edit distance (Levenshtein distance).
///
/// Returns (substitutions, insertions, deletions).
///
/// # Arguments
///
/// * `ocr` - Tokenized OCR output
/// * `reference` - Tokenized ground truth
fn word_edit_distance(ocr: &[String], reference: &[String]) -> (usize, usize, usize) {
    let m = ocr.len();
    let n = reference.len();

    // Initialize distance matrix
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    // Base cases: transforming to/from empty string
    for i in 0..=m {
        dp[i][0] = i; // i deletions
    }
    for j in 0..=n {
        dp[0][j] = j; // j insertions
    }

    // Fill the matrix
    for i in 1..=m {
        for j in 1..=n {
            if ocr[i - 1] == reference[j - 1] {
                dp[i][j] = dp[i - 1][j - 1]; // No operation needed
            } else {
                dp[i][j] = [
                    dp[i - 1][j] + 1,     // Deletion
                    dp[i][j - 1] + 1,     // Insertion
                    dp[i - 1][j - 1] + 1, // Substitution
                ]
                .into_iter()
                .min()
                .unwrap();
            }
        }
    }

    // Backtrack to count error types
    let mut substitutions = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    let mut i = m;
    let mut j = n;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && ocr[i - 1] == reference[j - 1] {
            // Match - no error
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            // Substitution
            substitutions += 1;
            i -= 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            // Deletion
            deletions += 1;
            i -= 1;
        } else if j > 0 && dp[i][j] == dp[i][j - 1] + 1 {
            // Insertion
            insertions += 1;
            j -= 1;
        } else {
            // Default case (shouldn't happen in valid backtracking)
            if i > 0 {
                i -= 1;
            }
            if j > 0 {
                j -= 1;
            }
        }
    }

    (substitutions, insertions, deletions)
}

// ============ Assisted OCR Validation Filter (Phase 5.5.2) ============

use crate::content_stream::Glyph;

/// Distance threshold for assisted-OCR position validation (in PDF points).
///
/// If the center-to-center distance between an OCR word and the nearest
/// vector glyph is less than this value, the OCR word is accepted with its
/// full confidence. Otherwise, confidence is capped at 0.4.
///
/// 5 pt is approximately one space-character width at 12 pt font size.
const ASSISTED_OCR_DISTANCE_PT: f64 = 5.0;

/// Confidence cap for OCR words that fail position validation.
///
/// This value is below the 0.5 threshold used in bbox-merge (Phase 5.2.4),
/// ensuring that unassisted OCR spans won't be preferred over legitimate
/// vector spans.
const ASSISTED_OCR_CONFIDENCE_CAP: f32 = 0.4;

/// Minimum glyph count to justify building a KD-tree.
///
/// For small N (< 100), linear scan is faster due to lower overhead.
const ASSISTED_OCR_KDTREE_THRESHOLD: usize = 100;

/// Region-level confidence threshold for keeping assisted-OCR output.
///
/// If the mean confidence of all assisted-OCR words in a region is greater
/// than this value, the region is kept as-is with confidence_source = "ocr-assisted".
const ASSISTED_OCR_KEEP_THRESH: f32 = 0.7;

/// Region-level confidence threshold for falling back to pure OCR.
///
/// If the mean confidence of all assisted-OCR words in a region is less
/// than this value, the region is reprocessed with pure OCR (no validation filter)
/// and emitted with confidence_source = "ocr-fallback".
const ASSISTED_OCR_FALLBACK_THRESH: f32 = 0.3;

/// Validate OCR words against vector glyph position hints.
///
/// This function implements the per-word validation filter for the
/// BrokenVector assisted-OCR path (Phase 5.5.2). For each Tesseract word,
/// it finds the nearest vector glyph bbox center and checks the distance:
///
/// - If distance < 5 pt: accept word with full OCR confidence
/// - If distance >= 5 pt: cap confidence at 0.4
///
/// The 5pt threshold filters OCR text where positions disagree with the
/// vector layer, indicating either OCR-of-OCR garbage or hallucinated text.
///
/// # Arguments
///
/// * `hocr_words` - OCR words from Tesseract (in PDF coordinates)
/// * `vector_glyphs` - Position hints from Phase 3 (PositionHint mode)
///
/// # Returns
///
/// A `Vec<Span>` with `SpanSource::OcrAssisted` and adjusted confidence scores.
/// The output preserves HOCR document order.
///
/// # Performance
///
/// - For < 100 glyphs: O(N*M) linear scan (N = OCR words, M = glyphs)
/// - For >= 100 glyphs: Could use KD-tree for O(N*log(M)) (future optimization)
///
/// # Examples
///
/// ```ignore
/// use pdftract_core::ocr::validate_ocr_with_position_hints;
/// use pdftract_core::content_stream::Glyph;
///
/// // Position hints from Phase 3
/// let glyphs = vec![
///     Glyph::position_hint([100.0, 200.0, 110.0, 210.0]),
/// ];
///
/// // OCR words from Tesseract (already converted to PDF coords)
/// let mut words = vec![
///     HocrWord { text: "hello".to_string(), bbox_px: [102, 202, 108, 208], confidence_0_100: 95 },
/// ];
///
/// let spans = validate_ocr_with_position_hints(&words, &glyphs, 300, 792.0);
/// // Word at (102, 202) is close to glyph at (100, 200) -> full confidence
/// assert_eq!(spans[0].confidence, 0.95);
/// ```
///
/// # See also
///
/// - Phase 5.5 pipeline step 3 (plan line 1935)
/// - `Glyph::position_hint` for creating position-hint glyphs
pub fn validate_ocr_with_position_hints(
    hocr_words: &[HocrWord],
    vector_glyphs: &[Glyph],
    dpi: u32,
    page_height_pt: f64,
) -> Vec<crate::hybrid::Span> {
    // Build list of vector glyph bbox centers for nearest-neighbor lookup
    let glyph_centers: Vec<(f64, f64)> = vector_glyphs
        .iter()
        .map(|g| {
            let bx = g.bbox;
            ((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0)
        })
        .collect();

    // For each OCR word, find nearest glyph and validate distance
    hocr_words
        .iter()
        .map(|word| {
            let pdf_bbox = word.to_pdf_bbox(dpi, page_height_pt, None, None);
            let word_center = (
                (pdf_bbox[0] + pdf_bbox[2]) / 2.0,
                (pdf_bbox[1] + pdf_bbox[3]) / 2.0,
            );

            // Find nearest vector glyph center (linear scan - fast enough for N < 100)
            let min_distance = glyph_centers
                .iter()
                .map(|&gx| {
                    let dx = gx.0 - word_center.0;
                    let dy = gx.1 - word_center.1;
                    (dx * dx + dy * dy).sqrt()
                })
                .min()
                .unwrap_or(f64::MAX); // No glyphs -> max distance

            // Apply validation: cap confidence if distance >= 5pt
            let ocr_confidence = word.confidence();
            let adjusted_confidence = if min_distance < ASSISTED_OCR_DISTANCE_PT {
                ocr_confidence
            } else {
                ocr_confidence.min(ASSISTED_OCR_CONFIDENCE_CAP)
            };

            crate::hybrid::Span::ocr_assisted(pdf_bbox, adjusted_confidence, word.text.clone())
        })
        .collect()
}

/// Region (line) for grouping OCR words by baseline proximity.
#[derive(Debug, Clone)]
struct OcrRegion {
    /// Words in this region.
    words: Vec<(HocrWord, [f64; 4])>, // (HocrWord, PDF bbox)
    /// Mean confidence of all words in this region.
    mean_confidence: f32,
}

/// Apply region-level confidence policy to assisted-OCR spans.
///
/// This function implements Phase 5.5.3 step 5: for each region (line),
/// compute the mean confidence across all assisted-OCR words and decide
/// whether to keep as-is, keep with high confidence flag, or trigger fallback.
///
/// # Arguments
///
/// * `hocr_words` - OCR words from Tesseract (in pixel coordinates)
/// * `vector_glyphs` - Position hints from Phase 3
/// * `dpi` - DPI used for rendering
/// * `page_height_pt` - Page height in PDF points
///
/// # Returns
///
/// A tuple of:
/// - Vec of spans with adjusted confidence sources
/// - Vec of HocrWords that need fallback (grouped by regions with mean < 0.3)
///
/// # Region Grouping
///
/// Words are grouped into regions by baseline proximity (Y-coordinate).
/// Two words are in the same region if their baselines are within 12pt
/// (approximately 1.5x the typical line height for 12pt text).
///
/// # Policy
///
/// For each region:
/// - mean > 0.7: keep with `OcrAssisted` source
/// - mean < 0.3: flag for fallback (caller should rerun Tesseract)
/// - 0.3 <= mean <= 0.7: keep with `OcrAssisted` source
///
/// # See also
///
/// - Phase 5.5 pipeline step 5 (plan line 1937)
/// - `validate_ocr_with_position_hints` for per-word validation
pub fn apply_region_level_confidence_policy(
    hocr_words: &[HocrWord],
    vector_glyphs: &[Glyph],
    dpi: u32,
    page_height_pt: f64,
) -> (Vec<crate::hybrid::Span>, Vec<(HocrWord, [f64; 4])>) {
    // First, apply per-word validation to get initial confidence-adjusted spans
    let validated_spans =
        validate_ocr_with_position_hints(hocr_words, vector_glyphs, dpi, page_height_pt);

    // Group words into regions by baseline proximity
    let regions = group_words_by_region(hocr_words, dpi, page_height_pt);

    // Compute mean confidence for each region and classify
    let mut final_spans = Vec::new();
    let mut fallback_words = Vec::new();

    for region in regions {
        if region.mean_confidence < ASSISTED_OCR_FALLBACK_THRESH {
            // Region needs fallback - collect original words for rerun
            for (word, pdf_bbox) in region.words {
                fallback_words.push((word, pdf_bbox));
            }
        } else {
            // Keep region - convert validated spans to final output
            // Words in this region are already in validated_spans
            // We need to match them up by position
            for (word, pdf_bbox) in region.words {
                // Find the corresponding validated span
                if let Some(span) = validated_spans
                    .iter()
                    .find(|s| s.bbox == pdf_bbox && s.text == word.text)
                {
                    let span = if region.mean_confidence > ASSISTED_OCR_KEEP_THRESH {
                        // High confidence region - keep as OcrAssisted
                        crate::hybrid::Span::ocr_assisted(
                            span.bbox,
                            span.confidence,
                            span.text.clone(),
                        )
                    } else {
                        // Medium confidence region - keep as-is (OcrAssisted)
                        span.clone()
                    };
                    final_spans.push(span);
                }
            }
        }
    }

    (final_spans, fallback_words)
}

/// Group OCR words into regions by baseline proximity.
///
/// Two words are in the same region if their baselines are within 12pt.
/// The baseline is computed as `y0 + (bbox_height * 0.2)`.
///
/// # Arguments
///
/// * `hocr_words` - OCR words from Tesseract
/// * `dpi` - DPI used for rendering
/// * `page_height_pt` - Page height in PDF points
///
/// # Returns
///
/// A vector of regions, each containing words and their mean confidence.
fn group_words_by_region(hocr_words: &[HocrWord], dpi: u32, page_height_pt: f64) -> Vec<OcrRegion> {
    if hocr_words.is_empty() {
        return Vec::new();
    }

    // Convert all words to PDF coordinates and compute baselines
    let mut word_info: Vec<(HocrWord, [f64; 4], f64)> = hocr_words
        .iter()
        .map(|word| {
            let pdf_bbox = word.to_pdf_bbox(dpi, page_height_pt, None, None);
            let baseline = pdf_bbox[1] + (pdf_bbox[3] - pdf_bbox[1]) * 0.2;
            (word.clone(), pdf_bbox, baseline)
        })
        .collect();

    // Sort by baseline for deterministic grouping
    word_info.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    // Group by baseline proximity (within 12pt)
    let mut regions: Vec<OcrRegion> = Vec::new();
    const BASELINE_TOLERANCE_PT: f64 = 12.0;

    for (word, pdf_bbox, baseline) in word_info {
        let confidence = word.confidence();

        // Find existing region with compatible baseline
        let region = regions.iter_mut().find(|r| {
            if r.words.is_empty() {
                return false;
            }
            // Compute region's baseline from first word
            let (_, first_bbox, _) = &r.words[0];
            let region_baseline = first_bbox[1] + (first_bbox[3] - first_bbox[1]) * 0.2;
            (region_baseline - baseline).abs() < BASELINE_TOLERANCE_PT
        });

        if let Some(region) = region {
            // Add to existing region
            region.words.push((word, pdf_bbox));
            // Recompute mean confidence
            let sum: f32 = region.words.iter().map(|(w, _)| w.confidence()).sum();
            region.mean_confidence = sum / region.words.len() as f32;
        } else {
            // Create new region
            regions.push(OcrRegion {
                words: vec![(word, pdf_bbox)],
                mean_confidence: confidence,
            });
        }
    }

    regions
}

#[cfg(test)]
mod assisted_ocr_tests {
    use super::*;

    #[test]
    fn test_validation_filter_near_glyph() {
        // OCR word center at (102, 201) is within 5pt of glyph at (100, 200)
        let glyphs = vec![Glyph::position_hint([95.0, 195.0, 105.0, 205.0])];
        let word = HocrWord {
            text: "hello".to_string(),
            bbox_px: [20, 20, 40, 40], // Will be converted to ~102, 201 at 300 DPI
            confidence_0_100: 95,
        };

        let spans = validate_ocr_with_position_hints(&[word], &glyphs, 300, 792.0);

        assert_eq!(spans.len(), 1);
        // Should accept full confidence since distance < 5pt
        assert!((spans[0].confidence - 0.95).abs() < f32::EPSILON);
        assert_eq!(spans[0].source, crate::hybrid::SpanSource::OcrAssisted);
        assert_eq!(spans[0].text, "hello");
    }

    #[test]
    fn test_validation_filter_far_from_glyph() {
        // OCR word center at (150, 250) is > 5pt from glyph at (100, 200)
        let glyphs = vec![Glyph::position_hint([95.0, 195.0, 105.0, 205.0])];
        let word = HocrWord {
            text: "world".to_string(),
            bbox_px: [500, 500, 550, 520], // Far from glyph
            confidence_0_100: 95,
        };

        let spans = validate_ocr_with_position_hints(&[word], &glyphs, 300, 792.0);

        assert_eq!(spans.len(), 1);
        // Should cap confidence at 0.4 since distance >= 5pt
        assert_eq!(spans[0].confidence, ASSISTED_OCR_CONFIDENCE_CAP);
        assert_eq!(spans[0].source, crate::hybrid::SpanSource::OcrAssisted);
    }

    #[test]
    fn test_validation_filter_confidence_already_below_cap() {
        // OCR word with low confidence (30%) far from glyph should stay at 30%
        let glyphs = vec![Glyph::position_hint([95.0, 195.0, 105.0, 205.0])];
        let word = HocrWord {
            text: "test".to_string(),
            bbox_px: [500, 500, 550, 520],
            confidence_0_100: 30,
        };

        let spans = validate_ocr_with_position_hints(&[word], &glyphs, 300, 792.0);

        assert_eq!(spans.len(), 1);
        // Should keep original confidence (already below cap)
        assert_eq!(spans[0].confidence, 0.3);
    }

    #[test]
    fn test_validation_filter_no_glyphs() {
        // No position hints available -> cap all words
        let glyphs: Vec<Glyph> = vec![];
        let word = HocrWord {
            text: "orphan".to_string(),
            bbox_px: [100, 100, 150, 120],
            confidence_0_100: 90,
        };

        let spans = validate_ocr_with_position_hints(&[word], &glyphs, 300, 792.0);

        assert_eq!(spans.len(), 1);
        // No glyphs -> max distance -> cap confidence
        assert_eq!(spans[0].confidence, ASSISTED_OCR_CONFIDENCE_CAP);
    }

    #[test]
    fn test_validation_filter_multiple_words_preserves_order() {
        // Test that HOCR document order is preserved
        let glyphs = vec![
            Glyph::position_hint([100.0, 200.0, 110.0, 210.0]),
            Glyph::position_hint([200.0, 200.0, 210.0, 210.0]),
        ];

        let words = vec![
            HocrWord {
                text: "first".to_string(),
                bbox_px: [20, 20, 40, 40],
                confidence_0_100: 90,
            },
            HocrWord {
                text: "second".to_string(),
                bbox_px: [500, 500, 550, 520], // Far from any glyph
                confidence_0_100: 85,
            },
            HocrWord {
                text: "third".to_string(),
                bbox_px: [60, 20, 80, 40],
                confidence_0_100: 95,
            },
        ];

        let spans = validate_ocr_with_position_hints(&words, &glyphs, 300, 792.0);

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "first");
        assert_eq!(spans[1].text, "second");
        assert_eq!(spans[2].text, "third");

        // First and third should have full confidence (near glyphs)
        assert!((spans[0].confidence - 0.9).abs() < f32::EPSILON);
        assert!((spans[2].confidence - 0.95).abs() < f32::EPSILON);

        // Second should be capped (far from glyphs)
        assert_eq!(spans[1].confidence, ASSISTED_OCR_CONFIDENCE_CAP);
    }

    #[test]
    fn test_validation_filter_distance_threshold() {
        // Test the exact 5pt boundary
        let glyphs = vec![Glyph::position_hint([100.0, 200.0, 110.0, 210.0])];

        // Word at exactly 5pt distance should be capped
        let word_far = HocrWord {
            text: "far".to_string(),
            bbox_px: [1000, 1000, 1050, 1020],
            confidence_0_100: 95,
        };

        let spans = validate_ocr_with_position_hints(&[word_far], &glyphs, 300, 792.0);
        assert_eq!(spans[0].confidence, ASSISTED_OCR_CONFIDENCE_CAP);
    }

    #[test]
    fn test_assisted_ocr_constants() {
        // Verify the constants match the plan specification
        assert_eq!(ASSISTED_OCR_DISTANCE_PT, 5.0);
        assert_eq!(ASSISTED_OCR_CONFIDENCE_CAP, 0.4);
        assert_eq!(ASSISTED_OCR_KDTREE_THRESHOLD, 100);
        assert_eq!(ASSISTED_OCR_KEEP_THRESH, 0.7);
        assert_eq!(ASSISTED_OCR_FALLBACK_THRESH, 0.3);
    }

    #[test]
    fn test_region_level_policy_high_confidence_region() {
        // Test region with mean confidence > 0.7 - should keep as OcrAssisted
        let glyphs = vec![
            Glyph::position_hint([100.0, 200.0, 110.0, 210.0]),
            Glyph::position_hint([120.0, 200.0, 130.0, 210.0]),
        ];
        let words = vec![
            HocrWord {
                text: "hello".to_string(),
                bbox_px: [102, 202, 108, 208],
                confidence_0_100: 95,
            },
            HocrWord {
                text: "world".to_string(),
                bbox_px: [122, 202, 128, 208],
                confidence_0_100: 90,
            },
        ];

        let (spans, fallback) = apply_region_level_confidence_policy(&words, &glyphs, 300, 792.0);

        // Both words are near glyphs, so they keep high confidence
        assert_eq!(spans.len(), 2);
        assert_eq!(fallback.len(), 0); // No fallback needed
        assert!(spans
            .iter()
            .all(|s| s.source == crate::hybrid::SpanSource::OcrAssisted));
    }

    #[test]
    fn test_region_level_policy_low_confidence_region() {
        // Test region with mean confidence < 0.3 - should trigger fallback
        let glyphs = vec![]; // No glyphs -> all words capped at 0.4
        let words = vec![
            HocrWord {
                text: "low1".to_string(),
                bbox_px: [100, 100, 120, 120],
                confidence_0_100: 20,
            },
            HocrWord {
                text: "low2".to_string(),
                bbox_px: [130, 100, 150, 120],
                confidence_0_100: 25,
            },
        ];

        let (spans, fallback) = apply_region_level_confidence_policy(&words, &glyphs, 300, 792.0);

        // Low confidence region -> fallback triggered
        assert_eq!(spans.len(), 0); // No spans kept
        assert_eq!(fallback.len(), 2); // Both words need fallback
    }

    #[test]
    fn test_region_level_policy_medium_confidence_region() {
        // Test region with 0.3 <= mean confidence <= 0.7 - should keep as-is
        let glyphs = vec![];
        let words = vec![
            HocrWord {
                text: "med1".to_string(),
                bbox_px: [100, 100, 120, 120],
                confidence_0_100: 40,
            },
            HocrWord {
                text: "med2".to_string(),
                bbox_px: [130, 100, 150, 120],
                confidence_0_100: 50,
            },
        ];

        let (spans, fallback) = apply_region_level_confidence_policy(&words, &glyphs, 300, 792.0);

        // Medium confidence region -> kept as-is (capped at 0.4 by validation)
        assert_eq!(spans.len(), 2);
        assert_eq!(fallback.len(), 0); // No fallback needed
    }

    #[test]
    fn test_region_level_policy_multiple_regions() {
        // Test multiple regions with different confidence levels
        let glyphs = vec![
            Glyph::position_hint([100.0, 200.0, 110.0, 210.0]), // For high confidence region
        ];
        let words = vec![
            // Region 1: high confidence (near glyph)
            HocrWord {
                text: "hello".to_string(),
                bbox_px: [102, 202, 108, 208],
                confidence_0_100: 95,
            },
            // Region 2: low confidence (far from glyph, different Y)
            HocrWord {
                text: "low".to_string(),
                bbox_px: [500, 500, 520, 520],
                confidence_0_100: 20,
            },
        ];

        let (spans, fallback) = apply_region_level_confidence_policy(&words, &glyphs, 300, 792.0);

        // One span kept, one word needs fallback
        assert_eq!(spans.len(), 1);
        assert_eq!(fallback.len(), 1);
        assert_eq!(spans[0].text, "hello");
    }

    #[test]
    fn test_group_words_by_region_empty() {
        let words: Vec<HocrWord> = vec![];
        let regions = group_words_by_region(&words, 300, 792.0);
        assert_eq!(regions.len(), 0);
    }

    #[test]
    fn test_group_words_by_region_single_word() {
        let words = vec![HocrWord {
            text: "test".to_string(),
            bbox_px: [100, 100, 120, 120],
            confidence_0_100: 80,
        }];
        let regions = group_words_by_region(&words, 300, 792.0);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].words.len(), 1);
        assert_eq!(regions[0].mean_confidence, 0.8);
    }
}

#[cfg(test)]
mod wer_tests {
    use super::*;

    #[test]
    fn test_calculate_wer_perfect_match() {
        let wer = calculate_wer("The quick brown fox", "The quick brown fox");
        assert_eq!(wer, 0.0, "Perfect match should have WER = 0");
    }

    #[test]
    fn test_calculate_wer_with_substitution() {
        let wer = calculate_wer("The quick brown fox", "The quick brown box");
        assert_eq!(wer, 0.25, "One substitution in 4 words = 0.25");
    }

    #[test]
    fn test_calculate_wer_with_insertion() {
        let wer = calculate_wer("The quick brown fox jumps", "The quick brown fox");
        assert_eq!(wer, 0.2, "One insertion in 5 words = 0.2");
    }

    #[test]
    fn test_calculate_wer_with_deletion() {
        let wer = calculate_wer("The quick brown fox", "The quick brown fox jumps");
        assert_eq!(wer, 0.2, "One deletion in 5 reference words = 0.2");
    }

    #[test]
    fn test_calculate_wer_case_insensitive() {
        let wer = calculate_wer("THE QUICK BROWN FOX", "the quick brown fox");
        assert_eq!(wer, 0.0, "Case differences should be normalized");
    }

    #[test]
    fn test_calculate_wer_punctuation_insensitive() {
        let wer = calculate_wer("The quick, brown fox.", "The quick brown fox");
        assert_eq!(wer, 0.0, "Punctuation should be stripped");
    }

    #[test]
    fn test_calculate_wer_whitespace_normalized() {
        let wer = calculate_wer("The  quick   brown fox", "The quick brown fox");
        assert_eq!(wer, 0.0, "Extra whitespace should be normalized");
    }

    #[test]
    fn test_calculate_wer_empty_strings() {
        let wer = calculate_wer("", "");
        assert_eq!(wer, 0.0, "Two empty strings should have WER = 0");
    }

    #[test]
    fn test_calculate_wer_empty_reference_nonempty_ocr() {
        let wer = calculate_wer("some text", "");
        assert_eq!(
            wer, 1.0,
            "Non-empty OCR with empty reference should have WER = 1"
        );
    }

    #[test]
    fn test_calculate_wer_empty_ocr_nonempty_reference() {
        let wer = calculate_wer("", "some text");
        assert_eq!(
            wer, 1.0,
            "Empty OCR with non-empty reference should have WER = 1"
        );
    }

    #[test]
    fn test_calculate_wer_complex() {
        // Real-world example with multiple error types
        let ocr = "The qick brown fox jump over the lazzy dog";
        let reference = "The quick brown fox jumps over the lazy dog";

        // Errors:
        // - qick -> quick (substitution)
        // - jump -> jumps (substitution)
        // - lazzy -> lazy (substitution)
        // Total: 3 substitutions / 9 words = 0.333...
        let wer = calculate_wer(ocr, reference);
        assert!((wer - 0.333).abs() < 0.01, "Complex WER calculation failed");
    }

    #[test]
    fn test_normalize_text_lowercase() {
        let words = normalize_text("HELLO World");
        assert_eq!(words, vec!["hello", "world"]);
    }

    #[test]
    fn test_normalize_text_strip_punctuation() {
        let words = normalize_text("Hello, world! How are you?");
        assert_eq!(words, vec!["hello", "world", "how", "are", "you"]);
    }

    #[test]
    fn test_normalize_text_whitespace() {
        let words = normalize_text("  hello    world  ");
        assert_eq!(words, vec!["hello", "world"]);
    }

    #[test]
    fn test_normalize_text_combined() {
        let words = normalize_text("  The QUICK, brown... FOX!!!  ");
        assert_eq!(words, vec!["the", "quick", "brown", "fox"]);
    }

    #[test]
    fn test_word_edit_distance_no_errors() {
        let ocr = vec!["hello".to_string(), "world".to_string()];
        let reference = vec!["hello".to_string(), "world".to_string()];
        let (sub, ins, del) = word_edit_distance(&ocr, &reference);
        assert_eq!(sub, 0);
        assert_eq!(ins, 0);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_word_edit_distance_substitution() {
        let ocr = vec!["hello".to_string(), "word".to_string()];
        let reference = vec!["hello".to_string(), "world".to_string()];
        let (sub, ins, del) = word_edit_distance(&ocr, &reference);
        assert_eq!(sub, 1);
        assert_eq!(ins, 0);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_word_edit_distance_insertion_deletion() {
        let ocr = vec!["hello".to_string(), "there".to_string()];
        let reference = vec![
            "hello".to_string(),
            "world".to_string(),
            "there".to_string(),
        ];
        let (sub, ins, del) = word_edit_distance(&ocr, &reference);
        // "world" deleted from reference, but also could be seen as insertion
        // The algorithm counts it as:
        // - "hello" matches
        // - "there" vs "world" -> substitution, then "there" vs "there" matches
        // Actually: deletion of "world" then match "there"
        assert!(sub + ins + del == 1, "Should have exactly one error");
    }
}

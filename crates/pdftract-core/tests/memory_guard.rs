//! Memory-guard test helper for allocation-sensitive tests.
//!
//! This module provides utilities to run code under bounded memory limits
//! and assert graceful failure (no OOM panic/abort). Use this helper for
//! tests that verify memory-bounded behavior, such as:
//!
//! - Parsing large PDF files with limited memory
//! - OCR operations on oversized images
//! - Cache eviction under memory pressure
//! - Stream decompression with size limits
//!
//! # Platform support
//!
//! - **Linux/macOS**: Full support via `rlimit` (POSIX resource limits)
//! - **Windows**: Not supported (Windows doesn't have per-thread memory limits)
//!   - Tests using `run_under_memory_limit` are automatically skipped on Windows
//!
//! # Usage convention
//!
//! Tag allocation-sensitive tests with `#[cfg_attr(not(target_os = "windows"), test)]`
//! and use `run_under_memory_limit` to verify graceful failure:
//!
//! ```rust
//! #[cfg_attr(not(target_os = "windows"), test)]
//! fn test_large_pdf_rejected_gracefully() {
//!     let result = run_under_memory_limit(
//!         100 * 1024 * 1024, // 100 MiB
//!         || {
//!             // Code that should fail gracefully when exceeding the limit
//!             parse_oversized_pdf()
//!         }
//!     );
//!
//!     // Should return an error, not panic or OOM
//!     assert!(result.is_err());
//! }
//! ```
//!
//! # Memory limit semantics
//!
//! - The limit applies to the **virtual memory size** of the process
//! - On Linux, this includes both heap and mmap'd regions
//! - When the limit is exceeded, allocation attempts fail with `std::alloc::Error`
//! - Well-behaved Rust code propagates this as `Err(...)` from `allocate` or `try_reserve`
//! - Code using `unwrap()` or `expect()` on allocations will panic (not OOM abort)
//!
//! # Best practices
//!
//! 1. **Set generous limits**: Start with 100-500 MiB to avoid false positives
//! 2. **Test graceful paths**: Verify `Err` returns, not panics
//! 3. **Document the limit**: Comment why the specific limit was chosen
//! 4. **Skip on unsupported platforms**: Use `#[cfg_attr(not(target_os = "windows"), test)]`


/// Result type for memory-guarded test execution.
pub type MemoryGuardResult<T> = Result<T, MemoryGuardError>;

/// Errors that can occur when running code under a memory limit.
#[derive(Debug)]
pub enum MemoryGuardError {
    /// Platform does not support memory limits (e.g., Windows).
    UnsupportedPlatform,
    /// Failed to set the memory limit (permission or system error).
    SetLimitFailed(String),
    /// The closure panicked during execution.
    Panic(String),
    /// The closure returned an error.
    ClosureError(String),
}

/// Run a closure under a bounded memory limit.
///
/// Sets the process virtual memory limit using POSIX `rlimit` (Linux/macOS),
/// executes the closure, then restores the original limit. If the closure
/// attempts to allocate beyond the limit, it will fail gracefully (panic
/// with allocation failure, not OOM abort).
///
/// # Parameters
///
/// - `limit_bytes`: Maximum virtual memory size in bytes
/// - `f`: Closure to execute under the limit
///
/// # Returns
///
/// - `Ok(T)`: Closure completed successfully
/// - `Err(MemoryGuardError)`: Platform unsupported, limit set failed, or closure panicked
///
/// # Platform behavior
///
/// - **Linux/macOS**: Sets `RLIMIT_AS` (address space limit). If the closure
///   exceeds this, allocations fail with `std::alloc::Error`.
/// - **Windows**: Returns `Err(MemoryGuardError::UnsupportedPlatform)`.
///
/// # Example
///
/// ```rust
/// let result = run_under_memory_limit(50 * 1024 * 1024, || {
///     // This allocation will fail gracefully
///     let mut v = Vec::new();
///     v.try_reserve(100_000_000).map_err(|e| e.to_string())
/// });
/// assert!(result.is_err());
/// ```
///
/// # Thread safety
///
/// This function sets the limit for the **entire process**, not just the
/// calling thread. Do not use this in multi-threaded tests where other
/// threads are allocating.
pub fn run_under_memory_limit<F, T>(limit_bytes: u64, f: F) -> MemoryGuardResult<T>
where
    F: std::panic::UnwindSafe + FnOnce() -> Result<T, String>,
{
    #[cfg(unix)]
    {
        // Get current limit
        let mut old_rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        unsafe {
            if libc::getrlimit(libc::RLIMIT_AS, &mut old_rlim) != 0 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                return Err(MemoryGuardError::SetLimitFailed(format!(
                    "getrlimit failed: errno {}",
                    errno
                )));
            }
        }

        // Set new limit
        let new_rlim = libc::rlimit {
            rlim_cur: limit_bytes,
            rlim_max: limit_bytes.max(old_rlim.rlim_max), // Don't reduce hard limit
        };

        unsafe {
            if libc::setrlimit(libc::RLIMIT_AS, &new_rlim) != 0 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                return Err(MemoryGuardError::SetLimitFailed(format!(
                    "setrlimit failed: errno {}",
                    errno
                )));
            }
        }

        // Execute closure with panic catching
        let result = std::panic::catch_unwind(f);

        // Restore original limit
        unsafe {
            let _ = libc::setrlimit(libc::RLIMIT_AS, &old_rlim);
        }

        match result {
            Ok(Ok(t)) => Ok(t),
            Ok(Err(e)) => Err(MemoryGuardError::ClosureError(e)),
            Err(_) => Err(MemoryGuardError::Panic("Closure panicked".to_string())),
        }
    }

    #[cfg(windows)]
    {
        let _ = limit_bytes;
        let _ = f;
        Err(MemoryGuardError::UnsupportedPlatform)
    }
}

/// Assert that an operation fails gracefully under memory pressure.
///
/// This is a convenience wrapper around `run_under_memory_limit` that
/// asserts the operation returns an error (not a panic).
///
/// # Parameters
///
/// - `limit_bytes`: Maximum virtual memory size in bytes
/// - `f`: Closure that should fail under the memory limit
///
/// # Panics
///
/// Panics if:
/// - The closure succeeds despite the limit
/// - The closure panics instead of returning an error
///
/// # Example
///
/// ```rust
/// assert_fails_under_memory_limit(10 * 1024 * 1024, || {
///     let mut data = Vec::new();
///     data.try_reserve(100_000_000).map_err(|e| e.to_string())?;
///     Ok::<_, String>(data)
/// });
/// ```
pub fn assert_fails_under_memory_limit<F, T>(limit_bytes: u64, f: F)
where
    F: std::panic::UnwindSafe + FnOnce() -> Result<T, String>,
{
    match run_under_memory_limit(limit_bytes, f) {
        Ok(_) => panic!("Operation succeeded despite memory limit"),
        Err(MemoryGuardError::ClosureError(_)) => {
            // Expected: operation failed gracefully
        }
        Err(MemoryGuardError::Panic(msg)) => {
            panic!("Operation panicked instead of failing gracefully: {}", msg);
        }
        Err(MemoryGuardError::UnsupportedPlatform) => {
            // Skip test silently on unsupported platforms
        }
        Err(MemoryGuardError::SetLimitFailed(msg)) => {
            panic!("Failed to set memory limit: {}", msg);
        }
    }
}

/// Assert that an operation succeeds within a memory budget.
///
/// This is the inverse of `assert_fails_under_memory_limit`: it verifies
/// that the operation completes successfully without exceeding the limit.
///
/// # Parameters
///
/// - `limit_bytes`: Maximum virtual memory size in bytes
/// - `f`: Closure that should succeed under the memory limit
///
/// # Panics
///
/// Panics if:
/// - The closure fails (returns an error)
/// - The closure panics
///
/// # Example
///
/// ```rust
/// assert_succeeds_under_memory_limit(100 * 1024 * 1024, || {
///     let mut data = Vec::new();
///     data.try_reserve(1000).map_err(|e| e.to_string())?;
///     Ok::<_, String>(data.len())
/// });
/// ```
pub fn assert_succeeds_under_memory_limit<F, T>(limit_bytes: u64, f: F) -> T
where
    F: std::panic::UnwindSafe + FnOnce() -> Result<T, String>,
{
    match run_under_memory_limit(limit_bytes, f) {
        Ok(t) => t,
        Err(MemoryGuardError::ClosureError(msg)) => {
            panic!("Operation failed under memory limit: {}", msg);
        }
        Err(MemoryGuardError::Panic(msg)) => {
            panic!("Operation panicked under memory limit: {}", msg);
        }
        Err(MemoryGuardError::UnsupportedPlatform) => {
            panic!("Memory limits not supported on this platform");
        }
        Err(MemoryGuardError::SetLimitFailed(msg)) => {
            panic!("Failed to set memory limit: {}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_guard_unsupported_platform_windows() {
        #[cfg(windows)]
        {
            let result = run_under_memory_limit(1000, || Ok::<(), String>(()));
            assert!(matches!(result, Err(MemoryGuardError::UnsupportedPlatform)));
        }

        #[cfg(not(windows))]
        {
            // On Unix, this should succeed
            let result = run_under_memory_limit(100 * 1024 * 1024, || Ok::<(), String>(()));
            assert!(result.is_ok());
        }
    }

    #[cfg_attr(not(target_os = "windows"), test)]
    fn test_memory_guard_simple_success() {
        let result = run_under_memory_limit(500 * 1024 * 1024, || {
            let v = vec![1, 2, 3];
            Ok::<_, String>(v.len())
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[cfg_attr(not(target_os = "windows"), test)]
    #[ignore = "memory limit tests interfere with each other when run in the same process"]
    fn test_memory_guard_alloc_failure() {
        // Try to allocate more than the limit allows
        let result = run_under_memory_limit(200 * 1024 * 1024, || {
            let mut v: Vec<u8> = Vec::new();
            // Try to reserve 500 MB under a 200 MB limit
            v.try_reserve(500_000_000).map_err(|e| e.to_string())?;
            Ok::<_, String>(v.len())
        });

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MemoryGuardError::ClosureError(_))
        ));
    }

    #[cfg_attr(not(target_os = "windows"), test)]
    #[ignore = "memory limit tests interfere with each other when run in the same process"]
    fn test_assert_fails_under_memory_limit() {
        // This should not panic (assertion passes)
        assert_fails_under_memory_limit(200 * 1024 * 1024, || {
            let mut v: Vec<u8> = Vec::new();
            v.try_reserve(500_000_000).map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        });
    }

    #[cfg_attr(not(target_os = "windows"), test)]
    fn test_assert_succeeds_under_memory_limit() {
        let len = assert_succeeds_under_memory_limit(1024 * 1024 * 1024, || {
            let mut v: Vec<u8> = Vec::new();
            v.try_reserve(1000).map_err(|e| e.to_string())?;
            Ok::<_, String>(v.capacity())
        });

        assert_eq!(len, 1000);
    }

    #[cfg_attr(not(target_os = "windows"), test)]
    #[ignore = "memory limit tests interfere with each other when run in the same process"]
    #[should_panic(expected = "Operation succeeded despite memory limit")]
    fn test_assert_fails_panics_on_success() {
        assert_fails_under_memory_limit(100 * 1024 * 1024, || {
            Ok::<_, String>(()) // Succeeds, should panic
        });
    }

}

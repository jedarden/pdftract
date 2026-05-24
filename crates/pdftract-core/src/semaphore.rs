//! A simple counting semaphore for bounding concurrent operations.
//!
//! This module provides a thread-safe semaphore implemented using std::sync primitives.
//! It's used to cap the number of in-flight page extractions to keep memory usage bounded.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};

/// A counting semaphore that limits concurrent access to a resource.
///
/// This semaphore tracks available permits atomically and allows threads
/// to wait until a permit becomes available.
pub struct Semaphore {
    /// Current number of available permits.
    permits: AtomicUsize,
    /// Mutex and condition variable for blocking when no permits are available.
    cv: Mutex<()>,
    condvar: Condvar,
}

impl Semaphore {
    /// Create a new semaphore with the specified number of permits.
    ///
    /// # Arguments
    /// * `permits` - The maximum number of concurrent operations allowed
    pub fn new(permits: usize) -> Self {
        Self {
            permits: AtomicUsize::new(permits),
            cv: Mutex::new(()),
            condvar: Condvar::new(),
        }
    }

    /// Acquire a permit, blocking if necessary until one becomes available.
    ///
    /// This function will block the current thread until a permit is available.
    /// When the function returns, the caller holds one permit.
    pub fn acquire(&self) {
        loop {
            // Try to decrement the permit count atomically
            let current = self.permits.load(Ordering::Acquire);
            if current > 0 {
                match self.permits.compare_exchange(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return,    // Successfully acquired
                    Err(_) => continue, // Retry
                }
            }

            // No permits available - wait
            let guard = self.cv.lock().unwrap();
            let _guard = self.condvar.wait(guard).unwrap();
        }
    }

    /// Release a permit, making it available to other waiters.
    ///
    /// # Panics
    /// Panics if releasing would cause the permit count to exceed the initial limit.
    pub fn release(&self) {
        let prev = self.permits.fetch_add(1, Ordering::Release);
        // Notify one waiting thread
        self.condvar.notify_one();
    }

    /// Get the current number of available permits.
    pub fn available(&self) -> usize {
        self.permits.load(Ordering::Acquire)
    }
}

/// RAII guard that automatically releases a semaphore permit when dropped.
pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
}

impl<'a> SemaphoreGuard<'a> {
    /// Create a new guard from an acquired semaphore.
    fn new(semaphore: &'a Semaphore) -> Self {
        Self { semaphore }
    }
}

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

/// Extension trait to add an `acquire_guard` method to Semaphore.
pub trait SemaphoreExt {
    /// Acquire a permit and return an RAII guard that releases it on drop.
    fn acquire_guard(&self) -> SemaphoreGuard<'_>;
}

impl SemaphoreExt for Semaphore {
    fn acquire_guard(&self) -> SemaphoreGuard<'_> {
        self.acquire();
        SemaphoreGuard::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_semaphore_creation() {
        let sem = Semaphore::new(5);
        assert_eq!(sem.available(), 5);
    }

    #[test]
    fn test_semaphore_acquire_release() {
        let sem = Semaphore::new(2);

        sem.acquire();
        assert_eq!(sem.available(), 1);

        sem.acquire();
        assert_eq!(sem.available(), 0);

        sem.release();
        assert_eq!(sem.available(), 1);
    }

    #[test]
    fn test_semaphore_guard() {
        let sem = Arc::new(Semaphore::new(2));

        {
            let _guard1 = sem.acquire_guard();
            assert_eq!(sem.available(), 1);

            {
                let _guard2 = sem.acquire_guard();
                assert_eq!(sem.available(), 0);
            }

            // Guard 2 dropped, permit released
            assert_eq!(sem.available(), 1);
        }

        // Guard 1 dropped, permit released
        assert_eq!(sem.available(), 2);
    }

    #[test]
    fn test_semaphore_concurrent() {
        let sem = Arc::new(Semaphore::new(2));
        let mut handles = vec![];

        // Spawn 4 threads that each try to acquire and hold for a bit
        for i in 0..4 {
            let sem_clone = Arc::clone(&sem);
            let handle = thread::spawn(move || {
                let _guard = sem_clone.acquire_guard();
                // Hold the permit briefly
                thread::sleep(std::time::Duration::from_millis(50));
                i
            });
            handles.push(handle);
        }

        // All threads should complete
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.len(), 4);

        // All permits should be released
        assert_eq!(sem.available(), 2);
    }

    #[test]
    fn test_semaphore_blocking() {
        let sem = Arc::new(Semaphore::new(1));
        let sem_clone = Arc::clone(&sem);

        // Acquire the only permit
        let _guard1 = sem.acquire_guard();
        assert_eq!(sem.available(), 0);

        // Spawn a thread that will block
        let handle = thread::spawn(move || {
            let _guard2 = sem_clone.acquire_guard();
            // This will block until guard1 is released
        });

        // Give the thread time to try acquiring
        thread::sleep(std::time::Duration::from_millis(50));

        // The thread should be blocked
        assert!(!handle.is_finished());

        // Drop guard1 to release the permit
        drop(_guard1);

        // Now the thread should complete
        handle.join().unwrap();
    }
}

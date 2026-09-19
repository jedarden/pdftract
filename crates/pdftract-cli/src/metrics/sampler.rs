//! Rayon pool utilization sampling for `pdftract_rayon_pool_utilization`.
//!
//! Rayon does not expose a busy-thread count, so busy-ness is tracked at
//! the CLI's own rayon entry points: each task wraps itself in a
//! [`BusyGuard`] for the duration of its execution, and
//! [`sample_utilization`] divides the live guard count by the pool's
//! current worker-thread count. Work submitted to the pool from inside
//! pdftract-core (OCR page recognition) has no CLI-side entry point and
//! therefore reads as idle; the serve/mcp sampler task publishes the
//! sample every few seconds via
//! [`Registry::set_rayon_pool_utilization`](super::registry::Registry::set_rayon_pool_utilization).

use std::sync::atomic::{AtomicUsize, Ordering};

/// Number of rayon tasks currently executing under a [`BusyGuard`].
static RAYON_BUSY_TASKS: AtomicUsize = AtomicUsize::new(0);

/// RAII marker for one executing rayon task; dropping it decrements the
/// busy count, so the count stays correct across panics and early returns.
#[must_use = "the guard must be held for the task's whole execution"]
pub struct BusyGuard(());

impl BusyGuard {
    /// Mark one rayon task as started (call on a pool worker thread).
    pub fn begin() -> Self {
        RAYON_BUSY_TASKS.fetch_add(1, Ordering::Relaxed);
        BusyGuard(())
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        RAYON_BUSY_TASKS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Fraction of the rayon pool's worker threads currently busy (0..1).
pub fn sample_utilization() -> f64 {
    let threads = rayon::current_num_threads().max(1);
    (RAYON_BUSY_TASKS.load(Ordering::Relaxed) as f64 / threads as f64).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    // The busy counter is process-global, so every assertion that depends
    // on its value lives in this one test — a second test running in
    // parallel could observe this test's guard.
    #[test]
    fn busy_guards_raise_utilization_until_dropped() {
        assert_eq!(super::sample_utilization(), 0.0);

        let guard = super::BusyGuard::begin();
        assert!(super::sample_utilization() > 0.0);

        drop(guard);
        assert_eq!(super::sample_utilization(), 0.0);
    }
}

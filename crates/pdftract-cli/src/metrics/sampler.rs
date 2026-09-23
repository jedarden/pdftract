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

use super::Registry;

/// Number of rayon tasks currently executing under a [`BusyGuard`].
static RAYON_BUSY_TASKS: AtomicUsize = AtomicUsize::new(0);

/// RAII marker for one executing rayon task; dropping it decrements the
/// busy count, so the count stays correct across panics and early returns.
#[must_use = "the guard must be held for the task's whole execution"]
pub struct BusyGuard(());

/// RAII marker for work associated with a registry.  In addition to the
/// process-wide count used by the sampler, publish an immediate sample so
/// short requests are visible without waiting for the periodic task.
pub struct RegisteredBusyGuard {
    registry: Registry,
}

impl BusyGuard {
    /// Mark one rayon task as started (call on a pool worker thread).
    pub fn begin() -> Self {
        RAYON_BUSY_TASKS.fetch_add(1, Ordering::Relaxed);
        BusyGuard(())
    }

    /// Mark one task as busy and publish the new utilization immediately.
    ///
    /// The periodic sampler remains the source of truth for long-lived idle
    /// periods, while this hook keeps the gauge responsive for short-lived
    /// extraction requests.
    pub fn begin_with_registry(registry: Registry) -> RegisteredBusyGuard {
        RAYON_BUSY_TASKS.fetch_add(1, Ordering::Relaxed);
        registry.set_rayon_pool_utilization(sample_utilization());
        RegisteredBusyGuard { registry }
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        RAYON_BUSY_TASKS.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Drop for RegisteredBusyGuard {
    fn drop(&mut self) {
        RAYON_BUSY_TASKS.fetch_sub(1, Ordering::Relaxed);
        self.registry
            .set_rayon_pool_utilization(sample_utilization());
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

        let registry = super::Registry::new();
        let registered_guard = super::BusyGuard::begin_with_registry(registry.clone());
        assert!(registry.rayon_pool_utilization() > 0.0);

        drop(registered_guard);
        assert_eq!(registry.rayon_pool_utilization(), 0.0);
    }
}

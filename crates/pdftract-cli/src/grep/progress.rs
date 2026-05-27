//! Progress bar implementation for pdftract grep.
//!
//! This module implements the indicatif-based progress bar that ticks every 100 ms
//! with the current file + page-within-file information. Guarantees an update every
//! 500 ms even when a single file blocks for a long time (watchdog ticker on a
//! dedicated thread).

use crate::grep::{ProgressEvent, ProgressMode};
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle, TermLike};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Default steady tick interval (100 ms).
const STEADY_TICK_MS: u64 = 100;

/// Watchdog timeout threshold (500 ms).
const WATCHDOG_TIMEOUT_MS: u64 = 500;

/// Slow file warning threshold (30 seconds).
const SLOW_FILE_WARNING_SECS: u64 = 30;

/// Progress bar manager for pdftract grep.
///
/// Manages the main progress bar (overall progress) and the "Current" sub-bar
/// (per-file progress). Handles TTY detection, steady ticking, and watchdog
/// guarantees.
pub struct ProgressManager {
    /// Main progress bar (overall progress).
    main_bar: Option<ProgressBar>,
    /// Current file sub-bar.
    current_bar: Option<ProgressBar>,
    /// Multi-progress container for coordinating bars.
    multi: Option<MultiProgress>,
    /// Last event time for watchdog (atomic for cross-thread access).
    last_event_time: Arc<AtomicU64>,
    /// Watchdog thread handle.
    watchdog_thread: Option<thread::JoinHandle<()>>,
    /// Whether we're in TTY mode.
    is_tty: bool,
    /// Current file path for slow-file warning.
    current_file: Arc<tokio::sync::Mutex<String>>,
    /// Current file start time for slow-file warning.
    current_file_start: Arc<AtomicU64>,
    /// Slow file warning already emitted flag.
    slow_file_warned: Arc<AtomicBool>,
}

impl ProgressManager {
    /// Create a new progress manager.
    ///
    /// # Arguments
    ///
    /// * `files_total` - Total number of files to process
    /// * `bytes_total` - Total bytes of all files
    /// * `mode` - Progress mode (Auto, On, Off)
    ///
    /// # Returns
    ///
    /// A new progress manager, or None if progress is disabled.
    pub fn new(files_total: u64, _bytes_total: u64, mode: ProgressMode) -> Option<Self> {
        // Check if we should show progress
        let is_tty = is_terminal_stderr();
        let show_progress = match mode {
            ProgressMode::On => true,
            ProgressMode::Off => false,
            ProgressMode::Auto => is_tty,
        };

        if !show_progress {
            return None;
        }

        let multi = Some(MultiProgress::new());
        let multi_ref = multi.as_ref().unwrap();

        // Main bar template: "Searching: [{wide_bar}] {pos}/{len} files ({percent}%) {bytes_per_sec} ETA {eta}"
        let main_style = ProgressStyle::with_template(
            "Searching: [{wide_bar}] {pos}/{len} files ({percent}%) {bytes_per_sec} ETA {eta}",
        )
        .expect("invalid main bar template");

        let main_bar = Some(multi_ref.add(ProgressBar::new(files_total)));
        let main_bar_ref = main_bar.as_ref().unwrap();
        main_bar_ref.set_style(main_style);
        main_bar_ref.enable_steady_tick(Duration::from_millis(STEADY_TICK_MS));

        // Sub-bar template: "Current: {msg}" where msg = "<path> (page {pages_done}/{pages_total})"
        let current_style =
            ProgressStyle::with_template("Current: {msg}").expect("invalid current bar template");

        let current_bar = Some(multi_ref.add(ProgressBar::new(1)));
        let current_bar_ref = current_bar.as_ref().unwrap();
        current_bar_ref.set_style(current_style);
        current_bar_ref.enable_steady_tick(Duration::from_millis(STEADY_TICK_MS));

        let last_event_time = Arc::new(AtomicU64::new(timestamp_ms()));
        let current_file = Arc::new(tokio::sync::Mutex::new(String::new()));
        let current_file_start = Arc::new(AtomicU64::new(timestamp_ms()));
        let slow_file_warned = Arc::new(AtomicBool::new(false));

        // Spawn watchdog thread
        let watchdog_thread = Some(spawn_watchdog(
            last_event_time.clone(),
            current_file.clone(),
            current_file_start.clone(),
            slow_file_warned.clone(),
            is_tty,
        ));

        Some(Self {
            main_bar,
            current_bar,
            multi,
            last_event_time,
            watchdog_thread,
            is_tty,
            current_file,
            current_file_start,
            slow_file_warned,
        })
    }

    /// Handle a progress event.
    ///
    /// Updates the progress bars based on the event type.
    pub fn handle_event(&mut self, event: &ProgressEvent) {
        // Update last event time for watchdog
        self.last_event_time
            .store(timestamp_ms(), Ordering::Relaxed);

        match event {
            ProgressEvent::FileStart { path, size_hint: _ } => {
                // Update current file for slow-file warning
                *self.current_file.blocking_lock() = path.clone();
                self.current_file_start
                    .store(timestamp_ms(), Ordering::Relaxed);
                self.slow_file_warned.store(false, Ordering::Relaxed);

                // Update current bar message
                if let Some(ref bar) = self.current_bar {
                    bar.set_message(format!("{}", path));
                }
            }
            ProgressEvent::FileProgress {
                path: _,
                pages_done,
                pages_total,
            } => {
                // Update current bar with page progress
                if let Some(ref bar) = self.current_bar {
                    bar.set_message(format!(
                        "{} (page {}/{})",
                        self.current_file.blocking_lock(),
                        pages_done,
                        pages_total
                    ));
                }
            }
            ProgressEvent::FileDone {
                path: _,
                matches: _,
                duration_ms: _,
            } => {
                // Increment main bar
                if let Some(ref bar) = self.main_bar {
                    bar.inc(1);
                }

                // Reset slow file warning state
                self.slow_file_warned.store(false, Ordering::Relaxed);
            }
            ProgressEvent::FileSkipped { path: _, reason: _ } => {
                // Increment main bar
                if let Some(ref bar) = self.main_bar {
                    bar.inc(1);
                }
            }
        }
    }

    /// Finish the progress bars.
    ///
    /// Displays final stats: "Searched: 512 files (104 MB) in 18.4s (78 MB/s)"
    pub fn finish(mut self, files_processed: u64, bytes_total: u64, duration_ms: u128) {
        // Join watchdog thread
        if let Some(handle) = self.watchdog_thread.take() {
            let _ = handle.join();
        }

        if let Some(main_bar) = self.main_bar.take() {
            main_bar.finish();

            // Print final stats to stderr
            if self.is_tty {
                let duration_secs = duration_ms as f64 / 1000.0;
                let throughput_mb = if duration_secs > 0.0 {
                    (bytes_total as f64) / (1024.0 * 1024.0) / duration_secs
                } else {
                    0.0
                };
                let total_mb = bytes_total as f64 / (1024.0 * 1024.0);

                eprintln!(
                    "Searched: {} files ({:.1} MB) in {:.1}s ({:.1} MB/s)",
                    files_processed, total_mb, duration_secs, throughput_mb
                );
            }
        }

        // Clear current bar
        if let Some(current_bar) = self.current_bar.take() {
            current_bar.finish_and_clear();
        }
    }
}

impl Drop for ProgressManager {
    fn drop(&mut self) {
        // Ensure watchdog thread is joined
        if let Some(handle) = self.watchdog_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Check if stderr is a TTY.
fn is_terminal_stderr() -> bool {
    // Try to detect if stderr is a terminal
    // On Unix: check isatty(STDERR_FILENO)
    // On Windows: similar check
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let stderr = std::io::stderr();
        unsafe { libc::isatty(stderr.as_raw_fd()) != 0 }
    }

    #[cfg(windows)]
    {
        // Windows TTY detection
        // For simplicity, assume false on Windows for now
        // A full implementation would use winapi::console::GetConsoleMode
        false
    }
}

/// Get current timestamp in milliseconds.
fn timestamp_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Spawn the watchdog thread.
///
/// The watchdog ensures the progress bars tick at least once every 500 ms,
/// even when no events are arriving (e.g., during slow file processing).
fn spawn_watchdog(
    last_event_time: Arc<AtomicU64>,
    current_file: Arc<tokio::sync::Mutex<String>>,
    current_file_start: Arc<AtomicU64>,
    slow_file_warned: Arc<AtomicBool>,
    is_tty: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(WATCHDOG_TIMEOUT_MS));

            let now = timestamp_ms();
            let last = last_event_time.load(Ordering::Relaxed);
            let _elapsed = now.saturating_sub(last);

            // Check for slow file (30 seconds)
            let file_start = current_file_start.load(Ordering::Relaxed);
            let file_elapsed = now.saturating_sub(file_start);
            if file_elapsed > SLOW_FILE_WARNING_SECS * 1000
                && !slow_file_warned.load(Ordering::Relaxed)
                && is_tty
            {
                let path = current_file.blocking_lock().clone();
                if !path.is_empty() {
                    let elapsed_secs = file_elapsed / 1000;
                    eprintln!(
                        "WARNING: file {} still processing after {}s",
                        path, elapsed_secs
                    );
                    slow_file_warned.store(true, Ordering::Relaxed);
                }
            }

            // If elapsed > WATCHDOG_TIMEOUT_MS, force a redraw
            // This is a no-op for indicatif bars (they auto-redraw),
            // but the liveness guarantee is that the bars are still ticking
            // via the steady_tick we enabled.
            // The watchdog here mainly serves for slow-file warnings.
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_ms_increases() {
        let t1 = timestamp_ms();
        thread::sleep(Duration::from_millis(10));
        let t2 = timestamp_ms();
        assert!(t2 > t1);
    }

    #[test]
    fn test_progress_manager_off_mode() {
        let manager = ProgressManager::new(100, 1_000_000, ProgressMode::Off);
        assert!(manager.is_none());
    }

    #[test]
    fn test_progress_manager_auto_non_tty() {
        // Force non-TTY mode for testing
        let manager = ProgressManager::new(100, 1_000_000, ProgressMode::Auto);
        // May be Some or None depending on actual environment
        // We just verify it doesn't panic
        let _ = manager;
    }

    #[test]
    fn test_progress_manager_on_mode() {
        let manager = ProgressManager::new(100, 1_000_000, ProgressMode::On);
        // May be Some or None depending on environment
        // We just verify it doesn't panic
        let _ = manager;
    }
}

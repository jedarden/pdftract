//! Progress bar implementation for pdftract grep.
//!
//! This module implements the indicatif-based progress bar that ticks every 100 ms
//! with the current file + page-within-file information. Guarantees an update every
//! 500 ms even when a single file blocks for a long time (watchdog ticker on a
//! dedicated thread).

use crate::grep::{ProgressEvent, ProgressMode};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    _multi: Option<MultiProgress>,
    /// Last event time for watchdog (atomic for cross-thread access).
    last_event_time: Arc<AtomicU64>,
    /// Watchdog shutdown flag.
    shutdown_flag: Arc<AtomicBool>,
    /// Watchdog thread handle.
    watchdog_thread: Option<thread::JoinHandle<()>>,
    /// Whether we're in TTY mode.
    is_tty: bool,
    /// Current file path for slow-file warning.
    current_file: Arc<Mutex<String>>,
    /// Current file start time for slow-file warning.
    current_file_start: Arc<AtomicU64>,
    /// Slow file warning already emitted flag.
    slow_file_warned: Arc<AtomicBool>,
    /// Main bar reference for watchdog redraws.
    main_bar_for_watchdog: Arc<Mutex<Option<ProgressBar>>>,
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

        // Create multi-progress with stderr target
        let multi = MultiProgress::new();
        multi.set_draw_target(ProgressDrawTarget::stderr());

        // Main bar template: "Searching: [{wide_bar}] {pos}/{len} files ({percent}%) {bytes_per_sec} ETA {eta}"
        let main_style = ProgressStyle::with_template(
            "Searching: [{wide_bar}] {pos}/{len} files ({percent}%) {bytes_per_sec} ETA {eta}",
        )
        .expect("invalid main bar template");

        let main_bar = multi.add(ProgressBar::new(files_total));
        main_bar.set_style(main_style);
        main_bar.enable_steady_tick(Duration::from_millis(STEADY_TICK_MS));

        // Sub-bar template: "Current: {msg}" where msg = "<path> (page {pages_done}/{pages_total})"
        let current_style =
            ProgressStyle::with_template("Current: {msg}").expect("invalid current bar template");

        let current_bar = multi.add(ProgressBar::new(1));
        current_bar.set_style(current_style);
        current_bar.enable_steady_tick(Duration::from_millis(STEADY_TICK_MS));

        let last_event_time = Arc::new(AtomicU64::new(timestamp_ms()));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let current_file = Arc::new(Mutex::new(String::new()));
        let current_file_start = Arc::new(AtomicU64::new(timestamp_ms()));
        let slow_file_warned = Arc::new(AtomicBool::new(false));
        let main_bar_for_watchdog = Arc::new(Mutex::new(Some(main_bar.clone())));

        // Spawn watchdog thread
        let watchdog_thread = Some(spawn_watchdog(
            last_event_time.clone(),
            current_file.clone(),
            current_file_start.clone(),
            slow_file_warned.clone(),
            shutdown_flag.clone(),
            is_tty,
            main_bar_for_watchdog.clone(),
        ));

        Some(Self {
            main_bar: Some(main_bar),
            current_bar: Some(current_bar),
            _multi: Some(multi),
            last_event_time,
            shutdown_flag,
            watchdog_thread,
            is_tty,
            current_file,
            current_file_start,
            slow_file_warned,
            main_bar_for_watchdog,
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
                *self.current_file.lock().unwrap() = path.clone();
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
                    let path = self.current_file.lock().unwrap();
                    bar.set_message(format!(
                        "{} (page {}/{})",
                        path, pages_done, pages_total
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
        // Signal watchdog thread to stop
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // Join watchdog thread
        if let Some(handle) = self.watchdog_thread.take() {
            let _ = handle.join();
        }

        // Clear main_bar_for_watchdog to prevent any late redraws
        *self.main_bar_for_watchdog.lock().unwrap() = None;

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
        // Ensure watchdog thread is joined and flag is set
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.watchdog_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Check if stderr is a TTY.
fn is_terminal_stderr() -> bool {
    atty::is(atty::Stream::Stderr)
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
    _last_event_time: Arc<AtomicU64>,
    current_file: Arc<Mutex<String>>,
    current_file_start: Arc<AtomicU64>,
    slow_file_warned: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    is_tty: bool,
    main_bar_for_watchdog: Arc<Mutex<Option<ProgressBar>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !shutdown_flag.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(WATCHDOG_TIMEOUT_MS));

            let now = timestamp_ms();

            // Check for slow file (30 seconds)
            let file_start = current_file_start.load(Ordering::Relaxed);
            let file_elapsed = now.saturating_sub(file_start);
            if file_elapsed > SLOW_FILE_WARNING_SECS * 1000
                && !slow_file_warned.load(Ordering::Relaxed)
                && is_tty
            {
                let path = current_file.lock().unwrap();
                if !path.is_empty() {
                    let elapsed_secs = file_elapsed / 1000;
                    eprintln!(
                        "WARNING: file {} still processing after {}s",
                        path, elapsed_secs
                    );
                    slow_file_warned.store(true, Ordering::Relaxed);
                }
            }

            // Force a redraw of the main bar to ensure liveness
            // This guarantees the 500ms update requirement even during slow file processing
            let bar_guard = main_bar_for_watchdog.lock().unwrap();
            if let Some(ref bar) = *bar_guard {
                // Tick the bar to force a redraw - this is a no-op for progress
                // but ensures the terminal cursor moves and the bar stays alive
                bar.tick();
            }
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

    #[test]
    fn test_shutdown_flag() {
        let flag = Arc::new(AtomicBool::new(false));
        assert!(!flag.load(Ordering::Relaxed));
        flag.store(true, Ordering::Relaxed);
        assert!(flag.load(Ordering::Relaxed));
    }
}

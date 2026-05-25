use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

// Matcher module
mod matcher;
pub use matcher::{MatchRange, Matcher};

// Event and JSON output module
mod event;
pub use event::{CountEvent, FileOnlyEvent, JsonSink, MatchEvent};

/// Progress reporting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    /// Auto-detect: on if TTY, off otherwise
    Auto,
    /// Force on
    On,
    /// Force off
    Off,
}

/// Grep subcommand arguments
#[derive(Parser, Debug)]
pub struct GrepArgs {
    /// Search pattern (literal string by default; regex with -E)
    #[arg(value_name = "PATTERN")]
    pub pattern: String,

    /// Paths to search (files, directories, or URLs; default: ".")
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Recurse into directories (default: on when path is a directory)
    #[arg(short, long)]
    pub recursive: bool,

    /// Case-insensitive search
    #[arg(short, long)]
    pub ignore_case: bool,

    /// Treat PATTERN as a full regular expression
    #[arg(short = 'E', long)]
    pub extended_regexp: bool,

    /// Literal string match (default: on)
    #[arg(short = 'F', long)]
    pub fixed_strings: bool,

    /// Match on word boundaries
    #[arg(short = 'w', long)]
    pub word_regexp: bool,

    /// Invert match: print non-matching spans instead
    #[arg(short = 'v', long)]
    pub invert_match: bool,

    /// Print only filenames with at least one match
    #[arg(short = 'l', long)]
    pub files_with_matches: bool,

    /// Print match counts per file
    #[arg(short = 'c', long)]
    pub count: bool,

    /// Worker thread count (default: CPU count)
    #[arg(short = 'j', long, value_name = "N")]
    pub threads: Option<usize>,

    /// Run OCR on scanned pages too (slower)
    #[arg(long)]
    pub ocr: bool,

    /// JSON-Lines output (one match per line)
    #[arg(long)]
    pub json: bool,

    /// Write annotated PDFs to DIR/<name>-highlighted.pdf
    #[arg(long, value_name = "DIR")]
    pub highlight: Option<PathBuf>,

    /// Stop after N total matches
    #[arg(long, value_name = "N")]
    pub max_results: Option<usize>,

    /// Show progress bar (default: auto)
    #[arg(long)]
    pub progress: bool,

    /// Force-disable the progress bar
    #[arg(long)]
    pub no_progress: bool,

    /// Emit machine-readable progress events to stderr
    #[arg(long)]
    pub progress_json: bool,

    /// Suppress all output except exit code
    #[arg(long)]
    pub quiet: bool,
}

impl GrepArgs {
    /// Get the progress mode based on flags and TTY detection
    pub fn progress_mode(&self) -> ProgressMode {
        if self.progress_json {
            // JSON progress events don't use the progress bar
            return ProgressMode::Off;
        }
        if self.no_progress {
            return ProgressMode::Off;
        }
        if self.progress {
            return ProgressMode::On;
        }
        ProgressMode::Auto
    }

    /// Validate the arguments and return normalized values
    pub fn validate(&self) -> Result<GrepConfig> {
        // Check if the grep feature is enabled
        #[cfg(not(feature = "grep"))]
        {
            anyhow::bail!("feature 'grep' not compiled in. Build pdftract with: --features grep");
        }

        // Validate pattern is not empty
        if self.pattern.is_empty() {
            anyhow::bail!("PATTERN may not be empty");
        }

        // Validate pattern doesn't contain null byte
        if self.pattern.contains('\0') {
            anyhow::bail!("PATTERN may not contain null byte");
        }

        // Determine match mode (default: literal/-F)
        // -E explicitly enables regex, -F explicitly enables literal
        // When neither is set, default to literal (per plan and ripgrep compat)
        let use_regex = self.extended_regexp && !self.fixed_strings;

        // Determine if recursion should be used
        // Default: recursive if path is a directory (ripgrep compat)
        let recursive = if self.paths.iter().any(|p| p.is_dir()) {
            self.recursive || true // default to true for dirs
        } else {
            self.recursive
        };

        // Validate highlight directory
        let highlight_dir = if let Some(ref dir) = self.highlight {
            if !dir.exists() {
                std::fs::create_dir_all(dir).with_context(|| {
                    format!("Failed to create highlight directory: {}", dir.display())
                })?;
            }
            Some(dir.clone())
        } else {
            None
        };

        // Determine thread count
        let threads = self.threads.unwrap_or_else(num_cpus::get);

        Ok(GrepConfig {
            pattern: self.pattern.clone(),
            paths: self.paths.clone(),
            recursive,
            ignore_case: self.ignore_case,
            use_regex,
            word_regexp: self.word_regexp,
            invert_match: self.invert_match,
            files_with_matches: self.files_with_matches,
            count: self.count,
            threads,
            ocr: self.ocr,
            json: self.json,
            highlight_dir,
            max_results: self.max_results,
            progress_mode: self.progress_mode(),
            progress_json: self.progress_json,
            quiet: self.quiet,
        })
    }
}

/// Normalized grep configuration after validation
#[derive(Debug, Clone)]
pub struct GrepConfig {
    pub pattern: String,
    pub paths: Vec<PathBuf>,
    pub recursive: bool,
    pub ignore_case: bool,
    pub use_regex: bool,
    pub word_regexp: bool,
    pub invert_match: bool,
    pub files_with_matches: bool,
    pub count: bool,
    pub threads: usize,
    pub ocr: bool,
    pub json: bool,
    pub highlight_dir: Option<PathBuf>,
    pub max_results: Option<usize>,
    pub progress_mode: ProgressMode,
    pub progress_json: bool,
    pub quiet: bool,
}

/// Run the grep command
pub fn run_grep(args: GrepArgs) -> Result<()> {
    // Validate and normalize arguments
    let config = args.validate()?;

    // For now, just print the configuration
    // TODO: Implement the actual grep logic in subsequent beads (7.8.2-7.8.10)
    if !config.quiet {
        eprintln!("pdftract grep: mode not yet implemented");
        eprintln!("Pattern: {}", config.pattern);
        eprintln!("Paths: {:?}", config.paths);
        eprintln!(
            "Match mode: {}",
            if config.use_regex { "regex" } else { "literal" }
        );
        eprintln!("Case-insensitive: {}", config.ignore_case);
        eprintln!("Word boundaries: {}", config.word_regexp);
        eprintln!("Invert match: {}", config.invert_match);
    }

    std::process::exit(2);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<GrepConfig> {
        let args = GrepArgs::parse_from(args);
        args.validate()
    }

    #[test]
    fn test_default_literal_mode() {
        let config = parse_args(&["grep", "test"]).unwrap();
        assert!(!config.use_regex, "default should be literal mode");
        assert_eq!(config.pattern, "test");
        assert_eq!(config.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn test_extended_regex_mode() {
        let config = parse_args(&["grep", "-E", r"\d+"]).unwrap();
        assert!(config.use_regex, "-E should enable regex mode");
        assert_eq!(config.pattern, r"\d+");
    }

    #[test]
    fn test_fixed_strings_mode() {
        let config = parse_args(&["grep", "-F", "test"]).unwrap();
        assert!(!config.use_regex, "-F should enable literal mode");
        assert_eq!(config.pattern, "test");
    }

    #[test]
    fn test_ignore_case() {
        let config = parse_args(&["grep", "-i", "test"]).unwrap();
        assert!(config.ignore_case, "-i should enable case-insensitive");
    }

    #[test]
    fn test_word_regexp() {
        let config = parse_args(&["grep", "-w", "test"]).unwrap();
        assert!(config.word_regexp, "-w should enable word boundaries");
    }

    #[test]
    fn test_invert_match() {
        let config = parse_args(&["grep", "-v", "test"]).unwrap();
        assert!(config.invert_match, "-v should enable invert match");
    }

    #[test]
    fn test_files_with_matches() {
        let config = parse_args(&["grep", "-l", "test"]).unwrap();
        assert!(
            config.files_with_matches,
            "-l should enable files-with-matches"
        );
    }

    #[test]
    fn test_count() {
        let config = parse_args(&["grep", "-c", "test"]).unwrap();
        assert!(config.count, "-c should enable count mode");
    }

    #[test]
    fn test_json_output() {
        let config = parse_args(&["grep", "--json", "test"]).unwrap();
        assert!(config.json, "--json should enable JSON output");
    }

    #[test]
    fn test_ocr_flag() {
        let config = parse_args(&["grep", "--ocr", "test"]).unwrap();
        assert!(config.ocr, "--ocr should enable OCR");
    }

    #[test]
    fn test_quiet_flag() {
        let config = parse_args(&["grep", "--quiet", "test"]).unwrap();
        assert!(config.quiet, "--quiet should suppress output");
    }

    #[test]
    fn test_empty_pattern_rejected() {
        let result = parse_args(&["grep", ""]);
        assert!(result.is_err(), "empty pattern should be rejected");
    }

    #[test]
    fn test_null_byte_pattern_rejected() {
        let result = parse_args(&["grep", "test\0pattern"]);
        assert!(result.is_err(), "null byte in pattern should be rejected");
    }

    #[test]
    fn test_progress_mode_auto() {
        let config = parse_args(&["grep", "test"]).unwrap();
        assert_eq!(config.progress_mode, ProgressMode::Auto);
    }

    #[test]
    fn test_progress_mode_on() {
        let config = parse_args(&["grep", "--progress", "test"]).unwrap();
        assert_eq!(config.progress_mode, ProgressMode::On);
    }

    #[test]
    fn test_progress_mode_off() {
        let config = parse_args(&["grep", "--no-progress", "test"]).unwrap();
        assert_eq!(config.progress_mode, ProgressMode::Off);
    }

    #[test]
    fn test_progress_json_disables_bar() {
        let config = parse_args(&["grep", "--progress-json", "test"]).unwrap();
        assert_eq!(config.progress_mode, ProgressMode::Off);
        assert!(config.progress_json);
    }

    #[test]
    fn test_recursive_default_for_directory() {
        let config = parse_args(&["grep", "test", "/tmp"]).unwrap();
        assert!(
            config.recursive,
            "should default to recursive for directory paths"
        );
    }

    #[test]
    fn test_threads_default() {
        let config = parse_args(&["grep", "test"]).unwrap();
        assert_eq!(
            config.threads,
            num_cpus::get(),
            "default threads should be CPU count"
        );
    }

    #[test]
    fn test_threads_custom() {
        let config = parse_args(&["grep", "-j", "4", "test"]).unwrap();
        assert_eq!(config.threads, 4);
    }

    #[test]
    fn test_max_results() {
        let config = parse_args(&["grep", "--max-results", "100", "test"]).unwrap();
        assert_eq!(config.max_results, Some(100));
    }
}

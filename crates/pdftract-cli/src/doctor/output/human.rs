//! Human-readable table output for doctor subcommand

use anyhow::Result;
use crate::doctor::{CheckResult, CheckStatus};
use std::io::{IsTerminal, Write};

/// Options for text output
pub struct TextOptions {
    /// Force disable colors
    pub no_color: bool,
}

/// Output results as human-readable text
pub fn output_text(results: &[CheckResult], opts: &TextOptions) -> Result<()> {
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let color_choice = if opts.no_color || !std::io::stdout().is_terminal() {
        ColorChoice::Never
    } else {
        ColorChoice::Always
    };

    let mut stdout = StandardStream::stdout(color_choice);
    let mut stderr = StandardStream::stderr(color_choice);

    let mut ok = 0;
    let mut warn = 0;
    let mut fail = 0;

    // Print header
    stdout.set_color(ColorSpec::new().set_bold(true))?;
    writeln!(&mut stdout, "{:<30} {:<6} {}", "Check", "Status", "Detail")?;
    stdout.reset()?;

    // Print separator line (80 chars using ASCII dashes)
    let separator = "-".repeat(80);
    writeln!(&mut stdout, "{}", separator)?;

    for result in results {
        // Skip N/A checks in human output
        if result.status == CheckStatus::NotApplicable {
            continue;
        }

        let (color, status_str) = match result.status {
            CheckStatus::Ok => {
                ok += 1;
                (Color::Green, "OK")
            }
            CheckStatus::Warn => {
                warn += 1;
                (Color::Yellow, "WARN")
            }
            CheckStatus::Fail => {
                fail += 1;
                (Color::Red, "FAIL")
            }
            CheckStatus::NotApplicable => unreachable!(),
        };

        // Truncate name to 30 chars
        let name = if result.name.len() > 30 {
            format!("{}...", &result.name[..27])
        } else {
            result.name.to_string()
        };

        // Print check name
        write!(&mut stdout, "{:<30} ", name)?;

        // Print status badge with color
        stdout.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true))?;
        write!(&mut stdout, "{:<6} ", status_str)?;
        stdout.reset()?;

        // Print detail (truncate if too long for terminal)
        // For TTY, use actual terminal width; for non-TTY, assume 80 columns
        let term_width = if std::io::stdout().is_terminal() && !opts.no_color {
            terminal_size::terminal_size()
                .map(|(w, _)| w.0 as usize)
                .unwrap_or(80)
        } else {
            80
        };
        let max_detail = term_width.saturating_sub(38); // 30 + 1 + 6 + 1 = 38 columns before detail
        let detail = if result.detail.len() > max_detail {
            format!("{}...", &result.detail[..max_detail.saturating_sub(3)])
        } else {
            result.detail.clone()
        };

        writeln!(&mut stdout, "{}", detail)?;
    }

    // Print separator line
    writeln!(&mut stdout, "{}", separator)?;

    // Print summary
    stdout.set_color(ColorSpec::new().set_bold(true))?;
    write!(&mut stdout, "{} OK, {} WARN, {} FAIL", ok, warn, fail)?;
    stdout.reset()?;
    writeln!(&mut stdout)?;

    // If there are failures, also print to stderr
    if fail > 0 {
        writeln!(&mut stderr)?;
        stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        writeln!(&mut stderr, "FAILURES: {} check(s) failed", fail)?;
        stderr.reset()?;
    }

    Ok(())
}

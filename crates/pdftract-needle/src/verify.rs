//! Verification orchestration for NEEDLE
//!
//! This module provides the main API for per-bead verification, orchestrating
//! git worktree operations, Argo workflow submission, and result gating.

use crate::{git, workflow::WorkflowClient, NeedleContext, VerifyResult};
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{error, info, warn};

/// Verify a bead by running rust-verify on its worktree
///
/// This is the main entry point for NEEDLE per-bead verification.
///
/// # Arguments
/// * `ctx` - NEEDLE context with worker and bead info
/// * `worktree_path` - Path to the git worktree where work was done
/// * `test_args` - Optional test arguments (e.g., "-p miroir-core --lib")
///
/// # Returns
/// * `Ok(VerifyResult)` - Verification result with logs and exit code
/// * `Err` - If verification setup failed (not test failure)
///
/// # Lifecycle Integration
/// This function should be called BEFORE `bf close` to gate bead completion:
/// ```rust
/// let result = needle::verify::verify_bead(&ctx, &worktree_path, "").await?;
/// if !result.passed() {
///     return Err(anyhow!("Verification failed: {}", result.logs));
/// }
/// // Only now call `bf close`
/// ```
pub async fn verify_bead(
    ctx: &NeedleContext,
    worktree_path: &Path,
    test_args: &str,
) -> Result<VerifyResult> {
    info!(
        "Starting verification for bead {}: worker={}, worktree={}",
        ctx.bead_id,
        ctx.worker,
        worktree_path.display()
    );

    // Step 1: Create and push wip branch
    let (branch_name, commit_sha) = git::create_wip_branch(ctx, worktree_path)
        .context("Failed to create and push wip branch")?;

    info!(
        "Created wip branch {} at commit {}",
        branch_name, commit_sha
    );

    // Step 2: Submit rust-verify workflow
    let workflow_client = WorkflowClient::new(ctx.kubeconfig.clone());

    let workflow_name = workflow_client
        .submit_verify_workflow(ctx, &branch_name, test_args)
        .await
        .context("Failed to submit rust-verify workflow")?;

    info!("Submitted workflow: {}", workflow_name);

    // Step 3: Poll for completion
    let result = workflow_client
        .poll_workflow(&workflow_name)
        .await
        .context("Failed to poll workflow to completion")?;

    // Step 4: Return result for gating
    if result.passed() {
        info!(
            "Verification PASSED for bead {} (phase: {}, duration: {}s)",
            ctx.bead_id, result.phase, result.duration_secs
        );
    } else {
        warn!(
            "Verification FAILED for bead {} (phase: {}, duration: {}s)",
            ctx.bead_id, result.phase, result.duration_secs
        );
    }

    Ok(result)
}

/// Verify a bead with detailed error handling
///
/// This variant provides more detailed error messages for agent consumption.
pub async fn verify_bead_with_details(
    ctx: &NeedleContext,
    worktree_path: &Path,
    test_args: &str,
) -> Result<VerifyResult> {
    match verify_bead(ctx, worktree_path, test_args).await {
        Ok(result) => {
            if result.passed() {
                info!("✓ Verification passed: {}", ctx.bead_id);
                Ok(result)
            } else {
                error!("✗ Verification failed: {}", ctx.bead_id);
                error!("Logs excerpt:\n{}", extract_error_excerpt(&result.logs, 500));
                Ok(result) // Return failed result, not error
            }
        }
        Err(e) => {
            error!("Verification setup failed: {}", e);
            Err(e.context("Verification setup failure"))
        }
    }
}

/// Extract error excerpt from logs for debugging
///
/// Returns the last N characters of logs, focusing on failure messages.
fn extract_error_excerpt(logs: &str, max_chars: usize) -> String {
    if logs.len() <= max_chars {
        return logs.to_string();
    }

    // Find the last occurrence of common error patterns
    let error_patterns = ["FAILED:", "error:", "Error ", "failed"];

    let mut best_start = logs.len().saturating_sub(max_chars);

    for pattern in &error_patterns {
        if let Some(pos) = logs.rfind(pattern) {
            let start = pos.saturating_sub(100); // Include some context
            if start + max_chars < logs.len() {
                best_start = start;
                break;
            }
        }
    }

    format!("...{}\n[logs truncated]", &logs[best_start..])
}

/// Check if verification is required for a bead
///
/// Some beads may not require verification (e.g., documentation-only tasks).
/// This function checks bead labels or other metadata to determine if verification
/// should be skipped.
///
/// # Returns
/// * `true` - Verification should run
/// * `false` - Verification can be skipped
pub fn requires_verification(ctx: &NeedleContext) -> bool {
    // For now, always require verification
    // In the future, this could check bead metadata, labels, etc.
    true
}

/// Parse test arguments from bead description
///
/// Extracts test arguments from bead metadata or description.
/// For example, a bead might specify "test-args: -p pdftract-core --lib"
/// in its description to limit tests to a specific crate.
pub fn parse_test_args_from_description(description: &str) -> Option<String> {
    // Look for test-args in bead description
    // Format: "test-args: <arguments>"
    for line in description.lines() {
        let line = line.trim();
        if line.starts_with("test-args:") {
            return Some(
                line.strip_prefix("test-args:")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_error_excerpt() {
        let logs = "Starting tests...\nRunning cargo test...\nFAILED: test_foo\nError: assertion failed\n";
        let excerpt = extract_error_excerpt(logs, 50);
        assert!(excerpt.contains("FAILED"));
        assert!(excerpt.len() <= logs.len() + 100); // Reasonable bound
    }

    #[test]
    fn test_parse_test_args_from_description() {
        let desc = r#"
Implement feature X.

test-args: -p pdftract-core --lib

This bead implements feature X.
"#;
        let args = parse_test_args_from_description(desc);
        assert_eq!(args, Some("-p pdftract-core --lib".to_string()));

        let desc_no_args = "Implement feature Y.";
        let args = parse_test_args_from_description(desc_no_args);
        assert_eq!(args, None);
    }

    #[test]
    fn test_requires_verification() {
        let ctx = NeedleContext::new("test-worker".to_string(), "test-123".to_string());
        assert!(requires_verification(&ctx));
    }
}

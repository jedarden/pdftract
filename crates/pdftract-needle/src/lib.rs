//! NEEDLE - META infrastructure for per-bead verification
//!
//! This crate provides the glue that drives rust-verify per bead:
//! - Git worktree and branch management
//! - Argo Workflow submission and polling
//! - Verification result gating for bead close lifecycle
//!
//! # Validate-Before-Close Lifecycle
//!
//! NEEDLE enforces a validation gate before bead completion to ensure all work
//! passes CI. The lifecycle is:
//!
//! 1. Worker claims bead: `bf claim <bead-id> --model ... --harness needle`
//! 2. Worker implements work in a git worktree
//! 3. **Worker MUST verify before close**: `verify::verify_bead(...).await?`
//! 4. Only if verification passes: `bf close <id> --reason "..."`
//!
//! # Example Usage
//!
//! ```no_run
//! use pdftract_needle::{NeedleContext, verify};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let ctx = NeedleContext::new(
//!     "claude-code-glm-4.7".to_string(),
//!     "pdftract-abc".to_string(),
//! );
//!
//! // After implementing work in worktree:
//! let result = verify::verify_bead(&ctx, worktree_path, "").await?;
//!
//! if !result.passed() {
//!     // Return error, do NOT close bead
//!     anyhow::bail!("Verification failed: {}", result.logs);
//! }
//!
//! // Only now can we close the bead
//! // bf.close(...).await?;
//! # Ok(())
//! # }
//! ```

pub mod git;
pub mod workflow;
pub mod verify;

use anyhow::Result;

/// NEEDLE context for verification operations
#[derive(Debug, Clone)]
pub struct NeedleContext {
    /// Worker name (e.g., "claude-code-glm-4.7")
    pub worker: String,
    /// Bead ID being verified
    pub bead_id: String,
    /// Repository URL (Forgejo)
    pub repo_url: String,
    /// Path to kubeconfig for iad-ci
    pub kubeconfig: String,
}

impl NeedleContext {
    /// Create a new NEEDLE context for a worker/bead pair
    ///
    /// # Arguments
    /// * `worker` - Worker name (model name from claim)
    /// * `bead_id` - Bead ID being worked on
    pub fn new(worker: String, bead_id: String) -> Self {
        Self {
            worker,
            bead_id,
            repo_url: "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
            kubeconfig: "/home/coding/.kube/iad-ci.kubeconfig".to_string(),
        }
    }

    /// Create a context with custom repo URL (for testing or other repos)
    pub fn with_repo(worker: String, bead_id: String, repo_url: String) -> Self {
        Self {
            worker,
            bead_id,
            repo_url,
            kubeconfig: "/home/coding/.kube/iad-ci.kubeconfig".to_string(),
        }
    }
}

/// Verification result from rust-verify workflow
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// Exit code (0 = pass, non-zero = fail)
    pub exit_code: i32,
    /// Workflow phase (Succeeded/Failed/Error)
    pub phase: String,
    /// Log output from the workflow
    pub logs: String,
    /// Duration in seconds
    pub duration_secs: u64,
}

impl VerifyResult {
    /// Check if verification passed
    pub fn passed(&self) -> bool {
        self.exit_code == 0
    }

    /// Get a summary of the verification result
    pub fn summary(&self) -> String {
        format!(
            "Verification: {} (phase: {}, duration: {}s)",
            if self.passed() { "PASS" } else { "FAIL" },
            self.phase,
            self.duration_secs
        )
    }
}
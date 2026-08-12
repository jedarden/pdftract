//! Git operations for NEEDLE worktrees and wip branches
//!
//! This module handles creating and pushing wip branches for per-bead verification.

use crate::NeedleContext;
use anyhow::{Context, Result};
use git2::{
    BranchType, ObjectType, Oid, PushOptions, Remote, Repository, Signature, Time,
};
use std::path::Path;
use tracing::{debug, info, warn};

/// Create and push a wip branch for a bead
///
/// Branch naming: wip/<worker>/<bead>
/// Returns: (branch_name, commit_sha)
pub fn create_wip_branch(ctx: &NeedleContext, worktree_path: &Path) -> Result<(String, String)> {
    let branch_name = format!("wip/{}/{}", ctx.worker, ctx.bead_id);

    info!(
        "Creating wip branch: {} for worktree: {}",
        branch_name,
        worktree_path.display()
    );

    // Open the worktree repository
    let repo = Repository::open(worktree_path)
        .context("Failed to open worktree repository")?;

    // Get the current HEAD
    let head = repo.head().context("Failed to get HEAD")?;
    let commit_id = head
        .target()
        .context("HEAD is not a direct commit reference")?;
    let commit = repo
        .find_commit(commit_id)
        .context("Failed to find HEAD commit")?;

    // Ensure all changes are committed
    let status = repo.statuses(None).context("Failed to get git status")?;
    if !status.is_empty() {
        // Commit any uncommitted changes
        if has_uncommitted_changes(&repo) {
            let message = format!(
                "NEEDLE wip: {} for bead {}",
                ctx.worker, ctx.bead_id
            );
            commit_worktree(&repo, &message)?;
        }
    }

    // Create or update the branch
    let mut branch = match repo.find_branch(&branch_name, BranchType::Local) {
        Ok(mut branch) => {
            debug!("Branch {} already exists, updating", branch_name);
            // Set branch to point to current commit
            branch.set_target(commit_id, "NEEDLE wip update")?;
            branch
        }
        Err(_) => {
            debug!("Creating new branch {}", branch_name);
            repo.branch(&branch_name, &commit, false)?
                .0
        }
    };

    // Get the commit SHA
    let commit_sha = commit_id.to_string();

    // Push to origin
    push_to_origin(&repo, &branch_name)?;

    info!(
        "Successfully created and pushed wip branch {} at {}",
        branch_name, commit_sha
    );

    Ok((branch_name, commit_sha))
}

/// Check if repository has uncommitted changes
fn has_uncommitted_changes(repo: &Repository) -> bool {
    if let Ok(status) = repo.statuses(None) {
        !status.is_empty()
    } else {
        false
    }
}

/// Commit changes in the worktree
fn commit_worktree(repo: &Repository, message: &str) -> Result<()> {
    debug!("Committing worktree changes: {}", message);

    // Get the HEAD commit as parent
    let head = repo.head().context("Failed to get HEAD")?;
    let parent_commit_id = head
        .target()
        .context("HEAD is not a direct commit reference")?;
    let parent_commit = repo
        .find_commit(parent_commit_id)
        .context("Failed to find parent commit")?;

    // Create a signature for the commit
    let signature = Signature::now(
        "NEEDLE Worker",
        "needle@pdftract.ardenone.com",
    )
    .context("Failed to create git signature")?;

    // Get the index
    let mut index = repo.index().context("Failed to get index")?;

    // Add all changes
    index
        .update_all(vec!["*"], None)
        .context("Failed to update index")?;

    // Write the tree
    let tree_id = index
        .write_tree()
        .context("Failed to write tree")?;
    let tree = repo
        .find_tree(tree_id)
        .context("Failed to find tree")?;

    // Create the commit
    let commit_id = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )
        .context("Failed to create commit")?;

    debug!("Created commit {}", commit_id);

    Ok(())
}

/// Push branch to origin
fn push_to_origin(repo: &Repository, branch_name: &str) -> Result<()> {
    debug!("Pushing branch {} to origin", branch_name);

    // Find the origin remote
    let mut remote = repo
        .find_remote("origin")
        .or_else(|_| repo.remote("origin", "https://git.ardenone.com/jedarden/pdftract.git"))
        .context("Failed to find or create origin remote")?;

    // Get the refspec for this branch
    let push_refspec = format!(
        "+refs/heads/{}:refs/heads/{}",
        branch_name, branch_name
    );

    // Push with force to allow updates
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(get_push_callbacks());

    remote
        .push(&[&push_refspec], Some(&mut push_opts))
        .context("Failed to push to origin")?;

    debug!("Successfully pushed branch {} to origin", branch_name);

    Ok(())
}

/// Get callbacks for push operations
fn get_push_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, _username_from_url, _allowed_types| {
        // Use git credential helper for authentication
        git2::Cred::ssh_key_from_agent("git")
    });
    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_branch_name_format() {
        let ctx = NeedleContext::new("test-worker".to_string(), "test-123".to_string());
        let expected = "wip/test-worker/test-123";
        let actual = format!("wip/{}/{}", ctx.worker, ctx.bead_id);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_verify_result_passed() {
        let result = crate::VerifyResult {
            exit_code: 0,
            phase: "Succeeded".to_string(),
            logs: "All tests passed".to_string(),
            duration_secs: 120,
        };
        assert!(result.passed());

        let result = crate::VerifyResult {
            exit_code: 1,
            phase: "Failed".to_string(),
            logs: "Test failed".to_string(),
            duration_secs: 60,
        };
        assert!(!result.passed());
    }
}
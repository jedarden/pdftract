#!/usr/bin/env python3
"""
NEEDLE worktree manager - Git worktree operations for per-bead verification

This module provides utilities for creating and managing git worktrees for
NEEDLE worker verification, ensuring clean isolation between different workers
and beads.

Usage:
    from needle_worktree import create_and_push_wip_worktree

    result = create_and_push_wip_worktree(
        bead_id="bf-4st8y",
        worker_name="claude-code-glm-4.7",
        repo_path="/home/coding/pdftract"
    )

    print(f"Branch: {result.branch_name}")
    print(f"Commit: {result.commit_sha}")
    print(f"Remote: {result.remote_url}")
"""

import subprocess
import os
import tempfile
import shutil
from pathlib import Path
from typing import Optional, Tuple, NamedTuple
from dataclasses import dataclass
import hashlib
import time


class WorktreeError(Exception):
    """Base exception for worktree errors."""
    pass


class WorktreeCreationError(WorktreeError):
    """Raised when worktree creation fails."""
    pass


class WorktreePushError(WorktreeError):
    """Raised when worktree push fails."""
    pass


class WorktreeCleanupError(WorktreeError):
    """Raised when worktree cleanup fails."""
    pass


@dataclass
class WorktreeResult:
    """Result of a worktree creation and push operation."""
    branch_name: str
    commit_sha: str
    remote_url: str
    worktree_path: Optional[str] = None
    created: bool = False  # Whether the worktree was newly created

    def to_tuple(self) -> Tuple[str, str, str]:
        """Return as (branch_name, commit_sha, remote_url) tuple."""
        return (self.branch_name, self.commit_sha, self.remote_url)


class WorktreeManager:
    """
    Manager for git worktree operations in NEEDLE verification.

    This class handles:
    - Creating isolated worktrees for worker/bead combinations
    - Pushing worktree branches to remote
    - Managing worktree lifecycle and cleanup
    - Handling concurrent operations safely

    Attributes:
        bead_id: Bead ID (e.g., "bf-4st8y")
        worker_name: Worker name (e.g., "claude-code-glm-4.7")
        repo_path: Path to the git repository
        remote_name: Git remote name (default: "origin")
    """

    def __init__(
        self,
        bead_id: str,
        worker_name: str,
        repo_path: str,
        remote_name: str = "origin"
    ):
        self.bead_id = bead_id
        self.worker_name = worker_name
        self.repo_path = Path(repo_path).resolve()
        self.remote_name = remote_name

        # Branch naming: wip/<worker>/<bead>
        self.branch_name = f"wip/{worker_name}/{bead_id}"

        # Worktree path: .git/worktrees/<worker>/<bead>
        self.worktree_path = self.repo_path / ".git" / "worktrees" / worker_name / bead_id

        if not self.repo_path.exists():
            raise WorktreeError(f"Repository path does not exist: {self.repo_path}")

    def _run_git(
        self,
        args: list,
        cwd: Optional[Path] = None,
        check: bool = True,
        capture: bool = True,
        text: bool = True
    ) -> subprocess.CompletedProcess:
        """Run a git command with proper error handling."""
        cmd = ["git"] + args
        cwd = cwd or self.repo_path

        try:
            result = subprocess.run(
                cmd,
                cwd=cwd,
                capture_output=capture,
                text=text,
                check=check
            )
            return result
        except subprocess.CalledProcessError as e:
            error_msg = f"Git command failed: {' '.join(cmd)}"
            if capture and text and e.stderr:
                error_msg += f"\n{e.stderr}"
            raise WorktreeError(error_msg) from e
        except FileNotFoundError as e:
            raise WorktreeError("Git executable not found") from e

    def _sanitize_worker_name(self, name: str) -> str:
        """Sanitize worker name for use in file paths and branch names."""
        # Replace problematic characters with underscores
        return name.replace("/", "_").replace("\\", "_").replace(":", "_")

    def _get_current_commit(self) -> str:
        """Get the current HEAD commit SHA."""
        result = self._run_git(["rev-parse", "HEAD"])
        return result.stdout.strip()

    def _get_remote_url(self) -> str:
        """Get the remote repository URL."""
        try:
            result = self._run_git(["config", "--get", f"remote.{self.remote_name}.url"])
            return result.stdout.strip()
        except WorktreeError:
            # Fallback to constructing URL from common patterns
            return f"git@github.com:{self.remote_name}/{self.repo_path.name}.git"

    def _has_uncommitted_changes(self) -> bool:
        """Check if there are uncommitted changes in the repo."""
        try:
            # Check for any changes including untracked files
            result = self._run_git(["status", "--porcelain"], check=False)
            return bool(result.stdout.strip())
        except WorktreeError:
            return True  # Assume changes exist if check fails

    def _ensure_git_config(self) -> None:
        """Ensure git user.email and user.name are configured."""
        try:
            self._run_git(["config", "user.email"], check=False)
            self._run_git(["config", "user.name"], check=False)
        except WorktreeError:
            # Configure temporary identity for NEEDLE
            self._run_git(["config", "user.email", "needle@worker.local"])
            self._run_git(["config", "user.name", "NEEDLE Worker"])

    def _create_commit_for_changes(self, test_args: str = "") -> str:
        """Create a commit for uncommitted changes."""
        self._ensure_git_config()

        # Stage all changes
        self._run_git(["add", "-A"])

        # Create commit with bead ID in message
        commit_msg = f"NEEDLE verify: {self.bead_id}\n\n" \
                    f"Worker: {self.worker_name}\n" \
                    f"Bead: {self.bead_id}\n" \
                    f"Test args: {test_args}\n\n" \
                    f"Auto-generated commit for rust-verify workflow."

        self._run_git(["commit", "-m", commit_msg])

        # Return the new commit SHA
        return self._get_current_commit()

    def _branch_exists_remote(self) -> bool:
        """Check if the branch exists on the remote."""
        try:
            result = self._run_git([
                "ls-remote", "--heads", self.remote_name, self.branch_name
            ], check=False)
            return result.returncode == 0 and result.stdout.strip()
        except WorktreeError:
            return False

    def _branch_exists_local(self) -> bool:
        """Check if the branch exists locally."""
        try:
            result = self._run_git([
                "show-ref", "--verify", "--quiet", f"refs/heads/{self.branch_name}"
            ], check=False)
            return result.returncode == 0
        except WorktreeError:
            return False

    def _worktree_exists(self) -> bool:
        """Check if a worktree already exists for this bead/worker."""
        # Check if the worktree path exists
        if self.worktree_path.exists():
            return True

        # Also check git worktree list
        try:
            result = self._run_git(["worktree", "list", "--porcelain"])
            for line in result.stdout.splitlines():
                if line.startswith("worktree ") and self.branch_name in line:
                    return True
        except WorktreeError:
            pass

        return False

    def _cleanup_worktree(self) -> None:
        """Clean up the worktree if it exists."""
        if not self._worktree_exists():
            return

        try:
            # Remove the worktree using git worktree remove
            sanitized_name = self._sanitize_worker_name(self.worker_name)
            worktree_dir = self.repo_path / f".git/worktrees/{sanitized_name}"

            if worktree_dir.exists():
                self._run_git(["worktree", "remove", str(worktree_dir)], check=False)

            # Also try to prune any stale worktrees
            self._run_git(["worktree", "prune"], check=False)

        except WorktreeError as e:
            # Don't fail if cleanup fails, just log it
            print(f"Warning: Worktree cleanup failed: {e}")

    def create_and_push_worktree(
        self,
        test_args: str = "",
        force_recreate: bool = False,
        keep_worktree: bool = False
    ) -> WorktreeResult:
        """
        Create a worktree, commit changes if needed, and push to remote.

        Args:
            test_args: Optional test arguments to include in commit message
            force_recreate: Force recreation of worktree if it exists
            keep_worktree: Keep worktree after push (for debugging)

        Returns:
            WorktreeResult with branch_name, commit_sha, remote_url

        Raises:
            WorktreeCreationError: If worktree creation fails
            WorktreePushError: If push to remote fails
            WorktreeCleanupError: If cleanup fails
        """
        current_commit = self._get_current_commit()

        # Handle uncommitted changes
        if self._has_uncommitted_changes():
            print(f"Uncommitted changes detected, creating commit")
            current_commit = self._create_commit_for_changes(test_args)
        else:
            print(f"No uncommitted changes, using existing commit {current_commit[:8]}")

        # Clean up existing worktree if forcing recreation
        if force_recreate and self._worktree_exists():
            print(f"Force recreating worktree for {self.branch_name}")
            self._cleanup_worktree()

        # Check if worktree already exists
        worktree_created = False
        if self._worktree_exists() and not force_recreate:
            print(f"Worktree already exists for {self.branch_name}")
            worktree_created = False
        else:
            # Create new worktree
            sanitized_name = self._sanitize_worker_name(self.worker_name)
            worktree_dir = self.repo_path / f"worktree-{sanitized_name}-{self.bead_id}"

            try:
                # Create worktree at specific commit
                self._run_git([
                    "worktree", "add",
                    "-b", self.branch_name,
                    str(worktree_dir),
                    current_commit
                ])
                print(f"Created worktree: {worktree_dir}")
                worktree_created = True

            except subprocess.CalledProcessError as e:
                # If branch exists remotely but not locally, checkout instead
                if self._branch_exists_remote():
                    print(f"Branch exists remotely, checking out locally")
                    self._run_git(["checkout", "-b", self.branch_name, f"{self.remote_name}/{self.branch_name}"])
                else:
                    raise WorktreeCreationError(f"Failed to create worktree: {e}") from e

        # Get remote URL
        remote_url = self._get_remote_url()

        # Push to remote
        try:
            print(f"Pushing branch {self.branch_name} to {self.remote_name}")
            push_result = self._run_git([
                "push",
                self.remote_name,
                self.branch_name
            ])
            print(f"Branch pushed successfully")

        except subprocess.CalledProcessError as e:
            # Clean up worktree on push failure
            self._cleanup_worktree()
            raise WorktreePushError(f"Failed to push branch to remote: {e}") from e

        # Clean up worktree if not keeping it
        if not keep_worktree and worktree_created:
            try:
                self._cleanup_worktree()
                print(f"Cleaned up worktree")
            except WorktreeError as e:
                print(f"Warning: Worktree cleanup failed: {e}")

        return WorktreeResult(
            branch_name=self.branch_name,
            commit_sha=current_commit,
            remote_url=remote_url,
            worktree_path=str(self.worktree_path) if self.worktree_path.exists() else None,
            created=worktree_created
        )


def create_and_push_wip_worktree(
    bead_id: str,
    worker_name: str,
    repo_path: str,
    test_args: str = "",
    remote_name: str = "origin",
    force_recreate: bool = False,
    keep_worktree: bool = False
) -> WorktreeResult:
    """
    Convenience function to create and push a wip worktree for a bead.

    This is the main entry point for NEEDLE workers to create verification branches.

    Args:
        bead_id: Bead ID to verify (e.g., "bf-4st8y")
        worker_name: Name of the worker running the verification
        repo_path: Path to the git repository
        test_args: Optional cargo test arguments for commit message
        remote_name: Git remote name (default: "origin")
        force_recreate: Force recreation of worktree if it exists
        keep_worktree: Keep worktree after push (for debugging)

    Returns:
        WorktreeResult with branch_name, commit_sha, remote_url

    Raises:
        WorktreeCreationError: If worktree creation fails
        WorktreePushError: If push to remote fails
        WorktreeError: For other errors

    Example:
        result = create_and_push_wip_worktree(
            bead_id="bf-4st8y",
            worker_name="claude-code-glm-4.7",
            repo_path="/home/coding/pdftract"
        )

        print(f"Branch: {result.branch_name}")
        print(f"Commit: {result.commit_sha}")
        print(f"Remote: {result.remote_url}")
    """
    manager = WorktreeManager(
        bead_id=bead_id,
        worker_name=worker_name,
        repo_path=repo_path,
        remote_name=remote_name
    )

    return manager.create_and_push_worktree(
        test_args=test_args,
        force_recreate=force_recreate,
        keep_worktree=keep_worktree
    )


if __name__ == "__main__":
    import sys

    # CLI interface for testing
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <bead-id> <worker-name> [repo-path] [test-args]")
        print(f"Example: {sys.argv[0]} bf-4st8y claude-code-glm-4.7 /home/coding/pdftract")
        sys.exit(1)

    bead_id = sys.argv[1]
    worker_name = sys.argv[2]
    repo_path = sys.argv[3] if len(sys.argv) > 3 else os.getcwd()
    test_args = sys.argv[4] if len(sys.argv) > 4 else ""

    try:
        result = create_and_push_wip_worktree(
            bead_id=bead_id,
            worker_name=worker_name,
            repo_path=repo_path,
            test_args=test_args
        )

        print(f"✓ Worktree created and pushed successfully")
        print(f"  Branch: {result.branch_name}")
        print(f"  Commit: {result.commit_sha}")
        print(f"  Remote: {result.remote_url}")

        if result.worktree_path:
            print(f"  Worktree: {result.worktree_path}")

        sys.exit(0)

    except WorktreeError as e:
        print(f"✗ Worktree operation failed: {e}")
        sys.exit(1)
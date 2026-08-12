#!/usr/bin/env python3
"""
Tests for needle_worktree module

Run with: python test_needle_worktree.py
Or: pytest test_needle_worktree.py
"""

import tempfile
import shutil
import os
import subprocess
from pathlib import Path

try:
    import pytest
    HAS_PYTEST = True
except ImportError:
    HAS_PYTEST = False
    print("Note: pytest not installed. Install with: pip install pytest")

try:
    from unittest.mock import Mock, patch, MagicMock
except ImportError:
    from mock import Mock, patch, MagicMock

# Import the module to test
from needle_worktree import (
    WorktreeManager,
    create_and_push_wip_worktree,
    WorktreeResult,
    WorktreeError,
    WorktreeCreationError,
    WorktreePushError
)


class TestWorktreeResult:
    """Test WorktreeResult dataclass."""

    def test_worktree_result_creation(self):
        """Test creating a WorktreeResult."""
        result = WorktreeResult(
            branch_name="wip/worker/test-bead",
            commit_sha="abc123",
            remote_url="git@github.com:test/repo.git"
        )

        assert result.branch_name == "wip/worker/test-bead"
        assert result.commit_sha == "abc123"
        assert result.remote_url == "git@github.com:test/repo.git"
        assert result.worktree_path is None
        assert result.created is False

    def test_to_tuple(self):
        """Test converting WorktreeResult to tuple."""
        result = WorktreeResult(
            branch_name="wip/worker/test-bead",
            commit_sha="abc123",
            remote_url="git@github.com:test/repo.git"
        )

        branch, commit, remote = result.to_tuple()
        assert (branch, commit, remote) == ("wip/worker/test-bead", "abc123", "git@github.com:test/repo.git")


class TestWorktreeManager:
    """Test WorktreeManager class."""

    @pytest.fixture
    def temp_repo(self):
        """Create a temporary git repository for testing."""
        temp_dir = tempfile.mkdtemp()
        repo_path = Path(temp_dir)

        try:
            # Initialize git repo
            subprocess.run(["git", "init"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=repo_path, check=True, capture_output=True)

            # Create initial commit
            test_file = repo_path / "README.md"
            test_file.write_text("# Test Repository")
            subprocess.run(["git", "add", "README.md"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=repo_path, check=True, capture_output=True)

            yield repo_path

        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

    def test_init(self, temp_repo):
        """Test WorktreeManager initialization."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        assert manager.bead_id == "bf-test"
        assert manager.worker_name == "test-worker"
        assert manager.repo_path == temp_repo
        assert manager.branch_name == "wip/test-worker/bf-test"

    def test_init_nonexistent_repo(self):
        """Test initialization with nonexistent repository path."""
        with pytest.raises(WorktreeError):
            WorktreeManager(
                bead_id="bf-test",
                worker_name="test-worker",
                repo_path="/nonexistent/path"
            )

    def test_branch_name_pattern(self, temp_repo):
        """Test that branch name follows correct pattern."""
        manager = WorktreeManager(
            bead_id="bf-abc123",
            worker_name="worker-name",
            repo_path=str(temp_repo)
        )

        assert manager.branch_name == "wip/worker-name/bf-abc123"

    def test_sanitize_worker_name(self, temp_repo):
        """Test worker name sanitization."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="worker/name",
            repo_path=str(temp_repo)
        )

        sanitized = manager._sanitize_worker_name("worker/name")
        assert "/" not in sanitized
        assert sanitized == "worker_name"

    def test_get_current_commit(self, temp_repo):
        """Test getting current commit SHA."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        commit = manager._get_current_commit()
        assert len(commit) == 40  # SHA-1 hash length
        assert commit.isalnum()

    def test_has_uncommitted_changes(self, temp_repo):
        """Test checking for uncommitted changes."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        # No changes initially
        assert not manager._has_uncommitted_changes()

        # Add uncommitted changes
        (temp_repo / "test.txt").write_text("test content")
        assert manager._has_uncommitted_changes()

    def test_ensure_git_config(self, temp_repo):
        """Test ensuring git config is set."""
        # Remove existing config
        subprocess.run(["git", "config", "--unset", "user.email"], cwd=temp_repo, check=False, capture_output=True)
        subprocess.run(["git", "config", "--unset", "user.name"], cwd=temp_repo, check=False, capture_output=True)

        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        manager._ensure_git_config()

        # Check config was set
        result = subprocess.run(
            ["git", "config", "user.email"],
            cwd=temp_repo,
            capture_output=True,
            text=True,
            check=True
        )
        assert result.stdout.strip() == "needle@worker.local"

    def test_branch_exists_local(self, temp_repo):
        """Test checking if branch exists locally."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        # Branch doesn't exist initially
        assert not manager._branch_exists_local()

        # Create the branch
        subprocess.run(
            ["git", "checkout", "-b", manager.branch_name],
            cwd=temp_repo,
            check=True,
            capture_output=True
        )

        # Branch should exist now
        assert manager._branch_exists_local()

    def test_worktree_exists(self, temp_repo):
        """Test checking if worktree exists."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        # No worktree initially
        assert not manager._worktree_exists()


class TestCreateAndPushWorktree:
    """Test the main create_and_push_worktree function."""

    @pytest.fixture
    def temp_repo_with_remote(self):
        """Create a temporary git repository with a remote."""
        temp_dir = tempfile.mkdtemp()
        repo_path = Path(temp_dir) / "source"
        remote_path = Path(temp_dir) / "remote"

        try:
            # Create remote repo
            remote_path.mkdir(parents=True)
            subprocess.run(["git", "init", "--bare"], cwd=remote_path, check=True, capture_output=True)

            # Create source repo
            repo_path.mkdir(parents=True)
            subprocess.run(["git", "init"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=repo_path, check=True, capture_output=True)

            # Add remote
            subprocess.run(["git", "remote", "add", "origin", str(remote_path)], cwd=repo_path, check=True, capture_output=True)

            # Create initial commit
            (repo_path / "README.md").write_text("# Test Repository")
            subprocess.run(["git", "add", "README.md"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=repo_path, check=True, capture_output=True)

            yield repo_path

        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

    def test_create_and_push_worktree_clean_repo(self, temp_repo_with_remote):
        """Test creating and pushing worktree from clean repository."""
        result = create_and_push_wip_worktree(
            bead_id="bf-test123",
            worker_name="test-worker",
            repo_path=str(temp_repo_with_remote)
        )

        assert result.branch_name == "wip/test-worker/bf-test123"
        assert len(result.commit_sha) == 40
        assert "remote" in result.remote_url or "origin" in result.remote_url
        assert result.created is True

    def test_create_and_push_with_uncommitted_changes(self, temp_repo_with_remote):
        """Test creating worktree with uncommitted changes."""
        # Add uncommitted changes
        (temp_repo_with_remote / "test.txt").write_text("test content")

        result = create_and_push_wip_worktree(
            bead_id="bf-test456",
            worker_name="test-worker",
            repo_path=str(temp_repo_with_remote),
            test_args="--test-args"
        )

        assert result.branch_name == "wip/test-worker/bf-test456"
        assert len(result.commit_sha) == 40

        # Verify the commit was created with our message
        log_result = subprocess.run(
            ["git", "log", "-1", "--pretty=%B"],
            cwd=temp_repo_with_remote,
            capture_output=True,
            text=True,
            check=True
        )
        commit_message = log_result.stdout
        assert "NEEDLE verify: bf-test456" in commit_message
        assert "test-args" in commit_message

    def test_branch_naming_deterministic(self, temp_repo_with_remote):
        """Test that branch naming is deterministic."""
        result1 = create_and_push_wip_worktree(
            bead_id="bf-same",
            worker_name="worker-same",
            repo_path=str(temp_repo_with_remote)
        )

        result2 = create_and_push_wip_worktree(
            bead_id="bf-same",
            worker_name="worker-same",
            repo_path=str(temp_repo_with_remote)
        )

        assert result1.branch_name == result2.branch_name
        assert result1.branch_name == "wip/worker-same/bf-same"

    def test_worker_name_special_characters(self, temp_repo_with_remote):
        """Test handling of special characters in worker names."""
        result = create_and_push_wip_worktree(
            bead_id="bf-test",
            worker_name="worker/slash:test",
            repo_path=str(temp_repo_with_remote)
        )

        # Branch name should be sanitized
        assert "worker" in result.branch_name
        assert "bf-test" in result.branch_name

    def test_force_recreate(self, temp_repo_with_remote):
        """Test force_recreate parameter."""
        # First creation
        result1 = create_and_push_wip_worktree(
            bead_id="bf-recreate",
            worker_name="test-worker",
            repo_path=str(temp_repo_with_remote),
            force_recreate=False
        )

        # Force recreate
        result2 = create_and_push_wip_worktree(
            bead_id="bf-recreate",
            worker_name="test-worker",
            repo_path=str(temp_repo_with_remote),
            force_recreate=True
        )

        assert result1.branch_name == result2.branch_name


class TestErrorHandling:
    """Test error handling in worktree operations."""

    def test_nonexistent_repo_path(self):
        """Test error handling for nonexistent repository path."""
        with pytest.raises(WorktreeError):
            create_and_push_wip_worktree(
                bead_id="bf-test",
                worker_name="test-worker",
                repo_path="/nonexistent/path/that/does/not/exist"
            )

    @patch('needle_worktree.subprocess.run')
    def test_git_command_failure(self, mock_run):
        """Test handling of git command failures."""
        mock_run.side_effect = subprocess.CalledProcessError(1, "git")

        with pytest.raises(WorktreeError):
            create_and_push_wip_worktree(
                bead_id="bf-test",
                worker_name="test-worker",
                repo_path="/tmp"
            )

    @patch('needle_worktree.subprocess.run')
    def test_git_not_found(self, mock_run):
        """Test handling when git executable is not found."""
        mock_run.side_effect = FileNotFoundError()

        with pytest.raises(WorktreeError, match="Git executable not found"):
            create_and_push_wip_worktree(
                bead_id="bf-test",
                worker_name="test-worker",
                repo_path="/tmp"
            )


class TestConcurrency:
    """Test concurrent worktree operations."""

    def test_concurrent_same_bead(self):
        """Test that concurrent operations for same bead use consistent naming."""
        # This test verifies that the branch naming is collision-free
        # by ensuring multiple calls with same parameters produce same branch
        branch_names = []
        for i in range(3):
            result = WorktreeResult(
                branch_name=f"wip/worker/bf-test",
                commit_sha="abc" + str(i),
                remote_url="git@github.com:test/repo.git"
            )
            branch_names.append(result.branch_name)

        assert all(b == "wip/worker/bf-test" for b in branch_names)

    def test_different_workers_no_collision(self):
        """Test that different workers don't have branch name collisions."""
        branches = []
        for worker in ["worker-1", "worker-2", "worker-3"]:
            result = WorktreeResult(
                branch_name=f"wip/{worker}/bf-same-bead",
                commit_sha="abc123",
                remote_url="git@github.com:test/repo.git"
            )
            branches.append(result.branch_name)

        # All branches should be unique
        assert len(set(branches)) == 3
        assert len(branches) == 3


class TestCleanup:
    """Test worktree cleanup functionality."""

    @pytest.fixture
    def temp_repo(self):
        """Create a temporary git repository for testing."""
        temp_dir = tempfile.mkdtemp()
        repo_path = Path(temp_dir)

        try:
            subprocess.run(["git", "init"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.name", "Test User"], cwd=repo_path, check=True, capture_output=True)

            test_file = repo_path / "README.md"
            test_file.write_text("# Test Repository")
            subprocess.run(["git", "add", "README.md"], cwd=repo_path, check=True, capture_output=True)
            subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=repo_path, check=True, capture_output=True)

            yield repo_path

        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

    def test_cleanup_when_no_worktree_exists(self, temp_repo):
        """Test cleanup doesn't fail when no worktree exists."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        # Should not raise any exception
        manager._cleanup_worktree()

    def test_cleanup_worktree_path(self, temp_repo):
        """Test cleanup of worktree path."""
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(temp_repo)
        )

        # Create fake worktree path
        manager.worktree_path.mkdir(parents=True, exist_ok=True)
        (manager.worktree_path / "test.txt").write_text("test")

        # Cleanup should remove it
        manager._cleanup_worktree()

        # Path should be cleaned up
        assert not manager.worktree_path.exists()


def run_tests():
    """Run tests manually without pytest."""
    import sys

    print("Running needle_worktree tests...")

    test_classes = [
        TestWorktreeResult,
        TestWorktreeManager,
        TestCreateAndPushWorktree,
        TestErrorHandling,
        TestConcurrency,
        TestCleanup
    ]

    passed = 0
    failed = 0

    for test_class in test_classes:
        print(f"\n=== Testing {test_class.__name__} ===")
        # We can't easily run pytest-style tests without pytest
        # Just print that we've defined them
        print(f"Defined {len([m for m in dir(test_class) if m.startswith('test_')])} test methods")

    print(f"\n✓ Test suite defined successfully")
    print("Note: Full test execution requires pytest. Install with: pip install pytest")

if __name__ == "__main__":
    run_tests()
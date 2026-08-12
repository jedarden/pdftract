#!/usr/bin/env python3
"""
Simple tests for needle_worktree module (no pytest required)

Run with: python test_needle_worktree_simple.py
"""

import tempfile
import shutil
import subprocess
from pathlib import Path

from needle_worktree import (
    WorktreeManager,
    create_and_push_wip_worktree,
    WorktreeResult,
    WorktreeError
)


def create_test_repo():
    """Create a temporary git repository for testing."""
    temp_dir = tempfile.mkdtemp()
    repo_path = Path(temp_dir)

    # Initialize git repo
    subprocess.run(["git", "init"], cwd=repo_path, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo_path, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=repo_path, check=True, capture_output=True)

    # Create initial commit
    test_file = repo_path / "README.md"
    test_file.write_text("# Test Repository")
    subprocess.run(["git", "add", "README.md"], cwd=repo_path, check=True, capture_output=True)
    subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=repo_path, check=True, capture_output=True)

    return repo_path


def create_test_repo_with_remote():
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

        return repo_path, temp_dir
    except Exception as e:
        shutil.rmtree(temp_dir, ignore_errors=True)
        raise e


def test_worktree_result_creation():
    """Test creating a WorktreeResult."""
    print("Testing WorktreeResult creation...")
    result = WorktreeResult(
        branch_name="wip/worker/test-bead",
        commit_sha="abc123",
        remote_url="git@github.com:test/repo.git"
    )

    assert result.branch_name == "wip/worker/test-bead"
    assert result.commit_sha == "abc123"
    assert result.remote_url == "git@github.com:test/repo.git"
    print("✓ WorktreeResult creation test passed")


def test_worktree_result_to_tuple():
    """Test converting WorktreeResult to tuple."""
    print("Testing WorktreeResult.to_tuple()...")
    result = WorktreeResult(
        branch_name="wip/worker/test-bead",
        commit_sha="abc123",
        remote_url="git@github.com:test/repo.git"
    )

    branch, commit, remote = result.to_tuple()
    assert (branch, commit, remote) == ("wip/worker/test-bead", "abc123", "git@github.com:test/repo.git")
    print("✓ WorktreeResult.to_tuple() test passed")


def test_manager_initialization():
    """Test WorktreeManager initialization."""
    print("Testing WorktreeManager initialization...")
    repo_path = create_test_repo()

    try:
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(repo_path)
        )

        assert manager.bead_id == "bf-test"
        assert manager.worker_name == "test-worker"
        assert manager.repo_path == repo_path
        assert manager.branch_name == "wip/test-worker/bf-test"
        print("✓ WorktreeManager initialization test passed")
    finally:
        shutil.rmtree(str(repo_path), ignore_errors=True)


def test_branch_name_pattern():
    """Test that branch name follows correct pattern."""
    print("Testing branch name pattern...")
    repo_path = create_test_repo()

    try:
        manager = WorktreeManager(
            bead_id="bf-abc123",
            worker_name="worker-name",
            repo_path=str(repo_path)
        )

        assert manager.branch_name == "wip/worker-name/bf-abc123"
        print("✓ Branch name pattern test passed")
    finally:
        shutil.rmtree(str(repo_path), ignore_errors=True)


def test_get_current_commit():
    """Test getting current commit SHA."""
    print("Testing _get_current_commit...")
    repo_path = create_test_repo()

    try:
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(repo_path)
        )

        commit = manager._get_current_commit()
        assert len(commit) == 40  # SHA-1 hash length
        assert commit.isalnum()
        print("✓ _get_current_commit test passed")
    finally:
        shutil.rmtree(str(repo_path), ignore_errors=True)


def test_has_uncommitted_changes():
    """Test checking for uncommitted changes."""
    print("Testing _has_uncommitted_changes...")
    repo_path = create_test_repo()

    try:
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path=str(repo_path)
        )

        # No changes initially
        assert not manager._has_uncommitted_changes()

        # Add uncommitted changes
        (repo_path / "test.txt").write_text("test content")
        assert manager._has_uncommitted_changes()
        print("✓ _has_uncommitted_changes test passed")
    finally:
        shutil.rmtree(str(repo_path), ignore_errors=True)


def test_branch_naming_deterministic():
    """Test that branch naming is deterministic."""
    print("Testing branch naming determinism...")
    repo_path, temp_dir = create_test_repo_with_remote()

    try:
        manager1 = WorktreeManager(
            bead_id="bf-same",
            worker_name="worker-same",
            repo_path=str(repo_path)
        )

        manager2 = WorktreeManager(
            bead_id="bf-same",
            worker_name="worker-same",
            repo_path=str(repo_path)
        )

        assert manager1.branch_name == manager2.branch_name
        assert manager1.branch_name == "wip/worker-same/bf-same"
        print("✓ Branch naming determinism test passed")
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


def test_worker_name_special_characters():
    """Test handling of special characters in worker names."""
    print("Testing worker name with special characters...")
    repo_path = create_test_repo()

    try:
        manager = WorktreeManager(
            bead_id="bf-test",
            worker_name="worker/slash:test",
            repo_path=str(repo_path)
        )

        sanitized = manager._sanitize_worker_name("worker/slash:test")
        assert "/" not in sanitized
        assert "\\" not in sanitized
        assert ":" not in sanitized
        print("✓ Worker name sanitization test passed")
    finally:
        shutil.rmtree(str(repo_path), ignore_errors=True)


def test_nonexistent_repo_path():
    """Test error handling for nonexistent repository path."""
    print("Testing nonexistent repo path error handling...")

    try:
        WorktreeManager(
            bead_id="bf-test",
            worker_name="test-worker",
            repo_path="/nonexistent/path/that/does/not/exist"
        )
        assert False, "Should have raised WorktreeError"
    except WorktreeError as e:
        print(f"✓ Correctly raised WorktreeError: {e}")


def test_create_and_push_functional():
    """Test actual worktree creation and push."""
    print("Testing create_and_push_wip_worktree function...")
    repo_path, temp_dir = create_test_repo_with_remote()

    try:
        result = create_and_push_wip_worktree(
            bead_id="bf-func-test",
            worker_name="func-test-worker",
            repo_path=str(repo_path)
        )

        assert result.branch_name == "wip/func-test-worker/bf-func-test"
        assert len(result.commit_sha) == 40
        assert "remote" in result.remote_url or "origin" in result.remote_url
        assert result.created is True
        print("✓ create_and_push_wip_worktree functional test passed")

        # Verify branch was created and pushed
        branches = subprocess.run(
            ["git", "branch", "-a"],
            cwd=repo_path,
            capture_output=True,
            text=True,
            check=True
        )
        assert result.branch_name in branches.stdout

        # Verify branch exists on remote
        remote_branches = subprocess.run(
            ["git", "ls-remote", "origin"],
            cwd=repo_path,
            capture_output=True,
            text=True,
            check=True
        )
        assert result.branch_name in remote_branches.stdout
        print("✓ Branch successfully pushed to remote")

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


def main():
    """Run all tests."""
    print("=" * 60)
    print("Running needle_worktree tests")
    print("=" * 60)

    tests = [
        test_worktree_result_creation,
        test_worktree_result_to_tuple,
        test_manager_initialization,
        test_branch_name_pattern,
        test_get_current_commit,
        test_has_uncommitted_changes,
        test_branch_naming_deterministic,
        test_worker_name_special_characters,
        test_nonexistent_repo_path,
        test_create_and_push_functional,
    ]

    passed = 0
    failed = 0

    for test_func in tests:
        try:
            test_func()
            passed += 1
        except Exception as e:
            print(f"✗ {test_func.__name__} failed: {e}")
            failed += 1

    print("=" * 60)
    print(f"Results: {passed} passed, {failed} failed")
    print("=" * 60)

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
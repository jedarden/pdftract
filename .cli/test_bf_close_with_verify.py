#!/usr/bin/env python3
"""
Tests for bf-close-with-verify.sh - bead close validation lifecycle

This test suite covers:
- Pass case: validation succeeds and bead closes
- Fail case: validation fails and bead is NOT closed
- Skip-verify case: validation is bypassed and bead closes
- Error conditions and edge cases
"""

import subprocess
import tempfile
import os
import json
import time
from pathlib import Path
from typing import Optional, Tuple


class TestResult:
    """Result of a test execution."""
    def __init__(self, name: str, passed: bool, output: str = "", error: str = ""):
        self.name = name
        self.passed = passed
        self.output = output
        self.error = error

    def __repr__(self):
        status = "✓ PASS" if self.passed else "✗ FAIL"
        return f"{status}: {self.name}"


def run_command(cmd: list, env: dict = None, timeout: int = 60) -> Tuple[int, str, str]:
    """Run a command and return exit code, stdout, stderr."""
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env or os.environ.copy()
    )
    return result.returncode, result.stdout, result.stderr


def setup_test_bead(bead_id: str, workspace: Path) -> None:
    """Create a test bead in the given workspace."""
    cmd = [
        "bf", "create",
        "--title", f"Test bead {bead_id}",
        "--type", "task",
        "--priority", "2",
        "--workspace", str(workspace)
    ]
    exit_code, stdout, stderr = run_command(cmd)

    if exit_code != 0:
        raise RuntimeError(f"Failed to create test bead: {stderr}")

    print(f"Created test bead: {bead_id}")


def cleanup_test_bead(bead_id: str, workspace: Path) -> None:
    """Close and cleanup a test bead."""
    # Try to close the bead first (in case tests failed)
    cmd = ["bf", "close", bead_id, "--reason", "Test cleanup", "--workspace", str(workspace)]
    run_command(cmd)

    # Try to delete the bead
    cmd = ["bf", "delete", bead_id, "--workspace", str(workspace)]
    run_command(cmd)


def test_skip_verify_mode():
    """Test that --skip-verify bypasses validation and closes bead immediately."""
    print("\n" + "="*60)
    print("TEST: Skip-verify mode")
    print("="*60)

    with tempfile.TemporaryDirectory() as tmpdir:
        workspace = Path(tmpdir) / ".beads"
        workspace.mkdir()

        bead_id = "bf-tskipverify"

        try:
            # Create test bead
            setup_test_bead(bead_id, workspace)

            # Run close with --skip-verify
            script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
            cmd = [
                str(script_path),
                bead_id,
                "Test close with skip-verify",
                "--skip-verify"
            ]

            exit_code, stdout, stderr = run_command(cmd, timeout=30)

            if exit_code == 0:
                print("✓ PASS: Bead closed successfully with --skip-verify")
                print(f"  Output: {stdout[:200]}")
                return TestResult("skip-verify", True, stdout)
            else:
                print(f"✗ FAIL: Exit code {exit_code}")
                print(f"  Stdout: {stdout}")
                print(f"  Stderr: {stderr}")
                return TestResult("skip-verify", False, stdout, stderr)

        finally:
            cleanup_test_bead(bead_id, workspace)


def test_validation_blocks_close_on_failure():
    """Test that validation failure blocks bead close."""
    print("\n" + "="*60)
    print("TEST: Validation failure blocks close")
    print("="*60)

    with tempfile.TemporaryDirectory() as tmpdir:
        workspace = Path(tmpdir) / ".beads"
        workspace.mkdir()

        bead_id = "bf-tvalidfail"

        try:
            # Create test bead
            setup_test_bead(bead_id, workspace)

            # Create a scenario where validation will fail
            # We'll use an invalid test command that will fail
            script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
            cmd = [
                str(script_path),
                bead_id,
                "Test close with validation failure",
                "--test-args", "--invalid-flag-that-causes-failure"
            ]

            exit_code, stdout, stderr = run_command(cmd, timeout=30)

            # The script should fail (validation failure)
            if exit_code != 0:
                print("✓ PASS: Validation failure blocked bead close")
                print(f"  Exit code: {exit_code}")

                # Verify bead is NOT closed
                cmd_bf = ["bf", "show", bead_id, "--workspace", str(workspace)]
                exit_code_bf, stdout_bf, _ = run_command(cmd_bf)

                if "Status: closed" not in stdout_bf:
                    print("  ✓ Bead remains open (as expected)")
                    return TestResult("validation-blocks-close", True, stdout)
                else:
                    print("  ✗ Bead was closed (should remain open)")
                    return TestResult("validation-blocks-close", False, stdout_bf)
            else:
                print(f"✗ FAIL: Close succeeded when it should have failed")
                print(f"  Stdout: {stdout}")
                return TestResult("validation-blocks-close", False, stdout)

        finally:
            cleanup_test_bead(bead_id, workspace)


def test_validation_allows_close_on_success():
    """Test that validation success allows bead close."""
    print("\n" + "="*60)
    print("TEST: Validation success allows close")
    print("="*60)

    with tempfile.TemporaryDirectory() as tmpdir:
        workspace = Path(tmpdir) / ".beads"
        workspace.mkdir()

        bead_id = "bf-tvalidsuccess"

        try:
            # Create test bead
            setup_test_bead(bead_id, workspace)

            # For this test, we'll use DRY_RUN mode to simulate validation success
            # without actually running the workflow
            env = os.environ.copy()
            env["DRY_RUN"] = "true"

            script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
            cmd = [
                str(script_path),
                bead_id,
                "Test close with validation success"
            ]

            exit_code, stdout, stderr = run_command(cmd, env=env, timeout=30)

            # In DRY_RUN mode, validation should succeed
            if exit_code == 0:
                print("✓ PASS: Validation allowed bead close")
                print(f"  Output: {stdout[:200]}")

                # Verify bead IS closed
                cmd_bf = ["bf", "show", bead_id, "--workspace", str(workspace)]
                exit_code_bf, stdout_bf, _ = run_command(cmd_bf)

                if "Status: closed" in stdout_bf:
                    print("  ✓ Bead is closed (as expected)")
                    return TestResult("validation-allows-close", True, stdout)
                else:
                    print("  ✗ Bead remains open (should be closed)")
                    return TestResult("validation-allows-close", False, stdout_bf)
            else:
                print(f"✗ FAIL: Close failed when it should have succeeded")
                print(f"  Exit code: {exit_code}")
                print(f"  Stderr: {stderr}")
                return TestResult("validation-allows-close", False, stderr)

        finally:
            cleanup_test_bead(bead_id, workspace)


def test_invalid_bead_id_format():
    """Test that invalid bead ID format is rejected."""
    print("\n" + "="*60)
    print("TEST: Invalid bead ID format rejected")
    print("="*60)

    script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
    cmd = [
        str(script_path),
        "invalid-bead-id-format",  # Missing prefix
        "Test close"
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=10)

    if exit_code != 0 and ("Invalid bead ID format" in stderr or "ERROR" in stderr):
        print("✓ PASS: Invalid bead ID format rejected")
        print(f"  Stderr: {stderr[:200]}")
        return TestResult("invalid-bead-id", True, stderr)
    else:
        print(f"✗ FAIL: Invalid bead ID was accepted")
        print(f"  Exit code: {exit_code}")
        return TestResult("invalid-bead-id", False, stdout)


def test_missing_bead_id():
    """Test that missing bead ID argument is handled properly."""
    print("\n" + "="*60)
    print("TEST: Missing bead ID argument")
    print("="*60)

    script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
    cmd = [str(script_path)]  # No bead ID provided

    exit_code, stdout, stderr = run_command(cmd, timeout=10)

    if exit_code != 0 and ("Usage:" in stderr or "ERROR" in stderr or "bead-id" in stderr):
        print("✓ PASS: Missing bead ID handled properly")
        print(f"  Stderr: {stderr[:200]}")
        return TestResult("missing-bead-id", True, stderr)
    else:
        print(f"✗ FAIL: Missing bead ID not handled properly")
        print(f"  Exit code: {exit_code}")
        return TestResult("missing-bead-id", False, stderr)


def test_custom_close_reason():
    """Test that custom close reason is properly passed through."""
    print("\n" + "="*60)
    print("TEST: Custom close reason")
    print("="*60)

    with tempfile.TemporaryDirectory() as tmpdir:
        workspace = Path(tmpdir) / ".beads"
        workspace.mkdir()

        bead_id = "bf-tcustomreason"
        custom_reason = "Implementation complete with tests passing"

        try:
            # Create test bead
            setup_test_bead(bead_id, workspace)

            # Close with custom reason
            script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
            cmd = [
                str(script_path),
                bead_id,
                custom_reason,
                "--skip-verify"
            ]

            exit_code, stdout, stderr = run_command(cmd, timeout=30)

            if exit_code == 0:
                # Verify the close reason was set
                cmd_bf = ["bf", "show", bead_id, "--workspace", str(workspace)]
                exit_code_bf, stdout_bf, _ = run_command(cmd_bf)

                if custom_reason in stdout_bf:
                    print("✓ PASS: Custom close reason preserved")
                    print(f"  Reason: {custom_reason}")
                    return TestResult("custom-reason", True, stdout_bf)
                else:
                    print("✗ FAIL: Custom close reason not found in bead")
                    print(f"  Bead output: {stdout_bf}")
                    return TestResult("custom-reason", False, stdout_bf)
            else:
                print(f"✗ FAIL: Close failed")
                print(f"  Stderr: {stderr}")
                return TestResult("custom-reason", False, stderr)

        finally:
            cleanup_test_bead(bead_id, workspace)


def main():
    """Run all tests and report results."""
    print("="*60)
    print("bf-close-with-verify.sh Test Suite")
    print("="*60)

    # Check if required tools are available
    try:
        run_command(["bf", "--version"])
        print("✓ bf command available")
    except Exception:
        print("✗ FAIL: bf command not available")
        return

    script_path = Path(__file__).parent.resolve() / "bf-close-with-verify.sh"
    if not script_path.exists():
        print(f"✗ FAIL: Script not found: {script_path}")
        return
    print(f"✓ Script found: {script_path}")

    # Run tests
    tests = [
        test_skip_verify_mode,
        test_validation_blocks_close_on_failure,
        test_validation_allows_close_on_success,
        test_invalid_bead_id_format,
        test_missing_bead_id,
        test_custom_close_reason,
    ]

    results = []
    for test in tests:
        try:
            result = test()
            results.append(result)
        except Exception as e:
            print(f"✗ EXCEPTION in {test.__name__}: {e}")
            results.append(TestResult(test.__name__, False, error=str(e)))
        time.sleep(1)  # Brief pause between tests

    # Summary
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)

    passed = sum(1 for r in results if r.passed)
    failed = len(results) - passed

    for result in results:
        print(result)

    print(f"\nTotal: {len(results)} tests")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")

    if failed == 0:
        print("\n✓ ALL TESTS PASSED")
        exit(0)
    else:
        print(f"\n✗ {failed} TEST(S) FAILED")
        exit(1)


if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
NEEDLE per-bead verify wrapper - Python helper

This module provides a Pythonic interface to the needle-verify-wrapper.sh script,
making it easier to integrate with NEEDLE worker code and providing better error handling.

Usage:
    from needle_verify import NeedleVerifier

    verifier = NeedleVerifier(
        bead_id="bf-4st8y",
        worker_name="claude-code-glm-4.7",
        repo_path="/home/coding/pdftract"
    )

    try:
        result = verifier.run(test_args="-p pdftract-core --lib")
        if result.passed:
            print("✓ Verification passed")
            print(result.output)
        else:
            print("✗ Verification failed")
            print(result.output)
    except VerificationError as e:
        print(f"Verification error: {e}")
"""

import subprocess
import os
import json
import tempfile
from pathlib import Path
from typing import Optional, NamedTuple
from dataclasses import dataclass


class VerificationError(Exception):
    """Base exception for verification errors."""
    pass


class WorkflowSubmissionError(VerificationError):
    """Raised when workflow submission fails."""
    pass


class WorkflowTimeoutError(VerificationError):
    """Raised when workflow does not complete within timeout."""
    pass


@dataclass
class VerificationResult:
    """Result of a verification workflow run."""
    passed: bool
    output: str
    workflow_name: str
    branch_name: str
    exit_code: int

    def to_dict(self) -> dict:
        return {
            "passed": self.passed,
            "output": self.output,
            "workflow_name": self.workflow_name,
            "branch_name": self.branch_name,
            "exit_code": self.exit_code
        }


class NeedleVerifier:
    """
    Wrapper for NEEDLE per-bead verification.

    This class handles:
    - Creating a wip/<worker>/<bead> branch
    - Committing and pushing changes
    - Submitting the rust-verify Argo workflow
    - Polling for completion
    - Returning results

    Attributes:
        bead_id: Bead ID (e.g., "bf-4st8y")
        worker_name: Worker name (e.g., "claude-code-glm-4.7")
        repo_path: Path to the git repository
        kubeconfig: Path to kubectl config (default: ~/.kube/iad-ci.kubeconfig)
    """

    def __init__(
        self,
        bead_id: str,
        worker_name: str,
        repo_path: str,
        kubeconfig: Optional[str] = None
    ):
        self.bead_id = bead_id
        self.worker_name = worker_name
        self.repo_path = Path(repo_path).resolve()
        self.kubeconfig = kubeconfig or os.path.expanduser("~/.kube/iad-ci.kubeconfig")

        # Branch naming: wip/<worker>/<bead>
        self.branch_name = f"wip/{worker_name}/{bead_id}"

        # Path to wrapper script (resolve to absolute path)
        script_dir = Path(__file__).parent.resolve()
        self.wrapper_script = script_dir / "needle-verify-wrapper.sh"

        if not self.wrapper_script.exists():
            raise VerificationError(f"Wrapper script not found: {self.wrapper_script}")

        if not self.repo_path.exists():
            raise VerificationError(f"Repository path does not exist: {self.repo_path}")

        if not Path(self.kubeconfig).exists():
            raise VerificationError(f"kubeconfig not found: {self.kubeconfig}")

    def run(
        self,
        test_args: str = "",
        timeout: int = 1800,
        dry_run: bool = False
    ) -> VerificationResult:
        """
        Run the verification workflow.

        Args:
            test_args: Arguments to pass to cargo test (e.g., "-p pdftract-core --lib")
            timeout: Maximum time to wait for workflow completion (seconds)
            dry_run: If True, skip workflow submission (for testing)

        Returns:
            VerificationResult with workflow output and status

        Raises:
            WorkflowSubmissionError: If workflow submission fails
            WorkflowTimeoutError: If workflow does not complete within timeout
            VerificationError: For other verification errors
        """
        env = os.environ.copy()
        env["KUBECONFIG"] = str(self.kubeconfig)

        if dry_run:
            env["DRY_RUN"] = "true"

        cmd = [
            str(self.wrapper_script),
            self.bead_id,
            self.worker_name,
            str(self.repo_path),
            test_args
        ]

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                env=env,
                check=False  # We check exit code manually
            )

            # Parse output
            output = result.stdout
            if result.stderr:
                output += "\n" + result.stderr

            # Determine workflow name from output
            workflow_name = self._extract_workflow_name(output)

            # Result passes if exit code is 0
            passed = result.returncode == 0

            return VerificationResult(
                passed=passed,
                output=output,
                workflow_name=workflow_name,
                branch_name=self.branch_name,
                exit_code=result.returncode
            )

        except subprocess.TimeoutExpired as e:
            raise WorkflowTimeoutError(
                f"Workflow did not complete within {timeout}s. "
                f"Partial output:\n{e.stdout.decode() if e.stdout else ''}"
            )
        except Exception as e:
            raise VerificationError(f"Verification failed: {e}")

    def _extract_workflow_name(self, output: str) -> str:
        """Extract workflow name from script output."""
        for line in output.splitlines():
            if "Workflow submitted:" in line or "Monitoring workflow:" in line:
                # Extract name from line like "Workflow submitted: rust-verify-bf-4st8y-123456"
                parts = line.split(":")
                if len(parts) > 1:
                    return parts[-1].strip()
        return f"rust-verify-{self.bead_id}-{self.worker_name}"


def verify_and_gate(
    bead_id: str,
    worker_name: str,
    repo_path: str,
    test_args: str = "",
    timeout: int = 1800
) -> bool:
    """
    Convenience function to run verification and gate on result.

    This is the main entry point for NEEDLE workers. Usage:

        if not verify_and_gate("bf-4st8y", "claude-code-glm-4.7", "/home/coding/pdftract"):
            sys.exit(1)  # Block bead close

    Args:
        bead_id: Bead ID to verify
        worker_name: Name of the worker running the verification
        repo_path: Path to the git repository
        test_args: Optional cargo test arguments
        timeout: Maximum time to wait for workflow completion

    Returns:
        True if verification passed, False otherwise

    Raises:
        VerificationError: If verification cannot be completed
    """
    verifier = NeedleVerifier(
        bead_id=bead_id,
        worker_name=worker_name,
        repo_path=repo_path
    )

    try:
        result = verifier.run(test_args=test_args, timeout=timeout)

        if result.passed:
            print(f"✓ Verification passed for {bead_id}")
            print(f"  Workflow: {result.workflow_name}")
            print(f"  Branch: {result.branch_name}")
            return True
        else:
            print(f"✗ Verification failed for {bead_id}")
            print(f"  Workflow: {result.workflow_name}")
            print(f"  Exit code: {result.exit_code}")
            if result.output:
                print(f"  Output:\n{result.output}")
            return False

    except VerificationError as e:
        print(f"✗ Verification error for {bead_id}: {e}")
        return False


if __name__ == "__main__":
    import sys

    # CLI interface for testing
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <bead-id> <worker-name> <repo-path> [test-args]")
        sys.exit(1)

    bead_id = sys.argv[1]
    worker_name = sys.argv[2]
    repo_path = sys.argv[3]
    test_args = sys.argv[4] if len(sys.argv) > 4 else ""

    if verify_and_gate(bead_id, worker_name, repo_path, test_args):
        sys.exit(0)
    else:
        sys.exit(1)

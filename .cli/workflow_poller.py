#!/usr/bin/env python3
"""
Workflow polling module for Argo Workflows on iad-ci.

This module provides functionality to poll workflow status until completion,
handling timeouts and returning the final phase.

Usage:
    from workflow_poller import WorkflowPoller

    poller = WorkflowPoller(kubeconfig="~/.kube/iad-ci.kubeconfig")
    phase = poller.poll_until_completion("workflow-name", timeout=1800)
    if phase == "Succeeded":
        print("Workflow succeeded!")
"""

import subprocess
import time
import os
from pathlib import Path
from typing import Optional
from dataclasses import dataclass


class WorkflowPollingError(Exception):
    """Base exception for workflow polling errors."""
    pass


class WorkflowTimeoutError(WorkflowPollingError):
    """Raised when workflow does not complete within timeout."""
    pass


@dataclass
class WorkflowStatus:
    """Current status of a workflow."""
    phase: str
    message: Optional[str] = None

    def is_terminal(self) -> bool:
        """Check if workflow is in a terminal phase."""
        return self.phase in ("Succeeded", "Failed", "Errored")

    def is_success(self) -> bool:
        """Check if workflow succeeded."""
        return self.phase == "Succeeded"


class WorkflowPoller:
    """
    Polls Argo Workflow status until completion.

    This class handles polling workflow status on iad-ci cluster,
    with configurable polling intervals and timeout.

    Attributes:
        kubeconfig: Path to kubectl config (default: ~/.kube/iad-ci.kubeconfig)
        poll_interval: Seconds between status checks (default: 10)
        timeout: Maximum time to wait for completion (default: 1800)
    """

    TERMINAL_PHASES = ("Succeeded", "Failed", "Errored")

    def __init__(
        self,
        kubeconfig: Optional[str] = None,
        poll_interval: int = 10,
        timeout: int = 1800
    ):
        """
        Initialize workflow poller.

        Args:
            kubeconfig: Path to kubectl config (default: ~/.kube/iad-ci.kubeconfig)
            poll_interval: Seconds between status checks (default: 10)
            timeout: Maximum time to wait for completion in seconds (default: 1800)
        """
        self.kubeconfig = Path(kubeconfig or os.path.expanduser("~/.kube/iad-ci.kubeconfig"))
        self.poll_interval = poll_interval
        self.timeout = timeout

        if not self.kubeconfig.exists():
            raise WorkflowPollingError(f"kubeconfig not found: {self.kubeconfig}")

    def get_workflow_status(self, workflow_name: str, namespace: str = "argo-workflows") -> WorkflowStatus:
        """
        Get current workflow status via kubectl.

        Args:
            workflow_name: Name of the workflow to check
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            WorkflowStatus with current phase

        Raises:
            WorkflowPollingError: If kubectl command fails
        """
        cmd = [
            "kubectl",
            "--kubeconfig", str(self.kubeconfig),
            "get", "workflow", workflow_name,
            "-n", namespace,
            "-o", "jsonpath='{.status.phase}'"
        ]

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True,
                timeout=30
            )

            # Extract phase from output (remove quotes if present)
            phase = result.stdout.strip().strip("'").strip('"')

            if not phase:
                raise WorkflowPollingError("Empty workflow phase returned")

            return WorkflowStatus(phase=phase)

        except subprocess.TimeoutExpired as e:
            raise WorkflowPollingError(f"kubectl command timed out: {e}")
        except subprocess.CalledProcessError as e:
            raise WorkflowPollingError(
                f"kubectl command failed (exit {e.returncode}): {e.stderr}"
            )
        except Exception as e:
            raise WorkflowPollingError(f"Failed to get workflow status: {e}")

    def poll_until_completion(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows",
        timeout: Optional[int] = None
    ) -> str:
        """
        Poll workflow status until completion.

        Continuously checks workflow status until it reaches a terminal phase
        (Succeeded, Failed, or Errored) or the timeout is exceeded.

        Args:
            workflow_name: Name of the workflow to poll
            namespace: Kubernetes namespace (default: argo-workflows)
            timeout: Maximum time to wait in seconds (overrides instance timeout)

        Returns:
            Final workflow phase (Succeeded, Failed, or Errored)

        Raises:
            WorkflowTimeoutError: If workflow does not complete within timeout
            WorkflowPollingError: If polling fails for other reasons
        """
        timeout = timeout or self.timeout
        start_time = time.time()

        while True:
            elapsed = time.time() - start_time

            if elapsed > timeout:
                raise WorkflowTimeoutError(
                    f"Workflow '{workflow_name}' did not complete within {timeout}s. "
                    f"Last check at {elapsed:.1f}s."
                )

            try:
                status = self.get_workflow_status(workflow_name, namespace)

                if status.is_terminal():
                    return status.phase

                # Check timeout before sleeping
                remaining = timeout - elapsed
                if remaining <= 0:
                    raise WorkflowTimeoutError(
                        f"Workflow '{workflow_name}' did not complete within {timeout}s."
                    )

                # Sleep for the poll interval, but don't exceed remaining time
                sleep_time = min(self.poll_interval, remaining)
                time.sleep(sleep_time)

            except WorkflowTimeoutError:
                raise
            except WorkflowPollingError as e:
                # If we can't get status, wait a bit and retry
                # (unless we're very close to timeout)
                remaining = timeout - (time.time() - start_time)
                if remaining < self.poll_interval:
                    raise WorkflowPollingError(
                        f"Polling failed with {remaining:.1f}s remaining: {e}"
                    )
                time.sleep(self.poll_interval)


def poll_workflow(
    workflow_name: str,
    kubeconfig: Optional[str] = None,
    timeout: int = 1800,
    poll_interval: int = 10
) -> str:
    """
    Convenience function to poll workflow until completion.

    Args:
        workflow_name: Name of the workflow to poll
        kubeconfig: Path to kubectl config
        timeout: Maximum time to wait in seconds (default: 1800)
        poll_interval: Seconds between status checks (default: 10)

    Returns:
        Final workflow phase (Succeeded, Failed, or Errored)

    Raises:
        WorkflowTimeoutError: If workflow does not complete within timeout
        WorkflowPollingError: If polling fails
    """
    poller = WorkflowPoller(
        kubeconfig=kubeconfig,
        poll_interval=poll_interval,
        timeout=timeout
    )
    return poller.poll_until_completion(workflow_name)


if __name__ == "__main__":
    import sys

    # CLI interface for testing
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <workflow-name> [namespace] [timeout]")
        sys.exit(1)

    workflow_name = sys.argv[1]
    namespace = sys.argv[2] if len(sys.argv) > 2 else "argo-workflows"
    timeout = int(sys.argv[3]) if len(sys.argv) > 3 else 1800

    try:
        phase = poll_workflow(workflow_name, timeout=timeout)
        print(f"Workflow '{workflow_name}' completed with phase: {phase}")
        sys.exit(0 if phase == "Succeeded" else 1)
    except WorkflowTimeoutError as e:
        print(f"Timeout: {e}")
        sys.exit(2)
    except WorkflowPollingError as e:
        print(f"Error: {e}")
        sys.exit(3)
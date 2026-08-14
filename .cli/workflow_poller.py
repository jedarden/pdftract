#!/usr/bin/env python3
"""
Workflow polling and log collection module for Argo Workflows on iad-ci.

This module provides functionality to:
- Poll workflow status until completion
- Collect pod logs from completed workflows
- Handle timeouts and return final phase

Usage:
    from workflow_poller import WorkflowPoller

    poller = WorkflowPoller(kubeconfig="~/.kube/iad-ci.kubeconfig")

    # Poll workflow status
    phase = poller.poll_until_completion("workflow-name", timeout=1800)
    if phase == "Succeeded":
        print("Workflow succeeded!")

    # Collect logs from completed workflow
    logs = poller.collect_workflow_logs("workflow-name")
    print(logs)
"""

import subprocess
import time
import os
import json
import random
from pathlib import Path
from typing import Optional, List
from dataclasses import dataclass
from typing import Dict, Any


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


@dataclass
class WorkflowResult:
    """Structured result from a completed workflow."""
    exit_code: int  # 0 = pass, non-zero = fail
    phase: str     # Succeeded, Failed, or Errored
    output: str    # Full output log
    result: str    # "pass" or "fail" from output parameters
    message: Optional[str] = None  # Error message if failed

    def is_success(self) -> bool:
        """Check if workflow completed successfully."""
        return self.exit_code == 0 and self.phase == "Succeeded" and self.result == "pass"


class WorkflowPoller:
    """
    Polls Argo Workflow status until completion.

    This class handles polling workflow status on iad-ci cluster,
    with exponential backoff and jitter for intelligent polling.

    Attributes:
        kubeconfig: Path to kubectl config (default: ~/.kube/iad-ci.kubeconfig)
        initial_poll_interval: Initial seconds between status checks (default: 5)
        max_poll_interval: Maximum seconds between status checks (default: 60)
        timeout: Maximum time to wait for completion (default: 1800)
        jitter_percent: Percentage of jitter to apply (default: 0.2 for ±20%)
    """

    TERMINAL_PHASES = ("Succeeded", "Failed", "Errored")

    def __init__(
        self,
        kubeconfig: Optional[str] = None,
        poll_interval: int = 10,
        timeout: int = 1800,
        initial_poll_interval: int = 5,
        max_poll_interval: int = 60,
        jitter_percent: float = 0.2
    ):
        """
        Initialize workflow poller.

        Args:
            kubeconfig: Path to kubectl config (default: ~/.kube/iad-ci.kubeconfig)
            poll_interval: DEPRECATED - Seconds between status checks (default: 10)
            timeout: Maximum time to wait for completion in seconds (default: 1800)
            initial_poll_interval: Initial seconds between status checks (default: 5)
            max_poll_interval: Maximum seconds between status checks (default: 60)
            jitter_percent: Percentage of jitter to apply, 0.2 = ±20% (default: 0.2)
        """
        self.kubeconfig = Path(kubeconfig or os.path.expanduser("~/.kube/iad-ci.kubeconfig"))
        # Support legacy poll_interval parameter
        if poll_interval != 10:
            self.initial_poll_interval = poll_interval
            self.max_poll_interval = min(poll_interval * 12, 60)  # Cap at 60s
        else:
            self.initial_poll_interval = initial_poll_interval
            self.max_poll_interval = max_poll_interval
        self.timeout = timeout
        self.jitter_percent = jitter_percent

        if not self.kubeconfig.exists():
            raise WorkflowPollingError(f"kubeconfig not found: {self.kubeconfig}")

    def _calculate_poll_interval(self, attempt: int) -> float:
        """
        Calculate poll interval with exponential backoff and jitter.

        Args:
            attempt: Current poll attempt number (1-indexed)

        Returns:
            Sleep time in seconds with jitter applied
        """
        # Exponential backoff: start at initial_interval, double each attempt, max max_interval
        base_interval = min(self.initial_poll_interval * (2 ** (attempt - 1)), self.max_poll_interval)

        # Add jitter (± jitter_percent)
        jitter = base_interval * self.jitter_percent
        interval = base_interval + random.uniform(-jitter, jitter)

        return max(0, interval)  # Ensure non-negative

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

    def get_workflow_output_parameter(
        self,
        workflow_name: str,
        parameter_name: str,
        namespace: str = "argo-workflows"
    ) -> Optional[str]:
        """
        Get a specific output parameter from a completed workflow.

        Args:
            workflow_name: Name of the workflow
            parameter_name: Name of the output parameter to retrieve
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            Parameter value as string, or None if not found
        """
        cmd = [
            "kubectl",
            "--kubeconfig", str(self.kubeconfig),
            "get", "workflow", workflow_name,
            "-n", namespace,
            "-o", f"jsonpath='{{.status.outputs.parameters[?(@.name==\"{parameter_name}\")].value}}'"
        ]

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True,
                timeout=30
            )

            # Extract value (remove quotes if present)
            value = result.stdout.strip().strip("'").strip('"')
            return value if value else None

        except subprocess.CalledProcessError:
            # Parameter may not exist or workflow not complete
            return None
        except subprocess.TimeoutExpired as e:
            raise WorkflowPollingError(f"kubectl command timed out: {e}")
        except Exception as e:
            raise WorkflowPollingError(f"Failed to get output parameter: {e}")

    def get_workflow_outputs(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows"
    ) -> Dict[str, str]:
        """
        Get all output parameters from a completed workflow.

        Args:
            workflow_name: Name of the workflow
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            Dictionary mapping parameter names to values
        """
        outputs = {}

        # Try to get common output parameters
        for param_name in ["result", "output", "exit-code"]:
            value = self.get_workflow_output_parameter(workflow_name, param_name, namespace)
            if value is not None:
                outputs[param_name] = value

        return outputs

    def get_rust_verify_result(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows"
    ) -> WorkflowResult:
        """
        Get structured result from a rust-verify workflow.

        Parses workflow output parameters to extract exit code and output.
        The rust-verify workflow template provides:
        - result: "pass" or "fail"
        - output: Full build/test log

        Args:
            workflow_name: Name of the workflow
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            WorkflowResult with exit_code, phase, output, and result

        Raises:
            WorkflowPollingError: If result cannot be extracted
        """
        # Get workflow phase
        status = self.get_workflow_status(workflow_name, namespace)

        # Get output parameters
        outputs = self.get_workflow_outputs(workflow_name, namespace)

        # Extract result (pass/fail)
        result = outputs.get("result", "").lower()
        if result not in ("pass", "fail"):
            result = "fail"  # Default to fail if unclear

        # Extract output log
        output = outputs.get("output", "")

        # Extract message if workflow failed
        message = status.message
        if not message and status.phase != "Succeeded":
            message = f"Workflow completed with phase: {status.phase}"

        # Determine exit code (0 for pass, 1 for fail)
        exit_code = 0 if (status.phase == "Succeeded" and result == "pass") else 1

        return WorkflowResult(
            exit_code=exit_code,
            phase=status.phase,
            output=output,
            result=result,
            message=message
        )

    def poll_workflow_result(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows",
        timeout: Optional[int] = None
    ) -> WorkflowResult:
        """
        Poll workflow until completion and return structured result.

        This is a convenience method that combines polling and result extraction
        for a complete workflow execution result.

        Args:
            workflow_name: Name of the workflow to poll
            namespace: Kubernetes namespace (default: argo-workflows)
            timeout: Maximum time to wait in seconds (overrides instance timeout)

        Returns:
            WorkflowResult with exit code, phase, and output

        Raises:
            WorkflowTimeoutError: If workflow does not complete within timeout
            WorkflowPollingError: If polling or result extraction fails
        """
        # First poll until completion
        final_phase = self.poll_until_completion(workflow_name, namespace, timeout)

        # Then get the structured result
        return self.get_rust_verify_result(workflow_name, namespace)

    def poll_until_completion(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows",
        timeout: Optional[int] = None
    ) -> str:
        """
        Poll workflow status until completion with exponential backoff and jitter.

        Continuously checks workflow status until it reaches a terminal phase
        (Succeeded, Failed, or Errored) or the timeout is exceeded.

        Uses exponential backoff starting at 5s doubling each attempt up to 60s max,
        with ±20% jitter applied to each interval to avoid thundering herd.

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
        attempt = 0

        while True:
            attempt += 1
            elapsed = time.time() - start_time

            if elapsed > timeout:
                raise WorkflowTimeoutError(
                    f"Workflow '{workflow_name}' did not complete within {timeout}s. "
                    f"Last check at {elapsed:.1f}s after {attempt} attempts."
                )

            try:
                status = self.get_workflow_status(workflow_name, namespace)

                if status.is_terminal():
                    return status.phase

                # Calculate next poll interval with backoff and jitter
                poll_interval = self._calculate_poll_interval(attempt)

                # Check timeout before sleeping
                remaining = timeout - elapsed
                if remaining <= 0:
                    raise WorkflowTimeoutError(
                        f"Workflow '{workflow_name}' did not complete within {timeout}s."
                    )

                # Sleep for the calculated interval, but don't exceed remaining time
                sleep_time = min(poll_interval, remaining)
                time.sleep(sleep_time)

            except WorkflowTimeoutError:
                raise
            except WorkflowPollingError as e:
                # If we can't get status, wait a bit and retry
                # (unless we're very close to timeout)
                remaining = timeout - (time.time() - start_time)
                if remaining < self.initial_poll_interval:
                    raise WorkflowPollingError(
                        f"Polling failed with {remaining:.1f}s remaining: {e}"
                    )

                # Use current backoff interval for retry
                poll_interval = self._calculate_poll_interval(attempt)
                sleep_time = min(poll_interval, remaining)
                time.sleep(sleep_time)

    def discover_workflow_pods(
        self,
        workflow_name: str,
        namespace: str = "argo-workflows"
    ) -> List[str]:
        """
        Discover all pods created by a workflow.

        Args:
            workflow_name: Name of the workflow to discover pods for
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            List of pod names created by the workflow

        Raises:
            WorkflowPollingError: If pod discovery fails
        """
        cmd = [
            "kubectl",
            "--kubeconfig", str(self.kubeconfig),
            "get", "pods",
            "-n", namespace,
            "-l", f"workflows.argoproj.io/workflow={workflow_name}",
            "-o", "jsonpath='{.items[*].metadata.name}'"
        ]

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True,
                timeout=30
            )

            # Extract pod names from output
            pod_output = result.stdout.strip().strip("'").strip('"')

            if not pod_output:
                # No pods found - workflow may have been deleted or no pods created
                return []

            # Split by whitespace to get individual pod names
            pod_names = pod_output.split()
            return pod_names

        except subprocess.TimeoutExpired as e:
            raise WorkflowPollingError(f"kubectl command timed out: {e}")
        except subprocess.CalledProcessError as e:
            # If pods not found (404), return empty list gracefully
            stderr_lower = e.stderr.lower() if e.stderr else ""
            if "not found" in stderr_lower or "notfound" in stderr_lower.replace(" ", ""):
                return []
            raise WorkflowPollingError(
                f"kubectl command failed (exit {e.returncode}): {e.stderr}"
            )
        except Exception as e:
            raise WorkflowPollingError(f"Failed to discover workflow pods: {e}")

    def get_pod_logs(
        self,
        pod_name: str,
        container: str = "main",
        namespace: str = "argo-workflows"
    ) -> str:
        """
        Fetch logs from a specific pod container.

        Args:
            pod_name: Name of the pod to fetch logs from
            container: Container name (default: main)
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            Log output as string

        Raises:
            WorkflowPollingError: If log fetching fails
        """
        cmd = [
            "kubectl",
            "--kubeconfig", str(self.kubeconfig),
            "logs", pod_name,
            "-n", namespace,
            "-c", container
        ]

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True,
                timeout=30
            )

            return result.stdout

        except subprocess.TimeoutExpired as e:
            raise WorkflowPollingError(f"kubectl logs command timed out: {e}")
        except subprocess.CalledProcessError as e:
            # If container hasn't started yet or no logs, treat gracefully
            stderr_lower = e.stderr.lower() if e.stderr else ""
            if ("waiting to start" in stderr_lower or "no logs" in stderr_lower or
                "containercreating" in stderr_lower or "container creating" in stderr_lower):
                return f"[No logs available for {pod_name}/{container}]"
            raise WorkflowPollingError(
                f"kubectl logs failed for {pod_name}/{container} (exit {e.returncode}): {e.stderr}"
            )
        except Exception as e:
            raise WorkflowPollingError(f"Failed to get logs from {pod_name}: {e}")

    def collect_workflow_logs(
        self,
        workflow_name: str,
        container: str = "main",
        namespace: str = "argo-workflows"
    ) -> str:
        """
        Collect logs from all pods created by a workflow.

        This method discovers all pods belonging to the workflow and fetches
        logs from the specified container, returning concatenated output.

        Args:
            workflow_name: Name of the workflow to collect logs from
            container: Container name (default: main)
            namespace: Kubernetes namespace (default: argo-workflows)

        Returns:
            Concatenated log output from all workflow pods as string

        Raises:
            WorkflowPollingError: If log collection fails
        """
        pod_names = self.discover_workflow_pods(workflow_name, namespace)

        if not pod_names:
            return f"[No pods found for workflow '{workflow_name}']"

        # Collect logs from each pod
        all_logs = []
        for pod_name in pod_names:
            try:
                pod_logs = self.get_pod_logs(pod_name, container, namespace)
                all_logs.append(f"=== Logs from {pod_name} ===")
                all_logs.append(pod_logs)
                all_logs.append("")  # Empty line between pods
            except WorkflowPollingError as e:
                # Include error in output but continue with other pods
                all_logs.append(f"=== Error fetching logs from {pod_name} ===")
                all_logs.append(f"Error: {e}")
                all_logs.append("")

        return "\n".join(all_logs)


def poll_workflow_result(
    workflow_name: str,
    kubeconfig: Optional[str] = None,
    timeout: int = 1800,
    poll_interval: int = 10,
    initial_poll_interval: int = 5,
    max_poll_interval: int = 60,
    jitter_percent: float = 0.2
) -> WorkflowResult:
    """
    Convenience function to poll workflow and return structured result.

    Args:
        workflow_name: Name of the workflow to poll
        kubeconfig: Path to kubectl config
        timeout: Maximum time to wait in seconds (default: 1800)
        poll_interval: DEPRECATED - Seconds between status checks (default: 10)
        initial_poll_interval: Initial seconds between status checks (default: 5)
        max_poll_interval: Maximum seconds between status checks (default: 60)
        jitter_percent: Percentage of jitter to apply (default: 0.2 for ±20%)

    Returns:
        WorkflowResult with exit_code, phase, output, and result

    Raises:
        WorkflowTimeoutError: If workflow does not complete within timeout
        WorkflowPollingError: If polling or result extraction fails
    """
    poller = WorkflowPoller(
        kubeconfig=kubeconfig,
        poll_interval=poll_interval,
        timeout=timeout,
        initial_poll_interval=initial_poll_interval,
        max_poll_interval=max_poll_interval,
        jitter_percent=jitter_percent
    )
    return poller.poll_workflow_result(workflow_name)


def poll_workflow(
    workflow_name: str,
    kubeconfig: Optional[str] = None,
    timeout: int = 1800,
    poll_interval: int = 10,
    initial_poll_interval: int = 5,
    max_poll_interval: int = 60,
    jitter_percent: float = 0.2
) -> str:
    """
    Convenience function to poll workflow until completion.

    Args:
        workflow_name: Name of the workflow to poll
        kubeconfig: Path to kubectl config
        timeout: Maximum time to wait in seconds (default: 1800)
        poll_interval: DEPRECATED - Seconds between status checks (default: 10)
        initial_poll_interval: Initial seconds between status checks (default: 5)
        max_poll_interval: Maximum seconds between status checks (default: 60)
        jitter_percent: Percentage of jitter to apply (default: 0.2 for ±20%)

    Returns:
        Final workflow phase (Succeeded, Failed, or Errored)

    Raises:
        WorkflowTimeoutError: If workflow does not complete within timeout
        WorkflowPollingError: If polling fails
    """
    poller = WorkflowPoller(
        kubeconfig=kubeconfig,
        poll_interval=poll_interval,
        timeout=timeout,
        initial_poll_interval=initial_poll_interval,
        max_poll_interval=max_poll_interval,
        jitter_percent=jitter_percent
    )
    return poller.poll_until_completion(workflow_name)


def collect_workflow_logs(
    workflow_name: str,
    kubeconfig: Optional[str] = None,
    container: str = "main",
    namespace: str = "argo-workflows"
) -> str:
    """
    Convenience function to collect logs from a completed workflow.

    Args:
        workflow_name: Name of the workflow to collect logs from
        kubeconfig: Path to kubectl config
        container: Container name (default: main)
        namespace: Kubernetes namespace (default: argo-workflows)

    Returns:
        Concatenated log output from all workflow pods

    Raises:
        WorkflowPollingError: If log collection fails
    """
    poller = WorkflowPoller(kubeconfig=kubeconfig)
    return poller.collect_workflow_logs(workflow_name, container, namespace)


if __name__ == "__main__":
    import sys

    # CLI interface for testing
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <workflow-name> [--poll | --logs | --result] [namespace] [timeout]")
        print("  --poll: Poll workflow until completion (default)")
        print("  --logs: Collect logs from completed workflow")
        print("  --result: Poll workflow and return structured result with exit code")
        sys.exit(1)

    workflow_name = sys.argv[1]
    mode = "poll"  # default mode

    # Check if mode is specified
    if len(sys.argv) >= 3 and sys.argv[2] in ("--poll", "--logs", "--result"):
        mode = sys.argv[2].lstrip("--")
        offset = 2
    else:
        offset = 0

    namespace = sys.argv[offset + 2] if len(sys.argv) > offset + 2 else "argo-workflows"
    timeout = int(sys.argv[offset + 3]) if len(sys.argv) > offset + 3 else 1800

    try:
        if mode == "poll":
            phase = poll_workflow(workflow_name, timeout=timeout)
            print(f"Workflow '{workflow_name}' completed with phase: {phase}")
            sys.exit(0 if phase == "Succeeded" else 1)
        elif mode == "logs":
            logs = collect_workflow_logs(workflow_name, namespace=namespace)
            print(f"Logs from workflow '{workflow_name}':")
            print(logs)
            sys.exit(0)
        elif mode == "result":
            result = poll_workflow_result(workflow_name, timeout=timeout)
            print(f"Workflow '{workflow_name}' completed:")
            print(f"  Phase: {result.phase}")
            print(f"  Exit Code: {result.exit_code}")
            print(f"  Result: {result.result}")
            if result.message:
                print(f"  Message: {result.message}")
            if result.output:
                print(f"\nOutput:")
                print(result.output[:1000])  # Print first 1000 chars
                if len(result.output) > 1000:
                    print(f"\n... ({len(result.output) - 1000} more characters)")
            sys.exit(result.exit_code)
    except WorkflowTimeoutError as e:
        print(f"Timeout: {e}")
        sys.exit(2)
    except WorkflowPollingError as e:
        print(f"Error: {e}")
        sys.exit(3)
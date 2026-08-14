#!/usr/bin/env python3
"""
Tests for workflow polling module.

Tests cover success, failure, and timeout paths for workflow polling,
using mocked kubectl commands to avoid requiring actual cluster access.
"""

import subprocess
import time
import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
import tempfile
import os

# Import the module to test
import sys
sys.path.insert(0, str(Path(__file__).parent))

from workflow_poller import (
    WorkflowPoller,
    WorkflowPollingError,
    WorkflowTimeoutError,
    WorkflowStatus,
    poll_workflow,
    collect_workflow_logs
)


class TestWorkflowStatus(unittest.TestCase):
    """Tests for WorkflowStatus dataclass."""

    def test_is_terminal_success(self):
        """Test that Succeeded is recognized as terminal."""
        status = WorkflowStatus(phase="Succeeded")
        self.assertTrue(status.is_terminal())
        self.assertTrue(status.is_success())

    def test_is_terminal_failed(self):
        """Test that Failed is recognized as terminal but not success."""
        status = WorkflowStatus(phase="Failed")
        self.assertTrue(status.is_terminal())
        self.assertFalse(status.is_success())

    def test_is_terminal_errored(self):
        """Test that Errored is recognized as terminal but not success."""
        status = WorkflowStatus(phase="Errored")
        self.assertTrue(status.is_terminal())
        self.assertFalse(status.is_success())

    def test_is_terminal_running(self):
        """Test that Running is not recognized as terminal."""
        status = WorkflowStatus(phase="Running")
        self.assertFalse(status.is_terminal())
        self.assertFalse(status.is_success())

    def test_is_terminal_pending(self):
        """Test that Pending is not recognized as terminal."""
        status = WorkflowStatus(phase="Pending")
        self.assertFalse(status.is_terminal())
        self.assertFalse(status.is_success())


class TestWorkflowPoller(unittest.TestCase):
    """Tests for WorkflowPoller class."""

    def test_init_default_kubeconfig(self):
        """Test initialization with default kubeconfig path."""
        # Test that default kubeconfig path is set correctly
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)
            # Verify default values
            self.assertEqual(poller.poll_interval, 10)
            self.assertEqual(poller.timeout, 1800)
            self.assertTrue(poller.kubeconfig.exists())
        finally:
            os.unlink(temp_kubeconfig)

    def test_init_custom_kubeconfig(self):
        """Test initialization with custom kubeconfig path."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)
            self.assertEqual(str(poller.kubeconfig), temp_kubeconfig)
            self.assertEqual(poller.poll_interval, 10)
            self.assertEqual(poller.timeout, 1800)
        finally:
            os.unlink(temp_kubeconfig)

    def test_init_missing_kubeconfig(self):
        """Test that missing kubeconfig raises error."""
        with self.assertRaises(WorkflowPollingError) as cm:
            WorkflowPoller(kubeconfig="/nonexistent/path/kubeconfig")
        self.assertIn("kubeconfig not found", str(cm.exception))

    def test_get_workflow_status_success(self):
        """Test getting workflow status successfully."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl command
            mock_result = MagicMock()
            mock_result.stdout = "'Running'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                status = poller.get_workflow_status("test-workflow")
                self.assertEqual(status.phase, "Running")
                self.assertFalse(status.is_terminal())
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_workflow_status_with_quotes(self):
        """Test that quotes are stripped from phase output."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Test with single quotes
            mock_result = MagicMock()
            mock_result.stdout = "'Succeeded'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                status = poller.get_workflow_status("test-workflow")
                self.assertEqual(status.phase, "Succeeded")

            # Test with double quotes
            mock_result.stdout = '"Failed"'
            with patch('subprocess.run', return_value=mock_result):
                status = poller.get_workflow_status("test-workflow")
                self.assertEqual(status.phase, "Failed")
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_workflow_status_kubectl_failure(self):
        """Test that kubectl failure raises error."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl command failure
            with patch('subprocess.run', side_effect=subprocess.CalledProcessError(1, 'kubectl')):
                with self.assertRaises(WorkflowPollingError) as cm:
                    poller.get_workflow_status("test-workflow")
                self.assertIn("kubectl command failed", str(cm.exception))
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_workflow_status_empty_response(self):
        """Test that empty phase response raises error."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock empty response
            mock_result = MagicMock()
            mock_result.stdout = ""
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                with self.assertRaises(WorkflowPollingError) as cm:
                    poller.get_workflow_status("test-workflow")
                self.assertIn("Empty workflow phase", str(cm.exception))
        finally:
            os.unlink(temp_kubeconfig)


class TestPollingIntegration(unittest.TestCase):
    """Integration tests for full polling flow."""

    def test_poll_until_completion_succeeded(self):
        """Test successful polling for workflow that succeeds."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=10)

            # Mock workflow progression: Running -> Succeeded
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()
                if call_count[0] == 1:
                    mock_result.stdout = "'Running'"
                else:
                    mock_result.stdout = "'Succeeded'"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                phase = poller.poll_until_completion("test-workflow")
                self.assertEqual(phase, "Succeeded")
                self.assertEqual(call_count[0], 2)  # Should check twice
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_until_completion_failed(self):
        """Test polling for workflow that fails."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=10)

            # Mock workflow progression: Running -> Failed
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()
                if call_count[0] == 1:
                    mock_result.stdout = "'Running'"
                else:
                    mock_result.stdout = "'Failed'"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                phase = poller.poll_until_completion("test-workflow")
                self.assertEqual(phase, "Failed")
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_until_completion_errored(self):
        """Test polling for workflow that errors."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=10)

            # Mock workflow progression: Running -> Errored
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()
                if call_count[0] == 1:
                    mock_result.stdout = "'Running'"
                else:
                    mock_result.stdout = "'Errored'"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                phase = poller.poll_until_completion("test-workflow")
                self.assertEqual(phase, "Errored")
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_until_completion_timeout(self):
        """Test that timeout is raised when workflow takes too long."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            # Short timeout for testing
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=3)

            # Mock workflow that never completes
            mock_result = MagicMock()
            mock_result.stdout = "'Running'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                with self.assertRaises(WorkflowTimeoutError) as cm:
                    poller.poll_until_completion("test-workflow")
                self.assertIn("did not complete within", str(cm.exception))
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_until_completion_custom_timeout(self):
        """Test that custom timeout parameter overrides instance timeout."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=30)

            # Mock workflow that never completes
            mock_result = MagicMock()
            mock_result.stdout = "'Running'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                # Use shorter custom timeout
                with self.assertRaises(WorkflowTimeoutError) as cm:
                    poller.poll_until_completion("test-workflow", timeout=2)
                self.assertIn("did not complete within 2s", str(cm.exception))
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_until_completion_long_running(self):
        """Test polling for workflow that takes multiple checks."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=10)

            # Mock workflow progression: Running (x3) -> Succeeded
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()
                if call_count[0] <= 3:
                    mock_result.stdout = "'Running'"
                else:
                    mock_result.stdout = "'Succeeded'"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                phase = poller.poll_until_completion("test-workflow")
                self.assertEqual(phase, "Succeeded")
                self.assertEqual(call_count[0], 4)  # Should check 4 times
        finally:
            os.unlink(temp_kubeconfig)


class TestConvenienceFunction(unittest.TestCase):
    """Tests for the convenience poll_workflow function."""

    def test_poll_workflow_success(self):
        """Test convenience function for successful workflow."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()
                if call_count[0] == 1:
                    mock_result.stdout = "'Running'"
                else:
                    mock_result.stdout = "'Succeeded'"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                phase = poll_workflow("test-workflow", kubeconfig=temp_kubeconfig, timeout=10, poll_interval=1)
                self.assertEqual(phase, "Succeeded")
        finally:
            os.unlink(temp_kubeconfig)

    def test_poll_workflow_custom_params(self):
        """Test convenience function with custom parameters."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            mock_result = MagicMock()
            mock_result.stdout = "'Succeeded'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                phase = poll_workflow(
                    "test-workflow",
                    kubeconfig=temp_kubeconfig,
                    timeout=100,
                    poll_interval=5
                )
                self.assertEqual(phase, "Succeeded")
        finally:
            os.unlink(temp_kubeconfig)


class TestErrorRecovery(unittest.TestCase):
    """Tests for error handling and recovery during polling."""

    def test_polling_retries_on_transient_error(self):
        """Test that transient errors during polling are handled."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig, poll_interval=1, timeout=10)

            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                # First call fails with subprocess error, second succeeds
                if call_count[0] == 1:
                    raise subprocess.CalledProcessError(1, 'kubectl', stderr='Transient error')
                else:
                    mock_result = MagicMock()
                    mock_result.stdout = "'Succeeded'"
                    mock_result.returncode = 0
                    return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                # The transient error should be handled by retry logic
                # After the error, it should retry and succeed
                phase = poller.poll_until_completion("test-workflow")
                self.assertEqual(phase, "Succeeded")
                self.assertEqual(call_count[0], 2)  # Should have called twice (fail + success)
        finally:
            os.unlink(temp_kubeconfig)


class TestPodLogCollection(unittest.TestCase):
    """Tests for pod log collection functionality."""

    def test_discover_workflow_pods_success(self):
        """Test discovering pods created by workflow."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl get pods response
            mock_result = MagicMock()
            mock_result.stdout = "'pod-1 pod-2 pod-3'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                pod_names = poller.discover_workflow_pods("test-workflow")
                self.assertEqual(pod_names, ["pod-1", "pod-2", "pod-3"])
        finally:
            os.unlink(temp_kubeconfig)

    def test_discover_workflow_pods_empty(self):
        """Test discovering pods when no pods found."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock empty response (no pods)
            mock_result = MagicMock()
            mock_result.stdout = "''"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                pod_names = poller.discover_workflow_pods("test-workflow")
                self.assertEqual(pod_names, [])
        finally:
            os.unlink(temp_kubeconfig)

    def test_discover_workflow_pods_not_found(self):
        """Test discovering pods when workflow not found."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl get pods with NotFound error
            with patch('subprocess.run', side_effect=subprocess.CalledProcessError(1, 'kubectl', stderr='NotFound')):
                pod_names = poller.discover_workflow_pods("test-workflow")
                self.assertEqual(pod_names, [])
        finally:
            os.unlink(temp_kubeconfig)

    def test_discover_workflow_pods_single(self):
        """Test discovering a single pod."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock single pod response
            mock_result = MagicMock()
            mock_result.stdout = "'workflow-pod-abc123'"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                pod_names = poller.discover_workflow_pods("test-workflow")
                self.assertEqual(pod_names, ["workflow-pod-abc123"])
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_pod_logs_success(self):
        """Test fetching logs from a pod."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl logs response
            mock_result = MagicMock()
            mock_result.stdout = "Running cargo test...\nTest result: ok"
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                logs = poller.get_pod_logs("test-pod")
                self.assertEqual(logs, "Running cargo test...\nTest result: ok")
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_pod_logs_empty(self):
        """Test fetching logs when container has no logs."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock empty logs response
            mock_result = MagicMock()
            mock_result.stdout = ""
            mock_result.returncode = 0

            with patch('subprocess.run', return_value=mock_result):
                logs = poller.get_pod_logs("test-pod")
                self.assertEqual(logs, "")
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_pod_logs_container_not_started(self):
        """Test fetching logs when container hasn't started yet."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl logs error for container waiting to start
            with patch('subprocess.run', side_effect=subprocess.CalledProcessError(1, 'kubectl', stderr='ContainerCreating')):
                logs = poller.get_pod_logs("test-pod")
                self.assertIn("[No logs available", logs)
        finally:
            os.unlink(temp_kubeconfig)

    def test_get_pod_logs_failure(self):
        """Test that kubectl logs failure raises error."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock kubectl logs failure
            with patch('subprocess.run', side_effect=subprocess.CalledProcessError(1, 'kubectl', stderr='Pod not found')):
                with self.assertRaises(WorkflowPollingError) as cm:
                    poller.get_pod_logs("test-pod")
                self.assertIn("kubectl logs failed", str(cm.exception))
        finally:
            os.unlink(temp_kubeconfig)

    def test_collect_workflow_logs_multiple_pods(self):
        """Test collecting logs from multiple pods in a workflow."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock pod discovery
            mock_pods_result = MagicMock()
            mock_pods_result.stdout = "'pod-1 pod-2'"
            mock_pods_result.returncode = 0

            # Mock logs from each pod
            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                mock_result = MagicMock()

                # First call is pod discovery
                if call_count[0] == 1:
                    return mock_pods_result

                # Subsequent calls are log fetching
                pod_num = (call_count[0] - 2) % 2
                if pod_num == 0:
                    mock_result.stdout = "Logs from pod-1"
                else:
                    mock_result.stdout = "Logs from pod-2"

                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                logs = poller.collect_workflow_logs("test-workflow")

                self.assertIn("=== Logs from pod-1 ===", logs)
                self.assertIn("Logs from pod-1", logs)
                self.assertIn("=== Logs from pod-2 ===", logs)
                self.assertIn("Logs from pod-2", logs)
        finally:
            os.unlink(temp_kubeconfig)

    def test_collect_workflow_logs_no_pods(self):
        """Test collecting logs when no pods found."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock empty pod discovery
            mock_pods_result = MagicMock()
            mock_pods_result.stdout = "''"
            mock_pods_result.returncode = 0

            with patch('subprocess.run', return_value=mock_pods_result):
                logs = poller.collect_workflow_logs("test-workflow")
                self.assertIn("[No pods found for workflow 'test-workflow']", logs)
        finally:
            os.unlink(temp_kubeconfig)

    def test_collect_workflow_logs_single_pod(self):
        """Test collecting logs from a single pod workflow."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock single pod discovery
            mock_pods_result = MagicMock()
            mock_pods_result.stdout = "'workflow-pod-123'"
            mock_pods_result.returncode = 0

            # Mock logs response
            mock_logs_result = MagicMock()
            mock_logs_result.stdout = "Single pod log output\nTest passed"
            mock_logs_result.returncode = 0

            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                if call_count[0] == 1:
                    return mock_pods_result
                else:
                    return mock_logs_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                logs = poller.collect_workflow_logs("test-workflow")

                self.assertIn("=== Logs from workflow-pod-123 ===", logs)
                self.assertIn("Single pod log output", logs)
                self.assertIn("Test passed", logs)
        finally:
            os.unlink(temp_kubeconfig)

    def test_collect_workflow_logs_pod_failure(self):
        """Test collecting logs when one pod fails but others succeed."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            poller = WorkflowPoller(kubeconfig=temp_kubeconfig)

            # Mock pod discovery with 3 pods
            mock_pods_result = MagicMock()
            mock_pods_result.stdout = "'pod-1 pod-2 pod-3'"
            mock_pods_result.returncode = 0

            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1

                # First call is pod discovery
                if call_count[0] == 1:
                    return mock_pods_result

                # Simulate pod-2 failing
                pod_num = (call_count[0] - 2) % 3
                if pod_num == 1:
                    raise subprocess.CalledProcessError(1, 'kubectl', stderr='Pod not found')

                mock_result = MagicMock()
                mock_result.stdout = f"Logs from pod-{pod_num + 1}"
                mock_result.returncode = 0
                return mock_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                logs = poller.collect_workflow_logs("test-workflow")

                # Should have logs from pod-1 and pod-3, error from pod-2
                self.assertIn("=== Logs from pod-1 ===", logs)
                self.assertIn("=== Error fetching logs from pod-2 ===", logs)
                self.assertIn("=== Logs from pod-3 ===", logs)
        finally:
            os.unlink(temp_kubeconfig)


class TestConvenienceFunctions(unittest.TestCase):
    """Tests for convenience functions."""

    def test_collect_workflow_logs_convenience(self):
        """Test the collect_workflow_logs convenience function."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.kubeconfig', delete=False) as f:
            temp_kubeconfig = f.name
            f.write("# mock kubeconfig\n")

        try:
            # Mock pod discovery and logs
            mock_pods_result = MagicMock()
            mock_pods_result.stdout = "'test-pod'"
            mock_pods_result.returncode = 0

            mock_logs_result = MagicMock()
            mock_logs_result.stdout = "Test log output"
            mock_logs_result.returncode = 0

            call_count = [0]

            def mock_kubectl(*args, **kwargs):
                call_count[0] += 1
                if call_count[0] == 1:
                    return mock_pods_result
                else:
                    return mock_logs_result

            with patch('subprocess.run', side_effect=mock_kubectl):
                logs = collect_workflow_logs("test-workflow", kubeconfig=temp_kubeconfig)
                self.assertIn("=== Logs from test-pod ===", logs)
                self.assertIn("Test log output", logs)
        finally:
            os.unlink(temp_kubeconfig)


if __name__ == "__main__":
    # Run tests
    unittest.main()
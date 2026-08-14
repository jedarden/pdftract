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
    poll_workflow
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


if __name__ == "__main__":
    # Run tests
    unittest.main()
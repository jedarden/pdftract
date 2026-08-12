#!/usr/bin/env python3
"""
Tests for workflow submission functionality

Run with: python test_workflow_submission.py
Or: pytest test_workflow_submission.py
"""

import os
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock, call

try:
    import pytest
    HAS_PYTEST = True
except ImportError:
    HAS_PYTEST = False
    pytest = None
    print("Note: pytest not installed. Install with: pip install pytest")

# Import the module to test
from needle_verify import (
    WorkflowSubmitter,
    VerificationError,
    WorkflowSubmissionError
)


class TestWorkflowSubmitter:
    """Test WorkflowSubmitter class."""

    @patch.object(Path, 'exists', return_value=True)
    def test_workflow_submitter_init(self, mock_exists):
        """Test WorkflowSubmitter initialization."""
        # Test with default kubeconfig
        submitter = WorkflowSubmitter()
        assert submitter.kubeconfig.endswith("/.kube/iad-ci.kubeconfig")

        # Test with custom kubeconfig
        custom_kubeconfig = "/custom/path/kubeconfig"
        submitter = WorkflowSubmitter(kubeconfig=custom_kubeconfig)
        assert submitter.kubeconfig == custom_kubeconfig

    @patch.object(Path, 'exists', return_value=False)
    def test_workflow_submitter_init_invalid_kubeconfig(self, mock_exists):
        """Test WorkflowSubmitter initialization with invalid kubeconfig."""
        with pytest.raises(VerificationError, match="kubeconfig not found"):
            WorkflowSubmitter(kubeconfig="/nonexistent/kubeconfig")

    @patch.object(Path, 'exists', return_value=True)
    def test_generate_workflow_manifest(self, mock_exists):
        """Test workflow manifest generation."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        manifest = submitter._generate_workflow_manifest(
            repo_url="https://git.ardenone.com/jedarden/pdftract.git",
            revision="refs/heads/wip/worker/bf-1sgmv8",
            test_args="-p pdftract-core --lib",
            bead_id="bf-1sgmv8",
            worker_name="claude-code-glm-4.7",
            builder_image="ronaldraygun/needle-ci-builder:with-deps"
        )

        assert "apiVersion: argoproj.io/v1alpha1" in manifest
        assert "kind: Workflow" in manifest
        assert "namespace: argo-workflows" in manifest
        assert "workflowTemplateRef:" in manifest
        assert "name: rust-verify" in manifest
        assert "repo:" in manifest
        assert "https://git.ardenone.com/jedarden/pdftract.git" in manifest
        assert "revision:" in manifest
        assert "refs/heads/wip/worker/bf-1sgmv8" in manifest
        assert "test-args:" in manifest
        assert "-p pdftract-core --lib" in manifest
        assert "bead-id: bf-1sgmv8" in manifest
        assert "worker-name: claude-code-glm-4.7" in manifest

    @patch.object(Path, 'exists', return_value=True)
    def test_generate_workflow_manifest_minimal(self, mock_exists):
        """Test workflow manifest generation with minimal parameters."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        manifest = submitter._generate_workflow_manifest(
            repo_url="https://github.com/test/repo.git",
            revision="main",
            test_args="",
            bead_id="",
            worker_name="",
            builder_image="custom-builder:latest"
        )

        assert "repo:" in manifest
        assert "https://github.com/test/repo.git" in manifest
        assert "revision:" in manifest
        assert "main" in manifest
        assert "test-args:" in manifest
        # Verify labels are not added when bead_id and worker_name are empty
        assert "bead-id:" not in manifest
        assert "worker-name:" not in manifest

    @patch.object(Path, 'exists', return_value=True)
    @patch('subprocess.run')
    @patch('tempfile.NamedTemporaryFile')
    def test_submit_rust_verify_success(self, mock_tempfile, mock_run, mock_exists):
        """Test successful workflow submission."""
        # Mock kubectl response
        mock_result = MagicMock()
        mock_result.returncode = 0
        mock_result.stdout = "workflow.argoproj.io/rust-verify-bf-1sgmv8-123456 created"
        mock_result.stderr = ""
        mock_run.return_value = mock_result

        # Mock temp file
        mock_file = MagicMock()
        mock_file.name = "/tmp/workflow.yaml"
        mock_tempfile.return_value.__enter__.return_value = mock_file

        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        workflow_name = submitter.submit_rust_verify(
            repo_url="https://git.ardenone.com/jedarden/pdftract.git",
            revision="refs/heads/wip/worker/bf-1sgmv8",
            test_args="-p pdftract-core --lib",
            bead_id="bf-1sgmv8",
            worker_name="claude-code-glm-4.7"
        )

        assert workflow_name == "rust-verify-bf-1sgmv8-123456"

        # Verify kubectl was called correctly
        mock_run.assert_called_once()
        call_args = mock_run.call_args
        assert call_args[0][0][0] == "kubectl"
        assert "--kubeconfig=/tmp/test.kubeconfig" in call_args[0][0]
        assert "create" in call_args[0][0]
        assert "-f" in call_args[0][0]

    @patch.object(Path, 'exists', return_value=True)
    @patch('subprocess.run')
    @patch('tempfile.NamedTemporaryFile')
    def test_submit_rust_verify_kubectl_failure(self, mock_tempfile, mock_run, mock_exists):
        """Test workflow submission when kubectl fails."""
        # Mock kubectl failure
        mock_result = MagicMock()
        mock_result.returncode = 1
        mock_result.stdout = ""
        mock_result.stderr = "Error: namespace not found"
        mock_run.return_value = mock_result

        # Mock temp file
        mock_file = MagicMock()
        mock_file.name = "/tmp/workflow.yaml"
        mock_tempfile.return_value.__enter__.return_value = mock_file

        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        with pytest.raises(WorkflowSubmissionError, match="kubectl create failed"):
            submitter.submit_rust_verify(
                repo_url="https://git.ardenone.com/jedarden/pdftract.git",
                revision="refs/heads/wip/worker/bf-1sgmv8",
                test_args="",
                bead_id="bf-1sgmv8",
                worker_name="claude-code-glm-4.7"
            )

    @patch.object(Path, 'exists', return_value=True)
    @patch('subprocess.run')
    @patch('tempfile.NamedTemporaryFile')
    def test_submit_rust_verify_timeout(self, mock_tempfile, mock_run, mock_exists):
        """Test workflow submission when kubectl times out."""
        import subprocess

        # Mock kubectl timeout
        mock_run.side_effect = subprocess.TimeoutExpired(cmd="kubectl", timeout=30)

        # Mock temp file
        mock_file = MagicMock()
        mock_file.name = "/tmp/workflow.yaml"
        mock_tempfile.return_value.__enter__.return_value = mock_file

        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        with pytest.raises(WorkflowSubmissionError, match="kubectl create timed out"):
            submitter.submit_rust_verify(
                repo_url="https://git.ardenone.com/jedarden/pdftract.git",
                revision="refs/heads/wip/worker/bf-1sgmv8",
                test_args="",
                bead_id="bf-1sgmv8",
                worker_name="claude-code-glm-4.7"
            )

    @patch.object(Path, 'exists', return_value=True)
    def test_parse_workflow_name_from_stdout(self, mock_exists):
        """Test parsing workflow name from kubectl output."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        # Test standard kubectl output
        name = submitter._parse_workflow_name(
            "workflow.argoproj.io/rust-verify-bf-1sgmv8-123456 created",
            ""
        )
        assert name == "rust-verify-bf-1sgmv8-123456"

    @patch.object(Path, 'exists', return_value=True)
    def test_parse_workflow_name_from_stderr(self, mock_exists):
        """Test parsing workflow name from stderr."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        # Some kubectl versions output to stderr
        name = submitter._parse_workflow_name(
            "",
            "workflow.argoproj.io/rust-verify-bf-1sgmv8-123456 created"
        )
        assert name == "rust-verify-bf-1sgmv8-123456"

    @patch.object(Path, 'exists', return_value=True)
    def test_parse_workflow_name_no_match(self, mock_exists):
        """Test parsing workflow name with no match."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        name = submitter._parse_workflow_name("some other output", "error message")
        assert name is None

    @patch.object(Path, 'exists', return_value=True)
    @patch('subprocess.run')
    def test_query_latest_workflow(self, mock_run, mock_exists):
        """Test querying for latest workflow."""
        mock_result = MagicMock()
        mock_result.returncode = 0
        mock_result.stdout = "rust-verify-bf-1sgmv8-123456"
        mock_run.return_value = mock_result

        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        workflow_name = submitter._query_latest_workflow("bf-1sgmv8", "claude-code-glm-4.7")
        assert workflow_name == "rust-verify-bf-1sgmv8-123456"

        # Verify kubectl get was called with correct labels
        call_args = mock_run.call_args
        assert "get" in call_args[0][0]
        assert "workflows" in call_args[0][0]
        assert "-l=bead-id=bf-1sgmv8,worker-name=claude-code-glm-4.7" in call_args[0][0]

    @patch.object(Path, 'exists', return_value=True)
    def test_query_latest_workflow_no_bead_id(self, mock_exists):
        """Test querying for latest workflow without bead_id."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        workflow_name = submitter._query_latest_workflow("", "claude-code-glm-4.7")
        assert workflow_name is None

    @patch.object(Path, 'exists', return_value=True)
    @patch('subprocess.run')
    def test_query_latest_workflow_kubectl_error(self, mock_run, mock_exists):
        """Test querying for latest workflow when kubectl fails."""
        mock_result = MagicMock()
        mock_result.returncode = 1
        mock_result.stdout = ""
        mock_run.return_value = mock_result

        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        workflow_name = submitter._query_latest_workflow("bf-1sgmv8", "claude-code-glm-4.7")
        assert workflow_name is None


class TestWorkflowSubmissionIntegration:
    """Integration-style tests for workflow submission."""

    @patch.object(Path, 'exists', return_value=True)
    def test_workflow_submission_interface(self, mock_exists):
        """Test the workflow submission public interface."""
        submitter = WorkflowSubmitter(kubeconfig="/tmp/test.kubeconfig")

        # Verify the method exists and has the right signature
        assert hasattr(submitter, 'submit_rust_verify')
        assert callable(submitter.submit_rust_verify)

        # Verify default parameters
        import inspect
        sig = inspect.signature(submitter.submit_rust_verify)
        params = sig.parameters

        assert 'repo_url' in params
        assert 'revision' in params
        assert 'test_args' in params
        assert 'bead_id' in params
        assert 'worker_name' in params
        assert 'builder_image' in params

        # Check default values
        assert params['test_args'].default == ""
        assert params['bead_id'].default == ""
        assert params['worker_name'].default == ""
        assert params['builder_image'].default == "ronaldraygun/needle-ci-builder:with-deps"


if __name__ == "__main__":
    if HAS_PYTEST:
        print("Running tests with pytest...")
        import sys
        sys.exit(pytest.main([__file__, "-v"]))
    else:
        print("Running basic tests without pytest...")
        import sys
        sys.exit(0)  # Skip basic tests if pytest not available

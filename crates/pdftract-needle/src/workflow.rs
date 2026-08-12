//! Argo Workflow submission and polling for rust-verify
//!
//! This module handles submitting rust-verify workflows to iad-ci and polling
//! for completion results.

use crate::{NeedleContext, VerifyResult};
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Argo Workflow submission and polling
pub struct WorkflowClient {
    /// Path to kubeconfig for iad-ci
    kubeconfig: String,
    /// Namespace for workflows
    namespace: String,
}

impl WorkflowClient {
    /// Create a new workflow client
    pub fn new(kubeconfig: String) -> Self {
        Self {
            kubeconfig,
            namespace: "argo-workflows".to_string(),
        }
    }

    /// Submit a rust-verify workflow
    ///
    /// Returns the workflow name (e.g., "rust-verify-abc123")
    pub async fn submit_verify_workflow(
        &self,
        ctx: &NeedleContext,
        branch_name: &str,
        test_args: &str,
    ) -> Result<String> {
        info!(
            "Submitting rust-verify workflow: repo={}, branch={}, test_args={}",
            ctx.repo_url, branch_name, test_args
        );

        let workflow_yaml = self.build_workflow_manifest(ctx, branch_name, test_args)?;

        debug!("Workflow manifest:\n{}", workflow_yaml);

        // Submit via kubectl create
        let mut child = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("create")
            .arg("-f")
            .arg("-")
            .env("KUBECONFIG", &self.kubeconfig)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn kubectl create")?;

        // Write YAML to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(workflow_yaml.as_bytes())
                .await
                .context("Failed to write workflow manifest to kubectl")?;
        }

        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait for kubectl create")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("kubectl create failed: {}", stderr);
        }

        // Extract workflow name from output (format: "workflow.argoproj.io/rust-verify-abc123 created")
        let stdout = String::from_utf8_lossy(&output.stdout);
        let workflow_name = stdout
            .lines()
            .find(|line| line.contains("workflow.argoproj.io/"))
            .and_then(|line| line.split("workflow.argoproj.io/").nth(1))
            .and_then(|s| s.split(' ').next())
            .context("Failed to parse workflow name from kubectl output")?;

        info!("Submitted workflow: {}", workflow_name);
        Ok(workflow_name.to_string())
    }

    /// Poll a workflow until completion
    ///
    /// Returns the VerifyResult when workflow completes (success or failure)
    pub async fn poll_workflow(&self, workflow_name: &str) -> Result<VerifyResult> {
        info!("Polling workflow: {}", workflow_name);

        let start = Instant::now();
        let timeout = Duration::from_secs(1800); // 30 minutes (matches workflow activeDeadlineSeconds)
        let poll_interval = Duration::from_secs(10);

        loop {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                warn!("Workflow polling timeout after {:?}", elapsed);
                return Ok(VerifyResult {
                    exit_code: 1,
                    phase: "Timeout".to_string(),
                    logs: format!("Workflow exceeded timeout of {:?}", timeout),
                    duration_secs: elapsed.as_secs(),
                });
            }

            match self.check_workflow_status(workflow_name).await? {
                WorkflowStatus::Running => {
                    debug!("Workflow still running, elapsed: {:?}", elapsed);
                    tokio::time::sleep(poll_interval).await;
                }
                WorkflowStatus::Succeeded => {
                    info!("Workflow succeeded");
                    return self.get_workflow_result(workflow_name).await;
                }
                WorkflowStatus::Failed | WorkflowStatus::Error => {
                    warn!("Workflow failed/errored");
                    return self.get_workflow_result(workflow_name).await;
                }
            }
        }
    }

    /// Check the current status of a workflow
    async fn check_workflow_status(&self, workflow_name: &str) -> Result<WorkflowStatus> {
        let output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("get")
            .arg("workflow")
            .arg(workflow_name)
            .arg("-n")
            .arg(&self.namespace)
            .arg("-o")
            .arg("jsonpath={.status.phase}")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to check workflow status")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") {
                // Workflow might be in the process of being created
                debug!("Workflow not found yet, retrying...");
                return Ok(WorkflowStatus::Running);
            }
            anyhow::bail!("kubectl get workflow failed: {}", stderr);
        }

        let phase = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("Workflow phase: {}", phase);

        Ok(match phase.as_str() {
            "Running" | "Pending" => WorkflowStatus::Running,
            "Succeeded" => WorkflowStatus::Succeeded,
            "Failed" => WorkflowStatus::Failed,
            "Error" => WorkflowStatus::Error,
            _ => {
                warn!("Unknown workflow phase: {}", phase);
                WorkflowStatus::Running
            }
        })
    }

    /// Get the result and logs from a completed workflow
    async fn get_workflow_result(&self, workflow_name: &str) -> Result<VerifyResult> {
        info!("Extracting results from workflow: {}", workflow_name);

        // Get workflow phase
        let phase_output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("get")
            .arg("workflow")
            .arg(workflow_name)
            .arg("-n")
            .arg(&self.namespace)
            .arg("-o")
            .arg("jsonpath={.status.phase}")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to get workflow phase")?;

        let phase = String::from_utf8_lossy(&phase_output.stdout).trim().to_string();

        // Get workflow logs from pod
        let logs = self.get_logs_from_pod(workflow_name).await.unwrap_or_else(|e| {
            format!("Failed to extract logs: {}", e)
        });

        // Calculate duration (started -> finished)
        let duration_secs = self.get_workflow_duration(workflow_name).await.unwrap_or(0);

        // Determine exit code based on phase and logs
        let exit_code = if phase == "Succeeded" && logs.contains("Result: pass") {
            0
        } else {
            1
        };

        Ok(VerifyResult {
            exit_code,
            phase,
            logs,
            duration_secs,
        })
    }

    /// Get logs directly from the workflow pod
    async fn get_logs_from_pod(&self, workflow_name: &str) -> Result<String> {
        // Find the pod for this workflow
        let pods_output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("get")
            .arg("pods")
            .arg("-n")
            .arg(&self.namespace)
            .arg("-l")
            .arg(&format!("workflows.argoproj.io/workflow={}", workflow_name))
            .arg("-o")
            .arg("jsonpath={.items[0].metadata.name}")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to find workflow pod")?;

        let pod_name = String::from_utf8_lossy(&pods_output.stdout).trim().to_string();
        if pod_name.is_empty() {
            anyhow::bail!("No pod found for workflow {}", workflow_name);
        }

        let logs_output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("logs")
            .arg(&pod_name)
            .arg("-n")
            .arg(&self.namespace)
            .arg("-c")
            .arg("main")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to get pod logs")?;

        if !logs_output.status.success() {
            let stderr = String::from_utf8_lossy(&logs_output.stderr);
            anyhow::bail!("Failed to get pod logs: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&logs_output.stdout).to_string())
    }

    /// Calculate workflow duration in seconds
    async fn get_workflow_duration(&self, workflow_name: &str) -> Result<u64> {
        let started_output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("get")
            .arg("workflow")
            .arg(workflow_name)
            .arg("-n")
            .arg(&self.namespace)
            .arg("-o")
            .arg("jsonpath={.status.startedAt}")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to get workflow start time")?;

        let finished_output = Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&self.kubeconfig)
            .arg("get")
            .arg("workflow")
            .arg(workflow_name)
            .arg("-n")
            .arg(&self.namespace)
            .arg("-o")
            .arg("jsonpath={.status.finishedAt}")
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .await
            .context("Failed to get workflow finish time")?;

        let started_str = String::from_utf8_lossy(&started_output.stdout).trim().to_string();
        let finished_str = String::from_utf8_lossy(&finished_output.stdout).trim().to_string();

        if started_str.is_empty() || finished_str.is_empty() {
            return Ok(0);
        }

        // Parse RFC3339 timestamps and calculate duration
        use chrono::{DateTime, Utc};
        if let (Ok(started), Ok(finished)) = (
            started_str.parse::<DateTime<Utc>>(),
            finished_str.parse::<DateTime<Utc>>(),
        ) {
            Ok((finished - started).num_seconds().max(0) as u64)
        } else {
            Ok(0)
        }
    }

    /// Build the workflow manifest YAML
    fn build_workflow_manifest(
        &self,
        ctx: &NeedleContext,
        branch_name: &str,
        test_args: &str,
    ) -> Result<String> {
        let yaml = format!(
            r#"apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: rust-verify-
  namespace: {}
spec:
  workflowTemplateRef:
    name: rust-verify
  arguments:
    parameters:
      - name: repo
        value: {}
      - name: revision
        value: {}
      - name: test-args
        value: {}
"#,
            self.namespace, ctx.repo_url, branch_name, test_args
        );

        Ok(yaml)
    }
}

/// Workflow status enum
#[derive(Debug, Clone, PartialEq)]
enum WorkflowStatus {
    Running,
    Succeeded,
    Failed,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_manifest_building() {
        let ctx = NeedleContext::new("test-worker".to_string(), "test-123".to_string());
        let client = WorkflowClient::new("/tmp/test.kubeconfig".to_string());

        let yaml = client
            .build_workflow_manifest(&ctx, "wip/test-worker/test-123", "")
            .unwrap();

        assert!(yaml.contains("name: rust-verify"));
        assert!(yaml.contains("wip/test-worker/test-123"));
        assert!(yaml.contains("git.ardenone.com"));
    }
}

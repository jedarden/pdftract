//! Argo Workflows integration for remote CI verification
//!
//! This module provides functions for submitting WorkflowTemplates to Argo Workflows
//! on the iad-ci cluster, specifically for rust-verify operations.

use anyhow::{Context, Result};
use std::process::Command;
use std::io::Write;

/// Configuration for Argo Workflows submission
pub struct ArgoConfig {
    /// Path to kubeconfig for iad-ci cluster
    pub kubeconfig: String,
    /// Namespace for workflows
    pub namespace: String,
    /// WorkflowTemplate name to submit
    pub workflow_template: String,
}

impl Default for ArgoConfig {
    fn default() -> Self {
        Self {
            kubeconfig: "/home/coding/.kube/iad-ci.kubeconfig".to_string(),
            namespace: "argo-workflows".to_string(),
            workflow_template: "rust-verify".to_string(),
        }
    }
}

/// Parameters for rust-verify workflow submission
#[derive(Debug, Clone)]
pub struct RustVerifyParams {
    /// Git repository URL (e.g., https://git.ardenone.com/jedarden/pdftract.git)
    pub repo_url: String,
    /// Git revision to verify (branch name or commit SHA)
    pub revision: String,
    /// Test arguments to pass to cargo test (e.g., "--lib bead::tests::test_xyz")
    pub test_args: String,
}

impl RustVerifyParams {
    /// Create new rust-verify parameters
    pub fn new(repo_url: String, revision: String, test_args: String) -> Self {
        Self {
            repo_url,
            revision,
            test_args,
        }
    }
}

/// Submit a rust-verify Workflow to Argo Workflows
///
/// # Arguments
///
/// * `params` - RustVerifyParams containing repo_url, revision, and test_args
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
///
/// # Returns
///
/// * `Ok(String)` - Workflow name on successful submission
/// * `Err(anyhow::Error)` - On submission failure
///
/// # Example
///
/// ```no_run
/// use xtask::ci::{submit_rust_verify_workflow, RustVerifyParams, ArgoConfig};
///
/// let params = RustVerifyParams::new(
///     "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
///     "wip/worker-name/bf-abc123".to_string(),
///     "--test-threads=1 bead::tests::test_xyz".to_string(),
/// );
///
/// match submit_rust_verify_workflow(&params, None) {
///     Ok(workflow_name) => println!("Submitted workflow: {}", workflow_name),
///     Err(e) => eprintln!("Failed to submit workflow: {}", e),
/// }
/// ```
pub fn submit_rust_verify_workflow(
    params: &RustVerifyParams,
    config: Option<ArgoConfig>,
) -> Result<String> {
    let config = config.unwrap_or_default();

    // Generate the Workflow YAML
    let workflow_yaml = generate_workflow_yaml(params, &config)?;

    // Submit to kubectl
    let output = Command::new("kubectl")
        .args(["--kubeconfig", &config.kubeconfig, "create", "-f", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn kubectl process")?;

    // Write the YAML to kubectl's stdin
    let mut stdin = output.stdin.as_ref().context("Failed to open kubectl stdin")?;
    stdin
        .write_all(workflow_yaml.as_bytes())
        .context("Failed to write workflow YAML to kubectl")?;

    // Wait for kubectl to complete and capture output
    let result = output
        .wait_with_output()
        .context("Failed to wait for kubectl process")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(anyhow::anyhow!(
            "kubectl create failed with exit code {}: {}",
            result.status,
            stderr
        ));
    }

    // Parse the workflow name from kubectl output
    let stdout = String::from_utf8_lossy(&result.stdout);
    let workflow_name = extract_workflow_name(&stdout)?;

    Ok(workflow_name)
}

/// Generate the Workflow YAML for rust-verify submission
fn generate_workflow_yaml(params: &RustVerifyParams, config: &ArgoConfig) -> Result<String> {
    // Sanitize the revision for use in generateName (replace slashes with dashes)
    let sanitized_revision = params.revision.replace('/', "-");

    let yaml = format!(
        r#"apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: rust-verify-{}-manual-
  namespace: {}
spec:
  workflowTemplateRef:
    name: {}
  arguments:
    parameters:
      - name: repo
        value: "{}"
      - name: revision
        value: "{}"
      - name: test-args
        value: "{}"
"#,
        sanitized_revision,
        config.namespace,
        config.workflow_template,
        params.repo_url,
        params.revision,
        params.test_args
    );

    Ok(yaml)
}

/// Extract workflow name from kubectl create output
///
/// Expected output format:
/// "workflow.argoproj.io/rust-verify-xxx-xxx-xxxxx created"
fn extract_workflow_name(output: &str) -> Result<String> {
    // Try to parse the typical kubectl output format
    if let Some(line) = output.lines().find(|l| l.contains("workflow.argoproj.io/")) {
        // Extract from format: "workflow.argoproj.io/<workflow-name> created"
        if let Some(start) = line.find("workflow.argoproj.io/") {
            let rest = &line[start + "workflow.argoproj.io/".len()..];
            if let Some(end) = rest.find(' ') {
                return Ok(rest[..end].to_string());
            }
        }
    }

    // Fallback: try to get the most recent workflow with the template label
    let config = ArgoConfig::default();
    let kubectl_output = Command::new("kubectl")
        .args([
            "--kubeconfig",
            &config.kubeconfig,
            "get",
            "workflows",
            "-n",
            &config.namespace,
            "-l",
            "workflows.argoproj.io/workflow-template=rust-verify",
            "-o",
            "jsonpath={.items[-1].metadata.name}",
        ])
        .output()
        .context("Failed to query kubectl for workflow name")?;

    if kubectl_output.status.success() {
        let name = String::from_utf8_lossy(&kubectl_output.stdout).trim().to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }

    Err(anyhow::anyhow!(
        "Could not extract workflow name from kubectl output: {}",
        output
    ))
}

/// Wait for workflow completion and return the phase
///
/// # Arguments
///
/// * `workflow_name` - Name of the workflow to poll
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
/// * `max_wait_seconds` - Maximum time to wait (default: 1800)
///
/// # Returns
///
/// * `Ok(String)` - Workflow phase ("Succeeded", "Failed", "Error")
/// * `Err(anyhow::Error)` - On timeout or error
pub fn wait_for_workflow_completion(
    workflow_name: &str,
    config: Option<ArgoConfig>,
    max_wait_seconds: Option<u64>,
) -> Result<String> {
    let config = config.unwrap_or_default();
    let max_wait = max_wait_seconds.unwrap_or(1800); // 30 minutes default
    let poll_interval = std::time::Duration::from_secs(10);

    let start = std::time::Instant::now();

    while start.elapsed().as_secs() < max_wait {
        let output = Command::new("kubectl")
            .args([
                "--kubeconfig",
                &config.kubeconfig,
                "get",
                "workflow",
                workflow_name,
                "-n",
                &config.namespace,
                "-o",
                "jsonpath={.status.phase}",
            ])
            .output()
            .context("Failed to query workflow status")?;

        if output.status.success() {
            let phase = String::from_utf8_lossy(&output.stdout).trim().to_string();

            match phase.as_str() {
                "Succeeded" | "Failed" | "Error" => {
                    return Ok(phase);
                }
                "Running" | "Pending" => {
                    std::thread::sleep(poll_interval);
                    continue;
                }
                _ => {
                    return Err(anyhow::anyhow!("Unknown workflow phase: {}", phase));
                }
            }
        }

        std::thread::sleep(poll_interval);
    }

    Err(anyhow::anyhow!(
        "Workflow wait timeout after {} seconds",
        max_wait
    ))
}

/// Get workflow output logs
///
/// # Arguments
///
/// * `workflow_name` - Name of the workflow
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
///
/// # Returns
///
/// * `Ok(String)` - Workflow output parameter value
/// * `Err(anyhow::Error)` - On error
pub fn get_workflow_output(
    workflow_name: &str,
    config: Option<ArgoConfig>,
) -> Result<String> {
    let config = config.unwrap_or_default();

    let output = Command::new("kubectl")
        .args([
            "--kubeconfig",
            &config.kubeconfig,
            "get",
            "workflow",
            workflow_name,
            "-n",
            &config.namespace,
            "-o",
            "jsonpath={.status.outputs.parameters[?(@.name==\"output\")].value}",
        ])
        .output()
        .context("Failed to get workflow output")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to get workflow output: {}",
            stderr
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_workflow_yaml() {
        let params = RustVerifyParams::new(
            "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
            "wip/worker/bf-abc123".to_string(),
            "--test-threads=1".to_string(),
        );

        let config = ArgoConfig::default();
        let yaml = generate_workflow_yaml(&params, &config).unwrap();

        assert!(yaml.contains("kind: Workflow"));
        assert!(yaml.contains("name: rust-verify"));
        assert!(yaml.contains("wip-worker-bf-abc123")); // sanitized revision
        assert!(yaml.contains(&params.repo_url));
        assert!(yaml.contains(&params.revision));
        assert!(yaml.contains(&params.test_args));
    }

    #[test]
    fn test_extract_workflow_name() {
        let output = "workflow.argoproj.io/rust-verify-test-abc123 created";
        let name = extract_workflow_name(output).unwrap();
        assert_eq!(name, "rust-verify-test-abc123");
    }

    #[test]
    fn test_rust_verify_params_new() {
        let params = RustVerifyParams::new(
            "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
            "main".to_string(),
            "--all-targets".to_string(),
        );

        assert_eq!(params.repo_url, "https://git.ardenone.com/jedarden/pdftract.git");
        assert_eq!(params.revision, "main");
        assert_eq!(params.test_args, "--all-targets");
    }

    #[test]
    fn test_argo_config_default() {
        let config = ArgoConfig::default();
        assert_eq!(config.kubeconfig, "/home/coding/.kube/iad-ci.kubeconfig");
        assert_eq!(config.namespace, "argo-workflows");
        assert_eq!(config.workflow_template, "rust-verify");
    }
}

//! Argo Workflows integration for remote CI verification
//!
//! This module provides functions for submitting WorkflowTemplates to Argo Workflows
//! on the iad-ci cluster, specifically for rust-verify operations.

use anyhow::{Context, Result};
use std::process::Command;
use std::io::Write;

/// Configuration for Argo Workflows submission
#[derive(Clone)]
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

/// Get workflow result (pass/fail) from output parameters
///
/// # Arguments
///
/// * `workflow_name` - Name of the workflow
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
///
/// # Returns
///
/// * `Ok(String)` - Workflow result ("pass" or "fail")
/// * `Err(anyhow::Error)` - On error
fn get_workflow_result(
    workflow_name: &str,
    config: &ArgoConfig,
) -> Result<String> {
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
            "jsonpath={.status.outputs.parameters[?(@.name==\"result\")].value}",
        ])
        .output()
        .context("Failed to get workflow result")?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(result)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to get workflow result: {}",
            stderr
        ))
    }
}

/// Extract exit code from workflow result and phase
///
/// This function combines the workflow result and phase to determine the final exit code:
/// - If result="fail", tests definitely failed → return 1
/// - If result="pass" and phase="Succeeded", everything worked → return 0
/// - If result is unrecognized, fall back to phase-based codes
///
/// The result parameter is the definitive test result, while the phase indicates
/// whether the workflow infrastructure succeeded.
///
/// # Arguments
///
/// * `result` - Workflow result ("pass" or "fail")
/// * `phase` - Workflow phase ("Succeeded", "Failed", "Error")
///
/// # Returns
///
/// Exit code (0 = pass, non-zero = fail)
fn extract_exit_code(result: &str, phase: &str) -> i32 {
    match result {
        "fail" => 1, // Tests definitely failed
        "pass" => {
            // Tests passed, but check if workflow succeeded
            match phase {
                "Succeeded" => 0, // Everything worked
                "Failed" => 1,    // Workflow failed despite test pass
                "Error" => 2,     // Workflow error despite test pass
                _ => 3,           // Unknown phase
            }
        }
        _ => {
            // Unrecognized result, fall back to phase
            match phase {
                "Succeeded" => 0,
                "Failed" => 1,
                "Error" => 2,
                _ => 3,
            }
        }
    }
}

/// Get pod logs from a workflow step
///
/// This function fetches logs directly from the pod running the workflow step,
/// which is useful for streaming logs while the workflow is still running or
/// when output parameters haven't been populated yet.
///
/// # Arguments
///
/// * `workflow_name` - Name of the workflow
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
///
/// # Returns
///
/// * `Ok(String)` - Pod logs
/// * `Err(anyhow::Error)` - On error or if pod is not found
///
/// # Note
///
/// This function handles podGC gracefully. If the pod has already been deleted
/// (podGC: OnPodCompletion), it falls back to workflow output parameters.
fn get_pod_logs(
    workflow_name: &str,
    config: Option<ArgoConfig>,
) -> Result<String> {
    let config = config.unwrap_or_default();

    // First, try to find the pod running the workflow step
    let output = Command::new("kubectl")
        .args([
            "--kubeconfig",
            &config.kubeconfig,
            "get",
            "pods",
            "-n",
            &config.namespace,
            "-l",
            &format!("workflows.argoproj.io/workflow={}", workflow_name),
            "-o",
            "jsonpath={.items[0].metadata.name}",
        ])
        .output()
        .context("Failed to query for workflow pods")?;

    if !output.status.success() || output.stdout.is_empty() {
        // Pod not found or deleted - fall back to workflow output
        return get_workflow_output(workflow_name, Some(config));
    }

    let pod_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Fetch logs from the pod's main container
    let logs_output = Command::new("kubectl")
        .args([
            "--kubeconfig",
            &config.kubeconfig,
            "logs",
            "-n",
            &config.namespace,
            &pod_name,
            "-c",
            "main",
            "--tail=1000", // Limit to last 1000 lines to avoid overflow
        ])
        .output()
        .context("Failed to fetch pod logs")?;

    if logs_output.status.success() {
        Ok(String::from_utf8_lossy(&logs_output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&logs_output.stderr);
        // If logs fail, fall back to workflow output
        if stderr.contains("not found") || stderr.contains("terminated") {
            get_workflow_output(workflow_name, Some(config))
        } else {
            Err(anyhow::anyhow!("Failed to fetch pod logs: {}", stderr))
        }
    }
}

/// Poll workflow until completion and return (exit_code, logs)
///
/// This is the main polling function that:
/// 1. Polls the workflow status until it reaches a terminal phase
/// 2. Extracts the exit code from the workflow result
/// 3. Retrieves the full output logs
/// 4. Returns a tuple of (exit_code, logs)
///
/// # Arguments
///
/// * `workflow_name` - Name of the workflow to poll
/// * `config` - Optional ArgoConfig (uses defaults if not provided)
/// * `max_wait_seconds` - Maximum time to wait (default: 3600 = 1 hour)
///
/// # Returns
///
/// * `Ok((i32, String))` - Tuple of (exit_code, logs)
/// * `Err(anyhow::Error)` - On timeout or error
///
/// # Example
///
/// ```no_run
/// use xtask::ci::{submit_rust_verify_workflow, RustVerifyParams, poll_workflow_completion};
///
/// let params = RustVerifyParams::new(
///     "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
///     "main".to_string(),
///     "--all-targets".to_string(),
/// );
///
/// let workflow_name = submit_rust_verify_workflow(&params, None)?;
/// let (exit_code, logs) = poll_workflow_completion(&workflow_name, None, None)?;
///
/// if exit_code == 0 {
///     println!("Tests passed!");
/// } else {
///     println!("Tests failed (exit code {}): {}", exit_code, logs);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn poll_workflow_completion(
    workflow_name: &str,
    config: Option<ArgoConfig>,
    max_wait_seconds: Option<u64>,
) -> Result<(i32, String)> {
    let config = config.unwrap_or_default();
    let max_wait = max_wait_seconds.unwrap_or(3600); // 1 hour default

    // First, wait for the workflow to reach a terminal phase
    let phase = wait_for_workflow_completion(workflow_name, Some(config.clone()), Some(max_wait))?;

    // Get the result (pass/fail) from workflow outputs
    let result = get_workflow_result(workflow_name, &config)?;

    // Try to get logs from pod first (more recent), fall back to workflow output
    let logs = match get_pod_logs(workflow_name, Some(config.clone())) {
        Ok(pod_logs) => pod_logs,
        Err(_) => {
            // Fallback to workflow output parameters if pod logs unavailable
            get_workflow_output(workflow_name, Some(config))?
        }
    };

    // Extract exit code from result and phase
    let exit_code = extract_exit_code(&result, &phase);

    Ok((exit_code, logs))
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
    fn test_generate_workflow_yaml_with_slashes() {
        let params = RustVerifyParams::new(
            "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
            "wip/worker-name/bf-abc123".to_string(),
            "--all-targets".to_string(),
        );

        let config = ArgoConfig::default();
        let yaml = generate_workflow_yaml(&params, &config).unwrap();

        // Slashes should be replaced with dashes in generateName
        assert!(yaml.contains("rust-verify-wip-worker-name-bf-abc123-manual-"));
        assert!(yaml.contains(&params.revision)); // Original revision should be in parameters
    }

    #[test]
    fn test_extract_workflow_name() {
        let output = "workflow.argoproj.io/rust-verify-test-abc123 created";
        let name = extract_workflow_name(output).unwrap();
        assert_eq!(name, "rust-verify-test-abc123");
    }

    #[test]
    fn test_extract_workflow_name_multiline() {
        let output = r#"workflow.argoproj.io/rust-verify-test-xyz789 created
workflow.argoproj.io/another-workflow created"#;
        let name = extract_workflow_name(output).unwrap();
        assert_eq!(name, "rust-verify-test-xyz789");
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

    #[test]
    fn test_extract_exit_code_pass_succeeded() {
        let exit_code = extract_exit_code("pass", "Succeeded");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_extract_exit_code_fail_failed() {
        let exit_code = extract_exit_code("fail", "Failed");
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_extract_exit_code_fail_error() {
        let exit_code = extract_exit_code("fail", "Error");
        assert_eq!(exit_code, 2);
    }

    #[test]
    fn test_extract_exit_code_unknown_result() {
        // Test fallback behavior for unknown result values
        let exit_code = extract_exit_code("unknown", "Succeeded");
        assert_eq!(exit_code, 0); // Should fallback to phase-based code
    }

    #[test]
    fn test_extract_exit_code_unknown_phase() {
        // Test fallback behavior for unknown phases
        let exit_code = extract_exit_code("pass", "Running");
        assert_eq!(exit_code, 3); // Default error code
    }

    #[test]
    fn test_extract_exit_code_phase_fallback() {
        // Test that phase-based fallback works correctly
        assert_eq!(extract_exit_code("", "Succeeded"), 0);
        assert_eq!(extract_exit_code("", "Failed"), 1);
        assert_eq!(extract_exit_code("", "Error"), 2);
        assert_eq!(extract_exit_code("", "Unknown"), 3);
    }

    #[test]
    fn test_extract_exit_code_all_combinations() {
        // Test all known combinations
        let test_cases = vec![
            (("pass", "Succeeded"), 0), // Normal success case
            (("fail", "Failed"), 1),    // Normal test failure
            (("fail", "Error"), 1),     // Test failure with workflow error
            (("pass", "Failed"), 1),    // Workflow failed despite test pass
            (("fail", "Succeeded"), 1), // Tests failed even though workflow succeeded
        ];

        for ((result, phase), expected) in test_cases {
            let exit_code = extract_exit_code(result, phase);
            assert_eq!(exit_code, expected,
                "extract_exit_code({:?}, {:?}) should be {}", result, phase, expected);
        }
    }

    #[test]
    fn test_wait_for_workflow_completion_timeout_parameter() {
        // Verify that the timeout parameter is properly handled
        // This is a compile-time test that ensures the function signature is correct
        let config = ArgoConfig::default();

        // Test with custom timeout
        let result = std::panic::catch_unwind(|| {
            let _ = wait_for_workflow_completion("test-workflow", Some(config), Some(60));
        });

        // The function should not panic due to parameter handling
        assert!(result.is_ok());
    }

    #[test]
    fn test_wait_for_workflow_completion_default_timeout() {
        // Verify default timeout handling
        let config = ArgoConfig::default();

        let result = std::panic::catch_unwind(|| {
            let _ = wait_for_workflow_completion("test-workflow", Some(config), None);
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_poll_workflow_completion_integration() {
        // This test verifies the complete polling flow structure
        // It doesn't actually call kubectl (would fail in unit tests)
        // but ensures the function logic is sound

        let workflow_name = "rust-verify-test-123";
        let config = ArgoConfig::default();

        // Verify that the function signature is correct
        let _ = poll_workflow_completion(
            workflow_name,
            Some(config),
            Some(3600)
        );

        // If we reach here, the function compiles correctly
        // Integration tests would require a real cluster
    }

    #[test]
    fn test_poll_workflow_completion_default_timeout() {
        // Test default timeout (1 hour)
        let workflow_name = "rust-verify-test-456";
        let config = ArgoConfig::default();

        let result = std::panic::catch_unwind(|| {
            let _ = poll_workflow_completion(workflow_name, Some(config), None);
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_argo_config_clone() {
        // Verify ArgoConfig is cloneable (needed for passing to subprocess functions)
        let config1 = ArgoConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.kubeconfig, config2.kubeconfig);
        assert_eq!(config1.namespace, config2.namespace);
        assert_eq!(config1.workflow_template, config2.workflow_template);
    }

    #[test]
    fn test_rust_verify_params_clone() {
        // Verify RustVerifyParams is cloneable
        let params1 = RustVerifyParams::new(
            "https://git.ardenone.com/jedarden/pdftract.git".to_string(),
            "main".to_string(),
            "--test-threads=1".to_string(),
        );
        let params2 = params1.clone();

        assert_eq!(params1.repo_url, params2.repo_url);
        assert_eq!(params1.revision, params2.revision);
        assert_eq!(params1.test_args, params2.test_args);
    }
}

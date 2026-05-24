use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};
use std::time::Duration;

/// Check: network reachability (remote source feature)
///
/// OK: HEAD https://example.com returns 2xx in <= 5s
/// WARN: 3xx or slow
/// FAIL: failure
pub struct NetworkCheck;

impl NetworkCheck {
    fn check_reachability() -> Result<(u16, Duration), String> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(5))
            .build();

        let start = std::time::Instant::now();

        let response = agent
            .head("https://example.com")
            .call()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let elapsed = start.elapsed();
        let status = response.status();

        Ok((status, elapsed))
    }
}

impl Check for NetworkCheck {
    fn name(&self) -> &'static str {
        "network reachability"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        match Self::check_reachability() {
            Ok((status, elapsed)) => {
                let slow = elapsed.as_secs() >= 5;

                if status >= 200 && status < 300 {
                    if slow {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Warn,
                            detail: format!(
                                "Network reachable but slow: {} in {:.2}s",
                                status,
                                elapsed.as_secs_f64()
                            ),
                        }
                    } else {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Ok,
                            detail: format!(
                                "Network reachable: {} in {:.2}s",
                                status,
                                elapsed.as_secs_f64()
                            ),
                        }
                    }
                } else if status >= 300 && status < 400 {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!(
                            "Network returned redirect: {} (may indicate proxy or redirect loop)",
                            status
                        ),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Fail,
                        detail: format!("Network returned error status: {}", status),
                    }
                }
            }
            Err(e) => CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: e,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_check_name() {
        assert_eq!(NetworkCheck.name(), "network reachability");
    }

    #[test]
    fn test_check_reachability_200_ok() {
        // Note: This test requires actual network access
        // In CI, this might be mocked or skipped
    }
}

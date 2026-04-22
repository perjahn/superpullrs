use anyhow::{Context, Result};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct ProcessManager {
    pub timeout: Duration,
}

impl ProcessManager {
    pub fn new(_throttle: usize, timeout_secs: u64) -> Self {
        ProcessManager {
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    pub async fn run_git_command(&self, working_dir: &str, args: &[&str]) -> Result<bool> {
        let output = self.run_command_inner("git", working_dir, args).await?;
        Ok(output.status.success())
    }

    pub async fn run_command_with_output(
        &self,
        working_dir: &str,
        program: &str,
        args: &[&str],
    ) -> Result<String> {
        let output = self.run_command_inner(program, working_dir, args).await?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    async fn run_command_inner(
        &self,
        program: &str,
        working_dir: &str,
        args: &[&str],
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new(program);
        cmd.current_dir(working_dir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let future = cmd.output();
        timeout(self.timeout, future)
            .await
            .context("Command timed out")?
            .context("Failed to execute command")
    }
}

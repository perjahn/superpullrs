use anyhow::{Context, Result};
use std::os::unix::process::ExitStatusExt;
use std::time::Duration;
use tokio::io::AsyncReadExt;
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

    pub async fn run_git_command(
        &self,
        working_dir: &str,
        args: &[&str],
    ) -> Result<(bool, String, bool)> {
        let (output, is_timeout) = self.run_command_inner("git", working_dir, args).await?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let combined = format!("{}\n{}", stdout, stderr).trim().to_string();
        Ok((output.status.success(), combined, is_timeout))
    }

    pub async fn run_command_with_output(
        &self,
        working_dir: &str,
        program: &str,
        args: &[&str],
    ) -> Result<String> {
        let (output, _is_timeout) = self.run_command_inner(program, working_dir, args).await?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    async fn run_command_inner(
        &self,
        program: &str,
        working_dir: &str,
        args: &[&str],
    ) -> Result<(std::process::Output, bool)> {
        let mut cmd = Command::new(program);
        cmd.current_dir(working_dir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn command")?;

        // Extract the output streams
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        // Read both pipes concurrently to avoid deadlock
        let stdout_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            if let Some(mut reader) = stdout_handle {
                let _ = reader.read_to_end(&mut buf).await;
            }
            buf
        });

        let stderr_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            if let Some(mut reader) = stderr_handle {
                let _ = reader.read_to_end(&mut buf).await;
            }
            buf
        });

        // Wait for process with timeout
        let wait_result = timeout(self.timeout, child.wait()).await;

        // Collect outputs from concurrent read tasks
        let stdout = stdout_task.await.unwrap_or_default();
        let stderr = stderr_task.await.unwrap_or_default();

        match wait_result {
            Ok(Ok(status)) => Ok((
                std::process::Output {
                    status,
                    stdout,
                    stderr,
                },
                false,
            )),
            Ok(Err(e)) => Err(e).context("Failed to wait for command"),
            Err(_) => {
                // Timeout occurred - kill the process
                let _ = child.kill().await;
                // Return partial output with timeout flag, using failure status
                Ok((
                    std::process::Output {
                        status: std::process::ExitStatus::from_raw(1),
                        stdout,
                        stderr,
                    },
                    true,
                ))
            }
        }
    }
}

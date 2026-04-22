use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub struct DockerContainer {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub port: u16,
}

impl DockerContainer {
    pub fn start(
        image: &str,
        name: &str,
        port: u16,
        host_port: u16,
        env_vars: &[(&str, &str)],
    ) -> Result<Self, String> {
        // Check if container already exists and remove it
        let _ = Command::new("docker")
            .args(&["rm", "-f", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        // Build docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run");
        cmd.arg("-d");
        cmd.arg("--name").arg(name);
        cmd.arg("-p")
            .arg(format!("127.0.0.1:{}:{}", host_port, port));

        for (key, value) in env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        cmd.arg(image);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to start container: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker run failed: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(DockerContainer {
            id: container_id,
            name: name.to_string(),
            port: host_port,
        })
    }

    pub fn wait_for_ready(
        &self,
        health_check: fn(u16) -> bool,
        max_retries: u32,
    ) -> Result<(), String> {
        for i in 0..max_retries {
            if health_check(self.port) {
                println!("{} is ready", self.name);
                return Ok(());
            }

            if i < max_retries - 1 {
                thread::sleep(Duration::from_secs(2));
            }
        }

        Err(format!(
            "{} failed to become ready after {} attempts",
            self.name, max_retries
        ))
    }

    pub fn get_logs(&self) -> String {
        Command::new("docker")
            .args(&["logs", &self.name])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .unwrap_or_else(|| "Unable to retrieve logs".to_string())
    }
}

impl Drop for DockerContainer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(&["rm", "-f", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }
}

#[allow(dead_code)]
pub fn is_docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// Python Sidecar Process Manager
// Walking Skeleton (MVP0) - Python Interpreter Detection Implementation

use anyhow::Result;
use std::path::{Path, PathBuf};
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Python interpreter detection errors (design.md compliant)
#[derive(Error, Debug)]
pub enum PythonDetectionError {
    #[error("Python interpreter not found in PATH or standard locations")]
    PythonNotFound,

    #[error("Python version {found} is outside supported range (3.9 <= version < 3.13)")]
    VersionMismatch { found: String },

    #[error("Python architecture {found} does not match system (expected 64-bit)")]
    ArchitectureMismatch { found: String },

    #[error("Configured Python path does not exist: {path}")]
    ConfiguredPathInvalid { path: PathBuf },

    #[error("Python validation command failed: {0}")]
    ValidationFailed(String),
}

/// Python sidecar manager errors
#[derive(Error, Debug)]
pub enum PythonSidecarError {
    #[error("Failed to start Python process: {0}")]
    StartupFailed(String),

    #[error("Failed to communicate with Python process: {0}")]
    CommunicationFailed(String),

    #[error("Python process not running")]
    ProcessNotRunning,

    #[error("Python detection failed: {0}")]
    DetectionFailed(#[from] PythonDetectionError),
}

/// IPC message types for communication with Python sidecar
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcMessage {
    /// Request to start processing audio
    StartProcessing { audio_data: Vec<u8> },

    /// Request to stop processing
    StopProcessing,

    /// Response with transcription result
    TranscriptionResult { text: String, timestamp: u64 },

    /// Ready signal from Python
    Ready,

    /// Error from Python
    Error { message: String },
}

/// Python sidecar process manager
pub struct PythonSidecarManager {
    process: Option<tokio::process::Child>,
    stdin: Option<tokio::process::ChildStdin>,
    stdout: Option<tokio::io::BufReader<tokio::process::ChildStdout>>,
}

impl PythonSidecarManager {
    pub fn new() -> Self {
        Self {
            process: None,
            stdin: None,
            stdout: None,
        }
    }

    /// Check if the Python process is currently running
    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }

    /// Detect Python executable following the 6-step algorithm (design.md compliant)
    ///
    /// Priority order:
    /// 1. Environment variable / config file (APP_PYTHON)
    /// 2. Active virtual environment (VIRTUAL_ENV, CONDA_PREFIX)
    /// 3. Windows: py.exe launcher
    /// 4. POSIX: PATH scan (python3.12 -> python3.11 -> python3.10 -> python3.9)
    /// 5. Global python3 / python
    /// 6. Version validation (3.9 <= version < 3.13) and 64-bit architecture
    pub async fn detect_python_executable() -> Result<PathBuf, PythonDetectionError> {
        let supported_versions = vec![(3, 9), (3, 10), (3, 11), (3, 12)];

        // Step 1: Check APP_PYTHON environment variable
        if let Ok(configured_path) = std::env::var("APP_PYTHON") {
            let path = PathBuf::from(&configured_path);

            // If it's an absolute path, check if exists
            if path.is_absolute() {
                if !path.exists() {
                    return Err(PythonDetectionError::ConfiguredPathInvalid { path });
                }
                if Self::validate_python(&path, &supported_versions).await? {
                    return Ok(path);
                }
                // If validation failed, fall through to other detection methods
            } else {
                // If it's a command name, try to find it with which
                match which::which(&configured_path) {
                    Ok(resolved_path) => {
                        if Self::validate_python(&resolved_path, &supported_versions).await? {
                            return Ok(resolved_path);
                        }
                        // If validation failed, fall through to other detection methods
                    }
                    Err(_) => {
                        return Err(PythonDetectionError::ConfiguredPathInvalid { path });
                    }
                }
            }
        }

        // Step 2: Check active virtual environment
        if let Ok(venv_path) = std::env::var("VIRTUAL_ENV")
            .or_else(|_| std::env::var("CONDA_PREFIX"))
        {
            let python_path = if cfg!(windows) {
                PathBuf::from(&venv_path).join("Scripts").join("python.exe")
            } else {
                PathBuf::from(&venv_path).join("bin").join("python")
            };

            if python_path.exists() && Self::validate_python(&python_path, &supported_versions).await? {
                return Ok(python_path);
            }
        }

        // Step 3: Windows - py.exe launcher (simplified for Walking Skeleton)
        #[cfg(target_os = "windows")]
        {
            // For Walking Skeleton, we use a simplified approach
            // Full py.exe -0p implementation will be in MVP1
            if let Ok(path) = which::which("py") {
                if Self::validate_python(&path, &supported_versions).await? {
                    return Ok(path);
                }
            }
        }

        // Step 4 & 5: PATH scan
        let candidates = vec![
            "python3.12", "python3.11", "python3.10", "python3.9",
            "python3", "python"
        ];

        for name in candidates {
            if let Ok(path) = which::which(name) {
                if Self::validate_python(&path, &supported_versions).await? {
                    return Ok(path);
                }
            }
        }

        Err(PythonDetectionError::PythonNotFound)
    }

    /// Validate Python version and architecture
    async fn validate_python(
        path: &Path,
        supported_versions: &[(u32, u32)]
    ) -> Result<bool, PythonDetectionError> {
        use tokio::process::Command;

        let output = Command::new(path)
            .arg("-c")
            .arg("import sys, platform; print(f'{sys.version_info.major}.{sys.version_info.minor}', platform.machine())")
            .output()
            .await
            .map_err(|e| PythonDetectionError::ValidationFailed(e.to_string()))?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();

        if parts.len() < 2 {
            return Ok(false);
        }

        // Validate version
        let version = parts[0];
        let version_parts: Vec<&str> = version.split('.').collect();
        if version_parts.len() != 2 {
            return Ok(false);
        }

        let major: u32 = version_parts[0].parse().unwrap_or(0);
        let minor: u32 = version_parts[1].parse().unwrap_or(0);

        if !supported_versions.contains(&(major, minor)) {
            // Version mismatch - return false to try next candidate
            return Ok(false);
        }

        // Validate architecture (64-bit)
        let arch = parts[1];
        let is_64bit = arch.contains("64") || arch.contains("x86_64") || arch.contains("amd64") || arch.contains("arm64");

        if !is_64bit {
            // Architecture mismatch - return false to try next candidate
            return Ok(false);
        }

        Ok(true)
    }

    /// Validate Python version only (for testing)
    pub async fn validate_python_version(path: &Path) -> Result<bool, PythonDetectionError> {
        let supported_versions = vec![(3, 9), (3, 10), (3, 11), (3, 12)];
        Self::validate_python(path, &supported_versions).await
    }

    /// Validate architecture only (for testing)
    pub async fn validate_architecture(path: &Path) -> Result<bool, PythonDetectionError> {
        use tokio::process::Command;

        let output = Command::new(path)
            .arg("-c")
            .arg("import platform; print(platform.machine())")
            .output()
            .await
            .map_err(|e| PythonDetectionError::ValidationFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let arch = stdout.trim();
        let is_64bit = arch.contains("64") || arch.contains("x86_64") || arch.contains("amd64") || arch.contains("arm64");

        if !is_64bit {
            // Architecture mismatch - return false
            return Ok(false);
        }

        Ok(true)
    }

    /// Start the Python sidecar process
    pub async fn start(&mut self) -> Result<(), PythonSidecarError> {
        use tokio::process::Command;
        use tokio::io::BufReader;

        // Prevent double start
        if self.process.is_some() {
            return Err(PythonSidecarError::StartupFailed(
                "Process already running".to_string()
            ));
        }

        // Detect Python executable
        let python_path = Self::detect_python_executable().await?;

        // Get Python script path (relative to project root)
        let script_path = std::env::current_dir()
            .map_err(|e| PythonSidecarError::StartupFailed(e.to_string()))?
            .parent()
            .ok_or_else(|| PythonSidecarError::StartupFailed("Cannot find project root".to_string()))?
            .join("python-stt")
            .join("main.py");

        if !script_path.exists() {
            return Err(PythonSidecarError::StartupFailed(
                format!("Python script not found: {:?}", script_path)
            ));
        }

        // Start Python process
        let mut child = Command::new(&python_path)
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| PythonSidecarError::StartupFailed(e.to_string()))?;

        // Extract stdin/stdout handles
        let stdin = child.stdin.take()
            .ok_or_else(|| PythonSidecarError::StartupFailed("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| PythonSidecarError::StartupFailed("Failed to get stdout".to_string()))?;

        // Store process and streams
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.process = Some(child);

        Ok(())
    }

    /// Wait for "ready" message from Python sidecar
    pub async fn wait_for_ready(&mut self) -> Result<(), PythonSidecarError> {
        use tokio::io::AsyncBufReadExt;

        let stdout = self.stdout.as_mut()
            .ok_or_else(|| PythonSidecarError::ProcessNotRunning)?;

        let mut line = String::new();
        stdout.read_line(&mut line).await
            .map_err(|e| PythonSidecarError::CommunicationFailed(e.to_string()))?;

        let msg: serde_json::Value = serde_json::from_str(&line)
            .map_err(|e| PythonSidecarError::CommunicationFailed(
                format!("Failed to parse ready message: {}", e)
            ))?;

        if msg.get("type").and_then(|v| v.as_str()) != Some("ready") {
            return Err(PythonSidecarError::CommunicationFailed(
                format!("Expected 'ready' message, got: {:?}", msg)
            ));
        }

        Ok(())
    }

    /// Send a JSON message to Python sidecar via stdin
    pub async fn send_message(&mut self, message: serde_json::Value) -> Result<(), PythonSidecarError> {
        use tokio::io::AsyncWriteExt;

        let stdin = self.stdin.as_mut()
            .ok_or_else(|| PythonSidecarError::ProcessNotRunning)?;

        let json_str = serde_json::to_string(&message)
            .map_err(|e| PythonSidecarError::CommunicationFailed(e.to_string()))?;

        stdin.write_all(json_str.as_bytes()).await
            .map_err(|e| PythonSidecarError::CommunicationFailed(e.to_string()))?;

        stdin.write_all(b"\n").await
            .map_err(|e| PythonSidecarError::CommunicationFailed(e.to_string()))?;

        stdin.flush().await
            .map_err(|e| PythonSidecarError::CommunicationFailed(e.to_string()))?;

        Ok(())
    }

    /// Receive a message from Python sidecar
    pub async fn receive_message(&mut self) -> Result<IpcMessage, PythonSidecarError> {
        unimplemented!("PythonSidecarManager::receive_message - to be implemented in Task 4.1")
    }

    /// Graceful shutdown: Send shutdown message and wait for process to exit
    pub async fn shutdown(&mut self) -> Result<(), PythonSidecarError> {
        if self.process.is_none() {
            return Err(PythonSidecarError::ProcessNotRunning);
        }

        // Send shutdown message
        let shutdown_msg = serde_json::json!({
            "type": "shutdown",
            "id": "shutdown-1"
        });

        if let Err(_) = self.send_message(shutdown_msg).await {
            // If send fails, just force kill
            return self.stop().await;
        }

        // Wait for process to exit gracefully (3 second timeout)
        if let Some(mut process) = self.process.take() {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                process.wait()
            ).await;

            match result {
                Ok(Ok(_)) => {
                    // Process exited successfully
                    self.stdin.take();
                    self.stdout.take();
                    Ok(())
                }
                Ok(Err(e)) => {
                    Err(PythonSidecarError::StartupFailed(
                        format!("Failed to wait for process: {}", e)
                    ))
                }
                Err(_) => {
                    // Timeout - kill the process
                    let _ = process.kill().await;
                    self.stdin.take();
                    self.stdout.take();
                    Ok(())
                }
            }
        } else {
            Err(PythonSidecarError::ProcessNotRunning)
        }
    }

    /// Force close stdin (for testing timeout behavior)
    pub fn force_close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Get process ID (for testing)
    pub fn get_process_id(&self) -> Option<u32> {
        self.process.as_ref().and_then(|p| p.id())
    }

    /// Stop the Python sidecar process (legacy method, use shutdown instead)
    pub async fn stop(&mut self) -> Result<(), PythonSidecarError> {
        if let Some(mut process) = self.process.take() {
            // Close stdin to signal shutdown
            self.stdin.take();

            // Wait for process to exit (with timeout)
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                process.wait()
            ).await;

            match result {
                Ok(Ok(_)) => {
                    // Process exited successfully
                    self.stdout.take();
                    Ok(())
                }
                Ok(Err(e)) => {
                    Err(PythonSidecarError::StartupFailed(
                        format!("Failed to wait for process: {}", e)
                    ))
                }
                Err(_) => {
                    // Timeout - kill the process
                    let _ = process.kill().await;
                    self.stdout.take();
                    Ok(())
                }
            }
        } else {
            Err(PythonSidecarError::ProcessNotRunning)
        }
    }
}

impl Drop for PythonSidecarManager {
    fn drop(&mut self) {
        // Best effort cleanup when manager is dropped
        if let Some(process) = self.process.take() {
            // Kill the process if still running
            if let Some(pid) = process.id() {
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = std::process::Command::new("kill")
                        .arg(pid.to_string())
                        .output();
                }

                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("taskkill")
                        .args(&["/PID", &pid.to_string(), "/F"])
                        .output();
                }
            }
        }
    }
}

// Python Sidecar Process Manager
// Walking Skeleton (MVP0) - Empty Implementation

use anyhow::Result;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Python sidecar manager errors
#[derive(Error, Debug)]
pub enum PythonSidecarError {
    #[error("Failed to start Python process: {0}")]
    StartupFailed(String),

    #[error("Failed to communicate with Python process: {0}")]
    CommunicationFailed(String),

    #[error("Python process not running")]
    ProcessNotRunning,
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
pub struct PythonSidecarManager;

impl PythonSidecarManager {
    pub fn new() -> Self {
        Self
    }

    /// Start the Python sidecar process
    pub async fn start(&mut self) -> Result<(), PythonSidecarError> {
        unimplemented!("PythonSidecarManager::start - to be implemented in Task 3.1")
    }

    /// Send a message to Python sidecar
    pub async fn send_message(&mut self, _message: IpcMessage) -> Result<(), PythonSidecarError> {
        unimplemented!("PythonSidecarManager::send_message - to be implemented in Task 4.1")
    }

    /// Receive a message from Python sidecar
    pub async fn receive_message(&mut self) -> Result<IpcMessage, PythonSidecarError> {
        unimplemented!("PythonSidecarManager::receive_message - to be implemented in Task 4.1")
    }

    /// Stop the Python sidecar process
    pub async fn stop(&mut self) -> Result<(), PythonSidecarError> {
        unimplemented!("PythonSidecarManager::stop - to be implemented in Task 5.1")
    }
}

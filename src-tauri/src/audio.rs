// Audio Device Abstraction
// Walking Skeleton (MVP0) - Fake Implementation with Timer Loop

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Audio chunk callback type
pub type AudioChunkCallback = Box<dyn Fn(Vec<u8>) + Send + Sync>;

/// Audio device trait for recording audio
pub trait AudioDevice: Send + Sync {
    /// Initialize the audio device
    fn initialize(&mut self) -> Result<()>;

    /// Start recording audio (async)
    fn start(&mut self) -> Result<()>;

    /// Stop recording audio
    fn stop(&mut self) -> Result<()>;
}

/// Fake audio device for testing (generates dummy data every 100ms)
pub struct FakeAudioDevice {
    /// Flag to track if the device is currently running
    is_running: bool,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Background task handle
    task_handle: Option<JoinHandle<()>>,
}

impl FakeAudioDevice {
    /// Create a new FakeAudioDevice
    pub fn new() -> Self {
        Self {
            is_running: false,
            shutdown_tx: None,
            task_handle: None,
        }
    }

    /// Generate 16 bytes of dummy audio data
    /// Returns: Vec<u8> with exactly 16 zero bytes
    pub fn generate_dummy_data(&self) -> Vec<u8> {
        vec![0u8; 16]
    }

    /// Check if the device is currently running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Start recording with callback (async version)
    pub async fn start_with_callback<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        if self.is_running {
            return Ok(());
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn background task for 100ms interval
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Generate dummy data
                        let data = vec![0u8; 16];
                        callback(data);
                    }
                    _ = shutdown_rx.recv() => {
                        // Shutdown signal received
                        break;
                    }
                }
            }
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.task_handle = Some(handle);
        self.is_running = true;

        Ok(())
    }
}

impl AudioDevice for FakeAudioDevice {
    /// Initialize the audio device
    /// Sets up internal state but doesn't start data generation
    fn initialize(&mut self) -> Result<()> {
        // Reset state
        self.is_running = false;
        self.shutdown_tx = None;
        self.task_handle = None;
        Ok(())
    }

    /// Start recording audio
    /// Note: For Walking Skeleton, use start_with_callback() instead
    /// This method just sets the flag for backward compatibility
    fn start(&mut self) -> Result<()> {
        self.is_running = true;
        Ok(())
    }

    /// Stop recording audio
    /// Halts data generation and cleans up resources
    fn stop(&mut self) -> Result<()> {
        self.is_running = false;

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());
        }

        // Wait for task to finish (non-blocking)
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        Ok(())
    }
}

impl Drop for FakeAudioDevice {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

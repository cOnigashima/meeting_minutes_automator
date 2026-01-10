// Audio Device Abstraction
// Walking Skeleton (MVP0) - Fake Implementation with Timer Loop

use crate::audio_device_adapter::{AudioDeviceAdapter, AudioDeviceEvent, AudioDeviceInfo, AudioEventSender};
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
/// Task 10.4: Extended with device event simulation for E2E testing
pub struct FakeAudioDevice {
    /// Flag to track if the device is currently running
    is_running: bool,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Background task handle
    task_handle: Option<JoinHandle<()>>,
    /// Event sender for device monitoring (Task 10.4)
    event_tx: Option<AudioEventSender>,
    /// Device ID for simulation (Task 10.4)
    device_id: Option<String>,
}

impl FakeAudioDevice {
    /// Create a new FakeAudioDevice
    pub fn new() -> Self {
        Self {
            is_running: false,
            shutdown_tx: None,
            task_handle: None,
            event_tx: None,
            device_id: None,
        }
    }

    /// Set event sender for device monitoring (Task 10.4)
    /// Must be called before start_recording to enable event simulation
    pub fn set_event_sender(&mut self, tx: AudioEventSender) {
        self.event_tx = Some(tx);
    }

    /// Simulate device disconnect (Task 10.4: STT-REQ-004.9)
    /// Sends DeviceGone event and stops recording
    pub fn simulate_disconnect(&mut self) -> Result<()> {
        if let Some(ref tx) = self.event_tx {
            if let Some(ref dev_id) = self.device_id {
                tx.send(AudioDeviceEvent::DeviceGone {
                    device_id: dev_id.clone(),
                })
                .map_err(|e| anyhow::anyhow!("Failed to send DeviceGone event: {}", e))?;
            }
        }
        self.stop()
    }

    /// Simulate successful reconnect (Task 10.4: STT-REQ-004.11)
    /// Restarts recording with the same device_id
    pub fn simulate_reconnect(&mut self) -> Result<()> {
        if !self.is_running {
            self.is_running = true;
        }
        Ok(())
    }

    /// Simulate stream error (Task 10.4: testing)
    /// Sends StreamError event without stopping recording
    pub fn simulate_stream_error(&self, error_msg: &str) -> Result<()> {
        if let Some(ref tx) = self.event_tx {
            tx.send(AudioDeviceEvent::StreamError(error_msg.to_string()))
                .map_err(|e| anyhow::anyhow!("Failed to send StreamError event: {}", e))?;
        }
        Ok(())
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

impl FakeAudioDevice {
    /// Task 9.1: Static device enumeration (decoupled from recorder instance)
    /// Returns dummy device list for MVP0/UI testing
    /// Note: This approach aligns with real device adapters (CoreAudio/WASAPI/ALSA)
    /// which perform static host queries without requiring an active stream.
    pub fn enumerate_devices_static() -> Result<Vec<AudioDeviceInfo>> {
        Ok(vec![
            AudioDeviceInfo {
                id: "fake-device-0".to_string(),
                name: "Fake Microphone".to_string(),
                sample_rate: 16000,
                channels: 1,
                is_loopback: false,
            },
            AudioDeviceInfo {
                id: "fake-device-1".to_string(),
                name: "Fake BlackHole 2ch".to_string(),
                sample_rate: 16000,
                channels: 2,
                is_loopback: true,
            },
        ])
    }
}

/// Task 9.1: Implement AudioDeviceAdapter for FakeAudioDevice
/// Returns dummy device list for UI testing
impl AudioDeviceAdapter for FakeAudioDevice {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        // Delegate to static method (backwards compatibility)
        Self::enumerate_devices_static()
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        self.start()
    }

    fn start_recording_with_callback(
        &mut self,
        device_id: &str,
        _callback: crate::audio_device_adapter::AudioChunkCallback,
    ) -> Result<()> {
        // Store device_id for simulation (Task 10.4)
        self.device_id = Some(device_id.to_string());
        // For MVP0, just set running flag
        self.is_running = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        self.stop()
    }

    fn is_recording(&self) -> bool {
        self.is_running
    }

    fn check_permission(&self) -> Result<()> {
        // Always return Ok for fake device
        Ok(())
    }

    fn set_event_sender(&mut self, _tx: crate::audio_device_adapter::AudioEventSender) {
        // Fake device: no-op (no real device monitoring)
    }
}

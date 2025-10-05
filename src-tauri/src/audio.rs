// Audio Device Abstraction
// Walking Skeleton (MVP0) - Fake Implementation for TDD

use anyhow::Result;

/// Audio device trait for recording audio
pub trait AudioDevice: Send + Sync {
    /// Initialize the audio device
    fn initialize(&mut self) -> Result<()>;

    /// Start recording audio
    fn start(&mut self) -> Result<()>;

    /// Stop recording audio
    fn stop(&mut self) -> Result<()>;
}

/// Fake audio device for testing (generates dummy data every 100ms)
///
/// For Walking Skeleton (MVP0), this is a simplified implementation.
/// Actual timer-based data generation will be added in Task 3 when
/// integrating with PythonSidecarManager in async context.
pub struct FakeAudioDevice {
    /// Flag to track if the device is currently running
    is_running: bool,
}

impl FakeAudioDevice {
    /// Create a new FakeAudioDevice
    pub fn new() -> Self {
        Self {
            is_running: false,
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
}

impl AudioDevice for FakeAudioDevice {
    /// Initialize the audio device
    /// Sets up internal state but doesn't start data generation
    fn initialize(&mut self) -> Result<()> {
        // Reset state
        self.is_running = false;
        Ok(())
    }

    /// Start recording audio
    /// Sets the running flag to true
    ///
    /// Note: Actual 100ms interval timer and data generation loop
    /// will be implemented in Task 3 when integrated with
    /// PythonSidecarManager in async context
    fn start(&mut self) -> Result<()> {
        self.is_running = true;
        Ok(())
    }

    /// Stop recording audio
    /// Halts data generation and cleans up resources
    fn stop(&mut self) -> Result<()> {
        self.is_running = false;
        Ok(())
    }
}

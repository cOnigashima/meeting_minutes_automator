// Audio Device Abstraction
// Walking Skeleton (MVP0) - Empty Implementation

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

/// Fake audio device for testing (generates dummy data)
pub struct FakeAudioDevice;

impl FakeAudioDevice {
    pub fn new() -> Self {
        Self
    }
}

impl AudioDevice for FakeAudioDevice {
    fn initialize(&mut self) -> Result<()> {
        unimplemented!("FakeAudioDevice::initialize - to be implemented in Task 2.1")
    }

    fn start(&mut self) -> Result<()> {
        unimplemented!("FakeAudioDevice::start - to be implemented in Task 2.1")
    }

    fn stop(&mut self) -> Result<()> {
        unimplemented!("FakeAudioDevice::stop - to be implemented in Task 2.1")
    }
}

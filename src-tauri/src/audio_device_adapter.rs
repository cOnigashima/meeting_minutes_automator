// Real Audio Device Adapter (MVP1 - Real STT)
// Cross-platform audio device management for macOS, Windows, Linux
// Requirement: STT-REQ-001 (Real Audio Device Management)

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

// OS-specific imports
#[cfg(target_os = "macos")]
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "linux")]
use cpal::traits::{DeviceTrait, HostTrait};

/// Audio device metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioDeviceInfo {
    /// Device ID (unique identifier)
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
}

/// Audio device adapter trait for real audio recording
/// Requirement: STT-REQ-001.1, STT-REQ-001.2, STT-REQ-001.4, STT-REQ-001.5
pub trait AudioDeviceAdapter: Send + Sync {
    /// Enumerate available audio input devices
    /// Requirement: STT-REQ-001.1, STT-REQ-001.2
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    /// Start recording from the specified device
    /// Requirement: STT-REQ-001.4, STT-REQ-001.5, STT-REQ-001.6
    fn start_recording(&mut self, device_id: &str) -> Result<()>;

    /// Stop recording
    /// Requirement: STT-REQ-001.7
    fn stop_recording(&mut self) -> Result<()>;

    /// Check if currently recording
    fn is_recording(&self) -> bool;
}

// ============================================================================
// OS-Specific Implementations
// ============================================================================

/// macOS CoreAudio adapter
#[cfg(target_os = "macos")]
pub struct CoreAudioAdapter {
    is_recording: bool,
}

#[cfg(target_os = "macos")]
impl CoreAudioAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
        }
    }
}

#[cfg(target_os = "macos")]
impl AudioDeviceAdapter for CoreAudioAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        // TODO: Implement using cpal::default_host().input_devices()
        Ok(vec![])
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement CoreAudio recording
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        // TODO: Implement stop recording
        self.is_recording = false;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }
}

/// Windows WASAPI adapter
#[cfg(target_os = "windows")]
pub struct WasapiAdapter {
    is_recording: bool,
}

#[cfg(target_os = "windows")]
impl WasapiAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
        }
    }
}

#[cfg(target_os = "windows")]
impl AudioDeviceAdapter for WasapiAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        // TODO: Implement using cpal::default_host().input_devices()
        Ok(vec![])
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement WASAPI recording
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        // TODO: Implement stop recording
        self.is_recording = false;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }
}

/// Linux ALSA adapter
#[cfg(target_os = "linux")]
pub struct AlsaAdapter {
    is_recording: bool,
}

#[cfg(target_os = "linux")]
impl AlsaAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
        }
    }
}

#[cfg(target_os = "linux")]
impl AudioDeviceAdapter for AlsaAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        // TODO: Implement using cpal::default_host().input_devices()
        Ok(vec![])
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement ALSA recording
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        // TODO: Implement stop recording
        self.is_recording = false;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }
}

// ============================================================================
// OS Detection and Adapter Selection
// ============================================================================

/// Create the appropriate audio device adapter for the current OS
/// Requirement: STT-REQ-004.3, STT-REQ-004.4, STT-REQ-004.5
pub fn create_audio_adapter() -> Result<Box<dyn AudioDeviceAdapter>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(CoreAudioAdapter::new()))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(WasapiAdapter::new()))
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(AlsaAdapter::new()))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(anyhow!("Unsupported operating system"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper: Mock adapter for testing
    struct MockAudioAdapter {
        is_recording: bool,
        devices: Vec<AudioDeviceInfo>,
    }

    impl MockAudioAdapter {
        fn new() -> Self {
            Self {
                is_recording: false,
                devices: vec![
                    AudioDeviceInfo {
                        id: "device-1".to_string(),
                        name: "Test Microphone".to_string(),
                        sample_rate: 16000,
                        channels: 1,
                    },
                ],
            }
        }
    }

    impl AudioDeviceAdapter for MockAudioAdapter {
        fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
            Ok(self.devices.clone())
        }

        fn start_recording(&mut self, _device_id: &str) -> Result<()> {
            self.is_recording = true;
            Ok(())
        }

        fn stop_recording(&mut self) -> Result<()> {
            self.is_recording = false;
            Ok(())
        }

        fn is_recording(&self) -> bool {
            self.is_recording
        }
    }

    #[test]
    fn test_enumerate_devices() {
        let adapter = MockAudioAdapter::new();
        let devices = adapter.enumerate_devices().unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "device-1");
        assert_eq!(devices[0].name, "Test Microphone");
        assert_eq!(devices[0].sample_rate, 16000);
        assert_eq!(devices[0].channels, 1);
    }

    #[test]
    fn test_start_stop_recording() {
        let mut adapter = MockAudioAdapter::new();

        // Initially not recording
        assert!(!adapter.is_recording());

        // Start recording
        adapter.start_recording("device-1").unwrap();
        assert!(adapter.is_recording());

        // Stop recording
        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }

    #[test]
    fn test_create_audio_adapter() {
        // Test OS-specific adapter creation
        let adapter = create_audio_adapter();
        assert!(adapter.is_ok(), "Should create audio adapter for current OS");

        let adapter = adapter.unwrap();
        assert!(!adapter.is_recording(), "Should not be recording initially");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_coreaudio_adapter() {
        let mut adapter = CoreAudioAdapter::new();
        assert!(!adapter.is_recording());

        adapter.start_recording("test-device").unwrap();
        assert!(adapter.is_recording());

        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_wasapi_adapter() {
        let mut adapter = WasapiAdapter::new();
        assert!(!adapter.is_recording());

        adapter.start_recording("test-device").unwrap();
        assert!(adapter.is_recording());

        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_alsa_adapter() {
        let mut adapter = AlsaAdapter::new();
        assert!(!adapter.is_recording());

        adapter.start_recording("test-device").unwrap();
        assert!(adapter.is_recording());

        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }
}

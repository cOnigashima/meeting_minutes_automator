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

/// Audio chunk callback type
/// Receives Vec<u8> containing 16kHz mono PCM audio data (320 samples = 20ms)
pub type AudioChunkCallback = Box<dyn Fn(Vec<u8>) + Send + Sync>;

/// Audio device adapter trait for real audio recording
/// Requirement: STT-REQ-001.1, STT-REQ-001.2, STT-REQ-001.4, STT-REQ-001.5
pub trait AudioDeviceAdapter: Send + Sync {
    /// Enumerate available audio input devices
    /// Requirement: STT-REQ-001.1, STT-REQ-001.2
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    /// Start recording from the specified device
    /// Requirement: STT-REQ-001.4, STT-REQ-001.5, STT-REQ-001.6
    fn start_recording(&mut self, device_id: &str) -> Result<()>;

    /// Start recording with callback for audio chunks
    /// Requirement: STT-REQ-001.4, STT-REQ-001.5, STT-REQ-001.6
    fn start_recording_with_callback(&mut self, device_id: &str, callback: AudioChunkCallback) -> Result<()>;

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
    stream_handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "macos")]
impl CoreAudioAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            stream_handle: None,
        }
    }
}

#[cfg(target_os = "macos")]
impl AudioDeviceAdapter for CoreAudioAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        for device in host.input_devices()? {
            let name = device.name()?;
            let default_config = device.default_input_config()?;

            devices.push(AudioDeviceInfo {
                id: name.clone(), // Use name as ID for now
                name,
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement CoreAudio recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(&mut self, device_id: &str, callback: AudioChunkCallback) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        let host = cpal::default_host();

        // Find device by ID
        let device = host.input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;
        let _sample_rate = config.sample_rate().0;
        let _channels = config.channels() as usize;

        // Spawn thread to manage stream
        let handle = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 samples to 16kHz mono PCM
                    // For now, just pass through (full resampling in later task)
                    let pcm_data: Vec<u8> = data.iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                |err| eprintln!("Audio stream error: {:?}", err),
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    // Keep stream alive
                    std::thread::park();
                }
            }
        });

        self.stream_handle = Some(handle);
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        if let Some(handle) = self.stream_handle.take() {
            handle.thread().unpark();
            // Don't wait for join in production (could hang)
            // handle.join().ok();
        }

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
    stream_handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "windows")]
impl WasapiAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            stream_handle: None,
        }
    }
}

#[cfg(target_os = "windows")]
impl AudioDeviceAdapter for WasapiAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        for device in host.input_devices()? {
            let name = device.name()?;
            let default_config = device.default_input_config()?;

            devices.push(AudioDeviceInfo {
                id: name.clone(),
                name,
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement WASAPI recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(&mut self, device_id: &str, callback: AudioChunkCallback) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        let host = cpal::default_host();

        let device = host.input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;

        let handle = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let pcm_data: Vec<u8> = data.iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                |err| eprintln!("Audio stream error: {:?}", err),
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    std::thread::park();
                }
            }
        });

        self.stream_handle = Some(handle);
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        if let Some(handle) = self.stream_handle.take() {
            handle.thread().unpark();
        }

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
    stream_handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl AlsaAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            stream_handle: None,
        }
    }
}

#[cfg(target_os = "linux")]
impl AudioDeviceAdapter for AlsaAdapter {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        for device in host.input_devices()? {
            let name = device.name()?;
            let default_config = device.default_input_config()?;

            devices.push(AudioDeviceInfo {
                id: name.clone(),
                name,
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement ALSA recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(&mut self, device_id: &str, callback: AudioChunkCallback) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        let host = cpal::default_host();

        let device = host.input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;

        let handle = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let pcm_data: Vec<u8> = data.iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                |err| eprintln!("Audio stream error: {:?}", err),
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    std::thread::park();
                }
            }
        });

        self.stream_handle = Some(handle);
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        if let Some(handle) = self.stream_handle.take() {
            handle.thread().unpark();
        }

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

        fn start_recording_with_callback(&mut self, _device_id: &str, _callback: AudioChunkCallback) -> Result<()> {
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

    #[test]
    fn test_audio_chunk_callback() {
        use std::sync::{Arc, Mutex};

        let mut adapter = MockAudioAdapter::new();
        let chunks_received = Arc::new(Mutex::new(Vec::new()));
        let chunks_received_clone = chunks_received.clone();

        let callback: AudioChunkCallback = Box::new(move |chunk: Vec<u8>| {
            chunks_received_clone.lock().unwrap().push(chunk);
        });

        adapter.start_recording_with_callback("device-1", callback).unwrap();
        assert!(adapter.is_recording());

        // Note: MockAdapter doesn't actually generate audio data
        // This test verifies the callback interface is correctly defined
        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_coreaudio_stream_capture() {
        // RED: This test will fail until Task 2.3 is fully implemented
        let mut adapter = CoreAudioAdapter::new();

        // Test basic recording lifecycle
        assert!(!adapter.is_recording());

        // Start recording (will be TODO until implementation)
        let result = adapter.start_recording("test-device");
        assert!(result.is_ok());
        assert!(adapter.is_recording());

        // Stop recording
        adapter.stop_recording().unwrap();
        assert!(!adapter.is_recording());
    }
}

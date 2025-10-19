// Real Audio Device Adapter (MVP1 - Real STT)
// Cross-platform audio device management for macOS, Windows, Linux
// Requirement: STT-REQ-001 (Real Audio Device Management)

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// OS-specific imports
#[cfg(target_os = "macos")]
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(target_os = "linux")]
use cpal::traits::{DeviceTrait, HostTrait};

// ============================================================================
// Audio Device Events (Task 2.5 - Device Disconnection Detection)
// ============================================================================

/// Audio device event types for monitoring device health
/// Requirement: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11
#[derive(Debug, Clone)]
pub enum AudioDeviceEvent {
    /// Stream error from cpal error callback
    /// Fired when cpal reports a stream error (device error, buffer underrun, etc.)
    StreamError(String),

    /// No audio data received (liveness watchdog timeout)
    /// Fired when audio callback hasn't been called for longer than threshold
    Stalled { elapsed_ms: u64 },

    /// Device disappeared from enumeration
    /// Fired when device polling detects the device is no longer available
    DeviceGone { device_id: String },
}

/// Event sender type for audio device events
pub type AudioEventSender = mpsc::Sender<AudioDeviceEvent>;

/// Event receiver type for audio device events
pub type AudioEventReceiver = mpsc::Receiver<AudioDeviceEvent>;

// ============================================================================
// Audio Device Metadata
// ============================================================================

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
    /// Whether this is a loopback/virtual device
    /// Requirement: STT-REQ-004.6, STT-REQ-004.7, STT-REQ-004.8
    pub is_loopback: bool,
}

/// Audio chunk callback type
/// Receives Vec<u8> containing 16kHz mono PCM audio data (320 samples = 20ms)
pub type AudioChunkCallback = Box<dyn Fn(Vec<u8>) + Send + Sync>;

/// Check if a device name indicates a loopback/virtual audio device
/// Requirement: STT-REQ-004.6, STT-REQ-004.7, STT-REQ-004.8
fn is_loopback_device(name: &str) -> bool {
    // macOS: BlackHole, Loopback Audio, etc.
    // Windows: WASAPI loopback devices (detected by name patterns)
    // Linux: PulseAudio monitor devices
    name.contains("BlackHole") ||
    name.contains("Loopback") ||
    name.contains("Monitor of") ||
    name.contains(".monitor") ||
    name.contains("Stereo Mix") ||  // Windows legacy
    name.contains("Wave Out Mix") // Windows legacy
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

    /// Start recording with callback for audio chunks
    /// Requirement: STT-REQ-001.4, STT-REQ-001.5, STT-REQ-001.6
    fn start_recording_with_callback(
        &mut self,
        device_id: &str,
        callback: AudioChunkCallback,
    ) -> Result<()>;

    /// Stop recording
    /// Requirement: STT-REQ-001.7
    fn stop_recording(&mut self) -> Result<()>;

    /// Check if currently recording
    fn is_recording(&self) -> bool;

    /// Check microphone permission status
    /// Requirement: STT-REQ-004.1, STT-REQ-004.2
    /// Returns Ok(()) if permission is granted, Err with user-friendly message if denied
    fn check_permission(&self) -> Result<()>;
}

// ============================================================================
// OS-Specific Implementations
// ============================================================================

/// macOS CoreAudio adapter with device monitoring
/// Task 2.5: Device disconnection detection and auto-reconnect
#[cfg(target_os = "macos")]
pub struct CoreAudioAdapter {
    /// Recording state
    is_recording: bool,

    /// Currently recording device ID (for monitoring)
    device_id: Option<String>,

    /// Stream thread handle (stream lives in this thread)
    stream_thread: Option<JoinHandle<()>>,

    /// Last callback timestamp (for liveness monitoring)
    last_callback: Arc<Mutex<Instant>>,

    /// Event sender for device events
    event_tx: Option<AudioEventSender>,

    /// Liveness watchdog thread handle
    watchdog_handle: Option<JoinHandle<()>>,

    /// Device polling thread handle
    polling_handle: Option<JoinHandle<()>>,

    /// Shutdown senders for all threads
    stream_shutdown_tx: Option<mpsc::Sender<()>>,
    watchdog_shutdown_tx: Option<mpsc::Sender<()>>,
    polling_shutdown_tx: Option<mpsc::Sender<()>>,
}

#[cfg(target_os = "macos")]
impl CoreAudioAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            device_id: None,
            stream_thread: None,
            last_callback: Arc::new(Mutex::new(Instant::now())),
            event_tx: None,
            watchdog_handle: None,
            polling_handle: None,
            stream_shutdown_tx: None,
            watchdog_shutdown_tx: None,
            polling_shutdown_tx: None,
        }
    }

    /// Set event sender for device monitoring
    /// Must be called before start_recording to enable monitoring
    pub fn set_event_sender(&mut self, tx: AudioEventSender) {
        self.event_tx = Some(tx);
    }

    /// Start liveness watchdog thread (Task 2.5: STT-REQ-004.9)
    /// Monitors last_callback timestamp and fires Stalled event if threshold exceeded
    /// Returns shutdown sender for thread coordination
    fn start_watchdog(&mut self) -> mpsc::Sender<()> {
        let last_cb = Arc::clone(&self.last_callback);
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let event_tx = self.event_tx.clone();

        let handle = std::thread::spawn(move || {
            const CHECK_INTERVAL_MS: u64 = 250;
            const STALL_THRESHOLD_MS: u128 = 1200;

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                std::thread::sleep(Duration::from_millis(CHECK_INTERVAL_MS));

                let elapsed = last_cb.lock().unwrap().elapsed();

                if elapsed.as_millis() > STALL_THRESHOLD_MS {
                    if let Some(tx) = &event_tx {
                        tx.send(AudioDeviceEvent::Stalled {
                            elapsed_ms: elapsed.as_millis() as u64,
                        })
                        .ok();
                    }
                    eprintln!(
                        "⚠️ Audio stream stalled: {}ms since last callback",
                        elapsed.as_millis()
                    );
                    break; // Stop watchdog on stall detection
                }
            }
        });

        self.watchdog_handle = Some(handle);
        shutdown_tx
    }

    /// Start device polling thread (Task 2.5: STT-REQ-004.9)
    /// Periodically checks if device still exists, fires DeviceGone event if not found
    /// Returns shutdown sender for thread coordination
    fn start_device_polling(&mut self) -> mpsc::Sender<()> {
        let device_id = self.device_id.clone().unwrap();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let event_tx = self.event_tx.clone();

        let handle = std::thread::spawn(move || {
            const POLL_INTERVAL_SEC: u64 = 3;

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SEC));

                // Check device existence
                let host = cpal::default_host();
                let exists = host
                    .input_devices()
                    .ok()
                    .and_then(|mut devices| {
                        devices.find(|d| d.name().ok().as_ref() == Some(&device_id))
                    })
                    .is_some();

                if !exists {
                    if let Some(tx) = &event_tx {
                        tx.send(AudioDeviceEvent::DeviceGone {
                            device_id: device_id.clone(),
                        })
                        .ok();
                    }
                    eprintln!("🔌 Device disconnected: {}", device_id);
                    break;
                }
            }
        });

        self.polling_handle = Some(handle);
        shutdown_tx
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
                name: name.clone(),
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
                is_loopback: is_loopback_device(&name),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement CoreAudio recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(
        &mut self,
        device_id: &str,
        callback: AudioChunkCallback,
    ) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        let host = cpal::default_host();

        // Find device by ID
        let device = host
            .input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;

        // Liveness tracking (Task 2.5)
        let last_cb = Arc::clone(&self.last_callback);
        *last_cb.lock().unwrap() = Instant::now();

        // Error event channel (Task 2.5)
        let event_tx_clone = self.event_tx.clone();

        // Create shutdown channel for stream thread
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Spawn stream thread (Task 2.5: reliable cleanup)
        let stream_thread = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Update liveness timestamp (Task 2.5)
                    *last_cb.lock().unwrap() = Instant::now();

                    // Convert f32 samples to 16kHz mono PCM
                    // For now, just pass through (full resampling in later task)
                    let pcm_data: Vec<u8> = data
                        .iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                move |err| {
                    // Send error event (Task 2.5: STT-REQ-004.9)
                    if let Some(tx) = &event_tx_clone {
                        tx.send(AudioDeviceEvent::StreamError(format!("{:?}", err)))
                            .ok();
                    }
                    eprintln!("❌ Audio stream error: {:?}", err);
                },
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    // Wait for shutdown signal
                    shutdown_rx.recv().ok();
                    // Stream will be dropped here (reliable cleanup)
                }
            }
        });

        self.stream_thread = Some(stream_thread);
        self.stream_shutdown_tx = Some(shutdown_tx);

        // Store device_id for monitoring
        self.device_id = Some(device_id.to_string());

        // Start watchdog & polling (Task 2.5)
        self.watchdog_shutdown_tx = Some(self.start_watchdog());
        self.polling_shutdown_tx = Some(self.start_device_polling());

        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        // Signal all threads to stop (Task 2.5: reliable cleanup)
        if let Some(tx) = self.stream_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.watchdog_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.polling_shutdown_tx.take() {
            tx.send(()).ok();
        }

        // Join stream thread (stream will be dropped)
        if let Some(handle) = self.stream_thread.take() {
            handle.join().ok();
        }

        // Join watchdog thread (Task 2.5)
        if let Some(handle) = self.watchdog_handle.take() {
            handle.join().ok();
        }

        // Join polling thread (Task 2.5)
        if let Some(handle) = self.polling_handle.take() {
            handle.join().ok();
        }

        // Reset state
        self.is_recording = false;
        self.device_id = None;

        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }

    fn check_permission(&self) -> Result<()> {
        // On macOS, cpal will trigger the OS permission dialog automatically
        // when attempting to enumerate devices. We verify permission by attempting
        // to enumerate devices. If successful, permission is granted.
        let host = cpal::default_host();

        match host.input_devices() {
            Ok(mut devices) => {
                // Try to access at least one device
                if devices.next().is_some() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "マイクアクセスが拒否されました。システム設定から許可してください"
                    ))
                }
            }
            Err(e) => {
                // Device enumeration failed - likely permission denied
                log_error!(
                    "audio_device_adapter",
                    "microphone_permission_denied",
                    format!("{:?}", e)
                );
                Err(anyhow!(
                    "マイクアクセスが拒否されました。システム設定から許可してください"
                ))
            }
        }
    }
}

/// Windows WASAPI adapter
#[cfg(target_os = "windows")]
pub struct WasapiAdapter {
    is_recording: bool,
    device_id: Option<String>,
    stream_thread: Option<JoinHandle<()>>,
    last_callback: Arc<Mutex<Instant>>,
    event_tx: Option<AudioEventSender>,
    watchdog_handle: Option<JoinHandle<()>>,
    polling_handle: Option<JoinHandle<()>>,
    stream_shutdown_tx: Option<mpsc::Sender<()>>,
    watchdog_shutdown_tx: Option<mpsc::Sender<()>>,
    polling_shutdown_tx: Option<mpsc::Sender<()>>,
}

#[cfg(target_os = "windows")]
impl WasapiAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            device_id: None,
            stream_thread: None,
            last_callback: Arc::new(Mutex::new(Instant::now())),
            event_tx: None,
            watchdog_handle: None,
            polling_handle: None,
            stream_shutdown_tx: None,
            watchdog_shutdown_tx: None,
            polling_shutdown_tx: None,
        }
    }

    pub fn set_event_sender(&mut self, tx: AudioEventSender) {
        self.event_tx = Some(tx);
    }

    fn start_watchdog(&mut self) -> mpsc::Sender<()> {
        let last_cb = Arc::clone(&self.last_callback);
        let event_tx = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let handle = std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));

            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            let elapsed = last_cb.lock().unwrap().elapsed();
            if elapsed > Duration::from_millis(1200) {
                if let Some(tx) = &event_tx {
                    tx.send(AudioDeviceEvent::Stalled {
                        elapsed_ms: elapsed.as_millis() as u64,
                    })
                    .ok();
                }
            }
        });

        self.watchdog_handle = Some(handle);
        shutdown_tx
    }

    fn start_device_polling(&mut self) -> mpsc::Sender<()> {
        let device_id = self.device_id.clone();
        let event_tx = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let handle = std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(3));

            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            if let Some(ref dev_id) = device_id {
                let host = cpal::default_host();
                let device_exists = host
                    .input_devices()
                    .ok()
                    .and_then(|mut devices| {
                        devices.find(|d| d.name().ok().as_ref() == Some(&dev_id))
                    })
                    .is_some();

                if !device_exists {
                    if let Some(tx) = &event_tx {
                        tx.send(AudioDeviceEvent::DeviceGone {
                            device_id: dev_id.clone(),
                        })
                        .ok();
                    }
                    break;
                }
            }
        });

        self.polling_handle = Some(handle);
        shutdown_tx
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
                name: name.clone(),
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
                is_loopback: is_loopback_device(&name),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement WASAPI recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(
        &mut self,
        device_id: &str,
        callback: AudioChunkCallback,
    ) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        self.device_id = Some(device_id.to_string());

        let host = cpal::default_host();

        let device = host
            .input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;

        // Initialize liveness timestamp
        let last_cb = Arc::clone(&self.last_callback);
        *last_cb.lock().unwrap() = Instant::now();

        let event_tx_clone = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let stream_thread = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Update liveness timestamp
                    *last_cb.lock().unwrap() = Instant::now();

                    let pcm_data: Vec<u8> = data
                        .iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                move |err| {
                    eprintln!("Audio stream error: {:?}", err);
                    if let Some(tx) = &event_tx_clone {
                        tx.send(AudioDeviceEvent::StreamError(format!("{:?}", err)))
                            .ok();
                    }
                },
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    shutdown_rx.recv().ok();
                }
            }
        });

        self.stream_thread = Some(stream_thread);
        self.stream_shutdown_tx = Some(shutdown_tx);
        self.watchdog_shutdown_tx = Some(self.start_watchdog());
        self.polling_shutdown_tx = Some(self.start_device_polling());
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        // Signal all threads to stop
        if let Some(tx) = self.stream_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.watchdog_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.polling_shutdown_tx.take() {
            tx.send(()).ok();
        }

        // Join all threads
        if let Some(handle) = self.stream_thread.take() {
            handle.join().ok();
        }
        if let Some(handle) = self.watchdog_handle.take() {
            handle.join().ok();
        }
        if let Some(handle) = self.polling_handle.take() {
            handle.join().ok();
        }

        self.is_recording = false;
        self.device_id = None;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }

    fn check_permission(&self) -> Result<()> {
        // On Windows, microphone access is managed through Settings > Privacy > Microphone
        // WASAPI will automatically handle permission. We verify by attempting device enumeration.
        let host = cpal::default_host();

        match host.input_devices() {
            Ok(mut devices) => {
                if devices.next().is_some() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "マイクアクセスが拒否されました。システム設定から許可してください"
                    ))
                }
            }
            Err(e) => {
                log_error!(
                    "audio_device_adapter",
                    "microphone_permission_denied",
                    format!("{:?}", e)
                );
                Err(anyhow!(
                    "マイクアクセスが拒否されました。システム設定から許可してください"
                ))
            }
        }
    }
}

/// Linux ALSA adapter
#[cfg(target_os = "linux")]
pub struct AlsaAdapter {
    is_recording: bool,
    device_id: Option<String>,
    stream_thread: Option<JoinHandle<()>>,
    last_callback: Arc<Mutex<Instant>>,
    event_tx: Option<AudioEventSender>,
    watchdog_handle: Option<JoinHandle<()>>,
    polling_handle: Option<JoinHandle<()>>,
    stream_shutdown_tx: Option<mpsc::Sender<()>>,
    watchdog_shutdown_tx: Option<mpsc::Sender<()>>,
    polling_shutdown_tx: Option<mpsc::Sender<()>>,
}

#[cfg(target_os = "linux")]
impl AlsaAdapter {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            device_id: None,
            stream_thread: None,
            last_callback: Arc::new(Mutex::new(Instant::now())),
            event_tx: None,
            watchdog_handle: None,
            polling_handle: None,
            stream_shutdown_tx: None,
            watchdog_shutdown_tx: None,
            polling_shutdown_tx: None,
        }
    }

    pub fn set_event_sender(&mut self, tx: AudioEventSender) {
        self.event_tx = Some(tx);
    }

    fn start_watchdog(&mut self) -> mpsc::Sender<()> {
        let last_cb = Arc::clone(&self.last_callback);
        let event_tx = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let handle = std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));

            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            let elapsed = last_cb.lock().unwrap().elapsed();
            if elapsed > Duration::from_millis(1200) {
                if let Some(tx) = &event_tx {
                    tx.send(AudioDeviceEvent::Stalled {
                        elapsed_ms: elapsed.as_millis() as u64,
                    })
                    .ok();
                }
            }
        });

        self.watchdog_handle = Some(handle);
        shutdown_tx
    }

    fn start_device_polling(&mut self) -> mpsc::Sender<()> {
        let device_id = self.device_id.clone();
        let event_tx = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let handle = std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(3));

            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            if let Some(ref dev_id) = device_id {
                let host = cpal::default_host();
                let device_exists = host
                    .input_devices()
                    .ok()
                    .and_then(|mut devices| {
                        devices.find(|d| d.name().ok().as_ref() == Some(&dev_id))
                    })
                    .is_some();

                if !device_exists {
                    if let Some(tx) = &event_tx {
                        tx.send(AudioDeviceEvent::DeviceGone {
                            device_id: dev_id.clone(),
                        })
                        .ok();
                    }
                    break;
                }
            }
        });

        self.polling_handle = Some(handle);
        shutdown_tx
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
                name: name.clone(),
                sample_rate: default_config.sample_rate().0,
                channels: default_config.channels(),
                is_loopback: is_loopback_device(&name),
            });
        }

        Ok(devices)
    }

    fn start_recording(&mut self, _device_id: &str) -> Result<()> {
        // TODO: Implement ALSA recording (Task 2.3)
        self.is_recording = true;
        Ok(())
    }

    fn start_recording_with_callback(
        &mut self,
        device_id: &str,
        callback: AudioChunkCallback,
    ) -> Result<()> {
        if self.is_recording {
            return Err(anyhow!("Already recording"));
        }

        self.device_id = Some(device_id.to_string());

        let host = cpal::default_host();

        let device = host
            .input_devices()?
            .find(|d| d.name().ok().as_ref() == Some(&device_id.to_string()))
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let config = device.default_input_config()?;

        // Initialize liveness timestamp
        let last_cb = Arc::clone(&self.last_callback);
        *last_cb.lock().unwrap() = Instant::now();

        let event_tx_clone = self.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let stream_thread = std::thread::spawn(move || {
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Update liveness timestamp
                    *last_cb.lock().unwrap() = Instant::now();

                    let pcm_data: Vec<u8> = data
                        .iter()
                        .map(|&sample| {
                            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            scaled.to_le_bytes()
                        })
                        .flatten()
                        .collect();

                    callback(pcm_data);
                },
                move |err| {
                    eprintln!("Audio stream error: {:?}", err);
                    if let Some(tx) = &event_tx_clone {
                        tx.send(AudioDeviceEvent::StreamError(format!("{:?}", err)))
                            .ok();
                    }
                },
            );

            if let Ok(stream) = stream {
                use cpal::traits::StreamTrait;
                if stream.play().is_ok() {
                    shutdown_rx.recv().ok();
                }
            }
        });

        self.stream_thread = Some(stream_thread);
        self.stream_shutdown_tx = Some(shutdown_tx);
        self.watchdog_shutdown_tx = Some(self.start_watchdog());
        self.polling_shutdown_tx = Some(self.start_device_polling());
        self.is_recording = true;
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        // Signal all threads to stop
        if let Some(tx) = self.stream_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.watchdog_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(tx) = self.polling_shutdown_tx.take() {
            tx.send(()).ok();
        }

        // Join all threads
        if let Some(handle) = self.stream_thread.take() {
            handle.join().ok();
        }
        if let Some(handle) = self.watchdog_handle.take() {
            handle.join().ok();
        }
        if let Some(handle) = self.polling_handle.take() {
            handle.join().ok();
        }

        self.is_recording = false;
        self.device_id = None;
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording
    }

    fn check_permission(&self) -> Result<()> {
        // On Linux, microphone access is typically managed by PulseAudio/PipeWire
        // We verify permission by attempting device enumeration
        let host = cpal::default_host();

        match host.input_devices() {
            Ok(mut devices) => {
                if devices.next().is_some() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "マイクアクセスが拒否されました。システム設定から許可してください"
                    ))
                }
            }
            Err(e) => {
                log_error!(
                    "audio_device_adapter",
                    "microphone_permission_denied",
                    format!("{:?}", e)
                );
                Err(anyhow!(
                    "マイクアクセスが拒否されました。システム設定から許可してください"
                ))
            }
        }
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
                devices: vec![AudioDeviceInfo {
                    id: "device-1".to_string(),
                    name: "Test Microphone".to_string(),
                    sample_rate: 16000,
                    channels: 1,
                    is_loopback: false,
                }],
            }
        }

        fn new_with_loopback() -> Self {
            Self {
                is_recording: false,
                devices: vec![
                    AudioDeviceInfo {
                        id: "device-1".to_string(),
                        name: "Test Microphone".to_string(),
                        sample_rate: 16000,
                        channels: 1,
                        is_loopback: false,
                    },
                    AudioDeviceInfo {
                        id: "device-2".to_string(),
                        name: "BlackHole 2ch".to_string(),
                        sample_rate: 48000,
                        channels: 2,
                        is_loopback: true,
                    },
                    AudioDeviceInfo {
                        id: "device-3".to_string(),
                        name: "Monitor of Built-in Audio".to_string(),
                        sample_rate: 44100,
                        channels: 2,
                        is_loopback: true,
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

        fn start_recording_with_callback(
            &mut self,
            _device_id: &str,
            _callback: AudioChunkCallback,
        ) -> Result<()> {
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

        fn check_permission(&self) -> Result<()> {
            // Mock always returns Ok (permission granted)
            Ok(())
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
        assert!(!devices[0].is_loopback);
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
        assert!(
            adapter.is_ok(),
            "Should create audio adapter for current OS"
        );

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

        adapter
            .start_recording_with_callback("device-1", callback)
            .unwrap();
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

    #[test]
    fn test_is_loopback_device_detection() {
        // macOS devices
        assert!(is_loopback_device("BlackHole 2ch"));
        assert!(is_loopback_device("BlackHole 16ch"));
        assert!(is_loopback_device("Loopback Audio"));

        // Linux PulseAudio monitor devices
        assert!(is_loopback_device("Monitor of Built-in Audio"));
        assert!(is_loopback_device(
            "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor"
        ));

        // Windows Stereo Mix
        assert!(is_loopback_device("Stereo Mix"));
        assert!(is_loopback_device("Wave Out Mix"));

        // Regular microphones (should be false)
        assert!(!is_loopback_device("Built-in Microphone"));
        assert!(!is_loopback_device("USB Audio Device"));
        assert!(!is_loopback_device("Blue Yeti"));
    }

    #[test]
    fn test_enumerate_includes_loopback_devices() {
        let adapter = MockAudioAdapter::new_with_loopback();
        let devices = adapter.enumerate_devices().unwrap();

        assert_eq!(devices.len(), 3);

        // Regular microphone
        assert_eq!(devices[0].name, "Test Microphone");
        assert!(!devices[0].is_loopback);

        // BlackHole
        assert_eq!(devices[1].name, "BlackHole 2ch");
        assert!(devices[1].is_loopback);

        // PulseAudio monitor
        assert_eq!(devices[2].name, "Monitor of Built-in Audio");
        assert!(devices[2].is_loopback);
    }

    #[test]
    fn test_filter_loopback_devices() {
        let adapter = MockAudioAdapter::new_with_loopback();
        let devices = adapter.enumerate_devices().unwrap();

        let loopback_devices: Vec<_> = devices.iter().filter(|d| d.is_loopback).collect();

        assert_eq!(loopback_devices.len(), 2);
        assert_eq!(loopback_devices[0].name, "BlackHole 2ch");
        assert_eq!(loopback_devices[1].name, "Monitor of Built-in Audio");
    }
}

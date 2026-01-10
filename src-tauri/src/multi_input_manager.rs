//! MultiInputManager - Parallel capture from multiple audio devices
//!
//! This module manages the lifecycle of multiple audio input streams,
//! enabling simultaneous recording from microphone and loopback devices.
//!
//! ## Design
//!
//! - Uses AdapterFactory to create independent adapter instances per device
//! - Each input has its own buffer for thread-safe audio data collection
//! - Supports partial failure handling (continue with remaining inputs)
//!
//! Requirements: STTMIX-REQ-002.1, STTMIX-REQ-002.3
//! Design: meeting-minutes-stt-multi-input/design.md §4.1

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::audio_device_adapter::{AudioChunkCallback, AudioDeviceAdapter, AudioDeviceEvent};
use crate::audio_device_recorder::AdapterFactory;

// ============================================================================
// Device Error Channel (Finding 1 fix)
// ============================================================================

/// Error event from an input device with device ID
pub type DeviceErrorEvent = (String, AudioDeviceEvent);
/// Sender for device error events
pub type DeviceErrorSender = mpsc::Sender<DeviceErrorEvent>;
/// Receiver for device error events
pub type DeviceErrorReceiver = mpsc::Receiver<DeviceErrorEvent>;

// ============================================================================
// Multi-Input Events (Task 6.1, 6.2)
// ============================================================================

/// Events from MultiInputManager for degradation handling
/// Requirement: STTMIX-REQ-006
#[derive(Debug, Clone)]
pub enum MultiInputEvent {
    /// A single input was lost (device disconnected or error)
    /// Requirement: STTMIX-REQ-006.1
    InputLost {
        device_id: String,
        reason: String,
        remaining_active: usize,
    },
    /// All inputs have been lost - recording cannot continue
    /// Requirement: STTMIX-REQ-006.3
    AllInputsLost {
        reason: String,
    },
}

/// Sender for multi-input events
pub type MultiInputEventSender = mpsc::Sender<MultiInputEvent>;
/// Receiver for multi-input events
pub type MultiInputEventReceiver = mpsc::Receiver<MultiInputEvent>;

// ============================================================================
// Types
// ============================================================================

/// Role of an audio input device
/// Requirement: STTMIX-REQ-001.1 (device role assignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InputRole {
    /// Microphone input (user's voice)
    Microphone,
    /// Loopback/system audio input (meeting participants)
    Loopback,
}

/// Status of a single input for UI display
/// Requirement: STTMIX-REQ-008.1 (observability)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputStatus {
    /// Device identifier
    pub device_id: String,
    /// Role of this input
    pub role: InputRole,
    /// Whether this input is currently active
    pub is_active: bool,
    /// Buffer occupancy as percentage (0.0 - 100.0)
    pub buffer_occupancy_percent: f32,
    /// Buffer level in bytes
    pub buffer_level_bytes: usize,
    /// Maximum buffer size in bytes
    pub buffer_max_bytes: usize,
    /// Gain setting in dB
    pub gain_db: f32,
    /// Whether this input is muted
    pub is_muted: bool,
    /// Frames dropped due to lock contention
    pub lock_contention_drops: u64,
}

/// Configuration for a single input
/// Requirement: STTMIX-REQ-005.1 (per-input gain control)
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// Device identifier
    pub device_id: String,
    /// Role of this input
    pub role: InputRole,
    /// Gain in dB (default: -6.0 for mixing headroom)
    pub gain_db: f32,
    /// Whether this input is muted
    pub muted: bool,
}

impl InputConfig {
    /// Create a new input configuration with default gain
    pub fn new(device_id: impl Into<String>, role: InputRole) -> Self {
        Self {
            device_id: device_id.into(),
            role,
            gain_db: -6.0, // Default -6dB for 2-input mixing
            muted: false,
        }
    }

    /// Create with custom gain
    pub fn with_gain(mut self, gain_db: f32) -> Self {
        self.gain_db = gain_db;
        self
    }

    /// Set muted state
    pub fn with_muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

/// Per-input audio buffer for collecting samples
/// Thread-safe ring buffer for audio data with real-time safety
///
/// # Real-time Safety
/// The `push()` method uses `try_lock()` to avoid blocking in audio callbacks.
/// If the lock cannot be acquired, the frame is dropped to maintain real-time priority.
pub struct InputBuffer {
    /// Raw audio samples (16kHz mono i16 as bytes)
    data: Mutex<Vec<u8>>,
    /// Maximum buffer size in bytes (e.g., 1 second = 32000 bytes at 16kHz mono 16-bit)
    max_size: usize,
    /// Counter for frames dropped due to lock contention (for metrics)
    lock_contention_drops: std::sync::atomic::AtomicU64,
}

impl InputBuffer {
    /// Create a new input buffer with specified max size
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
            lock_contention_drops: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Push audio data into the buffer (real-time safe)
    ///
    /// Uses try_lock() to avoid blocking in audio callbacks.
    /// Returns:
    /// - `Some(dropped_bytes)` if data was pushed (dropped_bytes from overflow)
    /// - `None` if lock couldn't be acquired (frame skipped)
    ///
    /// # Real-time Safety
    /// This method never blocks. If the lock is held by another thread,
    /// the frame is dropped and counted in `lock_contention_drops`.
    pub fn push(&self, audio_data: &[u8]) -> Option<usize> {
        use std::sync::atomic::Ordering;

        // Try to acquire lock without blocking (real-time safe)
        let mut data = match self.data.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Lock contention - skip this frame to avoid blocking audio callback
                self.lock_contention_drops.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        let available = self.max_size.saturating_sub(data.len());

        if audio_data.len() <= available {
            data.extend_from_slice(audio_data);
            Some(0)
        } else {
            // Buffer overflow - drop oldest data to make room
            let overflow = audio_data.len() - available;
            let drain_count = overflow.min(data.len());
            data.drain(0..drain_count);
            data.extend_from_slice(audio_data);
            Some(overflow)
        }
    }

    /// Get the number of frames dropped due to lock contention
    pub fn lock_contention_drops(&self) -> u64 {
        use std::sync::atomic::Ordering;
        self.lock_contention_drops.load(Ordering::Relaxed)
    }

    /// Take up to `max_bytes` from the buffer
    pub fn take(&self, max_bytes: usize) -> Vec<u8> {
        let mut data = self.data.lock().unwrap();
        let take_count = max_bytes.min(data.len());
        data.drain(0..take_count).collect()
    }

    /// Get current buffer level in bytes
    pub fn level(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// Clear the buffer
    pub fn clear(&self) {
        self.data.lock().unwrap().clear();
    }

    /// Get maximum buffer size in bytes
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

/// State of a single active input
struct InputState {
    /// Configuration for this input
    config: InputConfig,
    /// Active adapter instance (None if not recording)
    adapter: Option<Box<dyn AudioDeviceAdapter>>,
    /// Buffer for this input's audio data
    buffer: Arc<InputBuffer>,
    /// Whether this input started successfully
    is_active: bool,
}

// ============================================================================
// MultiInputManager
// ============================================================================

/// Manager for multiple parallel audio inputs
///
/// Requirement: STTMIX-REQ-002 (parallel capture)
/// Design: meeting-minutes-stt-multi-input/design.md §4.1
pub struct MultiInputManager {
    /// Factory for creating adapter instances
    adapter_factory: AdapterFactory,
    /// Active input states, keyed by device_id
    inputs: HashMap<String, InputState>,
    /// Whether recording is active
    is_recording: bool,
    /// Event sender for degradation notifications (Task 6.1, 6.2)
    event_tx: Option<MultiInputEventSender>,
    /// Sender for device errors (shared with adapters)
    device_error_tx: Option<DeviceErrorSender>,
}

impl MultiInputManager {
    /// Create a new MultiInputManager with the given adapter factory
    pub fn new(adapter_factory: AdapterFactory) -> Self {
        Self {
            adapter_factory,
            inputs: HashMap::new(),
            is_recording: false,
            event_tx: None,
            device_error_tx: None,
        }
    }

    /// Create device error channel and return the receiver
    ///
    /// The mixer thread should monitor this receiver to detect device errors
    /// and handle graceful degradation.
    ///
    /// Requirement: STTMIX-REQ-006 (Finding 1 fix)
    pub fn create_device_error_channel(&mut self) -> DeviceErrorReceiver {
        let (tx, rx) = mpsc::channel();
        self.device_error_tx = Some(tx);
        rx
    }

    /// Set the event sender for degradation notifications
    /// Requirement: STTMIX-REQ-006
    pub fn set_event_sender(&mut self, tx: MultiInputEventSender) {
        self.event_tx = Some(tx);
    }

    /// Create event channel and return both sender and receiver
    ///
    /// Returns (sender_clone, receiver) so that:
    /// - The manager keeps one sender for mark_input_lost()
    /// - The mixer thread gets a clone to send events
    /// - External code gets the receiver to monitor events
    ///
    /// Requirement: STTMIX-REQ-006
    pub fn create_event_channel(&mut self) -> (MultiInputEventSender, MultiInputEventReceiver) {
        let (tx, rx) = mpsc::channel();
        self.event_tx = Some(tx.clone());
        (tx, rx)
    }

    /// Get a clone of the event sender (if channel was created)
    ///
    /// Returns None if create_event_channel() hasn't been called yet.
    pub fn get_event_sender(&self) -> Option<MultiInputEventSender> {
        self.event_tx.clone()
    }

    /// Mark an input as lost and notify via event channel
    ///
    /// This method:
    /// 1. Stops the adapter for the lost input (releases resources)
    /// 2. Marks the input as inactive
    /// 3. Sends appropriate event (InputLost or AllInputsLost)
    ///
    /// Requirement: STTMIX-REQ-006.1, STTMIX-REQ-006.3
    pub fn mark_input_lost(&mut self, device_id: &str, reason: &str) {
        if let Some(state) = self.inputs.get_mut(device_id) {
            if state.is_active {
                // Stop the adapter to release resources (Finding 3 fix)
                if let Some(ref mut adapter) = state.adapter {
                    if let Err(e) = adapter.stop_recording() {
                        eprintln!("⚠️ Failed to stop adapter for '{}': {}", device_id, e);
                    }
                }
                state.adapter = None; // Release adapter
                state.is_active = false;

                eprintln!("⚠️ Input lost: {} - {}", device_id, reason);

                let remaining = self.active_input_count();

                if let Some(tx) = &self.event_tx {
                    if remaining == 0 {
                        // All inputs lost (Task 6.2)
                        tx.send(MultiInputEvent::AllInputsLost {
                            reason: format!("Last input '{}' lost: {}", device_id, reason),
                        })
                        .ok();
                    } else {
                        // Single input lost, continue with remaining (Task 6.1)
                        tx.send(MultiInputEvent::InputLost {
                            device_id: device_id.to_string(),
                            reason: reason.to_string(),
                            remaining_active: remaining,
                        })
                        .ok();
                    }
                }
            }
        }
    }

    /// Check if any inputs are still active
    pub fn has_active_inputs(&self) -> bool {
        self.active_input_count() > 0
    }

    /// Start recording from multiple inputs
    ///
    /// # Arguments
    /// * `configs` - Configuration for each input device
    /// * `continue_on_partial_failure` - If true, continue with remaining inputs if some fail
    ///
    /// # Returns
    /// * `Ok(started_count)` - Number of inputs successfully started
    /// * `Err` - If all inputs fail or critical error occurs
    ///
    /// Requirement: STTMIX-REQ-002.1, STTMIX-REQ-002.2
    pub fn start(&mut self, configs: Vec<InputConfig>, continue_on_partial_failure: bool) -> Result<usize> {
        if self.is_recording {
            anyhow::bail!("Already recording");
        }

        if configs.is_empty() {
            anyhow::bail!("At least one input configuration required");
        }

        // Validate max 2 inputs (STTMIX-CON-005)
        if configs.len() > 2 {
            anyhow::bail!("Maximum 2 inputs supported, got {}", configs.len());
        }

        // Validate no duplicate device_ids (prevents HashMap overwrite and resource leaks)
        {
            let mut seen_ids = std::collections::HashSet::new();
            for config in &configs {
                if !seen_ids.insert(&config.device_id) {
                    anyhow::bail!(
                        "Duplicate device_id detected: '{}'. Each input must use a unique device.",
                        config.device_id
                    );
                }
            }
        }

        let mut started_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for config in configs {
            let device_id = config.device_id.clone();

            // Create buffer for this input (1 second at 16kHz mono 16-bit = 32000 bytes)
            let buffer = Arc::new(InputBuffer::new(32000));
            let buffer_clone = Arc::clone(&buffer);

            // Create adapter instance via factory
            let adapter_result = (self.adapter_factory)();

            match adapter_result {
                Ok(mut adapter) => {
                    // Set up device error reporting (Finding 1 fix)
                    // Create a wrapper sender that includes the device_id
                    if let Some(ref error_tx) = self.device_error_tx {
                        let device_id_for_error = device_id.clone();
                        let error_tx_clone = error_tx.clone();
                        // Create a sender that wraps events with device_id
                        let (device_event_tx, device_event_rx) = mpsc::channel();

                        // Spawn a thread to forward events with device_id
                        std::thread::spawn(move || {
                            while let Ok(event) = device_event_rx.recv() {
                                if error_tx_clone
                                    .send((device_id_for_error.clone(), event))
                                    .is_err()
                                {
                                    break; // Main channel closed
                                }
                            }
                        });

                        adapter.set_event_sender(device_event_tx);
                    }

                    // Create callback that writes to this input's buffer (real-time safe)
                    let callback: AudioChunkCallback = Box::new(move |data: Vec<u8>| {
                        // push() uses try_lock() - if lock contention, frame is dropped
                        // This is intentional for real-time safety
                        let _ = buffer_clone.push(&data);
                    });

                    // Start recording on this adapter
                    match adapter.start_recording_with_callback(&device_id, callback) {
                        Ok(()) => {
                            self.inputs.insert(
                                device_id.clone(),
                                InputState {
                                    config,
                                    adapter: Some(adapter),
                                    buffer,
                                    is_active: true,
                                },
                            );
                            started_count += 1;
                        }
                        Err(e) => {
                            errors.push(format!("{}: {}", device_id, e));
                            // Store as inactive input for tracking
                            self.inputs.insert(
                                device_id.clone(),
                                InputState {
                                    config,
                                    adapter: None,
                                    buffer,
                                    is_active: false,
                                },
                            );
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("{}: adapter creation failed: {}", device_id, e));
                    self.inputs.insert(
                        device_id.clone(),
                        InputState {
                            config,
                            adapter: None,
                            buffer,
                            is_active: false,
                        },
                    );
                }
            }
        }

        // Check if we should continue or fail
        if started_count == 0 {
            self.inputs.clear();
            anyhow::bail!(
                "All inputs failed to start: {}",
                errors.join("; ")
            );
        }

        if !continue_on_partial_failure && !errors.is_empty() {
            // Stop any started inputs and fail
            self.stop()?;
            anyhow::bail!(
                "Some inputs failed and continue_on_partial_failure=false: {}",
                errors.join("; ")
            );
        }

        self.is_recording = true;
        Ok(started_count)
    }

    /// Stop all recording inputs
    ///
    /// Requirement: STTMIX-REQ-002.3
    pub fn stop(&mut self) -> Result<()> {
        let mut errors: Vec<String> = Vec::new();

        for (device_id, state) in self.inputs.iter_mut() {
            if let Some(ref mut adapter) = state.adapter {
                if let Err(e) = adapter.stop_recording() {
                    errors.push(format!("{}: {}", device_id, e));
                }
            }
            state.adapter = None;
            state.is_active = false;
        }

        self.inputs.clear();
        self.is_recording = false;

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!("Some inputs failed to stop: {}", errors.join("; ")))
        }
    }

    /// Check if recording is active
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get the number of active inputs
    pub fn active_input_count(&self) -> usize {
        self.inputs.values().filter(|s| s.is_active).count()
    }

    /// Get buffer for a specific input
    pub fn get_buffer(&self, device_id: &str) -> Option<Arc<InputBuffer>> {
        self.inputs.get(device_id).map(|s| Arc::clone(&s.buffer))
    }

    /// Get all active input buffers with their configs
    pub fn get_active_buffers(&self) -> Vec<(InputConfig, Arc<InputBuffer>)> {
        self.inputs
            .values()
            .filter(|s| s.is_active)
            .map(|s| (s.config.clone(), Arc::clone(&s.buffer)))
            .collect()
    }

    /// Get configuration for a specific input
    pub fn get_config(&self, device_id: &str) -> Option<&InputConfig> {
        self.inputs.get(device_id).map(|s| &s.config)
    }

    /// Get status of all inputs for UI display
    /// Requirement: STTMIX-REQ-008.1 (observability)
    pub fn get_all_input_status(&self) -> Vec<InputStatus> {
        self.inputs
            .iter()
            .map(|(device_id, state)| {
                let buffer_level = state.buffer.level();
                let buffer_max = state.buffer.max_size();
                let occupancy = if buffer_max > 0 {
                    (buffer_level as f32 / buffer_max as f32) * 100.0
                } else {
                    0.0
                };
                InputStatus {
                    device_id: device_id.clone(),
                    role: state.config.role,
                    is_active: state.is_active,
                    buffer_occupancy_percent: occupancy,
                    buffer_level_bytes: buffer_level,
                    buffer_max_bytes: buffer_max,
                    gain_db: state.config.gain_db,
                    is_muted: state.config.muted,
                    lock_contention_drops: state.buffer.lock_contention_drops(),
                }
            })
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_device_adapter::AudioDeviceInfo;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// Mock adapter for testing
    struct MockAudioAdapter {
        device_id: Option<String>,
        is_recording: AtomicBool,
        should_fail_start: bool,
    }

    impl MockAudioAdapter {
        fn new() -> Self {
            Self {
                device_id: None,
                is_recording: AtomicBool::new(false),
                should_fail_start: false,
            }
        }

        fn failing() -> Self {
            Self {
                device_id: None,
                is_recording: AtomicBool::new(false),
                should_fail_start: true,
            }
        }
    }

    impl AudioDeviceAdapter for MockAudioAdapter {
        fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
            Ok(vec![])
        }

        fn start_recording(&mut self, device_id: &str) -> Result<()> {
            if self.should_fail_start {
                anyhow::bail!("Mock start failure");
            }
            self.device_id = Some(device_id.to_string());
            self.is_recording.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn start_recording_with_callback(
            &mut self,
            device_id: &str,
            _callback: AudioChunkCallback,
        ) -> Result<()> {
            if self.should_fail_start {
                anyhow::bail!("Mock start failure");
            }
            self.device_id = Some(device_id.to_string());
            self.is_recording.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn stop_recording(&mut self) -> Result<()> {
            self.is_recording.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn is_recording(&self) -> bool {
            self.is_recording.load(Ordering::SeqCst)
        }

        fn check_permission(&self) -> Result<()> {
            Ok(())
        }

        fn set_event_sender(&mut self, _tx: crate::audio_device_adapter::AudioEventSender) {
            // Mock: no-op
        }
    }

    /// Create a mock factory that produces working adapters
    fn mock_factory() -> AdapterFactory {
        Arc::new(|| Ok(Box::new(MockAudioAdapter::new()) as Box<dyn AudioDeviceAdapter>))
    }

    /// Create a mock factory that produces failing adapters
    fn failing_factory() -> AdapterFactory {
        Arc::new(|| Ok(Box::new(MockAudioAdapter::failing()) as Box<dyn AudioDeviceAdapter>))
    }

    /// Create a factory that fails after N calls
    fn partial_failing_factory(fail_after: usize) -> AdapterFactory {
        let counter = Arc::new(AtomicUsize::new(0));
        Arc::new(move || {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count >= fail_after {
                Ok(Box::new(MockAudioAdapter::failing()) as Box<dyn AudioDeviceAdapter>)
            } else {
                Ok(Box::new(MockAudioAdapter::new()) as Box<dyn AudioDeviceAdapter>)
            }
        })
    }

    // ========================================================================
    // Test: InputRole
    // ========================================================================

    #[test]
    fn test_input_role() {
        assert_eq!(InputRole::Microphone, InputRole::Microphone);
        assert_ne!(InputRole::Microphone, InputRole::Loopback);
    }

    // ========================================================================
    // Test: InputConfig
    // ========================================================================

    #[test]
    fn test_input_config_new() {
        let config = InputConfig::new("mic-1", InputRole::Microphone);
        assert_eq!(config.device_id, "mic-1");
        assert_eq!(config.role, InputRole::Microphone);
        assert_eq!(config.gain_db, -6.0);
        assert!(!config.muted);
    }

    #[test]
    fn test_input_config_with_gain() {
        let config = InputConfig::new("mic-1", InputRole::Microphone).with_gain(-3.0);
        assert_eq!(config.gain_db, -3.0);
    }

    #[test]
    fn test_input_config_with_muted() {
        let config = InputConfig::new("mic-1", InputRole::Microphone).with_muted(true);
        assert!(config.muted);
    }

    // ========================================================================
    // Test: InputBuffer
    // ========================================================================

    #[test]
    fn test_input_buffer_push_and_take() {
        let buffer = InputBuffer::new(1000);

        // Push some data - returns Some(dropped_bytes)
        let data = vec![1u8, 2, 3, 4, 5];
        let result = buffer.push(&data);
        assert_eq!(result, Some(0)); // No overflow, lock acquired
        assert_eq!(buffer.level(), 5);

        // Take data
        let taken = buffer.take(3);
        assert_eq!(taken, vec![1, 2, 3]);
        assert_eq!(buffer.level(), 2);

        // Take remaining
        let taken = buffer.take(10);
        assert_eq!(taken, vec![4, 5]);
        assert_eq!(buffer.level(), 0);
    }

    #[test]
    fn test_input_buffer_overflow() {
        let buffer = InputBuffer::new(10);

        // Fill buffer
        let result = buffer.push(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(result, Some(0));
        assert_eq!(buffer.level(), 10);

        // Overflow - oldest data should be dropped
        let result = buffer.push(&[11, 12, 13]);
        assert_eq!(result, Some(3)); // 3 bytes dropped due to overflow
        assert_eq!(buffer.level(), 10);

        // Verify oldest data was dropped
        let data = buffer.take(10);
        assert_eq!(data, vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
    }

    #[test]
    fn test_input_buffer_clear() {
        let buffer = InputBuffer::new(100);
        let _ = buffer.push(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer.level(), 5);

        buffer.clear();
        assert_eq!(buffer.level(), 0);
    }

    #[test]
    fn test_input_buffer_lock_contention_counter() {
        let buffer = InputBuffer::new(100);
        // Initially no drops
        assert_eq!(buffer.lock_contention_drops(), 0);

        // Normal push should not increment counter
        let _ = buffer.push(&[1, 2, 3]);
        assert_eq!(buffer.lock_contention_drops(), 0);
    }

    // ========================================================================
    // Test: MultiInputManager creation
    // ========================================================================

    #[test]
    fn test_manager_new() {
        let manager = MultiInputManager::new(mock_factory());
        assert!(!manager.is_recording());
        assert_eq!(manager.active_input_count(), 0);
    }

    // ========================================================================
    // Test: Start with single input
    // ========================================================================

    #[test]
    fn test_manager_start_single_input() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![InputConfig::new("mic-1", InputRole::Microphone)];

        let result = manager.start(configs, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert!(manager.is_recording());
        assert_eq!(manager.active_input_count(), 1);
    }

    // ========================================================================
    // Test: Start with two inputs
    // ========================================================================

    #[test]
    fn test_manager_start_two_inputs() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        let result = manager.start(configs, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert!(manager.is_recording());
        assert_eq!(manager.active_input_count(), 2);
    }

    // ========================================================================
    // Test: Max 2 inputs constraint
    // ========================================================================

    #[test]
    fn test_manager_max_2_inputs() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
            InputConfig::new("mic-2", InputRole::Microphone),
        ];

        let result = manager.start(configs, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum 2 inputs"));
    }

    // ========================================================================
    // Test: Empty inputs error
    // ========================================================================

    #[test]
    fn test_manager_empty_inputs() {
        let mut manager = MultiInputManager::new(mock_factory());

        let result = manager.start(vec![], false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one input"));
    }

    // ========================================================================
    // Test: Already recording error
    // ========================================================================

    #[test]
    fn test_manager_already_recording() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![InputConfig::new("mic-1", InputRole::Microphone)];

        manager.start(configs.clone(), false).unwrap();

        let result = manager.start(configs, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Already recording"));
    }

    // ========================================================================
    // Test: Stop recording
    // ========================================================================

    #[test]
    fn test_manager_stop() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        manager.start(configs, false).unwrap();
        assert!(manager.is_recording());

        let result = manager.stop();
        assert!(result.is_ok());
        assert!(!manager.is_recording());
        assert_eq!(manager.active_input_count(), 0);
    }

    // ========================================================================
    // Test: Partial failure with continue_on_partial_failure=true
    // ========================================================================

    #[test]
    fn test_manager_partial_failure_continue() {
        // First adapter works, second fails
        let mut manager = MultiInputManager::new(partial_failing_factory(1));

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        let result = manager.start(configs, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Only 1 succeeded
        assert!(manager.is_recording());
        assert_eq!(manager.active_input_count(), 1);
    }

    // ========================================================================
    // Test: Partial failure with continue_on_partial_failure=false
    // ========================================================================

    #[test]
    fn test_manager_partial_failure_stop() {
        let mut manager = MultiInputManager::new(partial_failing_factory(1));

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        let result = manager.start(configs, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("continue_on_partial_failure=false"));
        assert!(!manager.is_recording());
    }

    // ========================================================================
    // Test: All inputs fail
    // ========================================================================

    #[test]
    fn test_manager_all_inputs_fail() {
        let mut manager = MultiInputManager::new(failing_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        let result = manager.start(configs, true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("All inputs failed"));
    }

    // ========================================================================
    // Test: Get buffer
    // ========================================================================

    #[test]
    fn test_manager_get_buffer() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![InputConfig::new("mic-1", InputRole::Microphone)];

        manager.start(configs, false).unwrap();

        let buffer = manager.get_buffer("mic-1");
        assert!(buffer.is_some());

        let buffer = manager.get_buffer("nonexistent");
        assert!(buffer.is_none());
    }

    // ========================================================================
    // Test: Get active buffers
    // ========================================================================

    #[test]
    fn test_manager_get_active_buffers() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];

        manager.start(configs, false).unwrap();

        let buffers = manager.get_active_buffers();
        assert_eq!(buffers.len(), 2);
    }

    // ========================================================================
    // Test: Get config
    // ========================================================================

    #[test]
    fn test_manager_get_config() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone).with_gain(-3.0),
        ];

        manager.start(configs, false).unwrap();

        let config = manager.get_config("mic-1");
        assert!(config.is_some());
        assert_eq!(config.unwrap().gain_db, -3.0);
    }

    #[test]
    fn test_manager_get_all_input_status() {
        let mut manager = MultiInputManager::new(mock_factory());

        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone).with_gain(-3.0),
            InputConfig::new("loopback-1", InputRole::Loopback).with_gain(-6.0).with_muted(true),
        ];

        manager.start(configs, false).unwrap();

        // Push some data to one buffer
        if let Some(buffer) = manager.get_buffer("mic-1") {
            buffer.push(&[1, 2, 3, 4]); // 4 bytes
        }

        let statuses = manager.get_all_input_status();
        assert_eq!(statuses.len(), 2);

        // Find mic-1 status
        let mic_status = statuses.iter().find(|s| s.device_id == "mic-1").unwrap();
        assert_eq!(mic_status.role, InputRole::Microphone);
        assert!(mic_status.is_active);
        assert_eq!(mic_status.gain_db, -3.0);
        assert!(!mic_status.is_muted);
        assert_eq!(mic_status.buffer_level_bytes, 4);
        assert!(mic_status.buffer_occupancy_percent > 0.0);

        // Find loopback-1 status
        let loop_status = statuses.iter().find(|s| s.device_id == "loopback-1").unwrap();
        assert_eq!(loop_status.role, InputRole::Loopback);
        assert!(loop_status.is_active);
        assert_eq!(loop_status.gain_db, -6.0);
        assert!(loop_status.is_muted);
        assert_eq!(loop_status.buffer_level_bytes, 0);
    }

    // ========================================================================
    // Test: Duplicate device_id detection
    // ========================================================================

    #[test]
    fn test_manager_duplicate_device_id() {
        let mut manager = MultiInputManager::new(mock_factory());

        // Same device_id used twice - should fail
        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("mic-1", InputRole::Loopback), // Duplicate!
        ];

        let result = manager.start(configs, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Duplicate device_id"));
        assert!(err_msg.contains("mic-1"));
    }

    #[test]
    fn test_manager_different_device_ids_ok() {
        let mut manager = MultiInputManager::new(mock_factory());

        // Different device_ids - should work
        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("mic-2", InputRole::Loopback),
        ];

        let result = manager.start(configs, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    // ========================================================================
    // Test: Event channel and mark_input_lost (Task 6.1, 6.2)
    // ========================================================================

    #[test]
    fn test_event_channel_creation() {
        let mut manager = MultiInputManager::new(mock_factory());

        let (_tx, rx) = manager.create_event_channel();

        // Channel should be created (no events yet)
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_mark_input_lost_sends_input_lost_event() {
        let mut manager = MultiInputManager::new(mock_factory());
        let (_tx, rx) = manager.create_event_channel();

        // Start with 2 inputs
        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];
        manager.start(configs, false).unwrap();
        assert_eq!(manager.active_input_count(), 2);

        // Mark one input as lost
        manager.mark_input_lost("mic-1", "Device disconnected");

        // Should receive InputLost event
        let event = rx.recv().unwrap();
        match event {
            MultiInputEvent::InputLost { device_id, reason, remaining_active } => {
                assert_eq!(device_id, "mic-1");
                assert!(reason.contains("Device disconnected"));
                assert_eq!(remaining_active, 1);
            }
            _ => panic!("Expected InputLost event"),
        }

        // Active count should be reduced
        assert_eq!(manager.active_input_count(), 1);
        assert!(manager.has_active_inputs());
    }

    #[test]
    fn test_mark_input_lost_sends_all_inputs_lost_event() {
        let mut manager = MultiInputManager::new(mock_factory());
        let (_tx, rx) = manager.create_event_channel();

        // Start with 1 input
        let configs = vec![InputConfig::new("mic-1", InputRole::Microphone)];
        manager.start(configs, false).unwrap();
        assert_eq!(manager.active_input_count(), 1);

        // Mark the only input as lost
        manager.mark_input_lost("mic-1", "Device disconnected");

        // Should receive AllInputsLost event
        let event = rx.recv().unwrap();
        match event {
            MultiInputEvent::AllInputsLost { reason } => {
                assert!(reason.contains("mic-1"));
                assert!(reason.contains("Device disconnected"));
            }
            _ => panic!("Expected AllInputsLost event"),
        }

        // No active inputs remaining
        assert_eq!(manager.active_input_count(), 0);
        assert!(!manager.has_active_inputs());
    }

    #[test]
    fn test_mark_input_lost_idempotent() {
        let mut manager = MultiInputManager::new(mock_factory());
        let (_tx, rx) = manager.create_event_channel();

        // Start with 2 inputs
        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];
        manager.start(configs, false).unwrap();

        // Mark same input as lost twice
        manager.mark_input_lost("mic-1", "First call");
        manager.mark_input_lost("mic-1", "Second call");

        // Should only receive one event (second call is no-op)
        let event1 = rx.recv().unwrap();
        assert!(matches!(event1, MultiInputEvent::InputLost { .. }));

        // No second event
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_mark_input_lost_nonexistent_device() {
        let mut manager = MultiInputManager::new(mock_factory());
        let (_tx, rx) = manager.create_event_channel();

        // Start with 1 input
        let configs = vec![InputConfig::new("mic-1", InputRole::Microphone)];
        manager.start(configs, false).unwrap();

        // Mark nonexistent device as lost
        manager.mark_input_lost("nonexistent", "Error");

        // Should not receive any event
        assert!(rx.try_recv().is_err());

        // Active count unchanged
        assert_eq!(manager.active_input_count(), 1);
    }

    #[test]
    fn test_has_active_inputs() {
        let mut manager = MultiInputManager::new(mock_factory());

        // Initially no inputs
        assert!(!manager.has_active_inputs());

        // Start with 2 inputs
        let configs = vec![
            InputConfig::new("mic-1", InputRole::Microphone),
            InputConfig::new("loopback-1", InputRole::Loopback),
        ];
        manager.start(configs, false).unwrap();
        assert!(manager.has_active_inputs());

        // Mark one as lost
        manager.mark_input_lost("mic-1", "Error");
        assert!(manager.has_active_inputs()); // Still has one

        // Mark second as lost
        manager.mark_input_lost("loopback-1", "Error");
        assert!(!manager.has_active_inputs()); // None left
    }
}

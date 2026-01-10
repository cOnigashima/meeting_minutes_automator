//! AudioDeviceRecorder - Facade for single/multi-input recording
//!
//! This module provides a unified interface for managing audio recording,
//! supporting both single-device and multi-device (mixer) modes.
//!
//! ## Design Notes
//!
//! The facade uses a **factory pattern** to support multiple adapter instances:
//! - Single mode: creates one adapter via factory
//! - Multi mode: MultiInputManager (Task 2.x) creates multiple adapters via factory
//!
//! This design avoids the structural limitation of holding a single adapter instance,
//! which cannot support parallel capture from multiple devices.
//!
//! Requirement: STTMIX-CON-001 (backward compatibility)
//! Design: meeting-minutes-stt-multi-input/design.md §4.0

use anyhow::Result;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::audio_device_adapter::{AudioChunkCallback, AudioDeviceAdapter, AudioDeviceInfo};
use crate::input_mixer::{InputMixer, MixerMetrics, FRAME_DURATION_MS};
use crate::multi_input_manager::{
    DeviceErrorReceiver, InputConfig, InputRole, InputStatus, MultiInputEvent,
    MultiInputEventReceiver, MultiInputManager,
};

// ============================================================================
// Adapter Factory
// ============================================================================

/// Factory function type for creating AudioDeviceAdapter instances
///
/// This enables the facade to create multiple adapter instances for multi-input mode.
/// Each call to the factory should return a fresh, independent adapter.
pub type AdapterFactory = Arc<dyn Fn() -> Result<Box<dyn AudioDeviceAdapter>> + Send + Sync>;

// ============================================================================
// Types
// ============================================================================

/// Recording mode configuration
/// Requirement: STTMIX-REQ-002 (parallel capture support)
#[derive(Debug, Clone)]
pub enum RecordingMode {
    /// Single device recording (existing behavior)
    Single { device_id: String },
    /// Multi-device recording with mixing
    /// Requirement: STTMIX-CON-005 (max 2 inputs)
    Multi {
        device_ids: Vec<String>,
        mixer_config: MixerConfig,
    },
}

/// Configuration for the input mixer
/// Requirement: STTMIX-REQ-005 (gain control)
#[derive(Debug, Clone)]
pub struct MixerConfig {
    /// Gain per input in dB (default: -6.0 for 2 inputs)
    pub gains: Vec<f32>,
    /// Continue recording if one input fails
    /// Requirement: STTMIX-REQ-006.2
    pub continue_on_partial_failure: bool,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            // Default -6dB for 2 inputs to prevent clipping
            // Requirement: STTMIX-REQ-005.2
            gains: vec![-6.0, -6.0],
            continue_on_partial_failure: true,
        }
    }
}

// ============================================================================
// AudioDeviceRecorder
// ============================================================================

/// Facade for unified single/multi-input recording
///
/// Design: meeting-minutes-stt-multi-input/design.md §4.0
/// - Uses factory pattern to create adapter instances on demand
/// - Single mode: creates one adapter, delegates directly
/// - Multi mode: MultiInputManager (Task 2.x) creates multiple adapters via factory
///
/// ## Why Factory Pattern?
///
/// The existing AudioDeviceAdapter is designed for single-stream operation:
/// - Holds single device_id, stream_thread, shutdown_tx
/// - Cannot run multiple parallel streams from one instance
///
/// The factory pattern allows creating independent adapter instances for each device,
/// enabling true parallel capture in multi-input mode.
pub struct AudioDeviceRecorder {
    /// Factory for creating adapter instances
    adapter_factory: AdapterFactory,
    /// Active adapter for single mode (None when not recording or in multi mode)
    single_adapter: Option<Box<dyn AudioDeviceAdapter>>,
    /// MultiInputManager for multi-device recording (None when not in multi mode)
    multi_input_manager: Option<MultiInputManager>,
    /// Mixer thread handle (for multi mode)
    mixer_thread: Option<JoinHandle<()>>,
    /// Shutdown channel for mixer thread
    mixer_shutdown_tx: Option<mpsc::Sender<()>>,
    /// Mixer metrics (for observability)
    mixer_metrics: Option<Arc<MixerMetrics>>,
    /// Current recording mode
    mode: Option<RecordingMode>,
    /// Recording state flag
    is_recording: bool,
    /// Event receiver for external monitoring (UI notification, stop trigger)
    /// When AllInputsLost is received, caller should call stop()
    event_rx: Option<MultiInputEventReceiver>,
}

/// Callback type for session error events
/// Called when device errors occur or all inputs are lost
pub type SessionErrorCallback = Arc<dyn Fn(MultiInputEvent) + Send + Sync>;

impl AudioDeviceRecorder {
    /// Create a new AudioDeviceRecorder with an adapter factory
    ///
    /// # Arguments
    /// * `adapter_factory` - Factory function that creates fresh adapter instances
    ///
    /// # Example
    /// ```ignore
    /// let recorder = AudioDeviceRecorder::new(Arc::new(|| {
    ///     Ok(Box::new(RealAudioDeviceAdapter::new()) as Box<dyn AudioDeviceAdapter>)
    /// }));
    /// ```
    pub fn new(adapter_factory: AdapterFactory) -> Self {
        Self {
            adapter_factory,
            single_adapter: None,
            multi_input_manager: None,
            mixer_thread: None,
            mixer_shutdown_tx: None,
            mixer_metrics: None,
            mode: None,
            is_recording: false,
            event_rx: None,
        }
    }

    /// Create a new AudioDeviceRecorder with a pre-existing adapter (legacy compatibility)
    ///
    /// This is provided for backward compatibility with existing code that passes
    /// a single adapter instance. Internally creates a factory that returns the adapter.
    ///
    /// **Note**: This constructor only supports single-mode recording.
    /// For multi-input support, use `new()` with a proper factory.
    #[deprecated(note = "Use new() with AdapterFactory for multi-input support")]
    pub fn new_with_adapter(adapter: Box<dyn AudioDeviceAdapter>) -> Self {
        // Wrap the single adapter in an Arc<Mutex> for the factory
        use std::sync::Mutex;
        let adapter_cell = Arc::new(Mutex::new(Some(adapter)));

        let factory: AdapterFactory = Arc::new(move || {
            adapter_cell
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| anyhow::anyhow!("Adapter already consumed (legacy mode only supports single use)"))
        });

        Self {
            adapter_factory: factory,
            single_adapter: None,
            multi_input_manager: None,
            mixer_thread: None,
            mixer_shutdown_tx: None,
            mixer_metrics: None,
            mode: None,
            is_recording: false,
            event_rx: None,
        }
    }

    /// Enumerate available audio devices
    ///
    /// Creates a temporary adapter to enumerate devices.
    pub fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let adapter = (self.adapter_factory)()?;
        adapter.enumerate_devices()
    }

    /// Check microphone permission
    ///
    /// Creates a temporary adapter to check permission.
    pub fn check_permission(&self) -> Result<()> {
        let adapter = (self.adapter_factory)()?;
        adapter.check_permission()
    }

    /// Get the adapter factory for use by MultiInputManager (Task 2.x)
    ///
    /// This allows MultiInputManager to create multiple adapter instances
    /// for parallel capture.
    pub fn adapter_factory(&self) -> &AdapterFactory {
        &self.adapter_factory
    }

    /// Start recording in the configured mode
    ///
    /// # Arguments
    /// * `mode` - Recording mode (Single or Multi)
    /// * `callback` - Callback to receive audio chunks (16kHz mono PCM)
    ///   - Single mode: callback receives data directly from the device
    ///   - Multi mode: callback will receive mixed data (requires Task 4.x mixer)
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if recording fails to start
    ///
    /// # Multi-Input Note
    /// In Multi mode, per-input buffers collect raw audio data.
    /// Use `get_multi_input_manager()` to access buffers for mixing.
    /// The mixer (Task 4.x) will combine inputs and call the callback.
    pub fn start(&mut self, mode: RecordingMode, callback: AudioChunkCallback) -> Result<()> {
        if self.is_recording {
            anyhow::bail!("Already recording");
        }

        match &mode {
            RecordingMode::Single { device_id } => {
                // Create adapter instance for single mode
                let mut adapter = (self.adapter_factory)()?;
                adapter.start_recording_with_callback(device_id, callback)?;
                self.single_adapter = Some(adapter);
            }
            RecordingMode::Multi { device_ids, mixer_config } => {
                // Validate max 2 inputs (STTMIX-CON-005)
                if device_ids.len() > 2 {
                    anyhow::bail!(
                        "Maximum 2 inputs supported, got {}",
                        device_ids.len()
                    );
                }
                if device_ids.is_empty() {
                    anyhow::bail!("At least one device must be specified");
                }

                // Create MultiInputManager with the adapter factory
                let mut manager = MultiInputManager::new(Arc::clone(&self.adapter_factory));

                // Build input configs with gains from mixer_config
                let configs: Vec<InputConfig> = device_ids
                    .iter()
                    .enumerate()
                    .map(|(i, device_id)| {
                        // Assign roles: first device = Microphone, second = Loopback
                        let role = if i == 0 {
                            InputRole::Microphone
                        } else {
                            InputRole::Loopback
                        };
                        let gain = mixer_config.gains.get(i).copied().unwrap_or(-6.0);
                        InputConfig::new(device_id.clone(), role).with_gain(gain)
                    })
                    .collect();

                // Start parallel capture
                let started = manager.start(configs.clone(), mixer_config.continue_on_partial_failure)?;

                if started < device_ids.len() {
                    // Log warning about partial start
                    eprintln!(
                        "⚠️ MultiInputManager: {} of {} inputs started",
                        started,
                        device_ids.len()
                    );
                }

                // Create device error channel for detecting device loss
                let device_error_rx = manager.create_device_error_channel();

                // Get active buffers for mixer thread
                let active_buffers = manager.get_active_buffers();

                // Create event channel - returns (tx, rx) without overwriting
                // - tx is used by mixer thread to send events
                // - rx is stored in self for external monitoring (Fix 1)
                let (event_tx, event_rx_for_external) = manager.create_event_channel();

                self.multi_input_manager = Some(manager);
                self.event_rx = Some(event_rx_for_external);

                // Task 4: Start mixer thread
                // The mixer reads from per-input buffers and calls the callback with mixed output
                let (shutdown_tx, shutdown_rx) = mpsc::channel();

                // Create mixer and get metrics handle
                let mixer = InputMixer::new();
                let mixer_metrics = mixer.metrics();

                // Clone buffers for the thread
                let buffers_for_thread = active_buffers;

                let mixer_thread = std::thread::spawn(move || {
                    Self::mixer_thread_loop(
                        mixer,
                        buffers_for_thread,
                        callback,
                        shutdown_rx,
                        device_error_rx,
                        event_tx,
                    );
                });

                self.mixer_thread = Some(mixer_thread);
                self.mixer_shutdown_tx = Some(shutdown_tx);
                self.mixer_metrics = Some(mixer_metrics);
            }
        }

        self.mode = Some(mode);
        self.is_recording = true;
        Ok(())
    }

    /// Stop recording
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        // Stop single adapter if active
        if let Some(ref mut adapter) = self.single_adapter {
            adapter.stop_recording()?;
        }
        self.single_adapter = None;

        // Stop mixer thread first (before stopping inputs)
        if let Some(tx) = self.mixer_shutdown_tx.take() {
            tx.send(()).ok();
        }
        if let Some(handle) = self.mixer_thread.take() {
            handle.join().ok();
        }
        self.mixer_metrics = None;

        // Stop MultiInputManager if active
        if let Some(ref mut manager) = self.multi_input_manager {
            manager.stop()?;
        }
        self.multi_input_manager = None;

        self.is_recording = false;
        self.mode = None;
        Ok(())
    }

    /// Get mixer metrics (for observability)
    ///
    /// Returns None if not in Multi mode or not recording.
    pub fn get_mixer_metrics(&self) -> Option<Arc<MixerMetrics>> {
        self.mixer_metrics.clone()
    }

    /// Get reference to MultiInputManager (for mixer access to per-input buffers)
    ///
    /// Returns None if not in Multi mode or not recording.
    pub fn get_multi_input_manager(&self) -> Option<&MultiInputManager> {
        self.multi_input_manager.as_ref()
    }

    /// Get mutable reference to MultiInputManager
    pub fn get_multi_input_manager_mut(&mut self) -> Option<&mut MultiInputManager> {
        self.multi_input_manager.as_mut()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get status of all inputs for UI display
    ///
    /// Returns empty vector if not in multi-input mode or not recording.
    /// Requirement: STTMIX-REQ-008.1 (observability), Task 8.3
    pub fn get_input_status(&self) -> Vec<InputStatus> {
        match &self.multi_input_manager {
            Some(manager) => manager.get_all_input_status(),
            None => Vec::new(),
        }
    }

    /// Get the current recording mode
    pub fn current_mode(&self) -> Option<&RecordingMode> {
        self.mode.as_ref()
    }

    /// Take the event receiver for external monitoring
    ///
    /// Returns the receiver that gets MultiInputEvent::InputLost and AllInputsLost.
    /// **Important**: When AllInputsLost is received, the caller MUST call stop().
    ///
    /// Returns None if not in multi-mode or already taken.
    ///
    /// Requirement: STTMIX-REQ-006
    pub fn take_event_receiver(&mut self) -> Option<MultiInputEventReceiver> {
        self.event_rx.take()
    }

    /// Mixer thread main loop
    ///
    /// Handles:
    /// - Mixing frames at 10ms cadence
    /// - Shutdown signal handling
    /// - Device error detection and event sending
    ///
    /// When device errors are detected, sends InputLost/AllInputsLost events
    /// to event_tx. External monitoring code should watch for AllInputsLost
    /// and call recorder.stop() when received.
    ///
    /// Requirement: STTMIX-REQ-004.1, STTMIX-REQ-006
    fn mixer_thread_loop(
        mut mixer: InputMixer,
        buffers: Vec<(InputConfig, Arc<crate::multi_input_manager::InputBuffer>)>,
        callback: AudioChunkCallback,
        shutdown_rx: mpsc::Receiver<()>,
        device_error_rx: DeviceErrorReceiver,
        event_tx: mpsc::Sender<MultiInputEvent>,
    ) {
        use serde_json::json;
        use std::collections::HashSet;
        use std::time::{Duration, Instant};

        let frame_duration = Duration::from_millis(FRAME_DURATION_MS as u64);
        let mut next_frame_time = Instant::now();

        // Task 9.2: Periodic metrics logging interval (10 seconds)
        const METRICS_LOG_INTERVAL_SECS: u64 = 10;
        let mut last_metrics_log = Instant::now();

        // Track active inputs locally (Finding 1 fix)
        let mut active_device_ids: HashSet<String> = buffers
            .iter()
            .map(|(config, _)| config.device_id.clone())
            .collect();
        let initial_count = active_device_ids.len();

        log_info_details!(
            "mixer::thread",
            "started",
            json!({
                "input_count": initial_count,
                "device_ids": buffers.iter().map(|(c, _)| &c.device_id).collect::<Vec<_>>()
            })
        );

        loop {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                log_info!("mixer::thread", "shutdown_signal_received");
                break;
            }

            // Check for device errors (Finding 1 fix - detection path)
            match device_error_rx.try_recv() {
                Ok((device_id, error)) => {
                    log_warn_details!(
                        "mixer::device",
                        "error",
                        json!({
                            "device_id": device_id,
                            "error": format!("{:?}", error)
                        })
                    );

                    if active_device_ids.remove(&device_id) {
                        let remaining = active_device_ids.len();

                        if remaining == 0 {
                            // All inputs lost - send event and exit (Finding 2 fix)
                            let _ = event_tx.send(MultiInputEvent::AllInputsLost {
                                reason: format!("Last input '{}' error: {:?}", device_id, error),
                            });
                            log_error!(
                                "mixer::thread",
                                "all_inputs_lost",
                                format!("Last input '{}' failed", device_id)
                            );
                            break;
                        } else {
                            // Single input lost - send event and continue
                            let _ = event_tx.send(MultiInputEvent::InputLost {
                                device_id: device_id.clone(),
                                reason: format!("{:?}", error),
                                remaining_active: remaining,
                            });
                            log_warn_details!(
                                "mixer::device",
                                "input_lost",
                                json!({
                                    "device_id": device_id,
                                    "remaining": remaining
                                })
                            );
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No device errors
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Error channel disconnected
                    log_warn!("mixer::thread", "error_channel_disconnected", "");
                }
            }

            // Wait until next frame time
            let now = Instant::now();
            if now < next_frame_time {
                std::thread::sleep(next_frame_time - now);
            }
            next_frame_time += frame_duration;

            // Mix one frame from all inputs
            if let Some(mixed_frame) = mixer.mix_frame(&buffers) {
                callback(mixed_frame);
            }

            // Task 9.2: Periodic metrics logging
            if last_metrics_log.elapsed().as_secs() >= METRICS_LOG_INTERVAL_SECS {
                let metrics = mixer.metrics();
                log_info_details!(
                    "mixer::metrics",
                    "periodic",
                    json!({
                        "frames_mixed": metrics.get_frames_mixed(),
                        "drift_corrections": metrics.get_drift_correction_count(),
                        "clips": metrics.get_clip_count(),
                        "silence_insertions": metrics.get_silence_insertion_count(),
                        "avg_latency_ms": metrics.get_avg_mix_latency_ms(),
                        "max_latency_ms": metrics.get_max_mix_latency_ms(),
                        "active_inputs": active_device_ids.len()
                    })
                );
                last_metrics_log = Instant::now();
            }
        }

        // Task 9.2: Final summary log with structured metrics
        let metrics = mixer.metrics();
        log_info_details!(
            "mixer::thread",
            "stopped",
            json!({
                "frames_mixed": metrics.get_frames_mixed(),
                "drift_corrections": metrics.get_drift_correction_count(),
                "clips": metrics.get_clip_count(),
                "silence_insertions": metrics.get_silence_insertion_count(),
                "avg_latency_ms": metrics.get_avg_mix_latency_ms(),
                "max_latency_ms": metrics.get_max_mix_latency_ms(),
                "initial_inputs": initial_count,
                "final_inputs": active_device_ids.len()
            })
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_device_adapter::AudioDeviceInfo;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock adapter for testing
    struct MockAudioAdapter {
        is_recording: bool,
        devices: Vec<AudioDeviceInfo>,
        start_called_with: Option<String>,
        permission_granted: bool,
    }

    impl MockAudioAdapter {
        fn new() -> Self {
            Self {
                is_recording: false,
                devices: vec![
                    AudioDeviceInfo {
                        id: "mic-1".to_string(),
                        name: "Built-in Microphone".to_string(),
                        sample_rate: 48000,
                        channels: 1,
                        is_loopback: false,
                    },
                    AudioDeviceInfo {
                        id: "loopback-1".to_string(),
                        name: "BlackHole 2ch".to_string(),
                        sample_rate: 48000,
                        channels: 2,
                        is_loopback: true,
                    },
                ],
                start_called_with: None,
                permission_granted: true,
            }
        }
    }

    impl AudioDeviceAdapter for MockAudioAdapter {
        fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
            Ok(self.devices.clone())
        }

        fn start_recording(&mut self, device_id: &str) -> Result<()> {
            self.start_called_with = Some(device_id.to_string());
            self.is_recording = true;
            Ok(())
        }

        fn start_recording_with_callback(
            &mut self,
            device_id: &str,
            _callback: AudioChunkCallback,
        ) -> Result<()> {
            self.start_called_with = Some(device_id.to_string());
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
            if self.permission_granted {
                Ok(())
            } else {
                anyhow::bail!("Microphone permission denied")
            }
        }

        fn set_event_sender(&mut self, _tx: crate::audio_device_adapter::AudioEventSender) {
            // Mock: no-op
        }
    }

    /// Helper to create a mock adapter factory
    fn mock_adapter_factory() -> AdapterFactory {
        Arc::new(|| Ok(Box::new(MockAudioAdapter::new()) as Box<dyn AudioDeviceAdapter>))
    }

    // ========================================================================
    // Test: RecordingMode enum
    // ========================================================================

    #[test]
    fn test_recording_mode_single() {
        let mode = RecordingMode::Single {
            device_id: "mic-1".to_string(),
        };

        if let RecordingMode::Single { device_id } = mode {
            assert_eq!(device_id, "mic-1");
        } else {
            panic!("Expected Single mode");
        }
    }

    #[test]
    fn test_recording_mode_multi() {
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig::default(),
        };

        if let RecordingMode::Multi {
            device_ids,
            mixer_config,
        } = mode
        {
            assert_eq!(device_ids.len(), 2);
            assert_eq!(mixer_config.gains.len(), 2);
            assert!(mixer_config.continue_on_partial_failure);
        } else {
            panic!("Expected Multi mode");
        }
    }

    // ========================================================================
    // Test: MixerConfig defaults
    // ========================================================================

    #[test]
    fn test_mixer_config_default() {
        let config = MixerConfig::default();

        // Default -6dB for 2 inputs (STTMIX-REQ-005.2)
        assert_eq!(config.gains.len(), 2);
        assert_eq!(config.gains[0], -6.0);
        assert_eq!(config.gains[1], -6.0);
        assert!(config.continue_on_partial_failure);
    }

    // ========================================================================
    // Test: AudioDeviceRecorder creation with factory
    // ========================================================================

    #[test]
    fn test_recorder_new_with_factory() {
        let recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        assert!(!recorder.is_recording());
        assert!(recorder.current_mode().is_none());
    }

    #[test]
    fn test_factory_creates_independent_instances() {
        // Track how many times factory is called
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let factory: AdapterFactory = Arc::new(move || {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(MockAudioAdapter::new()) as Box<dyn AudioDeviceAdapter>)
        });

        let recorder = AudioDeviceRecorder::new(factory);

        // Each enumerate_devices call should create a new adapter
        let _ = recorder.enumerate_devices();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        let _ = recorder.enumerate_devices();
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // This verifies factory creates independent instances
    }

    // ========================================================================
    // Test: Enumerate devices (via factory)
    // ========================================================================

    #[test]
    fn test_recorder_enumerate_devices() {
        let recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let devices = recorder.enumerate_devices().unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "mic-1");
        assert_eq!(devices[1].id, "loopback-1");
    }

    // ========================================================================
    // Test: Single mode recording
    // ========================================================================

    #[test]
    fn test_recorder_single_mode_start_stop() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Single {
            device_id: "mic-1".to_string(),
        };

        // Start recording
        recorder.start(mode, callback).unwrap();
        assert!(recorder.is_recording());

        // Check mode
        if let Some(RecordingMode::Single { device_id }) = recorder.current_mode() {
            assert_eq!(device_id, "mic-1");
        } else {
            panic!("Expected Single mode");
        }

        // Stop recording
        recorder.stop().unwrap();
        assert!(!recorder.is_recording());
        assert!(recorder.current_mode().is_none());
    }

    // ========================================================================
    // Test: Multi mode validation
    // ========================================================================

    #[test]
    fn test_recorder_multi_mode_max_2_inputs() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec![
                "mic-1".to_string(),
                "loopback-1".to_string(),
                "mic-2".to_string(), // 3rd input - should fail
            ],
            mixer_config: MixerConfig::default(),
        };

        // Should fail due to max 2 inputs constraint (STTMIX-CON-005)
        let result = recorder.start(mode, callback);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum 2 inputs"));
    }

    #[test]
    fn test_recorder_multi_mode_empty_inputs() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec![],
            mixer_config: MixerConfig::default(),
        };

        // Should fail due to empty inputs
        let result = recorder.start(mode, callback);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one device"));
    }

    #[test]
    fn test_recorder_multi_mode_valid_2_inputs() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig::default(),
        };

        // Should succeed with 2 inputs
        recorder.start(mode, callback).unwrap();
        assert!(recorder.is_recording());

        // Check mode
        if let Some(RecordingMode::Multi { device_ids, .. }) = recorder.current_mode() {
            assert_eq!(device_ids.len(), 2);
        } else {
            panic!("Expected Multi mode");
        }
    }

    // ========================================================================
    // Test: Cannot start while already recording
    // ========================================================================

    #[test]
    fn test_recorder_already_recording_error() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback1: AudioChunkCallback = Box::new(|_| {});
        let callback2: AudioChunkCallback = Box::new(|_| {});

        let mode = RecordingMode::Single {
            device_id: "mic-1".to_string(),
        };

        // First start should succeed
        recorder.start(mode.clone(), callback1).unwrap();

        // Second start should fail
        let result = recorder.start(mode, callback2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Already recording"));
    }

    // ========================================================================
    // Test: Stop when not recording is no-op
    // ========================================================================

    #[test]
    fn test_recorder_stop_when_not_recording() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        // Stop when not recording should be ok (no-op)
        let result = recorder.stop();
        assert!(result.is_ok());
    }

    // ========================================================================
    // Test: Check permission delegation
    // ========================================================================

    #[test]
    fn test_recorder_check_permission() {
        let recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let result = recorder.check_permission();
        assert!(result.is_ok());
    }

    // ========================================================================
    // Test: Adapter factory accessor
    // ========================================================================

    #[test]
    fn test_adapter_factory_accessor() {
        let recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        // Factory should be accessible for MultiInputManager (Task 2.x)
        let factory = recorder.adapter_factory();
        let adapter = factory().unwrap();
        let devices = adapter.enumerate_devices().unwrap();
        assert_eq!(devices.len(), 2);
    }

    // ========================================================================
    // Test: Legacy compatibility (deprecated new_with_adapter)
    // ========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_legacy_new_with_adapter() {
        let adapter = Box::new(MockAudioAdapter::new());
        let mut recorder = AudioDeviceRecorder::new_with_adapter(adapter);

        // Should work for single recording session
        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Single {
            device_id: "mic-1".to_string(),
        };

        recorder.start(mode, callback).unwrap();
        assert!(recorder.is_recording());
        recorder.stop().unwrap();
    }

    // ========================================================================
    // Test: Multi mode with MultiInputManager integration
    // ========================================================================

    #[test]
    fn test_recorder_multi_mode_creates_manager() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig::default(),
        };

        recorder.start(mode, callback).unwrap();

        // Should have a MultiInputManager
        let manager = recorder.get_multi_input_manager();
        assert!(manager.is_some());

        let manager = manager.unwrap();
        assert!(manager.is_recording());
        assert_eq!(manager.active_input_count(), 2);
    }

    #[test]
    fn test_recorder_multi_mode_manager_has_buffers() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig::default(),
        };

        recorder.start(mode, callback).unwrap();

        let manager = recorder.get_multi_input_manager().unwrap();

        // Should have buffers for both inputs
        assert!(manager.get_buffer("mic-1").is_some());
        assert!(manager.get_buffer("loopback-1").is_some());

        // Active buffers should have configs
        let active_buffers = manager.get_active_buffers();
        assert_eq!(active_buffers.len(), 2);
    }

    #[test]
    fn test_recorder_multi_mode_stop_cleans_up() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig::default(),
        };

        recorder.start(mode, callback).unwrap();
        assert!(recorder.get_multi_input_manager().is_some());

        recorder.stop().unwrap();

        // Manager should be cleaned up
        assert!(recorder.get_multi_input_manager().is_none());
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_recorder_single_mode_no_manager() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Single {
            device_id: "mic-1".to_string(),
        };

        recorder.start(mode, callback).unwrap();

        // Single mode should not create a MultiInputManager
        assert!(recorder.get_multi_input_manager().is_none());
    }

    #[test]
    fn test_recorder_multi_mode_custom_gains() {
        let mut recorder = AudioDeviceRecorder::new(mock_adapter_factory());

        let callback: AudioChunkCallback = Box::new(|_| {});
        let mode = RecordingMode::Multi {
            device_ids: vec!["mic-1".to_string(), "loopback-1".to_string()],
            mixer_config: MixerConfig {
                gains: vec![-3.0, -9.0],
                continue_on_partial_failure: true,
            },
        };

        recorder.start(mode, callback).unwrap();

        let manager = recorder.get_multi_input_manager().unwrap();

        // Verify gains were passed to InputConfig
        let mic_config = manager.get_config("mic-1").unwrap();
        assert_eq!(mic_config.gain_db, -3.0);

        let loopback_config = manager.get_config("loopback-1").unwrap();
        assert_eq!(loopback_config.gain_db, -9.0);
    }
}

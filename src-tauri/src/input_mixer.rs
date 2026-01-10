// Input Mixer Module
// STTMIX-REQ-004: Time alignment and mixing
// STTMIX-REQ-005: Gain control and clipping prevention
//
// This module provides the core mixing functionality for multi-input audio:
// - 10ms frame-based time alignment
// - Drift correction between inputs
// - Per-input gain application
// - Clipping detection and prevention

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::multi_input_manager::{InputBuffer, InputConfig};

// ============================================================================
// Constants
// ============================================================================

/// Frame duration in milliseconds
pub const FRAME_DURATION_MS: u32 = 10;

/// Samples per frame at 16kHz (160 samples = 10ms)
pub const SAMPLES_PER_FRAME: usize = 160;

/// Bytes per frame (160 samples * 2 bytes per sample)
pub const BYTES_PER_FRAME: usize = SAMPLES_PER_FRAME * 2;

/// Drift correction threshold in samples (±10 samples = ±0.625ms)
/// Design: meeting-minutes-stt-multi-input/design.md §6
pub const DRIFT_THRESHOLD_SAMPLES: i32 = 10;

/// Minimum interval between drift corrections (100ms)
/// Design: meeting-minutes-stt-multi-input/design.md §6
pub const DRIFT_CORRECTION_MIN_INTERVAL_MS: u64 = 100;

/// Default gain in dB (-6dB = 0.5 linear)
pub const DEFAULT_GAIN_DB: f32 = -6.0;

// ============================================================================
// Mixer Metrics
// ============================================================================

/// Metrics collected by the mixer for observability
/// Requirement: STTMIX-REQ-008.2
#[derive(Debug, Default)]
pub struct MixerMetrics {
    /// Number of drift corrections performed
    pub drift_correction_count: AtomicU64,
    /// Number of frames where clipping occurred
    pub clip_count: AtomicU64,
    /// Number of frames with missing input (silence inserted)
    pub silence_insertion_count: AtomicU64,
    /// Total frames mixed
    pub frames_mixed: AtomicU64,
    /// Maximum mix latency in microseconds (for p95 calculation)
    pub max_mix_latency_us: AtomicU64,
    /// Sum of mix latencies in microseconds (for average calculation)
    pub total_mix_latency_us: AtomicU64,
    /// Count of latency measurements
    pub latency_sample_count: AtomicU64,
}

impl MixerMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_drift_correction(&self) {
        self.drift_correction_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_clip(&self) {
        self.clip_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_silence_insertion(&self) {
        self.silence_insertion_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_frames_mixed(&self) {
        self.frames_mixed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_drift_correction_count(&self) -> u64 {
        self.drift_correction_count.load(Ordering::Relaxed)
    }

    pub fn get_clip_count(&self) -> u64 {
        self.clip_count.load(Ordering::Relaxed)
    }

    pub fn get_silence_insertion_count(&self) -> u64 {
        self.silence_insertion_count.load(Ordering::Relaxed)
    }

    pub fn get_frames_mixed(&self) -> u64 {
        self.frames_mixed.load(Ordering::Relaxed)
    }

    /// Record a mix latency measurement
    pub fn record_latency_us(&self, latency_us: u64) {
        // Update max latency
        let mut current_max = self.max_mix_latency_us.load(Ordering::Relaxed);
        while latency_us > current_max {
            match self.max_mix_latency_us.compare_exchange_weak(
                current_max,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => current_max = new_max,
            }
        }
        // Update total and count
        self.total_mix_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_sample_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get maximum mix latency in milliseconds
    pub fn get_max_mix_latency_ms(&self) -> f64 {
        self.max_mix_latency_us.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Get average mix latency in milliseconds
    pub fn get_avg_mix_latency_ms(&self) -> f64 {
        let total = self.total_mix_latency_us.load(Ordering::Relaxed);
        let count = self.latency_sample_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (total as f64 / count as f64) / 1000.0
        }
    }
}

// ============================================================================
// Input State for Mixer
// ============================================================================

/// State tracked per input for drift correction
struct InputDriftState {
    /// Cumulative sample count received
    samples_received: i64,
    /// Last drift correction time
    last_correction: Option<Instant>,
}

impl InputDriftState {
    fn new() -> Self {
        Self {
            samples_received: 0,
            last_correction: None,
        }
    }
}

// ============================================================================
// Input Mixer
// ============================================================================

/// Mixer for combining multiple audio inputs into a single output stream
///
/// Requirement: STTMIX-REQ-004 (time alignment and mixing)
/// Design: meeting-minutes-stt-multi-input/design.md §4.4
pub struct InputMixer {
    /// Metrics for observability
    metrics: Arc<MixerMetrics>,
    /// Drift state per input (keyed by device_id)
    drift_states: std::collections::HashMap<String, InputDriftState>,
    /// Reference time for drift calculation
    reference_samples: i64,
}

impl InputMixer {
    /// Create a new InputMixer
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(MixerMetrics::new()),
            drift_states: std::collections::HashMap::new(),
            reference_samples: 0,
        }
    }

    /// Get a reference to the metrics
    pub fn metrics(&self) -> Arc<MixerMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Initialize drift tracking for an input
    pub fn register_input(&mut self, device_id: &str) {
        self.drift_states
            .insert(device_id.to_string(), InputDriftState::new());
    }

    /// Remove drift tracking for an input
    pub fn unregister_input(&mut self, device_id: &str) {
        self.drift_states.remove(device_id);
    }

    /// Mix one 10ms frame from multiple inputs
    ///
    /// # Arguments
    /// * `inputs` - List of (config, buffer) pairs for active inputs
    ///
    /// # Returns
    /// * `Some(Vec<u8>)` - Mixed 10ms frame as i16 PCM bytes (320 bytes)
    /// * `None` - If no inputs are available
    ///
    /// Requirement: STTMIX-REQ-004.1, STTMIX-REQ-004.3
    pub fn mix_frame(&mut self, inputs: &[(InputConfig, Arc<InputBuffer>)]) -> Option<Vec<u8>> {
        if inputs.is_empty() {
            return None;
        }

        // Task 9.1: Measure mix latency
        let start_time = std::time::Instant::now();

        // Collect frames from each input
        let mut input_frames: Vec<(InputConfig, Vec<i16>)> = Vec::with_capacity(inputs.len());

        for (config, buffer) in inputs {
            let frame = self.extract_frame(config, buffer);
            input_frames.push((config.clone(), frame));
        }

        // Mix all frames together
        let mixed = self.mix_frames(&input_frames);

        self.metrics.increment_frames_mixed();
        self.reference_samples += SAMPLES_PER_FRAME as i64;

        // Task 9.1: Record latency
        let latency_us = start_time.elapsed().as_micros() as u64;
        self.metrics.record_latency_us(latency_us);

        Some(mixed)
    }

    /// Extract one 10ms frame from an input buffer
    ///
    /// Handles:
    /// - Normal case: extract BYTES_PER_FRAME bytes
    /// - Underrun: pad with silence
    /// - Drift correction: drop or duplicate samples as needed
    ///
    /// Requirement: STTMIX-REQ-004.1, STTMIX-REQ-004.2
    fn extract_frame(&mut self, config: &InputConfig, buffer: &InputBuffer) -> Vec<i16> {
        let device_id = &config.device_id;

        // Get or create drift state
        if !self.drift_states.contains_key(device_id) {
            self.register_input(device_id);
        }

        // Take bytes from buffer
        let raw_bytes = buffer.take(BYTES_PER_FRAME);
        let samples_taken = raw_bytes.len() / 2;

        // Update drift state
        if let Some(state) = self.drift_states.get_mut(device_id) {
            state.samples_received += samples_taken as i64;
        }

        // Convert bytes to i16 samples
        let mut samples: Vec<i16> = raw_bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        // Handle underrun (not enough samples)
        if samples.len() < SAMPLES_PER_FRAME {
            self.metrics.increment_silence_insertion();
            // Pad with silence
            samples.resize(SAMPLES_PER_FRAME, 0);
        }

        // Apply drift correction if needed
        self.apply_drift_correction(device_id, &mut samples);

        samples
    }

    /// Apply drift correction to samples if threshold exceeded
    ///
    /// Requirement: STTMIX-REQ-004.2
    fn apply_drift_correction(&mut self, device_id: &str, samples: &mut Vec<i16>) {
        let state = match self.drift_states.get_mut(device_id) {
            Some(s) => s,
            None => return,
        };

        // Calculate drift from reference
        let drift = state.samples_received - self.reference_samples;

        // Check if correction is needed
        if drift.abs() <= DRIFT_THRESHOLD_SAMPLES as i64 {
            return;
        }

        // Check minimum interval since last correction
        let now = Instant::now();
        if let Some(last) = state.last_correction {
            if now.duration_since(last).as_millis() < DRIFT_CORRECTION_MIN_INTERVAL_MS as u128 {
                return;
            }
        }

        // Apply correction
        if drift > 0 {
            // Input is ahead (too many samples) - drop 1 sample
            if samples.len() > 1 {
                samples.remove(samples.len() / 2); // Remove from middle
                state.samples_received -= 1;
            }
        } else {
            // Input is behind (too few samples) - duplicate 1 sample
            let mid = samples.len() / 2;
            if mid < samples.len() {
                let dup_sample = samples[mid];
                samples.insert(mid, dup_sample);
                state.samples_received += 1;
            }
        }

        // Ensure frame size is correct after correction
        samples.truncate(SAMPLES_PER_FRAME);
        if samples.len() < SAMPLES_PER_FRAME {
            samples.resize(SAMPLES_PER_FRAME, 0);
        }

        state.last_correction = Some(now);
        self.metrics.increment_drift_correction();
    }

    /// Mix multiple input frames into a single output frame
    ///
    /// Requirement: STTMIX-REQ-004.3, STTMIX-REQ-005
    fn mix_frames(&self, inputs: &[(InputConfig, Vec<i16>)]) -> Vec<u8> {
        let mut mixed: Vec<f32> = vec![0.0; SAMPLES_PER_FRAME];
        let mut clipped = false;

        for (config, samples) in inputs {
            // Skip muted inputs
            if config.muted {
                continue;
            }

            // Calculate linear gain from dB
            let gain = db_to_linear(config.gain_db);

            // Add samples with gain
            for (i, &sample) in samples.iter().enumerate() {
                if i < mixed.len() {
                    mixed[i] += (sample as f32 / 32768.0) * gain;
                }
            }
        }

        // Convert to i16 PCM with clipping detection
        let output: Vec<u8> = mixed
            .iter()
            .flat_map(|&sample| {
                // Detect clipping
                if sample > 1.0 || sample < -1.0 {
                    clipped = true;
                }
                // Clamp and convert
                let clamped = sample.clamp(-1.0, 1.0);
                let scaled = (clamped * 32767.0) as i16;
                scaled.to_le_bytes()
            })
            .collect();

        if clipped {
            self.metrics.increment_clip();
        }

        output
    }

    /// Reset the mixer state (e.g., when starting new recording)
    pub fn reset(&mut self) {
        self.drift_states.clear();
        self.reference_samples = 0;
    }
}

impl Default for InputMixer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert decibels to linear gain
///
/// # Arguments
/// * `db` - Gain in decibels (e.g., -6.0 for half amplitude)
///
/// # Returns
/// Linear gain multiplier (e.g., 0.5 for -6dB)
#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear gain to decibels
///
/// # Arguments
/// * `linear` - Linear gain multiplier
///
/// # Returns
/// Gain in decibels
#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.log10()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_input_manager::InputRole;

    // Helper to create a test buffer with data
    fn create_test_buffer(data: &[i16]) -> Arc<InputBuffer> {
        let buffer = Arc::new(InputBuffer::new(32000));
        let bytes: Vec<u8> = data.iter().flat_map(|&s| s.to_le_bytes()).collect();
        buffer.push(&bytes);
        buffer
    }

    // ========================================================================
    // Test: Constants
    // ========================================================================

    #[test]
    fn test_frame_constants() {
        // 10ms at 16kHz = 160 samples
        assert_eq!(SAMPLES_PER_FRAME, 160);
        // 160 samples * 2 bytes = 320 bytes
        assert_eq!(BYTES_PER_FRAME, 320);
    }

    // ========================================================================
    // Test: db_to_linear conversion
    // ========================================================================

    #[test]
    fn test_db_to_linear() {
        // 0 dB = 1.0 (unity gain)
        assert!((db_to_linear(0.0) - 1.0).abs() < 0.001);

        // -6 dB ≈ 0.5 (half amplitude)
        assert!((db_to_linear(-6.0) - 0.5).abs() < 0.02);

        // -20 dB = 0.1
        assert!((db_to_linear(-20.0) - 0.1).abs() < 0.001);

        // +6 dB ≈ 2.0 (double amplitude)
        assert!((db_to_linear(6.0) - 2.0).abs() < 0.02);
    }

    #[test]
    fn test_linear_to_db() {
        // 1.0 = 0 dB
        assert!((linear_to_db(1.0) - 0.0).abs() < 0.001);

        // 0.5 ≈ -6 dB
        assert!((linear_to_db(0.5) - (-6.02)).abs() < 0.1);

        // 0.1 = -20 dB
        assert!((linear_to_db(0.1) - (-20.0)).abs() < 0.001);
    }

    // ========================================================================
    // Test: MixerMetrics
    // ========================================================================

    #[test]
    fn test_mixer_metrics() {
        let metrics = MixerMetrics::new();

        assert_eq!(metrics.get_drift_correction_count(), 0);
        assert_eq!(metrics.get_clip_count(), 0);
        assert_eq!(metrics.get_silence_insertion_count(), 0);
        assert_eq!(metrics.get_frames_mixed(), 0);

        metrics.increment_drift_correction();
        metrics.increment_clip();
        metrics.increment_silence_insertion();
        metrics.increment_frames_mixed();

        assert_eq!(metrics.get_drift_correction_count(), 1);
        assert_eq!(metrics.get_clip_count(), 1);
        assert_eq!(metrics.get_silence_insertion_count(), 1);
        assert_eq!(metrics.get_frames_mixed(), 1);
    }

    #[test]
    fn test_latency_metrics() {
        let metrics = MixerMetrics::new();

        // Initial state
        assert_eq!(metrics.get_max_mix_latency_ms(), 0.0);
        assert_eq!(metrics.get_avg_mix_latency_ms(), 0.0);

        // Record some latencies (in microseconds)
        metrics.record_latency_us(1000); // 1ms
        metrics.record_latency_us(2000); // 2ms
        metrics.record_latency_us(3000); // 3ms

        // Max should be 3ms
        assert_eq!(metrics.get_max_mix_latency_ms(), 3.0);

        // Average should be (1+2+3)/3 = 2ms
        assert_eq!(metrics.get_avg_mix_latency_ms(), 2.0);

        // Record a new max
        metrics.record_latency_us(5000); // 5ms
        assert_eq!(metrics.get_max_mix_latency_ms(), 5.0);

        // New average: (1+2+3+5)/4 = 2.75ms
        assert!((metrics.get_avg_mix_latency_ms() - 2.75).abs() < 0.001);
    }

    // ========================================================================
    // Test: InputMixer creation
    // ========================================================================

    #[test]
    fn test_mixer_new() {
        let mixer = InputMixer::new();
        assert_eq!(mixer.metrics().get_frames_mixed(), 0);
    }

    // ========================================================================
    // Test: mix_frame with single input
    // ========================================================================

    #[test]
    fn test_mix_frame_single_input() {
        let mut mixer = InputMixer::new();

        // Create a buffer with one frame of samples
        let samples: Vec<i16> = (0..SAMPLES_PER_FRAME as i16).collect();
        let buffer = create_test_buffer(&samples);

        let config = InputConfig::new("mic-1", InputRole::Microphone);
        let inputs = vec![(config, buffer)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        let output = result.unwrap();
        assert_eq!(output.len(), BYTES_PER_FRAME);
        assert_eq!(mixer.metrics().get_frames_mixed(), 1);
    }

    // ========================================================================
    // Test: mix_frame with empty inputs
    // ========================================================================

    #[test]
    fn test_mix_frame_empty_inputs() {
        let mut mixer = InputMixer::new();
        let inputs: Vec<(InputConfig, Arc<InputBuffer>)> = vec![];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_none());
    }

    // ========================================================================
    // Test: mix_frame with underrun (silence insertion)
    // ========================================================================

    #[test]
    fn test_mix_frame_underrun() {
        let mut mixer = InputMixer::new();

        // Create a buffer with fewer samples than needed
        let samples: Vec<i16> = vec![1000; 50]; // Only 50 samples, need 160
        let buffer = create_test_buffer(&samples);

        let config = InputConfig::new("mic-1", InputRole::Microphone);
        let inputs = vec![(config, buffer)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        let output = result.unwrap();
        assert_eq!(output.len(), BYTES_PER_FRAME);
        assert_eq!(mixer.metrics().get_silence_insertion_count(), 1);
    }

    // ========================================================================
    // Test: mix_frame with two inputs
    // ========================================================================

    #[test]
    fn test_mix_frame_two_inputs() {
        let mut mixer = InputMixer::new();

        // Input 1: constant 16384 (0.5 in normalized)
        let samples1: Vec<i16> = vec![16384; SAMPLES_PER_FRAME];
        let buffer1 = create_test_buffer(&samples1);
        let config1 = InputConfig::new("mic-1", InputRole::Microphone);

        // Input 2: constant 16384 (0.5 in normalized)
        let samples2: Vec<i16> = vec![16384; SAMPLES_PER_FRAME];
        let buffer2 = create_test_buffer(&samples2);
        let config2 = InputConfig::new("loopback-1", InputRole::Loopback);

        let inputs = vec![(config1, buffer1), (config2, buffer2)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        let output = result.unwrap();
        assert_eq!(output.len(), BYTES_PER_FRAME);

        // With default -6dB gain on each:
        // 0.5 * 0.5 (gain) + 0.5 * 0.5 (gain) = 0.5
        // 0.5 * 32767 ≈ 16383
        let first_sample = i16::from_le_bytes([output[0], output[1]]);
        assert!((first_sample - 16383).abs() < 100);
    }

    // ========================================================================
    // Test: mix_frame with muted input
    // ========================================================================

    #[test]
    fn test_mix_frame_muted_input() {
        let mut mixer = InputMixer::new();

        // Input 1: loud signal
        let samples1: Vec<i16> = vec![32000; SAMPLES_PER_FRAME];
        let buffer1 = create_test_buffer(&samples1);
        let config1 = InputConfig::new("mic-1", InputRole::Microphone);

        // Input 2: muted
        let samples2: Vec<i16> = vec![32000; SAMPLES_PER_FRAME];
        let buffer2 = create_test_buffer(&samples2);
        let config2 = InputConfig::new("loopback-1", InputRole::Loopback).with_muted(true);

        let inputs = vec![(config1, buffer1), (config2, buffer2)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        // Only input 1 should contribute
        let output = result.unwrap();
        let first_sample = i16::from_le_bytes([output[0], output[1]]);
        // 32000/32768 * 0.5 (gain) * 32767 ≈ 15999
        assert!(first_sample > 0);
        assert!(first_sample < 20000); // Much less than full scale
    }

    // ========================================================================
    // Test: clipping detection
    // ========================================================================

    #[test]
    fn test_clipping_detection() {
        let mut mixer = InputMixer::new();

        // Input 1: max amplitude
        let samples1: Vec<i16> = vec![32767; SAMPLES_PER_FRAME];
        let buffer1 = create_test_buffer(&samples1);
        let config1 = InputConfig::new("mic-1", InputRole::Microphone).with_gain(0.0); // Unity gain

        // Input 2: max amplitude
        let samples2: Vec<i16> = vec![32767; SAMPLES_PER_FRAME];
        let buffer2 = create_test_buffer(&samples2);
        let config2 = InputConfig::new("loopback-1", InputRole::Loopback).with_gain(0.0);

        let inputs = vec![(config1, buffer1), (config2, buffer2)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        // Should detect clipping (1.0 + 1.0 = 2.0, exceeds 1.0)
        assert!(mixer.metrics().get_clip_count() > 0);

        // Output should be clamped to max
        let output = result.unwrap();
        let first_sample = i16::from_le_bytes([output[0], output[1]]);
        assert_eq!(first_sample, 32767);
    }

    // ========================================================================
    // Test: custom gain
    // ========================================================================

    #[test]
    fn test_custom_gain() {
        let mut mixer = InputMixer::new();

        // Input with 0dB gain (unity)
        let samples: Vec<i16> = vec![16384; SAMPLES_PER_FRAME]; // 0.5 normalized
        let buffer = create_test_buffer(&samples);
        let config = InputConfig::new("mic-1", InputRole::Microphone).with_gain(0.0);

        let inputs = vec![(config, buffer)];

        let result = mixer.mix_frame(&inputs);
        assert!(result.is_some());

        let output = result.unwrap();
        let first_sample = i16::from_le_bytes([output[0], output[1]]);
        // 0.5 * 1.0 (unity) * 32767 ≈ 16383
        assert!((first_sample - 16383).abs() < 100);
    }

    // ========================================================================
    // Test: drift correction
    // ========================================================================

    #[test]
    fn test_drift_tracking() {
        let mut mixer = InputMixer::new();
        mixer.register_input("mic-1");

        // Verify input is registered
        assert!(mixer.drift_states.contains_key("mic-1"));

        mixer.unregister_input("mic-1");
        assert!(!mixer.drift_states.contains_key("mic-1"));
    }

    // ========================================================================
    // Test: reset
    // ========================================================================

    #[test]
    fn test_mixer_reset() {
        let mut mixer = InputMixer::new();
        mixer.register_input("mic-1");
        mixer.reference_samples = 1000;

        mixer.reset();

        assert!(mixer.drift_states.is_empty());
        assert_eq!(mixer.reference_samples, 0);
    }

    // ========================================================================
    // Test: multiple frames
    // ========================================================================

    #[test]
    fn test_multiple_frames() {
        let mut mixer = InputMixer::new();

        for i in 0..10 {
            let samples: Vec<i16> = vec![i as i16 * 1000; SAMPLES_PER_FRAME];
            let buffer = create_test_buffer(&samples);
            let config = InputConfig::new("mic-1", InputRole::Microphone);

            let result = mixer.mix_frame(&[(config, buffer)]);
            assert!(result.is_some());
        }

        assert_eq!(mixer.metrics().get_frames_mixed(), 10);
    }
}

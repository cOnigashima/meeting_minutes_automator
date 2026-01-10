// Resampler and Downmix Module
// STTMIX-REQ-003: Audio normalization to 16kHz mono
//
// This module provides real-time audio processing functions:
// - Stereo to mono downmix (L/R average)
// - Averaging downsampling (native rate -> 16kHz)
// - Combined processing for cpal callbacks
//
// IMPORTANT: Only integer-ratio sample rates are supported (48000, 32000, 16000 Hz).
// Non-integer ratios (44100, 22050 Hz) would produce incorrect output sample rates.

/// Target sample rate for STT processing
pub const TARGET_SAMPLE_RATE: usize = 16000;

/// Supported sample rates that divide evenly to 16kHz
/// These produce exact integer downsample ratios for accurate 16kHz output
pub const SUPPORTED_SAMPLE_RATES: &[u32] = &[48000, 32000, 16000];

/// Error type for sample rate validation
#[derive(Debug, Clone)]
pub enum SampleRateError {
    /// Sample rate is below 16kHz (cannot downsample)
    TooLow { rate: u32 },
    /// Sample rate doesn't divide evenly to 16kHz
    NonIntegerRatio { rate: u32, ratio: f32 },
}

impl PartialEq for SampleRateError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SampleRateError::TooLow { rate: a }, SampleRateError::TooLow { rate: b }) => a == b,
            (
                SampleRateError::NonIntegerRatio { rate: a, .. },
                SampleRateError::NonIntegerRatio { rate: b, .. },
            ) => a == b, // Compare only rate, not the f32 ratio
            _ => false,
        }
    }
}

impl Eq for SampleRateError {}

impl std::fmt::Display for SampleRateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleRateError::TooLow { rate } => {
                write!(
                    f,
                    "Sample rate {}Hz is below minimum 16kHz. Upsampling not supported.",
                    rate
                )
            }
            SampleRateError::NonIntegerRatio { rate, ratio } => {
                write!(
                    f,
                    "Sample rate {}Hz has non-integer ratio {:.4} to 16kHz. \
                     Supported rates: {:?}",
                    rate, ratio, SUPPORTED_SAMPLE_RATES
                )
            }
        }
    }
}

impl std::error::Error for SampleRateError {}

/// Validate that a sample rate can be accurately downsampled to 16kHz
///
/// # Arguments
/// * `sample_rate` - The native sample rate to validate
///
/// # Returns
/// * `Ok(ratio)` - The integer downsample ratio if valid
/// * `Err(SampleRateError)` - If the sample rate is unsupported
///
/// # Examples
/// ```
/// use meeting_minutes_automator::resampler::validate_sample_rate;
///
/// assert!(validate_sample_rate(48000).is_ok()); // 48000/16000 = 3
/// assert!(validate_sample_rate(32000).is_ok()); // 32000/16000 = 2
/// assert!(validate_sample_rate(16000).is_ok()); // 16000/16000 = 1
/// assert!(validate_sample_rate(44100).is_err()); // 44100/16000 = 2.75625
/// assert!(validate_sample_rate(8000).is_err());  // Below 16kHz
/// ```
pub fn validate_sample_rate(sample_rate: u32) -> Result<usize, SampleRateError> {
    // Check minimum
    if sample_rate < TARGET_SAMPLE_RATE as u32 {
        return Err(SampleRateError::TooLow { rate: sample_rate });
    }

    // Check for integer ratio
    let ratio_f = sample_rate as f32 / TARGET_SAMPLE_RATE as f32;
    let ratio_i = sample_rate as usize / TARGET_SAMPLE_RATE;

    // Verify the ratio is exactly an integer (no remainder)
    if sample_rate as usize % TARGET_SAMPLE_RATE != 0 {
        return Err(SampleRateError::NonIntegerRatio {
            rate: sample_rate,
            ratio: ratio_f,
        });
    }

    Ok(ratio_i)
}

/// Check if a sample rate is supported without returning the ratio
pub fn is_sample_rate_supported(sample_rate: u32) -> bool {
    validate_sample_rate(sample_rate).is_ok()
}

/// Convert interleaved stereo samples to mono by averaging L/R pairs
///
/// Input: [L0, R0, L1, R1, L2, R2, ...]
/// Output: [(L0+R0)/2, (L1+R1)/2, (L2+R2)/2, ...]
///
/// # Arguments
/// * `stereo_samples` - Interleaved stereo f32 samples
///
/// # Returns
/// Mono f32 samples (half the length of input)
///
/// Requirement: STTMIX-REQ-003.2 (stereo to mono downmix)
#[inline]
pub fn stereo_to_mono(stereo_samples: &[f32]) -> Vec<f32> {
    stereo_samples
        .chunks_exact(2)
        .map(|pair| (pair[0] + pair[1]) * 0.5)
        .collect()
}

/// Downsample audio using averaging (simple low-pass filter)
///
/// Groups N consecutive samples and averages them, where N = ratio.
/// This provides basic anti-aliasing compared to simple decimation.
///
/// # Arguments
/// * `samples` - Mono f32 samples at native sample rate
/// * `ratio` - Downsample ratio (e.g., 3 for 48kHz -> 16kHz)
///
/// # Returns
/// Downsampled mono f32 samples
///
/// Requirement: STTMIX-REQ-003.1 (averaging downsampling)
#[inline]
pub fn downsample_average(samples: &[f32], ratio: usize) -> Vec<f32> {
    if ratio <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks(ratio)
        .map(|chunk| {
            let sum: f32 = chunk.iter().sum();
            sum / chunk.len() as f32
        })
        .collect()
}

/// Convert f32 samples to i16 PCM bytes (little-endian)
///
/// # Arguments
/// * `samples` - f32 samples in range [-1.0, 1.0]
///
/// # Returns
/// i16 PCM samples as bytes (2 bytes per sample, little-endian)
#[inline]
pub fn f32_to_i16_pcm(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&sample| {
            let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            scaled.to_le_bytes()
        })
        .collect()
}

/// Process raw cpal audio data to 16kHz mono i16 PCM
///
/// Combined function for use in audio callbacks:
/// 1. If stereo, convert to mono (L/R average)
/// 2. Downsample to 16kHz using averaging
/// 3. Convert to i16 PCM bytes
///
/// # Arguments
/// * `data` - Raw f32 samples from cpal (may be stereo interleaved)
/// * `channels` - Number of channels (1 = mono, 2 = stereo)
/// * `native_sample_rate` - Source sample rate in Hz
///
/// # Returns
/// 16kHz mono i16 PCM as bytes
///
/// Requirement: STTMIX-REQ-003 (full normalization pipeline)
pub fn process_audio_to_16khz_mono(
    data: &[f32],
    channels: u16,
    native_sample_rate: u32,
) -> Vec<u8> {
    // Step 1: Convert to mono if stereo
    let mono_samples: Vec<f32> = if channels >= 2 {
        stereo_to_mono(data)
    } else {
        data.to_vec()
    };

    // Step 2: Calculate downsample ratio
    let ratio = (native_sample_rate as usize) / TARGET_SAMPLE_RATE;

    // Step 3: Downsample to 16kHz
    let downsampled = downsample_average(&mono_samples, ratio);

    // Step 4: Convert to i16 PCM bytes
    f32_to_i16_pcm(&downsampled)
}

/// Calculate expected output samples for given input
///
/// Useful for buffer sizing and validation.
///
/// # Arguments
/// * `input_samples` - Number of input samples (per channel)
/// * `channels` - Number of channels
/// * `native_sample_rate` - Source sample rate
///
/// # Returns
/// Expected number of output samples at 16kHz mono
pub fn expected_output_samples(
    input_samples: usize,
    channels: u16,
    native_sample_rate: u32,
) -> usize {
    let mono_samples = if channels >= 2 {
        input_samples / 2
    } else {
        input_samples
    };
    let ratio = (native_sample_rate as usize) / TARGET_SAMPLE_RATE;
    if ratio == 0 {
        mono_samples
    } else {
        mono_samples / ratio
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stereo_to_mono() {
        // Simple stereo signal: L=1.0, R=0.0 -> mono=0.5
        let stereo = vec![1.0f32, 0.0, 0.5, 0.5, -1.0, 1.0];
        let mono = stereo_to_mono(&stereo);

        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.5).abs() < 0.001); // (1.0 + 0.0) / 2
        assert!((mono[1] - 0.5).abs() < 0.001); // (0.5 + 0.5) / 2
        assert!((mono[2] - 0.0).abs() < 0.001); // (-1.0 + 1.0) / 2
    }

    #[test]
    fn test_stereo_to_mono_empty() {
        let stereo: Vec<f32> = vec![];
        let mono = stereo_to_mono(&stereo);
        assert!(mono.is_empty());
    }

    #[test]
    fn test_stereo_to_mono_odd_length() {
        // Odd number of samples - last sample is ignored by chunks_exact
        let stereo = vec![1.0f32, 0.0, 0.5];
        let mono = stereo_to_mono(&stereo);
        assert_eq!(mono.len(), 1); // Only 1 complete pair
    }

    #[test]
    fn test_downsample_average_ratio_3() {
        // 48kHz -> 16kHz (ratio = 3)
        let samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let downsampled = downsample_average(&samples, 3);

        assert_eq!(downsampled.len(), 3);
        assert!((downsampled[0] - 0.2).abs() < 0.001); // avg(0.1, 0.2, 0.3)
        assert!((downsampled[1] - 0.5).abs() < 0.001); // avg(0.4, 0.5, 0.6)
        assert!((downsampled[2] - 0.8).abs() < 0.001); // avg(0.7, 0.8, 0.9)
    }

    #[test]
    fn test_downsample_average_ratio_1() {
        // No downsampling needed
        let samples = vec![0.1f32, 0.2, 0.3];
        let downsampled = downsample_average(&samples, 1);
        assert_eq!(downsampled, samples);
    }

    #[test]
    fn test_downsample_average_ratio_0() {
        // Edge case: ratio 0 should not divide by zero
        let samples = vec![0.1f32, 0.2, 0.3];
        let downsampled = downsample_average(&samples, 0);
        assert_eq!(downsampled, samples);
    }

    #[test]
    fn test_downsample_average_incomplete_chunk() {
        // Input not evenly divisible by ratio
        let samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5]; // 5 samples, ratio 3
        let downsampled = downsample_average(&samples, 3);

        assert_eq!(downsampled.len(), 2);
        assert!((downsampled[0] - 0.2).abs() < 0.001); // avg(0.1, 0.2, 0.3)
        assert!((downsampled[1] - 0.45).abs() < 0.001); // avg(0.4, 0.5) - partial chunk
    }

    #[test]
    fn test_f32_to_i16_pcm() {
        let samples = vec![0.0f32, 1.0, -1.0, 0.5];
        let pcm = f32_to_i16_pcm(&samples);

        assert_eq!(pcm.len(), 8); // 4 samples * 2 bytes

        // Decode back to verify
        let s0 = i16::from_le_bytes([pcm[0], pcm[1]]);
        let s1 = i16::from_le_bytes([pcm[2], pcm[3]]);
        let s2 = i16::from_le_bytes([pcm[4], pcm[5]]);
        let s3 = i16::from_le_bytes([pcm[6], pcm[7]]);

        assert_eq!(s0, 0);
        assert_eq!(s1, 32767);  // 1.0 * 32767
        assert_eq!(s2, -32767); // -1.0 * 32767
        assert_eq!(s3, 16383);  // 0.5 * 32767 ≈ 16383
    }

    #[test]
    fn test_f32_to_i16_pcm_clipping() {
        // Values outside [-1, 1] should be clamped
        let samples = vec![2.0f32, -2.0];
        let pcm = f32_to_i16_pcm(&samples);

        let s0 = i16::from_le_bytes([pcm[0], pcm[1]]);
        let s1 = i16::from_le_bytes([pcm[2], pcm[3]]);

        assert_eq!(s0, 32767);  // Clamped to max
        assert_eq!(s1, -32768); // Clamped to min
    }

    #[test]
    fn test_process_audio_mono_48khz() {
        // Mono 48kHz input -> 16kHz output
        // 9 samples at 48kHz -> 3 samples at 16kHz
        let data = vec![0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let pcm = process_audio_to_16khz_mono(&data, 1, 48000);

        assert_eq!(pcm.len(), 6); // 3 samples * 2 bytes
    }

    #[test]
    fn test_process_audio_stereo_48khz() {
        // Stereo 48kHz input -> 16kHz mono output
        // 18 samples (9 stereo frames) at 48kHz -> 3 mono samples at 16kHz
        let data: Vec<f32> = (0..18).map(|i| i as f32 / 18.0).collect();
        let pcm = process_audio_to_16khz_mono(&data, 2, 48000);

        // 18 stereo samples -> 9 mono samples -> 3 downsampled samples -> 6 bytes
        assert_eq!(pcm.len(), 6);
    }

    // ========================================================================
    // Test: Sample rate validation
    // ========================================================================

    #[test]
    fn test_validate_sample_rate_supported() {
        // Valid rates: integer ratio to 16kHz
        assert_eq!(validate_sample_rate(48000), Ok(3)); // 48000/16000 = 3
        assert_eq!(validate_sample_rate(32000), Ok(2)); // 32000/16000 = 2
        assert_eq!(validate_sample_rate(16000), Ok(1)); // 16000/16000 = 1
    }

    #[test]
    fn test_validate_sample_rate_non_integer() {
        // 44.1kHz has non-integer ratio (2.75625)
        let result = validate_sample_rate(44100);
        assert!(result.is_err());
        assert!(matches!(result, Err(SampleRateError::NonIntegerRatio { rate: 44100, .. })));

        // 22.05kHz also non-integer (1.378125)
        let result = validate_sample_rate(22050);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sample_rate_too_low() {
        // Below 16kHz - upsampling not supported
        let result = validate_sample_rate(8000);
        assert!(result.is_err());
        assert!(matches!(result, Err(SampleRateError::TooLow { rate: 8000 })));

        let result = validate_sample_rate(11025);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_sample_rate_supported() {
        assert!(is_sample_rate_supported(48000));
        assert!(is_sample_rate_supported(32000));
        assert!(is_sample_rate_supported(16000));
        assert!(!is_sample_rate_supported(44100));
        assert!(!is_sample_rate_supported(8000));
    }

    #[test]
    fn test_expected_output_samples() {
        // Mono 48kHz
        assert_eq!(expected_output_samples(48000, 1, 48000), 16000);

        // Stereo 48kHz
        assert_eq!(expected_output_samples(96000, 2, 48000), 16000);

        // Mono 32kHz
        assert_eq!(expected_output_samples(32000, 1, 32000), 16000);
    }

    #[test]
    fn test_full_pipeline_realistic() {
        // Simulate realistic 10ms of stereo 48kHz audio
        // 48000 Hz * 0.01s * 2 channels = 960 samples
        let sample_count = 960;
        let data: Vec<f32> = (0..sample_count)
            .map(|i| (i as f32 / 100.0).sin() * 0.5)
            .collect();

        let pcm = process_audio_to_16khz_mono(&data, 2, 48000);

        // 960 stereo samples -> 480 mono samples
        // 480 / 3 (downsample ratio) = 160 samples
        // 160 samples * 2 bytes = 320 bytes
        assert_eq!(pcm.len(), 320);

        // Verify 160 samples = 10ms at 16kHz
        let output_samples = pcm.len() / 2;
        let duration_ms = (output_samples as f32 / 16000.0) * 1000.0;
        assert!((duration_ms - 10.0).abs() < 0.1);
    }
}

// Test Fixtures Module (BLOCK-006)
// Provides mock audio data for E2E tests

/// Test audio files (16kHz mono 16-bit PCM WAV format)
pub mod test_audio {
    /// Short test audio (3 seconds)
    /// Pattern: 1s speech (440Hz) → 0.5s silence → 1s speech (550Hz) → 0.5s silence
    /// Use case: Basic VAD detection and single transcription segment testing
    pub const SHORT: &[u8] = include_bytes!("test_audio_short.wav");

    /// Long test audio (10 seconds)
    /// Pattern: Multiple speech/silence cycles with varying durations
    /// Use case: Partial/final text distribution, multiple VAD segments
    pub const LONG: &[u8] = include_bytes!("test_audio_long.wav");

    /// Silence audio (2 seconds)
    /// Pattern: Pure silence
    /// Use case: no_speech event handling
    pub const SILENCE: &[u8] = include_bytes!("test_audio_silence.wav");
}

/// Extract PCM samples from WAV file bytes
///
/// Skips 44-byte WAV header and returns raw 16-bit PCM samples.
///
/// # Arguments
/// * `wav_bytes` - WAV file bytes (must be 16kHz mono 16-bit PCM)
///
/// # Returns
/// * Vec<i16> - PCM samples
pub fn extract_pcm_samples(wav_bytes: &[u8]) -> Vec<i16> {
    // Skip 44-byte WAV header
    let pcm_bytes = &wav_bytes[44..];

    // Convert bytes to i16 samples (little-endian)
    pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Convert PCM samples to u8 bytes (for IPC transmission)
///
/// # Arguments
/// * `samples` - i16 PCM samples
///
/// # Returns
/// * Vec<u8> - Little-endian bytes
pub fn pcm_samples_to_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

/// Split PCM samples into chunks (for streaming simulation)
///
/// # Arguments
/// * `samples` - i16 PCM samples
/// * `chunk_duration_ms` - Chunk duration in milliseconds (e.g., 20ms for 320 samples at 16kHz)
///
/// # Returns
/// * Vec<Vec<i16>> - Chunked samples
pub fn chunk_pcm_samples(samples: &[i16], chunk_duration_ms: u32) -> Vec<Vec<i16>> {
    const SAMPLE_RATE: u32 = 16000;
    let chunk_size = (SAMPLE_RATE * chunk_duration_ms / 1000) as usize;

    samples
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pcm_samples_short() {
        let samples = extract_pcm_samples(test_audio::SHORT);

        // 3 seconds * 16000Hz = 48000 samples
        assert_eq!(
            samples.len(),
            48000,
            "Short audio should have 48000 samples"
        );

        // Check that samples are not all zeros (actual audio data)
        let non_zero_count = samples.iter().filter(|&&s| s != 0).count();
        assert!(
            non_zero_count > 10000,
            "Short audio should have significant non-zero samples"
        );
    }

    #[test]
    fn test_extract_pcm_samples_long() {
        let samples = extract_pcm_samples(test_audio::LONG);

        // 10 seconds * 16000Hz = 160000 samples
        assert_eq!(
            samples.len(),
            160000,
            "Long audio should have 160000 samples"
        );
    }

    #[test]
    fn test_extract_pcm_samples_silence() {
        let samples = extract_pcm_samples(test_audio::SILENCE);

        // 2 seconds * 16000Hz = 32000 samples
        assert_eq!(
            samples.len(),
            32000,
            "Silence audio should have 32000 samples"
        );

        // Check that most samples are zero (silence)
        let zero_count = samples.iter().filter(|&&s| s == 0).count();
        assert!(zero_count > 30000, "Silence audio should be mostly zeros");
    }

    #[test]
    fn test_pcm_samples_to_bytes_roundtrip() {
        let original_samples: Vec<i16> = vec![100, -200, 300, -400, 500];
        let bytes = pcm_samples_to_bytes(&original_samples);
        assert_eq!(bytes.len(), 10, "Should have 2 bytes per sample");

        // Roundtrip conversion
        let recovered_samples: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        assert_eq!(
            original_samples, recovered_samples,
            "Roundtrip conversion should preserve samples"
        );
    }

    #[test]
    fn test_chunk_pcm_samples_20ms() {
        let samples: Vec<i16> = (0..16000).map(|i| i as i16).collect(); // 1 second
        let chunks = chunk_pcm_samples(&samples, 20); // 20ms chunks

        // 1 second / 20ms = 50 chunks
        assert_eq!(chunks.len(), 50, "Should have 50 chunks of 20ms");

        // Each chunk should have 320 samples (16000Hz * 0.02s)
        assert_eq!(
            chunks[0].len(),
            320,
            "Each 20ms chunk should have 320 samples"
        );
    }

    #[test]
    fn test_chunk_pcm_samples_partial_last_chunk() {
        let samples: Vec<i16> = (0..16100).map(|i| i as i16).collect(); // 1.00625 seconds
        let chunks = chunk_pcm_samples(&samples, 20);

        // Last chunk should have remaining samples (100 samples)
        assert_eq!(
            chunks.last().unwrap().len(),
            100,
            "Last chunk should have 100 remaining samples"
        );
    }
}

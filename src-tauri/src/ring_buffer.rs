// Ring Buffer for Audio Streaming
// ADR-013: Sidecar Full-Duplex IPC Final Design
// Phase 2: Ring Buffer Integration
// Updated: Drop-oldest strategy for real-time priority

use ringbuf::{traits::*, HeapRb};
use std::sync::Arc;

/// Buffer capacity constants (ADR-013)
pub const SAMPLE_RATE: usize = 16000; // 16 kHz
pub const CHANNELS: usize = 1; // mono
pub const BYTES_PER_SAMPLE: usize = 2; // 16-bit PCM
pub const BUFFER_SECS: usize = 5; // 5 seconds

/// Total buffer capacity: 160,000 bytes = 156 KB
pub const BUFFER_CAPACITY: usize = SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE * BUFFER_SECS;

/// Buffer occupancy level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferLevel {
    /// Normal operation (0-50%)
    Normal,
    /// Warning level (50-70%)
    Warn,
    /// Critical level (70-100%)
    Critical,
}

impl BufferLevel {
    /// Determine buffer level from occupancy ratio
    pub fn from_occupancy(occupancy: f32) -> Self {
        match occupancy {
            x if x <= 0.5 => BufferLevel::Normal,
            x if x <= 0.7 => BufferLevel::Warn,
            _ => BufferLevel::Critical,
        }
    }
}

/// Shared audio ring buffer with drop-oldest strategy
///
/// This buffer sits between:
/// - Producer: CPAL audio callback (real-time, <10μs constraint)
/// - Consumer: Sidecar writer task (sends to Python via stdin)
///
/// The buffer provides:
/// - 5-second capacity (160 KB)
/// - Drop-oldest strategy: when full, old data is discarded to make room for new
/// - Real-time priority: latest audio is always preserved
pub type SharedRingBuffer = Arc<std::sync::Mutex<HeapRb<u8>>>;

/// Create a new shared ring buffer
pub fn new_shared_ring_buffer() -> SharedRingBuffer {
    Arc::new(std::sync::Mutex::new(HeapRb::new(BUFFER_CAPACITY)))
}

/// Push audio data with drop-oldest strategy
///
/// If buffer doesn't have enough space, drops oldest data to make room.
/// Returns (bytes_pushed, bytes_dropped, buffer_level)
///
/// # Real-time Safety
///
/// This function should be called with try_lock() in audio callbacks
/// to avoid blocking. If lock cannot be acquired, skip the frame.
pub fn push_audio_drop_oldest(
    rb: &mut HeapRb<u8>,
    data: &[u8],
) -> (usize, usize, BufferLevel) {
    let vacant = rb.vacant_len();
    let mut dropped = 0;

    // Drop oldest data if not enough space
    if vacant < data.len() {
        let need_to_drop = data.len() - vacant;
        dropped = rb.skip(need_to_drop);
    }

    // Now push the new data
    let pushed = rb.push_slice(data);

    // Calculate occupancy
    let occupancy = rb.occupied_len() as f32 / BUFFER_CAPACITY as f32;
    let level = BufferLevel::from_occupancy(occupancy);

    (pushed, dropped, level)
}

/// Pop audio data for sender task
///
/// Returns number of bytes read
pub fn pop_audio(rb: &mut HeapRb<u8>, buf: &mut [u8]) -> usize {
    rb.pop_slice(buf)
}

/// Get current occupancy ratio (0.0 to 1.0)
pub fn occupancy(rb: &HeapRb<u8>) -> f32 {
    rb.occupied_len() as f32 / BUFFER_CAPACITY as f32
}

/// Convert f32 PCM samples to 16-bit PCM bytes
///
/// Input: f32 samples in range [-1.0, 1.0]
/// Output: i16 little-endian bytes
pub fn pcm_f32_to_i16_bytes(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            let quantized = (clamped * 32767.0) as i16;
            quantized.to_le_bytes()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_level_from_occupancy() {
        assert_eq!(BufferLevel::from_occupancy(0.0), BufferLevel::Normal);
        assert_eq!(BufferLevel::from_occupancy(0.3), BufferLevel::Normal);
        assert_eq!(BufferLevel::from_occupancy(0.5), BufferLevel::Normal);
        assert_eq!(BufferLevel::from_occupancy(0.6), BufferLevel::Warn);
        assert_eq!(BufferLevel::from_occupancy(0.7), BufferLevel::Warn);
        assert_eq!(BufferLevel::from_occupancy(0.8), BufferLevel::Critical);
        assert_eq!(BufferLevel::from_occupancy(1.0), BufferLevel::Critical);
    }

    #[test]
    fn test_buffer_capacity() {
        // 16 kHz * 1 channel * 2 bytes * 5 seconds = 160,000 bytes
        assert_eq!(BUFFER_CAPACITY, 160_000);
    }

    #[test]
    fn test_shared_ring_buffer_creation() {
        let rb = new_shared_ring_buffer();
        let guard = rb.lock().unwrap();
        assert_eq!(occupancy(&guard), 0.0);
    }

    #[test]
    fn test_push_pop_basic() {
        let rb = new_shared_ring_buffer();
        let mut guard = rb.lock().unwrap();

        // Push 320 bytes (10ms frame)
        let data = vec![42u8; 320];
        let (pushed, dropped, level) = push_audio_drop_oldest(&mut guard, &data);

        assert_eq!(pushed, 320);
        assert_eq!(dropped, 0);
        assert_eq!(level, BufferLevel::Normal);

        // Pop 320 bytes
        let mut buf = vec![0u8; 320];
        let popped = pop_audio(&mut guard, &mut buf);

        assert_eq!(popped, 320);
        assert_eq!(buf, data);
    }

    #[test]
    fn test_drop_oldest_on_overflow() {
        let rb = new_shared_ring_buffer();
        let mut guard = rb.lock().unwrap();

        // Fill buffer to 100% (160,000 bytes)
        let chunk = vec![1u8; 32000]; // 32 KB chunks
        for _ in 0..5 {
            push_audio_drop_oldest(&mut guard, &chunk);
        }

        // Buffer is now full
        assert!(occupancy(&guard) > 0.99);

        // Push more data - should drop oldest
        let new_data = vec![2u8; 1000];
        let (pushed, dropped, _level) = push_audio_drop_oldest(&mut guard, &new_data);

        assert_eq!(pushed, 1000);
        assert_eq!(dropped, 1000); // Dropped 1000 bytes of old data

        // Verify new data is at the end
        // Pop all data and check last 1000 bytes are 2s
        let mut all_data = vec![0u8; BUFFER_CAPACITY];
        let total = pop_audio(&mut guard, &mut all_data);

        // Last 1000 bytes should be 2s (our new data)
        let last_bytes = &all_data[total - 1000..total];
        assert!(last_bytes.iter().all(|&b| b == 2));
    }

    #[test]
    fn test_pcm_f32_to_i16_conversion() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let bytes = pcm_f32_to_i16_bytes(&samples);

        // 5 samples * 2 bytes = 10 bytes
        assert_eq!(bytes.len(), 10);

        // Check first sample (0.0 -> 0i16)
        let val0 = i16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(val0, 0);

        // Check second sample (0.5 -> 16383i16)
        let val1 = i16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(val1, 16383);

        // Check third sample (-0.5 -> -16383i16 or -16384i16, both acceptable)
        let val2 = i16::from_le_bytes([bytes[4], bytes[5]]);
        assert!((val2 == -16383) || (val2 == -16384));
    }
}

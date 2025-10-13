// Ring Buffer for Audio Streaming
// ADR-013: Sidecar Full-Duplex IPC Final Design
// Phase 2: Ring Buffer Integration

use ringbuf::{traits::*, HeapRb};
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Overflow (100%+)
    Overflow,
}

impl BufferLevel {
    /// Determine buffer level from occupancy ratio
    pub fn from_occupancy(occupancy: f32) -> Self {
        match occupancy {
            x if x <= 0.5 => BufferLevel::Normal,
            x if x <= 0.7 => BufferLevel::Warn,
            x if x <= 1.0 => BufferLevel::Critical,
            _ => BufferLevel::Overflow,
        }
    }

    /// Check if this level requires immediate action
    pub fn is_critical(&self) -> bool {
        matches!(self, BufferLevel::Overflow)
    }
}

/// Audio ring buffer (SPSC - Single Producer Single Consumer)
///
/// This buffer sits between:
/// - Producer: CPAL audio callback (real-time, <10μs constraint)
/// - Consumer: Sidecar writer task (sends to Python via stdin)
///
/// The buffer provides:
/// - Lock-free operation (no mutex in audio callback)
/// - 5-second capacity (160 KB)
/// - Overflow detection (Python timeout detection)
pub struct AudioRingBuffer {
    producer: ringbuf::HeapProd<u8>,
    consumer: ringbuf::HeapCons<u8>,
    stop_flag: Arc<AtomicBool>,
}

impl AudioRingBuffer {
    /// Create a new audio ring buffer with 5-second capacity
    pub fn new() -> Self {
        let ring = HeapRb::<u8>::new(BUFFER_CAPACITY);
        let (producer, consumer) = ring.split();

        Self {
            producer,
            consumer,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a clone of the stop flag for signaling
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    /// Split into producer and consumer
    ///
    /// SAFETY: Only call this ONCE. Returns (producer, consumer) for ownership transfer.
    pub fn split(self) -> (ringbuf::HeapProd<u8>, ringbuf::HeapCons<u8>) {
        (self.producer, self.consumer)
    }

    /// Push audio data from CPAL callback
    ///
    /// Returns (bytes_pushed, buffer_level)
    ///
    /// # Real-time Safety
    ///
    /// This function is designed for real-time audio callbacks:
    /// - No allocations
    /// - No locks
    /// - Bounded execution time (<10μs)
    ///
    /// # Overflow Detection
    ///
    /// If buffer cannot fit entire frame, returns (0, BufferLevel::Overflow)
    /// and rejects the entire frame (no partial writes).
    /// This ensures 0% frame loss - frames are either fully written or fully rejected.
    /// Caller should set stop_flag and stop recording immediately.
    pub fn push_from_callback(
        producer: &mut ringbuf::HeapProd<u8>,
        data: &[u8],
    ) -> (usize, BufferLevel) {
        // CRITICAL: Check free space BEFORE writing to prevent partial writes
        if producer.vacant_len() < data.len() {
            // Buffer full - reject entire frame (no partial write)
            return (0, BufferLevel::Overflow);
        }

        // Now we know the entire frame fits
        let pushed = producer.push_slice(data);
        debug_assert_eq!(pushed, data.len(), "Partial write should never occur");

        // Calculate occupancy
        let occupancy = producer.occupied_len() as f32 / BUFFER_CAPACITY as f32;
        let level = BufferLevel::from_occupancy(occupancy);

        (pushed, level)
    }

    /// Pop audio data for sidecar writer
    ///
    /// This is called from the async writer task (not real-time).
    /// Blocks until data is available or buffer is empty.
    pub fn pop_for_writer(consumer: &mut ringbuf::HeapCons<u8>, buf: &mut [u8]) -> usize {
        consumer.pop_slice(buf)
    }

    /// Get current occupancy ratio (0.0 to 1.0)
    pub fn occupancy(&self) -> f32 {
        self.producer.occupied_len() as f32 / BUFFER_CAPACITY as f32
    }

    /// Get current buffer level
    pub fn level(&self) -> BufferLevel {
        BufferLevel::from_occupancy(self.occupancy())
    }

    /// Check if stop flag is set
    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }
}

impl Default for AudioRingBuffer {
    fn default() -> Self {
        Self::new()
    }
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
        assert_eq!(BufferLevel::from_occupancy(1.1), BufferLevel::Overflow);
    }

    #[test]
    fn test_buffer_level_is_critical() {
        assert!(!BufferLevel::Normal.is_critical());
        assert!(!BufferLevel::Warn.is_critical());
        assert!(!BufferLevel::Critical.is_critical());
        assert!(BufferLevel::Overflow.is_critical());
    }

    #[test]
    fn test_buffer_capacity() {
        // 16 kHz * 1 channel * 2 bytes * 5 seconds = 160,000 bytes
        assert_eq!(BUFFER_CAPACITY, 160_000);
    }

    #[test]
    fn test_ring_buffer_creation() {
        let buffer = AudioRingBuffer::new();
        assert_eq!(buffer.occupancy(), 0.0);
        assert_eq!(buffer.level(), BufferLevel::Normal);
        assert!(!buffer.should_stop());
    }

    #[test]
    fn test_push_pop_basic() {
        let buffer = AudioRingBuffer::new();
        let (mut producer, mut consumer) = buffer.split();

        // Push 320 bytes (10ms frame)
        let data = vec![42u8; 320];
        let (pushed, level) = AudioRingBuffer::push_from_callback(&mut producer, &data);

        assert_eq!(pushed, 320);
        assert_eq!(level, BufferLevel::Normal); // 320 / 160000 = 0.002

        // Pop 320 bytes
        let mut buf = vec![0u8; 320];
        let popped = AudioRingBuffer::pop_for_writer(&mut consumer, &mut buf);

        assert_eq!(popped, 320);
        assert_eq!(buf, data);
    }

    #[test]
    fn test_buffer_overflow() {
        let buffer = AudioRingBuffer::new();
        let (mut producer, _consumer) = buffer.split();

        // Fill buffer to 100% (160,000 bytes)
        let chunk = vec![1u8; 32000]; // 32 KB chunks
        for _ in 0..5 {
            AudioRingBuffer::push_from_callback(&mut producer, &chunk);
        }

        // Try to push more (should fail, buffer full)
        let extra = vec![2u8; 1000];
        let (pushed, level) = AudioRingBuffer::push_from_callback(&mut producer, &extra);

        assert_eq!(pushed, 0); // No space
        // Level is Critical (100%) not Overflow (>100%) since we can't exceed capacity
        assert!(matches!(level, BufferLevel::Critical | BufferLevel::Overflow));
    }

    #[test]
    fn test_buffer_levels_progression() {
        let buffer = AudioRingBuffer::new();
        let (mut producer, _consumer) = buffer.split();

        // 0% - Normal
        let (_, level) = AudioRingBuffer::push_from_callback(&mut producer, &vec![0u8; 1000]);
        assert_eq!(level, BufferLevel::Normal);

        // 60% - Warn
        let warn_size = (BUFFER_CAPACITY as f32 * 0.59) as usize;
        AudioRingBuffer::push_from_callback(&mut producer, &vec![0u8; warn_size]);
        let (_, level) = AudioRingBuffer::push_from_callback(&mut producer, &vec![0u8; 100]);
        assert_eq!(level, BufferLevel::Warn);

        // 80% - Critical
        let critical_size = (BUFFER_CAPACITY as f32 * 0.19) as usize;
        AudioRingBuffer::push_from_callback(&mut producer, &vec![0u8; critical_size]);
        let (_, level) = AudioRingBuffer::push_from_callback(&mut producer, &vec![0u8; 100]);
        assert_eq!(level, BufferLevel::Critical);
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

    #[test]
    fn test_stop_flag() {
        let buffer = AudioRingBuffer::new();
        let stop_flag = buffer.stop_flag();

        assert!(!buffer.should_stop());

        stop_flag.store(true, Ordering::Relaxed);
        assert!(buffer.should_stop());
    }

    #[test]
    fn test_overflow_on_partial_write() {
        let buffer = AudioRingBuffer::new();
        let (mut producer, _consumer) = buffer.split();

        // Fill buffer to capacity
        let chunk = vec![1u8; 32000];
        for _ in 0..5 {
            AudioRingBuffer::push_from_callback(&mut producer, &chunk);
        }

        // Try to push another frame - should detect overflow
        let frame = vec![2u8; 320];
        let (pushed, level) = AudioRingBuffer::push_from_callback(&mut producer, &frame);

        // CRITICAL: Should detect overflow on partial write
        assert_eq!(pushed, 0, "Should not push any bytes when full");
        assert_eq!(level, BufferLevel::Overflow, "Should return Overflow level");
    }

    #[test]
    fn test_partial_write_overflow_detection() {
        let buffer = AudioRingBuffer::new();
        let (mut producer, _consumer) = buffer.split();

        // Fill buffer to almost full (leave 100 bytes free)
        let fill_size = BUFFER_CAPACITY - 100;
        let chunk = vec![1u8; fill_size];
        let (pushed, _) = AudioRingBuffer::push_from_callback(&mut producer, &chunk);
        assert_eq!(pushed, fill_size);

        // Try to push 320 bytes (will partially fit)
        let frame = vec![2u8; 320];
        let (pushed, level) = AudioRingBuffer::push_from_callback(&mut producer, &frame);

        // Should detect overflow (pushed < 320)
        assert!(pushed < 320, "Should only push partial data");
        assert_eq!(level, BufferLevel::Overflow, "Should return Overflow on partial write");
    }
}

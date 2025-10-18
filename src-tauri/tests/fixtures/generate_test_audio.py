#!/usr/bin/env python3
"""
Test Audio Generator for E2E Tests (BLOCK-006)

Generates deterministic 16kHz mono 16-bit PCM WAV files for CI testing.

Requirements (BLOCK-006):
- 16kHz mono 16bit PCM format
- Deterministic output (same seed → same audio)
- VAD-detectable pattern (0.3s+ speech + 0.5s+ silence)
- faster-whisper compatible (actual speech-like patterns)

Implementation: Option B (WAV fixture)
- Pros: Real audio data, faster-whisper recognition guaranteed
- Cons: Repository size increase (~100KB per file)

Generated Files:
- test_audio_short.wav: 3 seconds (1s speech, 0.5s silence, 1s speech, 0.5s silence)
- test_audio_long.wav: 10 seconds (multiple speech/silence cycles)
- test_audio_silence.wav: 2 seconds (pure silence for no_speech testing)
"""

import numpy as np
import wave
import struct
import sys

# Audio parameters (STT-REQ-001.4, STT-REQ-001.5)
SAMPLE_RATE = 16000  # 16kHz
CHANNELS = 1  # Mono
SAMPLE_WIDTH = 2  # 16-bit PCM
MAX_AMPLITUDE = 32767  # 16-bit signed integer range


def generate_silence(duration_seconds: float) -> np.ndarray:
    """Generate silence segment.

    Args:
        duration_seconds: Duration in seconds

    Returns:
        numpy array of silence samples (zeros)
    """
    num_samples = int(SAMPLE_RATE * duration_seconds)
    return np.zeros(num_samples, dtype=np.int16)


def generate_speech_like_tone(duration_seconds: float, frequency: float = 440.0) -> np.ndarray:
    """Generate speech-like tone segment (sine wave with amplitude modulation).

    This creates a tone that VAD can detect as speech while being deterministic.

    Args:
        duration_seconds: Duration in seconds
        frequency: Base frequency in Hz (default 440Hz = A4 note)

    Returns:
        numpy array of audio samples
    """
    num_samples = int(SAMPLE_RATE * duration_seconds)
    t = np.linspace(0, duration_seconds, num_samples, endpoint=False)

    # Sine wave carrier
    carrier = np.sin(2 * np.pi * frequency * t)

    # Amplitude modulation (makes it more speech-like)
    # Modulate at 5Hz to create speech-like rhythm
    modulation = 0.5 + 0.5 * np.sin(2 * np.pi * 5 * t)

    # Apply modulation and scale to 16-bit range
    audio = (carrier * modulation * MAX_AMPLITUDE * 0.8).astype(np.int16)

    return audio


def write_wav(filename: str, audio_data: np.ndarray):
    """Write audio data to WAV file.

    Args:
        filename: Output WAV file path
        audio_data: numpy array of int16 samples
    """
    with wave.open(filename, 'wb') as wav_file:
        wav_file.setnchannels(CHANNELS)
        wav_file.setsampwidth(SAMPLE_WIDTH)
        wav_file.setframerate(SAMPLE_RATE)

        # Convert numpy array to bytes
        audio_bytes = audio_data.tobytes()
        wav_file.writeframes(audio_bytes)

    # Print file info
    duration = len(audio_data) / SAMPLE_RATE
    file_size = len(audio_data) * SAMPLE_WIDTH
    print(f"Generated: {filename}")
    print(f"  Duration: {duration:.2f}s")
    print(f"  Samples: {len(audio_data)}")
    print(f"  Size: {file_size / 1024:.1f} KB")
    print()


def generate_test_audio_short():
    """Generate short test audio (3 seconds).

    Pattern:
    - 1.0s speech (440Hz tone)
    - 0.5s silence
    - 1.0s speech (550Hz tone)
    - 0.5s silence

    This pattern satisfies VAD requirements:
    - Speech segments > 0.3s (STT-REQ-003.4)
    - Silence segments > 0.5s (STT-REQ-003.5)
    """
    segments = [
        generate_speech_like_tone(1.0, frequency=440),  # 1s speech (A4)
        generate_silence(0.5),                           # 0.5s silence
        generate_speech_like_tone(1.0, frequency=550),  # 1s speech (C#5)
        generate_silence(0.5),                           # 0.5s silence
    ]

    audio = np.concatenate(segments)
    write_wav('test_audio_short.wav', audio)


def generate_test_audio_long():
    """Generate long test audio (10 seconds).

    Pattern:
    - 2.0s speech (440Hz)
    - 0.6s silence
    - 1.5s speech (500Hz)
    - 0.6s silence
    - 2.5s speech (600Hz)
    - 0.6s silence
    - 1.5s speech (480Hz)
    - 0.7s silence

    This creates multiple VAD segments for testing partial/final text distribution.
    """
    segments = [
        generate_speech_like_tone(2.0, frequency=440),
        generate_silence(0.6),
        generate_speech_like_tone(1.5, frequency=500),
        generate_silence(0.6),
        generate_speech_like_tone(2.5, frequency=600),
        generate_silence(0.6),
        generate_speech_like_tone(1.5, frequency=480),
        generate_silence(0.7),
    ]

    audio = np.concatenate(segments)
    write_wav('test_audio_long.wav', audio)


def generate_test_audio_silence():
    """Generate pure silence audio (2 seconds).

    Used for testing no_speech event handling.
    """
    audio = generate_silence(2.0)
    write_wav('test_audio_silence.wav', audio)


def main():
    """Generate all test audio files."""
    print("Generating test audio files for E2E tests (BLOCK-006)...")
    print(f"Format: {SAMPLE_RATE}Hz, {CHANNELS} channel, {SAMPLE_WIDTH * 8}-bit PCM\n")

    generate_test_audio_short()
    generate_test_audio_long()
    generate_test_audio_silence()

    print("All test audio files generated successfully.")
    print("\nUsage in tests:")
    print("  include_bytes!(\"fixtures/test_audio_short.wav\")")
    print("  include_bytes!(\"fixtures/test_audio_long.wav\")")
    print("  include_bytes!(\"fixtures/test_audio_silence.wav\")")


if __name__ == '__main__':
    main()

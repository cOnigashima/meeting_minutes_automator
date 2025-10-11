"""
Voice Activity Detector using webrtcvad.

This module provides voice activity detection (VAD) functionality
using the webrtcvad library to identify speech segments in audio data.

Requirements:
- STT-REQ-003.1: Initialize webrtcvad with aggressiveness=2
- STT-REQ-003.2: Split audio data into 10ms frames
"""

import webrtcvad
import logging
from typing import List

logger = logging.getLogger(__name__)


class VoiceActivityDetector:
    """
    Voice Activity Detector using webrtcvad.

    Detects speech activity in audio data by splitting it into 10ms frames
    and using webrtcvad to determine if each frame contains speech.
    """

    def __init__(self, sample_rate: int = 16000, aggressiveness: int = 2):
        """
        Initialize VoiceActivityDetector.

        Args:
            sample_rate: Audio sample rate in Hz (default: 16000)
            aggressiveness: VAD aggressiveness mode 0-3 (default: 2 = medium)
                           0: Most permissive (more false positives)
                           3: Most aggressive (more false negatives)

        Requirements:
            STT-REQ-003.1: Initialize webrtcvad with aggressiveness=2
        """
        self.sample_rate = sample_rate
        self.aggressiveness = aggressiveness
        self.frame_duration_ms = 10  # webrtcvad requires 10, 20, or 30 ms frames

        # Initialize webrtcvad (STT-REQ-003.1)
        self.vad = webrtcvad.Vad()
        self.vad.set_mode(aggressiveness)

        logger.info(f"VoiceActivityDetector initialized: sample_rate={sample_rate}Hz, aggressiveness={aggressiveness}")

    def split_into_frames(self, audio_data: bytes) -> List[bytes]:
        """
        Split audio data into 10ms frames.

        Args:
            audio_data: Raw audio data in 16-bit PCM format

        Returns:
            List of audio frames (each 10ms long)

        Requirements:
            STT-REQ-003.2: Split audio data into 10ms frames
        """
        if not audio_data or len(audio_data) == 0:
            return []

        # Calculate frame size in bytes
        # 10ms at 16kHz = 160 samples
        # 160 samples * 2 bytes per sample (16-bit) = 320 bytes
        samples_per_frame = int(self.sample_rate * self.frame_duration_ms / 1000)
        bytes_per_frame = samples_per_frame * 2  # 2 bytes per sample (16-bit)

        frames = []
        offset = 0

        while offset + bytes_per_frame <= len(audio_data):
            frame = audio_data[offset:offset + bytes_per_frame]
            frames.append(frame)
            offset += bytes_per_frame

        # Discard partial frames (incomplete final frame)
        return frames

    def is_speech(self, frame: bytes) -> bool:
        """
        Determine if a frame contains speech.

        Args:
            frame: Audio frame (10ms of 16-bit PCM audio)

        Returns:
            True if frame contains speech, False otherwise

        Requirements:
            STT-REQ-003.3: Use webrtcvad for frame-by-frame speech detection
        """
        try:
            return self.vad.is_speech(frame, self.sample_rate)
        except Exception as e:
            logger.error(f"VAD is_speech error: {e}")
            return False

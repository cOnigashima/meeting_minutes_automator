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
from typing import List, Optional, Dict, Any

logger = logging.getLogger(__name__)


class VoiceActivityDetector:
    """
    Voice Activity Detector using webrtcvad.

    Detects speech activity in audio data by splitting it into 10ms frames
    and using webrtcvad to determine if each frame contains speech.
    """

    def __init__(
        self,
        sample_rate: int = 16000,
        aggressiveness: int = 2
    ):
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

        # State management for speech onset/offset detection (STT-REQ-003.4, STT-REQ-003.5)
        self.is_in_speech = False
        self.speech_frames = 0  # Consecutive speech frames counter
        self.silence_frames = 0  # Consecutive silence frames counter
        self.current_segment = []  # Accumulated audio frames for current segment

        # Thresholds (STT-REQ-003.4, STT-REQ-003.5)
        self.speech_onset_threshold = 30  # 0.3 seconds = 30 frames
        self.speech_offset_threshold = 50  # 0.5 seconds = 50 frames

        logger.info(
            f"VoiceActivityDetector initialized: sample_rate={sample_rate}Hz, "
            f"aggressiveness={aggressiveness}"
        )

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

    def process_frame(self, frame: bytes) -> dict:
        """
        Process a single audio frame and detect speech onset/offset.

        Args:
            frame: Audio frame (10ms of 16-bit PCM audio)

        Returns:
            Dictionary with event information, or None if no event occurred:
            - {'event': 'speech_start'} when speech onset is detected
            - {'event': 'speech_end', 'segment': {...}} when speech offset is detected

        Requirements:
            STT-REQ-003.3: Frame-by-frame speech detection
            STT-REQ-003.4: Speech onset detection (0.3s continuous speech)
            STT-REQ-003.5: Speech offset detection (0.5s continuous silence)
        """
        # Determine if frame contains speech (STT-REQ-003.3)
        is_speech_frame = self.is_speech(frame)

        # Always accumulate frames during active speech
        if self.is_in_speech:
            self.current_segment.append(frame)

        # Handle speech onset detection (STT-REQ-003.4)
        if not self.is_in_speech:
            if is_speech_frame:
                # Increment speech counter
                self.speech_frames += 1
                self.silence_frames = 0  # Reset silence counter

                # Check if speech onset threshold reached (30 frames = 0.3s)
                if self.speech_frames >= self.speech_onset_threshold:
                    self.is_in_speech = True
                    self.current_segment = []  # Start new segment
                    self.current_segment.append(frame)
                    logger.info("Speech onset detected")
                    return {'event': 'speech_start'}
            else:
                # Reset speech counter on silence
                self.speech_frames = 0

        # Handle speech offset detection (STT-REQ-003.5)
        else:  # self.is_in_speech == True
            if not is_speech_frame:
                # Increment silence counter
                self.silence_frames += 1
                self.speech_frames = 0  # Reset speech counter

                # Check if speech offset threshold reached (50 frames = 0.5s)
                if self.silence_frames >= self.speech_offset_threshold:
                    # Finalize segment
                    segment_audio = b''.join(self.current_segment)
                    duration_ms = len(self.current_segment) * self.frame_duration_ms

                    logger.info(f"Speech offset detected: segment duration={duration_ms}ms")

                    # Reset state
                    self.is_in_speech = False
                    self.silence_frames = 0
                    self.speech_frames = 0
                    self.current_segment = []

                    return {
                        'event': 'speech_end',
                        'segment': {
                            'audio_data': segment_audio,
                            'duration_ms': duration_ms
                        }
                    }
            else:
                # Reset silence counter on new speech
                self.silence_frames = 0
                self.speech_frames += 1

        return None

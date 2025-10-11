"""
Unit tests for VoiceActivityDetector (STT-REQ-003.1-003.2).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock


class TestVoiceActivityDetectorInitialization:
    """Test VoiceActivityDetector initialization (STT-REQ-003.1)."""

    def test_vad_initialization_with_default_aggressiveness(self):
        """WHEN VoiceActivityDetector is initialized
        THEN should create webrtcvad instance with aggressiveness=2."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            # Should create Vad instance
            mock_vad_class.assert_called_once()

            # Should set aggressiveness to 2 (medium)
            mock_vad_instance.set_mode.assert_called_once_with(2)

    def test_vad_initialization_with_custom_aggressiveness(self):
        """WHEN VoiceActivityDetector is initialized with custom aggressiveness
        THEN should use the specified value."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector(aggressiveness=3)

            mock_vad_instance.set_mode.assert_called_once_with(3)

    def test_vad_sample_rate_default(self):
        """WHEN VoiceActivityDetector is initialized
        THEN should use default sample rate of 16000 Hz."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):
            detector = VoiceActivityDetector()

            assert detector.sample_rate == 16000


class TestFrameSplitting:
    """Test audio frame splitting (STT-REQ-003.2)."""

    def test_split_audio_into_10ms_frames(self):
        """WHEN audio data is provided
        THEN should split into 10ms frames (160 samples at 16kHz)."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):
            detector = VoiceActivityDetector()

            # Create 1 second of audio (16000 samples at 16kHz)
            sample_rate = 16000
            duration = 1.0
            audio_samples = np.random.randint(-32768, 32767, int(sample_rate * duration), dtype=np.int16)
            audio_bytes = audio_samples.tobytes()

            frames = detector.split_into_frames(audio_bytes)

            # 1 second = 1000ms, each frame is 10ms -> 100 frames
            expected_frame_count = 100
            assert len(frames) == expected_frame_count

            # Each frame should be 160 samples * 2 bytes = 320 bytes
            expected_frame_size = 320
            for frame in frames:
                assert len(frame) == expected_frame_size

    def test_split_audio_with_partial_frame(self):
        """WHEN audio data has incomplete final frame
        THEN should discard partial frame."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):
            detector = VoiceActivityDetector()

            # Create 1.5 frames (240 samples = 480 bytes)
            # Only 1 complete frame should be returned
            audio_samples = np.random.randint(-32768, 32767, 240, dtype=np.int16)
            audio_bytes = audio_samples.tobytes()

            frames = detector.split_into_frames(audio_bytes)

            # Should only have 1 complete frame
            assert len(frames) == 1
            assert len(frames[0]) == 320

    def test_split_audio_empty_data(self):
        """WHEN empty audio data is provided
        THEN should return empty list."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):
            detector = VoiceActivityDetector()

            frames = detector.split_into_frames(b"")

            assert len(frames) == 0

    def test_frame_duration_calculation(self):
        """WHEN calculating frame duration
        THEN should correctly compute 10ms for 160 samples at 16kHz."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):
            detector = VoiceActivityDetector()

            # Frame duration should be 10ms
            assert detector.frame_duration_ms == 10

            # Frame size should be 160 samples (16kHz * 0.01s)
            # 160 samples * 2 bytes = 320 bytes
            expected_frame_size = int(detector.sample_rate * detector.frame_duration_ms / 1000) * 2
            assert expected_frame_size == 320


class TestVADIntegration:
    """Test webrtcvad integration."""

    def test_vad_is_speech_returns_boolean(self):
        """WHEN checking if frame contains speech
        THEN should return boolean value."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Mock is_speech to return True
            mock_vad_instance.is_speech.return_value = True

            detector = VoiceActivityDetector()

            # Create a 10ms frame (160 samples = 320 bytes)
            frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            result = detector.is_speech(frame)

            assert isinstance(result, bool)
            assert result is True

            # Verify is_speech was called with correct parameters
            mock_vad_instance.is_speech.assert_called_once_with(frame, 16000)

    def test_vad_is_speech_with_silence_frame(self):
        """WHEN checking a silent frame
        THEN should return False."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Mock is_speech to return False for silence
            mock_vad_instance.is_speech.return_value = False

            detector = VoiceActivityDetector()

            # Create a silent frame (all zeros)
            frame = np.zeros(160, dtype=np.int16).tobytes()

            result = detector.is_speech(frame)

            assert result is False


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

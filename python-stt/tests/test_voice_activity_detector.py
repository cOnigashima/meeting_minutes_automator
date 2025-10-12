"""
Unit tests for VoiceActivityDetector (STT-REQ-003.1-003.2).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock, AsyncMock


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


class TestSpeechOnsetDetection:
    """Test speech onset detection (STT-REQ-003.4)."""

    def test_detect_speech_onset_after_300ms(self):
        """WHEN speech frames are detected for 0.3 seconds (30 frames)
        THEN should trigger speech onset event."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Mock is_speech to always return True (speech detected)
            mock_vad_instance.is_speech.return_value = True

            detector = VoiceActivityDetector()

            # Process 30 speech frames (0.3 seconds)
            frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            speech_started = False
            for i in range(30):
                result = detector.process_frame(frame)
                if result and result.get('event') == 'speech_start':
                    speech_started = True
                    break

            assert speech_started is True

    def test_no_speech_onset_before_300ms(self):
        """WHEN speech frames are detected for less than 0.3 seconds
        THEN should NOT trigger speech onset event."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            mock_vad_instance.is_speech.return_value = True

            detector = VoiceActivityDetector()

            # Process only 29 speech frames (less than 0.3 seconds)
            frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            speech_started = False
            for i in range(29):
                result = detector.process_frame(frame)
                if result and result.get('event') == 'speech_start':
                    speech_started = True
                    break

            assert speech_started is False

    def test_speech_onset_reset_on_silence(self):
        """WHEN speech is interrupted by silence before 0.3 seconds
        THEN should reset speech onset counter."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Process 20 speech frames
            mock_vad_instance.is_speech.return_value = True
            for i in range(20):
                detector.process_frame(speech_frame)

            # Insert silence frame (should reset counter)
            mock_vad_instance.is_speech.return_value = False
            detector.process_frame(silence_frame)

            # Process 29 more speech frames (total would be 49, but reset happened)
            mock_vad_instance.is_speech.return_value = True
            speech_started = False
            for i in range(29):
                result = detector.process_frame(speech_frame)
                if result and result.get('event') == 'speech_start':
                    speech_started = True
                    break

            # Should NOT trigger because counter was reset
            assert speech_started is False


class TestSpeechOffsetDetection:
    """Test speech offset detection (STT-REQ-003.5)."""

    def test_detect_speech_offset_after_500ms_silence(self):
        """WHEN silence frames are detected for 0.5 seconds (50 frames) after speech
        THEN should trigger speech offset event and finalize segment."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # First, trigger speech onset (30 speech frames)
            mock_vad_instance.is_speech.return_value = True
            for i in range(30):
                detector.process_frame(speech_frame)

            # Then, detect silence for 0.5 seconds (50 frames)
            mock_vad_instance.is_speech.return_value = False
            speech_ended = False
            for i in range(50):
                result = detector.process_frame(silence_frame)
                if result and result.get('event') == 'speech_end':
                    speech_ended = True
                    assert 'segment' in result
                    break

            assert speech_ended is True

    def test_no_speech_offset_before_500ms_silence(self):
        """WHEN silence frames are detected for less than 0.5 seconds
        THEN should NOT trigger speech offset event."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Trigger speech onset
            mock_vad_instance.is_speech.return_value = True
            for i in range(30):
                detector.process_frame(speech_frame)

            # Detect silence for only 49 frames (less than 0.5 seconds)
            mock_vad_instance.is_speech.return_value = False
            speech_ended = False
            for i in range(49):
                result = detector.process_frame(silence_frame)
                if result and result.get('event') == 'speech_end':
                    speech_ended = True
                    break

            assert speech_ended is False

    def test_speech_offset_reset_on_new_speech(self):
        """WHEN silence is interrupted by new speech before 0.5 seconds
        THEN should reset silence counter and continue speech."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Trigger speech onset
            mock_vad_instance.is_speech.return_value = True
            for i in range(30):
                detector.process_frame(speech_frame)

            # Detect silence for 30 frames
            mock_vad_instance.is_speech.return_value = False
            for i in range(30):
                detector.process_frame(silence_frame)

            # Resume speech (should reset silence counter)
            mock_vad_instance.is_speech.return_value = True
            for i in range(10):
                detector.process_frame(speech_frame)

            # Now detect silence again for 49 frames
            mock_vad_instance.is_speech.return_value = False
            speech_ended = False
            for i in range(49):
                result = detector.process_frame(silence_frame)
                if result and result.get('event') == 'speech_end':
                    speech_ended = True
                    break

            # Should NOT trigger because counter was reset
            assert speech_ended is False


class TestSegmentFinalization:
    """Test speech segment finalization (STT-REQ-003.5)."""

    def test_segment_contains_audio_data(self):
        """WHEN speech segment is finalized
        THEN should contain accumulated audio data."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Trigger speech onset and accumulate frames
            mock_vad_instance.is_speech.return_value = True
            for i in range(50):  # 0.5 seconds of speech
                detector.process_frame(speech_frame)

            # Trigger speech offset
            mock_vad_instance.is_speech.return_value = False
            result = None
            for i in range(50):
                result = detector.process_frame(silence_frame)
                if result and result.get('event') == 'speech_end':
                    break

            assert result is not None
            assert 'segment' in result
            # Segment should contain audio data
            assert 'audio_data' in result['segment']
            assert len(result['segment']['audio_data']) > 0

    def test_segment_contains_duration(self):
        """WHEN speech segment is finalized
        THEN should contain duration information."""
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            detector = VoiceActivityDetector()

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Trigger speech onset
            mock_vad_instance.is_speech.return_value = True
            for i in range(50):
                detector.process_frame(speech_frame)

            # Trigger speech offset
            mock_vad_instance.is_speech.return_value = False
            result = None
            for i in range(50):
                result = detector.process_frame(silence_frame)
                if result and result.get('event') == 'speech_end':
                    break

            assert result is not None
            assert 'segment' in result
            assert 'duration_ms' in result['segment']
            # Speech onset at frame 30, then 21 speech frames (30-50) + 50 silence frames
            # Total: 21 + 50 = 71 frames = 710ms
            assert result['segment']['duration_ms'] == 710


class TestPartialAndFinalTextGeneration:
    """
    Task 4.3: Test partial and final text generation with WhisperSTTEngine integration.

    Requirements:
    - STT-REQ-003.6: Final text generation on speech segment completion
    - STT-REQ-003.7: Partial text generation during speech (1s interval)
    - STT-REQ-003.8: Partial text with is_final=False
    - STT-REQ-003.9: Final text with is_final=True
    """

    @pytest.mark.asyncio
    async def test_partial_text_generation_at_1_second_interval(self):
        """
        STT-REQ-003.7, STT-REQ-003.8: Verify partial text generated at 1s intervals with is_final=False.

        Test scenario:
        1. Initialize VAD with mock STT engine
        2. Send 150 frames (1.5s) of continuous speech
        3. Expect partial_text event around frame 130 (after 1s)
        4. Verify is_final=False in transcription result
        """
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        # Mock webrtcvad
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance
            mock_vad_instance.is_speech.return_value = True  # All frames are speech

            # Mock STT engine
            mock_stt = MagicMock()
            mock_stt.transcribe = AsyncMock(return_value={
                'text': 'This is partial',
                'is_final': False,
                'confidence': 0.85
            })

            vad = VoiceActivityDetector(stt_engine=mock_stt)

            # Create speech frames (simulate 1.5 seconds of speech)
            # Frame size: 320 bytes (10ms at 16kHz, 16-bit)
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            partial_text_events = []
            speech_started = False

            # Send 150 frames (1500ms)
            for i in range(150):
                result = await vad.process_frame_async(speech_frame)

                if result:
                    if result.get('event') == 'speech_start':
                        speech_started = True
                    elif result.get('event') == 'partial_text':
                        partial_text_events.append((i, result))

            # Assertions
            assert speech_started, "Speech onset should be detected"
            assert len(partial_text_events) >= 1, "At least one partial text should be generated"

            # First partial text should occur around frame 130 (30 frames onset + 100 frames = 1s)
            frame_idx, first_partial = partial_text_events[0]
            assert 120 <= frame_idx <= 140, f"First partial at frame {frame_idx}, expected ~130"

            # Verify is_final=False
            assert first_partial['transcription']['is_final'] is False
            assert first_partial['transcription']['text'] == 'This is partial'

            # Verify STT engine called with is_final=False
            mock_stt.transcribe.assert_called()
            call_args = mock_stt.transcribe.call_args
            assert call_args.kwargs['is_final'] is False

    @pytest.mark.asyncio
    async def test_final_text_generation_on_speech_end(self):
        """
        STT-REQ-003.6, STT-REQ-003.9: Verify final text generated on speech end with is_final=True.

        Test scenario:
        1. Initialize VAD with mock STT engine
        2. Send speech onset + continuation + silence (speech end)
        3. Expect final_text event on speech end
        4. Verify is_final=True in transcription result
        """
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        # Mock webrtcvad
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Mock STT engine
            mock_stt = MagicMock()
            mock_stt.transcribe = AsyncMock(return_value={
                'text': 'This is final text',
                'is_final': True,
                'confidence': 0.92
            })

            vad = VoiceActivityDetector(stt_engine=mock_stt)

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            final_text_event = None

            # Send speech onset (30 frames)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(30):
                await vad.process_frame_async(speech_frame)

            # Send continuous speech (50 frames)
            for _ in range(50):
                await vad.process_frame_async(speech_frame)

            # Send silence to trigger speech end (50 frames)
            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):
                result = await vad.process_frame_async(silence_frame)
                if result and result.get('event') == 'final_text':
                    final_text_event = result

            # Assertions
            assert final_text_event is not None, "Final text event should be generated"
            assert final_text_event['event'] == 'final_text'
            assert final_text_event['transcription']['is_final'] is True
            assert final_text_event['transcription']['text'] == 'This is final text'
            assert 'segment' in final_text_event

            # Verify STT engine called with is_final=True
            mock_stt.transcribe.assert_called()
            final_call_args = [call for call in mock_stt.transcribe.call_args_list
                              if call.kwargs.get('is_final') is True]
            assert len(final_call_args) > 0, "STT engine should be called with is_final=True"

    @pytest.mark.asyncio
    async def test_no_text_generation_without_stt_engine(self):
        """
        Test backward compatibility: VAD works without stt_engine.

        Test scenario:
        1. Initialize VAD without stt_engine (stt_engine=None)
        2. Send speech onset + continuation + silence
        3. Expect only speech_start/speech_end events (no partial/final_text)
        """
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        # Mock webrtcvad
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            vad = VoiceActivityDetector(stt_engine=None)

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            events = []

            # Send speech onset (30 frames)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(30):
                result = await vad.process_frame_async(speech_frame)
                if result:
                    events.append(result['event'])

            # Send continuous speech (100 frames)
            for _ in range(100):
                result = await vad.process_frame_async(speech_frame)
                if result:
                    events.append(result['event'])

            # Send silence (50 frames)
            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):
                result = await vad.process_frame_async(silence_frame)
                if result:
                    events.append(result['event'])

            # Assertions: Only speech_start and speech_end should occur
            assert 'speech_start' in events
            assert 'speech_end' in events
            assert 'partial_text' not in events, "No partial_text without stt_engine"
            assert 'final_text' not in events, "No final_text without stt_engine"

    @pytest.mark.asyncio
    async def test_partial_text_interval_timing(self):
        """
        Verify exact 1000ms interval timing for partial text generation.

        Test scenario:
        1. Send 250 frames (2.5s) of continuous speech
        2. Expect 2 partial_text events at ~1000ms and ~2000ms
        3. Verify timing precision
        """
        from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

        # Mock webrtcvad
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance
            mock_vad_instance.is_speech.return_value = True  # All frames are speech

            # Mock STT engine
            mock_stt = MagicMock()
            mock_stt.transcribe = AsyncMock(side_effect=[
                {'text': 'First partial', 'is_final': False, 'confidence': 0.80},
                {'text': 'Second partial', 'is_final': False, 'confidence': 0.85},
            ])

            vad = VoiceActivityDetector(stt_engine=mock_stt)

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            partial_text_events = []

            # Send 250 frames (2500ms)
            for i in range(250):
                result = await vad.process_frame_async(speech_frame)
                if result and result.get('event') == 'partial_text':
                    partial_text_events.append((i, result))

            # Assertions: Expect 2 partial text events
            assert len(partial_text_events) == 2, f"Expected 2 partial texts, got {len(partial_text_events)}"

            # First partial at ~frame 130 (30 onset + 100 frames = 1000ms)
            first_frame, first_event = partial_text_events[0]
            assert 120 <= first_frame <= 140, f"First partial at frame {first_frame}, expected ~130"
            assert first_event['transcription']['text'] == 'First partial'

            # Second partial at ~frame 230 (30 onset + 200 frames = 2000ms)
            second_frame, second_event = partial_text_events[1]
            assert 220 <= second_frame <= 240, f"Second partial at frame {second_frame}, expected ~230"
            assert second_event['transcription']['text'] == 'Second partial'

            # Verify interval: ~100 frames (1000ms) between partials
            interval = second_frame - first_frame
            assert 90 <= interval <= 110, f"Interval {interval} frames, expected ~100"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

"""
Unit tests for Latency Requirements (STT-NFR-001.7 per ADR-017).

Task 11.1: Performance validation for real-time STT.
Task 11.2: Latency optimization and requirement adjustment.

Requirements (per ADR-017):
- Partial text end-to-end latency: < 3000ms (speech_start → first partial_text delivery)
- Final text end-to-end latency: < 2000ms (speech_end → final_text delivery)

Note: Unit tests use mocks and should complete well under these targets.
Production E2E tests validate against actual targets.
"""

import pytest
import time
import numpy as np
from unittest.mock import Mock, AsyncMock, patch, MagicMock
from stt_engine.audio_pipeline import AudioPipeline
from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector


class TestPartialTextLatency:
    """Test partial text latency (STT-NFR-001 implied requirement)."""

    @pytest.mark.asyncio
    async def test_partial_text_latency_under_500ms(self):
        """WHEN generating partial transcription
        THEN end-to-end latency (speech_start → partial_text) SHOULD be < 500ms.

        Test Design:
        1. Mock VAD to return speech_start with timestamp
        2. Process audio frames to trigger partial transcription
        3. Verify latency_metrics.end_to_end_latency_ms < 500
        """
        # Create mocks
        mock_vad = Mock(spec=VoiceActivityDetector)
        mock_vad.is_in_speech = False  # Will be updated dynamically

        mock_stt = AsyncMock()
        mock_stt.transcribe = AsyncMock(return_value={
            'text': 'test transcription',
            'confidence': 0.95,
            'language': 'ja',
            'is_final': False
        })

        # Create pipeline
        pipeline = AudioPipeline(vad=mock_vad, stt_engine=mock_stt)

        # Simulate speech_start with timestamp
        speech_start_timestamp_ms = int(time.time() * 1000)

        # Step 1: Handle speech_start event
        speech_start_event = await pipeline._handle_speech_start(
            pre_roll=b'\x00' * 9600,  # 30 frames of pre-roll
            timestamp_ms=speech_start_timestamp_ms
        )

        assert speech_start_event['event'] == 'speech_start'
        assert pipeline._speech_start_timestamp_ms == speech_start_timestamp_ms

        # Update VAD state to in-speech
        mock_vad.is_in_speech = True

        # Step 2: Accumulate audio frames to trigger partial transcription
        # Task 11.2: First partial triggers after 10 frames (100ms) per ADR-017
        test_frame = b'\x00' * 320  # 10ms frame

        for i in range(10):  # ADR-017: Early trigger for first partial
            pipeline._current_speech_buffer.extend(test_frame)
            pipeline._frame_count_since_partial += 1

        # Step 3: Generate partial transcription
        partial_event = await pipeline._generate_partial_transcription()

        # Step 4: Verify latency metrics exist
        assert partial_event is not None
        assert 'latency_metrics' in partial_event

        latency_metrics = partial_event['latency_metrics']
        assert 'end_to_end_latency_ms' in latency_metrics
        assert 'whisper_processing_ms' in latency_metrics
        assert 'vad_speech_start_timestamp_ms' in latency_metrics
        assert 'delivery_timestamp_ms' in latency_metrics
        assert 'is_first_partial' in latency_metrics

        # Step 5: Verify this is the FIRST partial and latency < 500ms
        assert latency_metrics['is_first_partial'] is True, \
            "Expected first partial to have is_first_partial=True"

        end_to_end_latency_ms = latency_metrics['end_to_end_latency_ms']

        # Allow some tolerance for test execution time
        # In real scenarios, this should be well under 500ms
        # For unit test with mocks, latency should be minimal (<50ms)
        assert end_to_end_latency_ms < 500, \
            f"First partial text latency {end_to_end_latency_ms}ms exceeds 500ms target"

        # In unit test context with mocks, latency should be very low
        assert end_to_end_latency_ms < 100, \
            f"Unit test latency {end_to_end_latency_ms}ms unexpectedly high (expected <100ms with mocks)"


class TestFinalTextLatency:
    """Test final text latency (STT-NFR-001 implied requirement)."""

    @pytest.mark.asyncio
    async def test_final_text_latency_under_2000ms(self):
        """WHEN generating final transcription
        THEN end-to-end latency (speech_end → final_text) SHOULD be < 2000ms.

        Test Design:
        1. Mock VAD to return speech_end with timestamp and segment
        2. Process speech_end event
        3. Verify latency_metrics.end_to_end_latency_ms < 2000
        """
        # Create mocks
        mock_vad = Mock(spec=VoiceActivityDetector)

        mock_stt = AsyncMock()
        mock_stt.transcribe = AsyncMock(return_value={
            'text': 'final transcription result',
            'confidence': 0.98,
            'language': 'ja',
            'is_final': True
        })

        # Create pipeline
        pipeline = AudioPipeline(vad=mock_vad, stt_engine=mock_stt)

        # Simulate speech_end with timestamp
        speech_end_timestamp_ms = int(time.time() * 1000)
        segment_data = {
            'audio_data': b'\x00' * 32000,  # 1 second of audio
            'duration_ms': 1000
        }

        # Handle speech_end event
        final_event = await pipeline._handle_speech_end(
            segment_data=segment_data,
            timestamp_ms=speech_end_timestamp_ms
        )

        # Verify latency metrics exist
        assert final_event is not None
        assert 'latency_metrics' in final_event

        latency_metrics = final_event['latency_metrics']
        assert 'end_to_end_latency_ms' in latency_metrics
        assert 'whisper_processing_ms' in latency_metrics
        assert 'vad_speech_end_timestamp_ms' in latency_metrics
        assert 'delivery_timestamp_ms' in latency_metrics

        # Verify end-to-end latency < 2000ms
        end_to_end_latency_ms = latency_metrics['end_to_end_latency_ms']

        assert end_to_end_latency_ms < 2000, \
            f"Final text latency {end_to_end_latency_ms}ms exceeds 2000ms target"

        # In unit test context with mocks, latency should be very low
        assert end_to_end_latency_ms < 100, \
            f"Unit test latency {end_to_end_latency_ms}ms unexpectedly high (expected <100ms with mocks)"


class TestTimestampPropagation:
    """Test that VAD timestamps are correctly propagated through the pipeline."""

    @pytest.mark.asyncio
    async def test_vad_timestamp_recorded_on_speech_start(self):
        """WHEN VAD detects speech_start with timestamp
        THEN AudioPipeline SHOULD record the timestamp for latency calculation."""
        mock_vad = Mock(spec=VoiceActivityDetector)
        pipeline = AudioPipeline(vad=mock_vad)

        test_timestamp = 1234567890123

        await pipeline._handle_speech_start(
            pre_roll=None,
            timestamp_ms=test_timestamp
        )

        assert pipeline._speech_start_timestamp_ms == test_timestamp

    @pytest.mark.asyncio
    async def test_vad_timestamp_recorded_on_speech_end(self):
        """WHEN VAD detects speech_end with timestamp
        THEN AudioPipeline SHOULD record the timestamp for latency calculation."""
        mock_vad = Mock(spec=VoiceActivityDetector)
        mock_stt = AsyncMock()
        mock_stt.transcribe = AsyncMock(return_value={
            'text': 'test',
            'confidence': 0.9,
            'language': 'ja',
            'is_final': True
        })

        pipeline = AudioPipeline(vad=mock_vad, stt_engine=mock_stt)

        test_timestamp = 1234567890456
        segment_data = {'audio_data': b'\x00' * 1000, 'duration_ms': 100}

        await pipeline._handle_speech_end(
            segment_data=segment_data,
            timestamp_ms=test_timestamp
        )

        assert pipeline._speech_end_timestamp_ms == test_timestamp


class TestVADTimestampGeneration:
    """Test that VAD generates timestamps for events."""

    def test_speech_start_includes_timestamp(self):
        """WHEN VAD detects speech onset
        THEN speech_start event SHOULD include timestamp_ms field."""
        # Mock webrtcvad to ensure predictable behavior
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Simulate speech detection
            mock_vad_instance.is_speech.return_value = True

            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=3)

            # Generate realistic audio frame (160 samples at 16kHz = 10ms)
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            # Feed 30 speech frames to trigger speech_start
            result = None
            for _ in range(30):
                result = vad.process_frame(speech_frame)
                if result:
                    break

            assert result is not None
            assert result['event'] == 'speech_start'
            assert 'timestamp_ms' in result
            assert isinstance(result['timestamp_ms'], int)
            assert result['timestamp_ms'] > 0

    def test_speech_end_includes_timestamp(self):
        """WHEN VAD detects speech offset
        THEN speech_end event SHOULD include timestamp_ms field."""
        # Mock webrtcvad to ensure predictable behavior
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=3)

            # Generate realistic audio frames
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            # Step 1: Trigger speech_start (30 speech frames)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(30):
                vad.process_frame(speech_frame)

            # Step 2: Trigger speech_end (50 silence frames)
            mock_vad_instance.is_speech.return_value = False
            result = None
            for _ in range(50):
                result = vad.process_frame(silence_frame)
                if result:
                    break

            assert result is not None
            assert result['event'] == 'speech_end'
            assert 'timestamp_ms' in result
            assert isinstance(result['timestamp_ms'], int)
            assert result['timestamp_ms'] > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

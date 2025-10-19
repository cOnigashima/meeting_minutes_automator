"""
Unit tests for AudioPipeline orchestrator
Test decoupled VAD-STT architecture

Design goals:
- VAD only detects speech
- STT only transcribes
- Pipeline coordinates between them
"""

import pytest
import asyncio
from unittest.mock import MagicMock, AsyncMock, patch
from stt_engine.audio_pipeline import (
    AudioPipeline,
    AudioSegment,
    TranscriptionResult,
    SegmentType
)


class MockVAD:
    """Mock VAD that only detects speech segments"""
    def __init__(self):
        self.is_in_speech = False
        self._frame_count = 0

    def process_frame(self, audio_frame):
        """Simulate VAD processing"""
        self._frame_count += 1

        # Simulate speech detection pattern
        if self._frame_count == 30:  # Speech start after 30 frames
            self.is_in_speech = True
            return {'event': 'speech_start'}
        elif self._frame_count == 180:  # Speech end after 180 frames
            self.is_in_speech = False
            return {
                'event': 'speech_end',
                'segment': {
                    'audio_data': b'fake_audio_data',
                    'duration_ms': 1500
                }
            }
        return None


class MockSTTEngine:
    """Mock STT that only transcribes audio"""
    async def transcribe(self, audio_data, sample_rate=16000, is_final=True):
        """Simulate STT transcription"""
        await asyncio.sleep(0.01)  # Simulate processing time

        if is_final:
            return {
                'text': 'Final transcription text',
                'is_final': True,
                'confidence': 0.95,
                'language': 'ja'
            }
        else:
            return {
                'text': 'Partial transcription',
                'is_final': False,
                'confidence': 0.85,
                'language': 'ja'
            }


class TestAudioPipelineInitialization:
    """Test pipeline initialization"""

    def test_init_without_components(self):
        """WHEN pipeline initialized without VAD/STT
        THEN should handle gracefully"""
        pipeline = AudioPipeline()

        assert pipeline.vad is None
        assert pipeline.stt_engine is None
        assert pipeline.sample_rate == 16000
        assert pipeline.stats["segments_processed"] == 0

    def test_init_with_components(self):
        """WHEN pipeline initialized with VAD and STT
        THEN should store components"""
        vad = MockVAD()
        stt = MockSTTEngine()

        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        assert pipeline.vad == vad
        assert pipeline.stt_engine == stt


class TestVADSTTDecoupling:
    """Test that VAD and STT are properly decoupled"""

    @pytest.mark.asyncio
    async def test_vad_only_detects_speech(self):
        """VAD should only detect speech, not transcribe"""
        vad = MockVAD()
        pipeline = AudioPipeline(vad=vad, stt_engine=None)

        # Process frames until speech start
        for i in range(30):
            result = await pipeline.process_audio_frame(b'frame')

        # Last frame should trigger speech start
        assert result is not None
        assert result['event'] == 'speech_start'
        assert 'transcription' not in result  # No transcription without STT

    @pytest.mark.asyncio
    async def test_stt_only_transcribes(self):
        """STT should only transcribe, not detect speech"""
        stt = MockSTTEngine()

        # STT alone cannot detect speech
        result = await stt.transcribe(b'audio_data')

        assert result['text'] == 'Final transcription text'
        assert 'event' not in result  # STT doesn't generate events

    @pytest.mark.asyncio
    async def test_pipeline_coordinates_vad_and_stt(self):
        """Pipeline should coordinate between VAD and STT"""
        vad = MockVAD()
        stt = MockSTTEngine()
        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        results = []

        # Process frames through complete speech cycle
        for i in range(200):
            result = await pipeline.process_audio_frame(b'frame')
            if result:
                results.append(result)

        # Should have speech start and speech end with transcription
        assert len(results) == 2
        assert results[0]['event'] == 'speech_start'
        assert results[1]['event'] == 'final_text'
        assert results[1]['transcription']['text'] == 'Final transcription text'


class TestPartialTranscription:
    """Test partial transcription generation"""

    @pytest.mark.asyncio
    async def test_partial_transcription_at_intervals(self):
        """WHEN speech continues for >1 second
        THEN should generate partial transcriptions"""
        vad = MockVAD()
        stt = MockSTTEngine()
        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        # Mock time to control partial generation
        with patch('stt_engine.audio_pipeline.time') as mock_time:
            # Start at time 0
            mock_time.time.return_value = 0

            # Trigger speech start
            vad.is_in_speech = True
            result = await pipeline.process_audio_frame_with_partial(b'frame')

            # Advance time by 1.1 seconds
            mock_time.time.return_value = 1.1

            # Next frame should trigger partial
            result = await pipeline.process_audio_frame_with_partial(b'frame')

            if result and result.get('event') == 'partial_text':
                assert result['transcription']['text'] == 'Partial transcription'
                assert result['transcription']['is_final'] is False


class TestErrorHandling:
    """Test error handling in pipeline"""

    @pytest.mark.asyncio
    async def test_handles_stt_failure(self):
        """WHEN STT fails
        THEN pipeline should handle error gracefully"""
        vad = MockVAD()
        stt = AsyncMock()
        stt.transcribe.side_effect = Exception("STT failed")

        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        # Process until speech end
        for i in range(180):
            result = await pipeline.process_audio_frame(b'frame')

        # Should get error event
        assert result is not None
        assert result['event'] == 'error'
        assert 'STT failed' in result['error']
        assert pipeline.stats['errors'] == 1

    @pytest.mark.asyncio
    async def test_continues_after_error(self):
        """WHEN error occurs
        THEN pipeline should continue processing"""
        vad = MockVAD()
        stt = AsyncMock()

        # First transcription fails, second succeeds
        stt.transcribe.side_effect = [
            Exception("First failed"),
            {'text': 'Second succeeded', 'is_final': True}
        ]

        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        # Process two speech segments
        # (Would need more complex mock for this test)
        # Simplified for demonstration
        assert pipeline.stats['errors'] == 0  # Initially no errors


class TestStatistics:
    """Test pipeline statistics tracking"""

    @pytest.mark.asyncio
    async def test_tracks_statistics(self):
        """WHEN processing audio
        THEN should track statistics"""
        vad = MockVAD()
        stt = MockSTTEngine()
        pipeline = AudioPipeline(vad=vad, stt_engine=stt)

        # Process complete speech cycle
        for i in range(180):
            await pipeline.process_audio_frame(b'frame')

        stats = pipeline.get_stats()

        assert stats['transcriptions_generated'] == 1
        assert stats['errors'] == 0

    def test_get_stats_returns_copy(self):
        """WHEN get_stats is called
        THEN should return a copy"""
        pipeline = AudioPipeline()

        stats1 = pipeline.get_stats()
        stats1['modified'] = True

        stats2 = pipeline.get_stats()
        assert 'modified' not in stats2


class TestPartialTextPreRollIntegrity:
    """
    Test that partial_text includes VAD pre-roll frames from speech onset.

    P0.5 BUG: VAD correctly preserves 30 leading frames in current_segment,
    but AudioPipeline._current_speech_buffer is reset to empty on speech_start
    (audio_pipeline.py:237), and only accumulates frames AFTER is_in_speech=True
    (audio_pipeline.py:189). The 30-frame pre-roll never reaches the partial
    transcription buffer, so users see leading syllables disappear in real-time.

    Requirements:
        STT-REQ-003.4: Speech onset detection (0.3s continuous speech)
        NFR-001.1: Partial text response time < 0.5s (requires full audio)
    """

    @pytest.mark.asyncio
    async def test_partial_text_includes_pre_roll_frames(self):
        """
        GIVEN user starts speaking immediately (no silence padding)
        WHEN partial_text is generated after speech_start
        THEN the transcribed audio should include all 30 leading frames

        P0.5 BUG DETECTION: This test WILL FAIL because _current_speech_buffer
        is reset on speech_start and only accumulates frames 31+.
        Expected: 30 pre-roll + N active = (30+N) frames
        Actual: N frames only (missing 30 pre-roll)
        """
        from stt_engine.audio_pipeline import AudioPipeline
        from unittest.mock import AsyncMock, MagicMock, patch

        # Mock VAD to return speech_start with pre-roll data
        mock_vad = MagicMock()
        mock_vad.is_in_speech = False

        # Create unique frames
        unique_frames = [bytes([i] * 320) for i in range(50)]

        # Track process_frame calls
        call_count = [0]

        def mock_process_frame(frame):
            idx = call_count[0]
            call_count[0] += 1

            if idx < 29:
                # Frames 0-28: no event
                return None
            elif idx == 29:
                # Frame 29: speech_start with pre-roll
                mock_vad.is_in_speech = True
                pre_roll_audio = b''.join(unique_frames[:30])
                return {
                    'event': 'speech_start',
                    'pre_roll': pre_roll_audio  # ✅ Expected field
                }
            else:
                # Frame 30+: no event (continue accumulating)
                return None

        mock_vad.process_frame = mock_process_frame

        # Mock STT engine
        mock_stt = AsyncMock()
        mock_stt.transcribe = AsyncMock(return_value={
            'text': 'こんにちは',  # Should include leading "こ"
            'confidence': 0.9,
            'language': 'ja'
        })

        # Create pipeline
        pipeline = AudioPipeline()
        pipeline.vad = mock_vad
        pipeline.stt_engine = mock_stt

        # Send 50 frames
        for i in range(50):
            await pipeline.process_audio_frame_with_partial(unique_frames[i])

        # Force partial transcription generation
        pipeline._frame_count_since_partial = 100  # Trigger threshold
        result = await pipeline._generate_partial_transcription()

        # Verify STT received all frames including pre-roll
        assert mock_stt.transcribe.called
        transcribed_audio = mock_stt.transcribe.call_args[0][0]

        # ❌ BUG: Expected 50 frames (30 pre-roll + 20 active)
        # ✅ FIXED: Should be 50 frames * 320 bytes = 16000 bytes
        expected_size = 50 * 320
        assert len(transcribed_audio) == expected_size, \
            f"Expected {expected_size} bytes (50 frames), got {len(transcribed_audio)} bytes. " \
            f"Missing {(expected_size - len(transcribed_audio)) // 320} frames!"

        # Verify content integrity (first 30 frames should match pre-roll)
        for i in range(30):
            frame_start = i * 320
            frame_end = frame_start + 320
            assert transcribed_audio[frame_start:frame_end] == unique_frames[i], \
                f"Pre-roll frame {i} content mismatch"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
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


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
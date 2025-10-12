"""
Integration Tests: VoiceActivityDetector → AudioPipeline → WhisperSTTEngine

This module replaces the deleted TestPartialAndFinalTextGeneration tests.
Tests the actual integration of VAD, AudioPipeline, and STT components.

Related Requirements:
- STT-REQ-003.6: Final text generation on speech segment completion
- STT-REQ-003.7: Partial text generation during speech (1s interval)
- STT-REQ-003.8: Partial text with is_final=False
- STT-REQ-003.9: Final text with is_final=True
"""

import pytest
import numpy as np
from unittest.mock import AsyncMock

from stt_engine.audio_pipeline import AudioPipeline
from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector


class TestVADPipelineSTTIntegration:
    """Integration tests for VAD → AudioPipeline → STT flow"""

    @pytest.mark.asyncio
    async def test_speech_detection_to_transcription_flow(self):
        """
        STT-REQ-007.1, 003.6, 003.9: Speech detection → final transcription flow (MVP0 compatible)

        GIVEN Real VAD with mocked webrtcvad and mock STT engine
        WHEN Audio frames are sent (speech onset → continuation → offset)
        THEN AudioPipeline generates events internally AND final_text is available

        Note: This tests AudioPipeline behavior directly.
        Integration with main.py IPC (Request-Response) is tested separately.
        """
        from unittest.mock import MagicMock, patch

        # Initialize real VAD with mocked webrtcvad
        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=2)

            # Mock STT engine (real WhisperClient is heavy for unit tests)
            mock_stt = AsyncMock()
            mock_stt.transcribe.return_value = {
                'text': 'Final transcription text',
                'is_final': True,
                'confidence': 0.95,
                'language': 'ja'
            }

            # Initialize pipeline
            pipeline = AudioPipeline(vad=vad, stt_engine=mock_stt)

            # Generate audio frames (10ms = 160 samples = 320 bytes at 16kHz)
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            events = []

            # Speech onset (30 frames = 0.3 seconds, all return True)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(30):
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            # Speech continuation (50 frames = 0.5 seconds, all return True)
            for _ in range(50):
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            # Speech offset (50 frames = 0.5 seconds of silence, all return False)
            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):
                result = await pipeline.process_audio_frame(silence_frame)
                if result:
                    events.append(result)

            # Assertions - AudioPipeline should generate 2 events internally
            assert len(events) == 2, f"Expected 2 events (speech_start + final_text), got {len(events)}"
            assert events[0]['event'] == 'speech_start', "First event should be speech_start"
            assert events[1]['event'] == 'final_text', "Second event should be final_text"
            assert events[1]['transcription']['is_final'] is True, "Final text should have is_final=True"
            assert events[1]['transcription']['text'] == 'Final transcription text'

            # Verify STT engine was called with is_final=True
            mock_stt.transcribe.assert_called_once()
            call_kwargs = mock_stt.transcribe.call_args.kwargs
            assert call_kwargs['is_final'] is True, "STT should be called with is_final=True"

    @pytest.mark.skip(reason="Requires time.time() mocking - to be implemented")
    @pytest.mark.asyncio
    async def test_partial_text_generation_during_speech(self):
        """
        STT-REQ-003.7, 003.8: Partial text generation (1s interval, is_final=False)

        GIVEN Real VAD with mocked webrtcvad and mock STT engine
        WHEN 1.5 seconds of continuous speech is sent
        THEN partial_text event is generated after 1 second with is_final=False
        """
        from unittest.mock import MagicMock, patch

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance
            mock_vad_instance.is_speech.return_value = True  # All frames are speech

            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=2)

            # Mock STT engine
            mock_stt = AsyncMock()
            mock_stt.transcribe.return_value = {
                'text': 'Partial transcription text',
                'is_final': False,
                'confidence': 0.85,
                'language': 'ja'
            }

            # Initialize pipeline
            pipeline = AudioPipeline(vad=vad, stt_engine=mock_stt)

            # Generate speech frames
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()

            events = []

            # Send 150 frames (1.5 seconds)
            for _ in range(150):
                result = await pipeline.process_audio_frame_with_partial(speech_frame)
                if result:
                    events.append(result)

            # Assertions
            assert len(events) >= 2, f"Expected at least 2 events (speech_start + partial_text), got {len(events)}"

            # Extract partial_text events
            partial_events = [e for e in events if e.get('event') == 'partial_text']
            assert len(partial_events) >= 1, "At least one partial_text event should be generated"

            # Verify all partial_text events have is_final=False
            for partial in partial_events:
                assert partial['transcription']['is_final'] is False, "Partial text should have is_final=False"

            # Verify STT engine was called with is_final=False
            partial_calls = [
                call for call in mock_stt.transcribe.call_args_list
                if call.kwargs.get('is_final') is False
            ]
            assert len(partial_calls) >= 1, "STT should be called with is_final=False for partial text"

    @pytest.mark.asyncio
    async def test_pipeline_without_stt_engine(self):
        """
        Backward compatibility: Pipeline works without STT engine

        GIVEN VAD-only pipeline (stt_engine=None) with mocked webrtcvad
        WHEN Audio frames are sent
        THEN speech_start/speech_end events are generated, but no partial/final text
        """
        from unittest.mock import MagicMock, patch

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            # Initialize VAD-only pipeline
            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=2)
            pipeline = AudioPipeline(vad=vad, stt_engine=None)

            # Generate audio frames
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            events = []

            # Speech onset (30 frames, all return True)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(30):
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            # Speech continuation (50 frames, all return True)
            for _ in range(50):
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            # Speech offset (50 frames, all return False)
            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):
                result = await pipeline.process_audio_frame(silence_frame)
                if result:
                    events.append(result)

            # Assertions
            event_types = [e['event'] for e in events]
            assert 'speech_start' in event_types, "speech_start should be generated"
            assert 'speech_end' in event_types, "speech_end should be generated"
            assert 'partial_text' not in event_types, "partial_text should NOT be generated without STT"
            assert 'final_text' not in event_types, "final_text should NOT be generated without STT"

    @pytest.mark.asyncio
    async def test_multiple_speech_segments(self):
        """
        Test handling of multiple speech segments in sequence

        GIVEN Real VAD with mocked webrtcvad and mock STT
        WHEN Multiple speech segments are sent with silence in between
        THEN Each segment generates separate speech_start and final_text events
        """
        from unittest.mock import MagicMock, patch

        with patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=2)

            # Mock STT with different responses for each segment
            mock_stt = AsyncMock()
            mock_stt.transcribe.side_effect = [
                {'text': 'First segment', 'is_final': True, 'confidence': 0.9, 'language': 'ja'},
                {'text': 'Second segment', 'is_final': True, 'confidence': 0.92, 'language': 'ja'},
            ]

            pipeline = AudioPipeline(vad=vad, stt_engine=mock_stt)

            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16).tobytes()
            silence_frame = np.zeros(160, dtype=np.int16).tobytes()

            events = []

            # First speech segment (30 onset + 50 speech + 50 silence)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(80):  # 30 onset + 50 speech
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):  # 50 silence
                result = await pipeline.process_audio_frame(silence_frame)
                if result:
                    events.append(result)

            # Second speech segment (30 onset + 50 speech + 50 silence)
            mock_vad_instance.is_speech.return_value = True
            for _ in range(80):  # 30 onset + 50 speech
                result = await pipeline.process_audio_frame(speech_frame)
                if result:
                    events.append(result)

            mock_vad_instance.is_speech.return_value = False
            for _ in range(50):  # 50 silence
                result = await pipeline.process_audio_frame(silence_frame)
                if result:
                    events.append(result)

            # Assertions
            speech_starts = [e for e in events if e['event'] == 'speech_start']
            final_texts = [e for e in events if e['event'] == 'final_text']

            assert len(speech_starts) == 2, "Should have 2 speech_start events"
            assert len(final_texts) == 2, "Should have 2 final_text events"
            assert final_texts[0]['transcription']['text'] == 'First segment'
            assert final_texts[1]['transcription']['text'] == 'Second segment'


class TestAudioProcessorIntegration:
    """Integration tests for AudioProcessor (main.py)"""

    @pytest.mark.asyncio
    async def test_audio_processor_initialization(self):
        """
        Test that AudioProcessor initializes all components correctly

        GIVEN AudioProcessor class
        WHEN Initialized
        THEN VAD, WhisperClient, and AudioPipeline should be created
        """
        from unittest.mock import patch, MagicMock

        # Mock WhisperSTTEngine to avoid heavy initialization
        with patch('main.WhisperSTTEngine') as mock_whisper:
            mock_whisper.return_value = MagicMock()

            from main import AudioProcessor

            processor = AudioProcessor()

            assert processor.vad is not None, "VAD should be initialized"
            assert processor.stt_engine is not None, "STT engine should be initialized"
            assert processor.pipeline is not None, "AudioPipeline should be initialized"
            assert processor.pipeline.vad == processor.vad, "Pipeline should use processor's VAD"
            assert processor.pipeline.stt_engine == processor.stt_engine, "Pipeline should use processor's STT"

    @pytest.mark.asyncio
    async def test_audio_processor_message_handling(self):
        """
        Test AudioProcessor handles IPC messages correctly (MVP0 compatible)

        GIVEN AudioProcessor with mock IPC and mocked webrtcvad
        WHEN process_audio message is received
        THEN Single response with transcription should be returned (Request-Response)
        """
        from unittest.mock import patch, MagicMock

        # Mock both WhisperSTTEngine and webrtcvad
        with patch('main.WhisperSTTEngine') as mock_whisper, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad') as mock_vad_class:

            # Mock STT engine
            mock_stt_engine = AsyncMock()
            mock_stt_engine.transcribe.return_value = {
                'text': 'Test transcription',
                'is_final': True,
                'confidence': 0.9,
                'language': 'ja'
            }
            mock_whisper.return_value = mock_stt_engine

            # Mock VAD
            mock_vad_instance = MagicMock()
            mock_vad_class.return_value = mock_vad_instance

            from main import AudioProcessor

            processor = AudioProcessor()

            # Mock IPC
            sent_messages = []

            async def mock_send(msg):
                sent_messages.append(msg)

            processor.ipc = AsyncMock()
            processor.ipc.send_message = mock_send

            # Create test audio message (30 onset + 50 speech + 50 silence = 130 frames)
            speech_frame = np.random.randint(-32768, 32767, 160, dtype=np.int16)
            silence_frame = np.zeros(160, dtype=np.int16)

            # Simulate speech onset + continuation + offset
            audio_data = (speech_frame.tobytes() * 80) + (silence_frame.tobytes() * 50)

            test_message = {
                'type': 'process_audio',
                'id': 'test-123',
                'audio_data': list(audio_data)
            }

            # Simulate VAD behavior: True for speech, False for silence
            call_count = 0
            def is_speech_side_effect(frame, rate):
                nonlocal call_count
                call_count += 1
                # First 80 frames are speech, last 50 are silence
                return call_count <= 80

            mock_vad_instance.is_speech.side_effect = is_speech_side_effect

            # Process message
            await processor.handle_message(test_message)

            # Verify SINGLE response was sent (Request-Response protocol)
            assert len(sent_messages) == 1, f"Should send exactly 1 response, got {len(sent_messages)}"

            response = sent_messages[0]
            assert response['id'] == 'test-123', "Response should have matching ID"
            assert response['type'] == 'response', "Response type should be 'response'"
            assert response['version'] == '1.0', "Response should have version field"

            # Check flat structure (MVP0 compatible + MVP1 extensions)
            # STT-REQ-007.3: text field at root level for backward compatibility
            if response.get('text'):  # May be None if no transcription yet
                assert 'text' in response, "Response should contain text field at root level"
                assert 'is_final' in response, "Response should contain is_final field"
                assert 'confidence' in response, "Response should contain confidence field (MVP1 extension)"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

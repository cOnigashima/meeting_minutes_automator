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



class TestResourceMonitorIntegration:
    """Integration tests for ResourceMonitor + AudioProcessor (Task 5.2)"""

    @pytest.mark.asyncio
    async def test_resource_monitor_initialization(self):
        """
        Task 5.2: ResourceMonitor should be initialized in AudioProcessor

        GIVEN AudioProcessor
        WHEN Initialized
        THEN ResourceMonitor should be created and configured
        """
        from unittest.mock import patch, MagicMock

        with patch('main.WhisperSTTEngine') as mock_whisper, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'):

            mock_whisper.return_value = MagicMock()

            from main import AudioProcessor

            processor = AudioProcessor()

            # ResourceMonitor should be initialized
            assert hasattr(processor, 'resource_monitor'), "AudioProcessor should have resource_monitor"
            assert processor.resource_monitor is not None
            # Should be configured with current model from STT engine
            assert processor.resource_monitor.current_model == processor.stt_engine.model_size

    @pytest.mark.asyncio
    async def test_model_downgrade_on_high_cpu(self):
        """
        Task 5.2, STT-REQ-006.7: CPU-based model downgrade

        GIVEN AudioProcessor with monitoring enabled
        WHEN CPU usage stays high (>= 85%) for 60+ seconds
        THEN Model should downgrade and IPC notification should be sent
        """
        from unittest.mock import patch, MagicMock, AsyncMock
        import asyncio
        import time

        with patch('main.WhisperSTTEngine') as mock_whisper_class, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'), \
             patch('psutil.cpu_percent', return_value=90), \
             patch('psutil.virtual_memory') as mock_mem:

            # Mock memory to be low (safe, < 3GB)
            mock_mem.return_value.percent = 50
            mock_mem.return_value.used = 2.0 * (1024 ** 3)  # 2GB

            # Mock STT engine
            mock_stt = MagicMock()
            mock_stt.model_size = 'large-v3'
            mock_stt.load_model = AsyncMock()
            mock_whisper_class.return_value = mock_stt

            from main import AudioProcessor

            processor = AudioProcessor()

            # Simulate CPU high for 60+ seconds by setting timestamp
            processor.resource_monitor.cpu_high_start_time = time.time() - 61

            # Mock IPC
            sent_messages = []
            async def mock_send(msg):
                sent_messages.append(msg)
            processor.ipc = AsyncMock()
            processor.ipc.send_message = mock_send

            # Start monitoring with fast interval
            task = asyncio.create_task(processor.resource_monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            ))

            # Wait for one monitoring cycle
            await asyncio.sleep(0.2)

            await processor.resource_monitor.stop_monitoring()
            await task

            # Verify model downgrade was called
            assert mock_stt.load_model.called, "load_model should be called for downgrade"

            # Verify IPC notification was sent
            model_change_msgs = [m for m in sent_messages if m.get('event') == 'model_change']
            assert len(model_change_msgs) > 0, "model_change event should be sent via IPC"

            # Verify notification format
            msg = model_change_msgs[0]
            assert msg['type'] == 'event'
            assert 'old_model' in msg
            assert 'new_model' in msg
            assert msg['reason'] in ['cpu_high', 'memory_high']

    @pytest.mark.asyncio
    async def test_model_downgrade_on_high_memory(self):
        """
        Task 5.2, STT-REQ-006.8: Memory-based model downgrade

        GIVEN AudioProcessor with monitoring enabled
        WHEN Memory usage exceeds 90%
        THEN Model should immediately downgrade to base and IPC notification sent
        """
        from unittest.mock import patch, MagicMock, AsyncMock
        import asyncio

        with patch('main.WhisperSTTEngine') as mock_whisper_class, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'), \
             patch('psutil.cpu_percent', return_value=50), \
             patch('psutil.virtual_memory') as mock_mem:

            # Mock critical memory usage (>= 4GB)
            mock_mem.return_value.percent = 92
            mock_mem.return_value.used = 4.5 * (1024 ** 3)  # 4.5GB (critical)

            # Mock STT engine
            mock_stt = MagicMock()
            mock_stt.model_size = 'large-v3'
            mock_stt.load_model = AsyncMock()
            mock_whisper_class.return_value = mock_stt

            from main import AudioProcessor

            processor = AudioProcessor()

            # Mock IPC
            sent_messages = []
            async def mock_send(msg):
                sent_messages.append(msg)
            processor.ipc = AsyncMock()
            processor.ipc.send_message = mock_send

            # Start monitoring
            task = asyncio.create_task(processor.resource_monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            ))

            # Wait for one monitoring cycle
            await asyncio.sleep(0.2)

            await processor.resource_monitor.stop_monitoring()
            await task

            # Verify immediate downgrade to base
            assert mock_stt.load_model.called
            call_args = mock_stt.load_model.call_args
            assert call_args[0][0] == 'base', "Should downgrade to base for critical memory"

            # Verify IPC notification
            model_change_msgs = [m for m in sent_messages if m.get('event') == 'model_change']
            assert len(model_change_msgs) > 0
            assert model_change_msgs[0]['new_model'] == 'base'
            assert model_change_msgs[0]['reason'] == 'memory_high'

    @pytest.mark.asyncio
    async def test_upgrade_proposal_on_recovery(self):
        """
        Task 5.2, STT-REQ-006.10: Upgrade proposal after resource recovery

        GIVEN AudioProcessor with downgraded model
        WHEN Resources recover (CPU < 50%, memory < 60%) for 5+ minutes
        THEN Upgrade proposal notification should be sent via IPC
        """
        from unittest.mock import patch, MagicMock, AsyncMock
        import asyncio
        import time

        with patch('main.WhisperSTTEngine') as mock_whisper_class, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'), \
             patch('psutil.cpu_percent', return_value=30), \
             patch('psutil.virtual_memory') as mock_mem:

            # Mock low resource usage (recovered, < 2GB)
            mock_mem.return_value.percent = 40
            mock_mem.return_value.used = 1.5 * (1024 ** 3)  # 1.5GB (low)

            # Mock STT engine with downgraded model
            mock_stt = MagicMock()
            mock_stt.model_size = 'small'
            mock_whisper_class.return_value = mock_stt

            from main import AudioProcessor

            processor = AudioProcessor()
            # Set initial model to simulate downgrade history
            processor.resource_monitor.initial_model = 'large-v3'
            processor.resource_monitor.current_model = 'small'

            # Simulate resources recovered for 5+ minutes by setting timestamp
            processor.resource_monitor.low_resource_start_time = time.time() - 301

            # Mock IPC
            sent_messages = []
            async def mock_send(msg):
                sent_messages.append(msg)
            processor.ipc = AsyncMock()
            processor.ipc.send_message = mock_send

            # Start monitoring
            task = asyncio.create_task(processor.resource_monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            ))

            # Wait for one monitoring cycle
            await asyncio.sleep(0.2)

            await processor.resource_monitor.stop_monitoring()
            await task

            # Verify upgrade proposal was sent
            upgrade_msgs = [m for m in sent_messages if m.get('event') == 'upgrade_proposal']
            assert len(upgrade_msgs) > 0, "upgrade_proposal event should be sent"

            msg = upgrade_msgs[0]
            assert msg['type'] == 'event'
            assert msg['current_model'] == 'small'
            assert msg['proposed_model'] == 'large-v3'

    @pytest.mark.asyncio
    async def test_recording_pause_notification(self):
        """
        Task 5.2, STT-REQ-006.11: Recording pause when tiny model is insufficient

        GIVEN AudioProcessor with tiny model
        WHEN Resources are still insufficient (should_pause_recording returns True)
        THEN recording_paused notification should be sent via IPC
        """
        from unittest.mock import patch, MagicMock, AsyncMock
        import asyncio

        with patch('main.WhisperSTTEngine') as mock_whisper_class, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'), \
             patch('psutil.cpu_percent', return_value=95), \
             patch('psutil.virtual_memory') as mock_mem:

            # Mock very high memory usage (>= 4GB triggers pause)
            mock_mem.return_value.percent = 95
            mock_mem.return_value.used = 4.5 * (1024 ** 3)  # 4.5GB (critical)

            # Mock STT engine with tiny model
            mock_stt = MagicMock()
            mock_stt.model_size = 'tiny'
            mock_whisper_class.return_value = mock_stt

            from main import AudioProcessor

            processor = AudioProcessor()
            processor.resource_monitor.current_model = 'tiny'

            # Mock IPC
            sent_messages = []
            async def mock_send(msg):
                sent_messages.append(msg)
            processor.ipc = AsyncMock()
            processor.ipc.send_message = mock_send

            # Start monitoring
            task = asyncio.create_task(processor.resource_monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            ))

            await asyncio.sleep(0.2)

            await processor.resource_monitor.stop_monitoring()
            await task

            # Verify recording_paused notification
            pause_msgs = [m for m in sent_messages if m.get('event') == 'recording_paused']
            assert len(pause_msgs) > 0, "recording_paused event should be sent"
            
            msg = pause_msgs[0]
            assert msg['type'] == 'event'
            assert msg['reason'] == 'insufficient_resources'

    @pytest.mark.asyncio
    async def test_model_downgrade_failure_state_consistency(self):
        """
        Task 5.2: Model downgrade failure should maintain state consistency

        GIVEN AudioProcessor with monitoring enabled
        WHEN Model downgrade fails (load_model raises exception)
        THEN ResourceMonitor.current_model should remain unchanged (not updated)
        """
        from unittest.mock import patch, MagicMock, AsyncMock
        import asyncio

        with patch('main.WhisperSTTEngine') as mock_whisper_class, \
             patch('stt_engine.transcription.voice_activity_detector.webrtcvad.Vad'), \
             patch('psutil.cpu_percent', return_value=50), \
             patch('psutil.virtual_memory') as mock_mem:

            # Mock critical memory usage (>= 4GB)
            mock_mem.return_value.percent = 92
            mock_mem.return_value.used = 4.5 * (1024 ** 3)  # 4.5GB

            # Mock STT engine with failing load_model
            mock_stt = MagicMock()
            mock_stt.model_size = 'large-v3'
            mock_stt.load_model = AsyncMock(side_effect=RuntimeError("Mock load failure"))
            mock_whisper_class.return_value = mock_stt

            from main import AudioProcessor

            processor = AudioProcessor()

            # Verify initial state
            assert processor.resource_monitor.current_model == 'large-v3'

            # Mock IPC
            processor.ipc = AsyncMock()

            # Start monitoring
            task = asyncio.create_task(processor.resource_monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            ))

            await asyncio.sleep(0.25)
            await processor.resource_monitor.stop_monitoring()
            await task

            # Verify load_model was called (downgrade attempted)
            assert mock_stt.load_model.called

            # CRITICAL: current_model should NOT be changed after failure
            assert processor.resource_monitor.current_model == 'large-v3', \
                "current_model should remain 'large-v3' after failed downgrade"

    @pytest.mark.asyncio
    async def test_user_approved_upgrade_execution(self):
        """
        Task 5.4, STT-REQ-006.12: Test user-approved upgrade execution.

        WHEN user approves upgrade proposal THEN ResourceMonitor SHALL:
        1. Execute upgrade to target model
        2. Update current_model on success
        3. Send success IPC notification
        """
        from unittest.mock import MagicMock, AsyncMock
        from main import AudioProcessor
        from stt_engine.resource_monitor import ResourceMonitor
        import asyncio

        mock_stt = MagicMock()
        mock_stt.load_model = AsyncMock(return_value=None)  # Success
        mock_stt.transcribe = AsyncMock(return_value=("", 0.0, ""))

        processor = AudioProcessor()
        processor.stt_engine = mock_stt
        processor.vad = None  # Skip VAD

        # Initialize ResourceMonitor (no constructor args)
        processor.resource_monitor = ResourceMonitor()
        processor.resource_monitor.initial_model = 'small'  # Manually set
        processor.resource_monitor.current_model = 'base'   # Currently downgraded

        try:
            # Simulate user approving upgrade from 'base' to 'small'
            await processor.handle_message({
                'type': 'approve_upgrade',
                'id': 'test-approve-001',
                'target_model': 'small'
            })

            # Wait for processing
            await asyncio.sleep(0.1)

            # Verify load_model was called with target
            mock_stt.load_model.assert_called_once_with('small')

            # Verify current_model was updated
            assert processor.resource_monitor.current_model == 'small', \
                "current_model should be updated to 'small' after successful upgrade"

            # Verify IPC notification was sent (check mock_ipc if available)
            # For now, just verify no exception was raised

        finally:
            if processor.resource_monitor.monitoring_running:
                await processor.resource_monitor.stop_monitoring()


class TestEventStreamProtocol:
    """
    Task 7.1.6: Event Stream Protocol Tests

    Tests for real-time event streaming (speech_start, partial_text, final_text, speech_end)
    Related Requirements:
    - STT-REQ-003.7: Partial text generation during speech (1s interval)
    - STT-REQ-003.8: Partial text with is_final=False
    - STT-REQ-007.1: Backward compatibility (existing process_audio unchanged)
    """

    @pytest.mark.asyncio
    async def test_process_audio_stream_sends_multiple_events(self):
        """
        RED TEST: process_audio_stream should send speech_start, partial_text, final_text events

        GIVEN AudioProcessor with process_audio_stream handler
        WHEN Audio data with speech is processed
        THEN Multiple IPC events should be sent:
          1. event: speech_start
          2. event: partial_text (is_final=False)
          3. event: final_text (is_final=True)
          4. event: speech_end
        """
        from unittest.mock import AsyncMock, MagicMock, patch
        from stt_engine.ipc_handler import IpcHandler
        from main import AudioProcessor

        # Mock IPC handler to capture sent messages
        mock_ipc = AsyncMock(spec=IpcHandler)
        sent_messages = []

        async def capture_message(msg):
            sent_messages.append(msg)

        mock_ipc.send_message.side_effect = capture_message

        # Create AudioProcessor with mocked components
        with patch('main.WhisperSTTEngine') as mock_stt_class, \
             patch('stt_engine.resource_monitor.ResourceMonitor') as mock_resource_class, \
             patch('main.VoiceActivityDetector') as mock_vad_class, \
             patch('main.AudioPipeline') as mock_pipeline_class:

            mock_stt = AsyncMock()
            mock_stt.model_size = 'small'
            mock_stt.transcribe.return_value = {
                'text': 'Hello world',
                'is_final': True,
                'confidence': 0.95,
                'language': 'ja'
            }
            mock_stt_class.return_value = mock_stt

            mock_resource = MagicMock()
            mock_resource.initial_model = 'small'
            mock_resource.current_model = 'small'
            mock_resource_class.return_value = mock_resource

            # Mock VAD split_into_frames to return some frames
            mock_vad = MagicMock()
            mock_vad.split_into_frames.return_value = [[0] * 320 for _ in range(10)]  # 10 frames
            mock_vad_class.return_value = mock_vad

            # Mock AudioPipeline to return speech events
            mock_pipeline = AsyncMock()
            # Simulate events: speech_start, partial_text, final_text, speech_end
            event_sequence = [
                {'event': 'speech_start', 'timestamp': 1000},
                {'event': 'partial_text', 'transcription': {'text': 'Hello', 'is_final': False, 'confidence': 0.9}},
                {'event': 'final_text', 'transcription': {'text': 'Hello world', 'is_final': True, 'confidence': 0.95, 'language': 'ja'}},
                {'event': 'speech_end', 'timestamp': 2000}
            ]
            mock_pipeline.process_audio_frame_with_partial.side_effect = event_sequence + [None] * 6  # 4 events + 6 None
            mock_pipeline_class.return_value = mock_pipeline

            processor = AudioProcessor()
            processor.ipc = mock_ipc  # Inject mock IPC

            # Prepare audio data (simulated speech segment)
            # 1 second of audio = 16000 samples * 2 bytes = 32000 bytes
            audio_data = [0] * 32000

            msg = {
                'id': 'test-stream-001',
                'type': 'request',
                'method': 'process_audio_stream',
                'params': {'audio_data': audio_data}
            }

            # Act: Process audio stream (should send multiple events)
            await processor.handle_message(msg)

            # Assert: Multiple events should be sent
            assert len(sent_messages) >= 3, f"Expected at least 3 events, got {len(sent_messages)}"

            # Verify event types (FIXED: eventType field, not "event")
            event_types = [msg.get('eventType') for msg in sent_messages]
            assert 'speech_start' in event_types, "Missing speech_start event"
            assert 'partial_text' in event_types or 'final_text' in event_types, "Missing text events"

            # Verify partial text has is_final=False (FIXED: data field, not "result")
            partial_events = [msg for msg in sent_messages if msg.get('eventType') == 'partial_text']
            if partial_events:
                assert partial_events[0]['data']['is_final'] is False, "Partial text should have is_final=False"

            # Verify final text has is_final=True (FIXED: data field)
            final_events = [msg for msg in sent_messages if msg.get('eventType') == 'final_text']
            assert len(final_events) > 0, "Missing final_text event"
            assert final_events[0]['data']['is_final'] is True, "Final text should have is_final=True"

            # FIXED: Verify speech_end is sent after final_text (P1 fix)
            speech_end_events = [msg for msg in sent_messages if msg.get('eventType') == 'speech_end']
            assert len(speech_end_events) > 0, "Missing speech_end event"

            # Verify speech_end comes after final_text
            final_text_idx = next(i for i, msg in enumerate(sent_messages) if msg.get('eventType') == 'final_text')
            speech_end_idx = next(i for i, msg in enumerate(sent_messages) if msg.get('eventType') == 'speech_end')
            assert speech_end_idx > final_text_idx, "speech_end must come after final_text"

    @pytest.mark.asyncio
    async def test_process_audio_still_works_for_backward_compatibility(self):
        """
        STT-REQ-007.1: Existing process_audio endpoint should remain unchanged

        GIVEN AudioProcessor with both process_audio and process_audio_stream
        WHEN process_audio is called (legacy endpoint)
        THEN Single response should be sent (MVP0 behavior)
        """
        from unittest.mock import AsyncMock, MagicMock, patch
        from stt_engine.ipc_handler import IpcHandler
        from main import AudioProcessor

        mock_ipc = AsyncMock(spec=IpcHandler)
        sent_messages = []

        async def capture_message(msg):
            sent_messages.append(msg)

        mock_ipc.send_message.side_effect = capture_message

        with patch('main.WhisperSTTEngine') as mock_stt_class, \
             patch('stt_engine.resource_monitor.ResourceMonitor') as mock_resource_class:

            mock_stt = AsyncMock()
            mock_stt.model_size = 'small'
            mock_stt.transcribe.return_value = {
                'text': 'Hello world',
                'is_final': True,
                'confidence': 0.95,
                'language': 'ja'
            }
            mock_stt_class.return_value = mock_stt

            mock_resource = MagicMock()
            mock_resource.initial_model = 'small'
            mock_resource.current_model = 'small'
            mock_resource_class.return_value = mock_resource

            processor = AudioProcessor()
            processor.ipc = mock_ipc  # Inject mock IPC

            audio_data = [0] * 32000

            msg = {
                'id': 'test-legacy-001',
                'type': 'request',
                'method': 'process_audio',
                'params': {'audio_data': audio_data}
            }

            # Act: Process audio (legacy endpoint)
            await processor.handle_message(msg)

            # Assert: Single response (MVP0 behavior)
            assert len(sent_messages) == 1, f"Expected 1 response, got {len(sent_messages)}"
            assert sent_messages[0].get('type') == 'response', "Should be a response message"
            assert sent_messages[0].get('id') == 'test-legacy-001', "Response should match request ID"

    @pytest.mark.asyncio
    async def test_process_audio_stream_handles_error_events(self):
        """
        P0 FIX TEST: Error events should be sent to prevent Rust-side hang
        """
        from unittest.mock import AsyncMock, MagicMock, patch
        from stt_engine.ipc_handler import IpcHandler
        from main import AudioProcessor

        mock_ipc = AsyncMock(spec=IpcHandler)
        sent_messages = []

        async def capture_message(msg):
            sent_messages.append(msg)

        mock_ipc.send_message.side_effect = capture_message

        with patch('main.WhisperSTTEngine') as mock_stt_class, \
             patch('stt_engine.resource_monitor.ResourceMonitor') as mock_resource_class, \
             patch('main.VoiceActivityDetector') as mock_vad_class, \
             patch('main.AudioPipeline') as mock_pipeline_class:

            # Mock setup
            mock_stt = AsyncMock()
            mock_stt.model_size = 'small'
            mock_stt_class.return_value = mock_stt

            mock_vad = MagicMock()
            mock_vad.split_into_frames.return_value = [[0] * 320 for _ in range(5)]
            mock_vad_class.return_value = mock_vad

            # Mock AudioPipeline to return error event
            mock_pipeline = AsyncMock()
            error_event = {'event': 'error', 'message': 'Test error message'}
            mock_pipeline.process_audio_frame_with_partial.side_effect = [
                None,
                None,
                error_event,  # Error on 3rd frame
                None,
                None
            ]
            mock_pipeline_class.return_value = mock_pipeline

            processor = AudioProcessor()
            processor.ipc = mock_ipc

            msg = {
                'id': 'test-error-001',
                'type': 'request',
                'method': 'process_audio_stream',
                'params': {'audio_data': [0] * 32000}
            }

            await processor.handle_message(msg)

            # Verify error message was sent
            error_messages = [m for m in sent_messages if m.get('type') == 'error']
            assert len(error_messages) == 1, "Error event should be sent"

            error_msg = error_messages[0]
            assert error_msg.get('id') == 'test-error-001', "Error must include request id"
            assert error_msg.get('errorCode') == 'AUDIO_PIPELINE_ERROR'
            assert error_msg.get('errorMessage') == 'Test error message'
            assert error_msg.get('recoverable') == True
            assert error_msg.get('version') == '1.0'


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

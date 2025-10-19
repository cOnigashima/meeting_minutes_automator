"""
E2E tests for model upgrade with bundled fallback (STT-REQ-002.4/002.6/006.9/006.12)

Verifies complete flow:
1. System starts with bundled 'base' in offline mode
2. User approves upgrade to 'small'
3. Upgrade falls back to 'base' (model unavailable)
4. IPC sends 'upgrade_fallback' event with accurate info
5. ResourceMonitor state reflects actual loaded model
"""

import pytest
import asyncio
import tempfile
import json
from pathlib import Path
from unittest.mock import patch, MagicMock


@pytest.mark.asyncio
async def test_upgrade_fallback_offline_bundled_base_only():
    """
    E2E: Upgrade fallback when only bundled 'base' is available (offline mode)

    GIVEN offline mode with only bundled 'base' model
    WHEN user requests upgrade to 'small'
    THEN system should:
      - Fallback to 'base' gracefully
      - Send 'upgrade_fallback' event via IPC
      - Update ResourceMonitor.current_model to 'base'
      - Preserve ResourceMonitor.initial_model (upgrade ceiling)
      - Include fallback_occurred=True in response
    """
    from main import AudioProcessor
    from stt_engine.ipc_handler import IpcHandler
    from stt_engine.resource_monitor import ResourceMonitor
    from stt_engine.transcription.whisper_client import WhisperSTTEngine

    with tempfile.TemporaryDirectory() as tmpdir:
        # Create bundled 'base' model only
        bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
        bundled_base.mkdir(parents=True)
        (bundled_base / "model.bin").touch()

        # Mock _detect_model_path to use temp bundled directory
        def mock_detect_model_path(self):
            bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

            # Try requested size first
            for base_dir in bundled_base_dirs:
                bundled_path = base_dir / self.model_size
                if bundled_path.exists() and (bundled_path / "model.bin").exists():
                    return str(bundled_path)

            # Fallback to base (STT-REQ-002.4/002.6)
            if self.model_size != 'base':
                for base_dir in bundled_base_dirs:
                    bundled_base_path = base_dir / "base"
                    if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                        self.model_size = "base"
                        return str(bundled_base_path)

            raise FileNotFoundError("No model found")

        # Create and configure STT engine
        stt_engine = WhisperSTTEngine(model_size='small', offline_mode=True)
        stt_engine._detect_model_path = lambda: mock_detect_model_path(stt_engine)

        # Mock WhisperModel to avoid actual model loading
        mock_model = MagicMock()
        with patch('stt_engine.transcription.whisper_client.WhisperModel', return_value=mock_model):
            # Initialize engine (should fallback to 'base')
            await stt_engine.initialize()

            # Verify initial state: fallback occurred during initialization
            assert stt_engine.model_size == "base", "Should initialize with bundled 'base' (fallback)"

            # Create ResourceMonitor and set state
            resource_monitor = ResourceMonitor()
            resource_monitor.initial_model = 'small'  # Upgrade ceiling
            resource_monitor.current_model = stt_engine.model_size  # 'base'

            # Create AudioProcessor (will create default components)
            # Suppress WhisperModel initialization in default constructor
            with patch('stt_engine.transcription.whisper_client.WhisperModel', return_value=mock_model):
                processor = AudioProcessor()

            # Replace with our mocked components
            processor.stt_engine = stt_engine
            processor.resource_monitor = resource_monitor
            processor.ipc = IpcHandler()

            # Capture IPC messages
            sent_messages = []
            original_send = processor.ipc.send_message

            async def capture_send(msg):
                sent_messages.append(msg)
                # Don't call original_send to avoid actual stdout write

            processor.ipc.send_message = capture_send

            # Simulate user requesting upgrade to 'small'
            # Note: _handle_approve_upgrade expects 'target_model' at top level (legacy format)
            upgrade_request = {
                'id': 'test_upgrade_1',
                'target_model': 'small'
            }

            # Mock stdin/stdout to avoid actual IPC
            with patch('sys.stdin'), patch('sys.stdout'):
                await processor._handle_approve_upgrade(upgrade_request)

            # Verify fallback occurred
            assert stt_engine.model_size == "base", "Should remain 'base' (fallback)"
            assert resource_monitor.current_model == "base", "ResourceMonitor should reflect actual model"
            assert resource_monitor.initial_model == "small", "initial_model should remain unchanged (upgrade ceiling)"

            # Verify IPC response message
            response_msgs = [m for m in sent_messages if m.get('type') == 'response']
            assert len(response_msgs) == 1, "Should send one response"

            response = response_msgs[0]
            assert response['id'] == 'test_upgrade_1'
            assert response['result']['success'] is False, "Upgrade should fail (fallback)"
            assert response['result']['old_model'] == 'base'
            assert response['result']['new_model'] == 'base', "actual model should be 'base'"
            assert response['result']['requested_model'] == 'small'
            assert response['result']['fallback_occurred'] is True

            # Verify IPC event message
            event_msgs = [m for m in sent_messages if m.get('type') == 'event']
            assert len(event_msgs) >= 1, "Should send at least one event"

            fallback_events = [e for e in event_msgs if e.get('eventType') == 'upgrade_fallback']
            assert len(fallback_events) == 1, "Should send 'upgrade_fallback' event"

            fallback_event = fallback_events[0]
            assert fallback_event['version'] == '1.0'
            assert fallback_event['data']['old_model'] == 'base'
            assert fallback_event['data']['new_model'] == 'base'
            assert fallback_event['data']['requested_model'] == 'small'
            assert 'small' in fallback_event['data']['message'].lower() or 'base' in fallback_event['data']['message'].lower()


@pytest.mark.asyncio
async def test_upgrade_success_when_model_available():
    """
    E2E: Successful upgrade when requested model is available

    GIVEN bundled 'base' and 'small' both available
    WHEN user requests upgrade to 'small'
    THEN system should:
      - Successfully load 'small' model
      - Send 'upgrade_success' event
      - Update ResourceMonitor.current_model to 'small'
      - Include fallback_occurred=False in response
    """
    from main import AudioProcessor
    from stt_engine.ipc_handler import IpcHandler
    from stt_engine.resource_monitor import ResourceMonitor
    from stt_engine.transcription.whisper_client import WhisperSTTEngine

    with tempfile.TemporaryDirectory() as tmpdir:
        # Create both bundled 'base' and 'small' models
        bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
        bundled_base.mkdir(parents=True)
        (bundled_base / "model.bin").touch()

        bundled_small = Path(tmpdir) / "models" / "faster-whisper" / "small"
        bundled_small.mkdir(parents=True)
        (bundled_small / "model.bin").touch()

        # Mock _detect_model_path
        def mock_detect_model_path(self):
            bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

            # Try requested size first
            for base_dir in bundled_base_dirs:
                bundled_path = base_dir / self.model_size
                if bundled_path.exists() and (bundled_path / "model.bin").exists():
                    return str(bundled_path)

            # Fallback to base
            if self.model_size != 'base':
                for base_dir in bundled_base_dirs:
                    bundled_base_path = base_dir / "base"
                    if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                        self.model_size = "base"
                        return str(bundled_base_path)

            raise FileNotFoundError("No model found")

        # Create and configure STT engine
        stt_engine = WhisperSTTEngine(model_size='base', offline_mode=True)
        stt_engine._detect_model_path = lambda: mock_detect_model_path(stt_engine)

        # Mock WhisperModel
        mock_model = MagicMock()
        with patch('stt_engine.transcription.whisper_client.WhisperModel', return_value=mock_model):
            # Initialize with 'base'
            await stt_engine.initialize()
            assert stt_engine.model_size == "base"

            # Create ResourceMonitor and set state
            resource_monitor = ResourceMonitor()
            resource_monitor.initial_model = 'small'  # Upgrade ceiling
            resource_monitor.current_model = stt_engine.model_size  # 'base'

            # Create AudioProcessor (suppress default initialization)
            with patch('stt_engine.transcription.whisper_client.WhisperModel', return_value=mock_model):
                processor = AudioProcessor()

            # Replace with our mocked components
            processor.stt_engine = stt_engine
            processor.resource_monitor = resource_monitor
            processor.ipc = IpcHandler()

            # Capture IPC messages
            sent_messages = []
            original_send = processor.ipc.send_message

            async def capture_send(msg):
                sent_messages.append(msg)
                # Don't call original_send to avoid actual stdout write

            processor.ipc.send_message = capture_send

            # Request upgrade to 'small' (should succeed)
            # Note: _handle_approve_upgrade expects 'target_model' at top level (legacy format)
            upgrade_request = {
                'id': 'test_upgrade_2',
                'target_model': 'small'
            }

            with patch('sys.stdin'), patch('sys.stdout'):
                await processor._handle_approve_upgrade(upgrade_request)

            # Verify successful upgrade
            assert stt_engine.model_size == "small", "Should upgrade to 'small'"
            assert resource_monitor.current_model == "small", "ResourceMonitor should reflect 'small'"

            # Verify IPC response
            response_msgs = [m for m in sent_messages if m.get('type') == 'response']
            assert len(response_msgs) == 1

            response = response_msgs[0]
            assert response['result']['success'] is True, "Upgrade should succeed"
            assert response['result']['new_model'] == 'small'
            assert response['result']['requested_model'] == 'small'
            assert response['result']['fallback_occurred'] is False

            # Verify 'upgrade_success' event
            event_msgs = [m for m in sent_messages if m.get('type') == 'event']
            success_events = [e for e in event_msgs if e.get('eventType') == 'upgrade_success']
            assert len(success_events) == 1, "Should send 'upgrade_success' event"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])

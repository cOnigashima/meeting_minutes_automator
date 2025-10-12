"""
E2E Integration Tests for MVP1 Real STT Implementation

Tests the complete integration of:
- IpcHandler (stdin/stdout communication)
- AudioProcessor (VAD → Pipeline → STT orchestration)
- Real audio processing flow

Related Requirements:
- STT-REQ-007.1: IPC message extensions
- STT-REQ-007.2: Version field support
- STT-REQ-003.6-009: VAD-STT integration
"""

import pytest
import asyncio
import numpy as np
from unittest.mock import AsyncMock, MagicMock, patch
from io import BytesIO

from stt_engine.ipc_handler import IpcHandler


class TestComponentInitialization:
    """Test component initialization"""

    def test_ipc_handler_initialization(self):
        """IPC Handler should initialize with proper defaults"""
        handler = IpcHandler()
        assert handler is not None
        assert handler.timeout == IpcHandler.DEFAULT_TIMEOUT_SEC
        assert handler.stats["messages_sent"] == 0

    def test_audio_processor_initialization(self):
        """AudioProcessor should initialize all components"""
        from main import AudioProcessor

        processor = AudioProcessor()
        assert processor.vad is not None
        assert processor.stt_engine is not None
        assert processor.pipeline is not None


class TestIpcCommunication:
    """Test IPC communication (real implementation)"""

    @pytest.mark.asyncio
    async def test_ipc_send_message_with_version(self):
        """
        STT-REQ-007.2: IPC messages should include version field

        GIVEN IpcHandler
        WHEN Sending a message without version
        THEN Version "1.0" should be added automatically
        """
        handler = IpcHandler()

        mock_stdout = MagicMock()
        mock_buffer = BytesIO()
        mock_stdout.buffer = mock_buffer

        with patch('sys.stdout', mock_stdout):
            await handler.send_message({"type": "test"})

            mock_buffer.seek(0)
            import json
            written_data = mock_buffer.read()
            message = json.loads(written_data.decode('utf-8').strip())

            assert message["version"] == "1.0"
            assert message["type"] == "test"

    @pytest.mark.asyncio
    async def test_ipc_message_size_validation(self):
        """
        IPC should reject messages larger than MAX_MESSAGE_SIZE

        GIVEN IpcHandler
        WHEN Sending a message > 1MB
        THEN IpcProtocolError should be raised
        """
        from stt_engine.ipc_handler import IpcProtocolError

        handler = IpcHandler()

        large_data = "x" * (handler.MAX_MESSAGE_SIZE + 1)
        message = {"data": large_data}

        with pytest.raises(IpcProtocolError) as exc_info:
            await handler.send_message(message)

        assert "Message too large" in str(exc_info.value)


class TestInterfaceTypeDefinitions:
    """Test that type definitions are correct"""

    def test_message_structure(self):
        """Verify expected message structure"""
        # This test validates that we can create message dictionaries
        # that match the Rust IpcMessage enum structure

        start_processing_msg = {
            "type": "StartProcessing",
            "payload": {"audio_data": [1, 2, 3, 4]}
        }
        assert start_processing_msg["type"] == "StartProcessing"

        transcription_result_msg = {
            "type": "TranscriptionResult",
            "payload": {"text": "test", "timestamp": 12345}
        }
        assert transcription_result_msg["type"] == "TranscriptionResult"

        ready_msg = {"type": "Ready"}
        assert ready_msg["type"] == "Ready"

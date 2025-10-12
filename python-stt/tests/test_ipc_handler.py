"""
Unit tests for IpcHandler
Test-Driven Development for IPC communication

Requirements:
- STT-REQ-007: IPC Protocol Extension (Backward Compatible)
- STT-REQ-007.1: Add new fields
- STT-REQ-007.2: Version field support
- STT-REQ-007.3: Unknown field handling
"""

import pytest
import asyncio
import json
import sys
from io import StringIO, BytesIO
from unittest.mock import MagicMock, patch, AsyncMock
from stt_engine.ipc_handler import (
    IpcHandler,
    IpcProtocolError,
    IpcTimeoutError
)


class TestIpcHandlerInitialization:
    """Test IpcHandler initialization"""

    def test_init_with_defaults(self):
        """WHEN IpcHandler is initialized with defaults
        THEN should set proper default values"""
        handler = IpcHandler()

        assert handler.timeout == IpcHandler.DEFAULT_TIMEOUT_SEC
        assert handler.message_handler is None
        assert not handler._running
        assert handler.stats["messages_sent"] == 0
        assert handler.stats["messages_received"] == 0

    def test_init_with_custom_timeout(self):
        """WHEN IpcHandler is initialized with custom timeout
        THEN should use the provided timeout"""
        handler = IpcHandler(timeout=5.0)

        assert handler.timeout == 5.0

    def test_init_with_message_handler(self):
        """WHEN IpcHandler is initialized with message handler
        THEN should store the handler"""
        async def handler(msg):
            pass

        ipc = IpcHandler(message_handler=handler)
        assert ipc.message_handler == handler


class TestIpcMessageSending:
    """Test message sending functionality"""

    @pytest.mark.asyncio
    async def test_send_message_adds_version(self):
        """STT-REQ-007.2: WHEN sending a message without version
        THEN should add protocol version"""
        handler = IpcHandler()

        # Create mock stdout with buffer
        mock_stdout = MagicMock()
        mock_buffer = BytesIO()
        mock_stdout.buffer = mock_buffer

        with patch('sys.stdout', mock_stdout):
            await handler.send_message({"type": "test"})

            # Get written data
            mock_buffer.seek(0)
            written_data = mock_buffer.read()
            message = json.loads(written_data.decode('utf-8').strip())
            assert message["version"] == "1.0"

    @pytest.mark.asyncio
    async def test_send_message_adds_timestamp(self):
        """WHEN sending a message without timestamp
        THEN should add current timestamp"""
        handler = IpcHandler()

        # Create mock stdout with buffer
        mock_stdout = MagicMock()
        mock_buffer = BytesIO()
        mock_stdout.buffer = mock_buffer

        with patch('sys.stdout', mock_stdout):
            await handler.send_message({"type": "test"})

            mock_buffer.seek(0)
            written_data = mock_buffer.read()
            message = json.loads(written_data.decode('utf-8').strip())
            assert "timestamp" in message
            assert isinstance(message["timestamp"], int)

    @pytest.mark.asyncio
    async def test_send_message_validates_size(self):
        """WHEN sending a message larger than MAX_MESSAGE_SIZE
        THEN should raise IpcProtocolError"""
        handler = IpcHandler()

        # Create a message larger than 1MB
        large_data = "x" * (handler.MAX_MESSAGE_SIZE + 1)
        message = {"data": large_data}

        with pytest.raises(IpcProtocolError) as exc_info:
            await handler.send_message(message)

        assert "Message too large" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_send_message_increments_stats(self):
        """WHEN sending a message successfully
        THEN should increment messages_sent counter"""
        handler = IpcHandler()

        # Create mock stdout with buffer
        mock_stdout = MagicMock()
        mock_buffer = BytesIO()
        mock_stdout.buffer = mock_buffer

        with patch('sys.stdout', mock_stdout):
            await handler.send_message({"type": "test"})

            assert handler.stats["messages_sent"] == 1


class TestIpcMessageReceiving:
    """Test message receiving functionality"""

    @pytest.mark.asyncio
    async def test_receive_message_parses_json(self):
        """WHEN receiving valid JSON message
        THEN should parse and return the message"""
        handler = IpcHandler()
        test_message = {"type": "test", "data": "value"}

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.return_value = json.dumps(test_message)

            message = await handler.receive_message()

            assert message == test_message
            assert handler.stats["messages_received"] == 1

    @pytest.mark.asyncio
    async def test_receive_message_handles_timeout(self):
        """WHEN receive times out
        THEN should raise IpcTimeoutError"""
        handler = IpcHandler(timeout=0.1)

        async def slow_read():
            await asyncio.sleep(1.0)  # Longer than timeout
            return '{"type": "test"}'

        with patch.object(handler, '_read_line_async', side_effect=slow_read):
            with pytest.raises(IpcTimeoutError) as exc_info:
                await handler.receive_message()

            assert "Receive timeout" in str(exc_info.value)
            assert handler.stats["timeouts"] == 1

    @pytest.mark.asyncio
    async def test_receive_message_validates_json(self):
        """WHEN receiving invalid JSON
        THEN should raise IpcProtocolError"""
        handler = IpcHandler()

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.return_value = "invalid json {"

            with pytest.raises(IpcProtocolError) as exc_info:
                await handler.receive_message()

            assert "Invalid JSON" in str(exc_info.value)
            assert handler.stats["errors"] == 1

    @pytest.mark.asyncio
    async def test_receive_message_version_mismatch_warning(self):
        """STT-REQ-007.3: WHEN receiving message with different version
        THEN should log warning but continue processing"""
        handler = IpcHandler()
        test_message = {"type": "test", "version": "2.0"}

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.return_value = json.dumps(test_message)

            with patch('stt_engine.ipc_handler.logger') as mock_logger:
                message = await handler.receive_message()

                # Should still return the message
                assert message == test_message

                # Should log warning about version mismatch
                mock_logger.warning.assert_called_once()
                warning_msg = mock_logger.warning.call_args[0][0]
                assert "version mismatch" in warning_msg.lower()

    @pytest.mark.asyncio
    async def test_receive_message_handles_unknown_fields(self):
        """STT-REQ-007.3: WHEN receiving message with unknown fields
        THEN should process normally (forward compatibility)"""
        handler = IpcHandler()
        test_message = {
            "type": "test",
            "version": "1.0",
            "unknown_field": "value",
            "future_feature": 123
        }

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.return_value = json.dumps(test_message)

            message = await handler.receive_message()

            # Should return complete message including unknown fields
            assert message == test_message
            assert message["unknown_field"] == "value"
            assert message["future_feature"] == 123


class TestIpcEventLoop:
    """Test IPC event loop functionality"""

    @pytest.mark.asyncio
    async def test_start_processes_messages(self):
        """WHEN event loop is started
        THEN should process incoming messages"""
        messages_processed = []

        async def message_handler(msg):
            messages_processed.append(msg)

        handler = IpcHandler(message_handler=message_handler)

        # Mock messages to receive
        test_messages = [
            '{"type": "test1"}',
            '{"type": "test2"}',
            None  # EOF to stop loop
        ]

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.side_effect = test_messages

            await handler.start()

            assert len(messages_processed) == 2
            assert messages_processed[0]["type"] == "test1"
            assert messages_processed[1]["type"] == "test2"

    @pytest.mark.asyncio
    async def test_start_handles_handler_errors(self):
        """WHEN message handler raises error
        THEN should send error response and continue"""
        async def failing_handler(msg):
            raise ValueError("Handler error")

        handler = IpcHandler(message_handler=failing_handler)
        sent_messages = []

        # Capture sent messages
        original_send = handler.send_message
        async def mock_send(msg):
            sent_messages.append(msg)
        handler.send_message = mock_send

        test_messages = [
            '{"type": "test"}',
            None  # EOF
        ]

        with patch.object(handler, '_read_line_async', new_callable=AsyncMock) as mock_read:
            mock_read.side_effect = test_messages

            await handler.start()

            # Should have sent an error response
            assert len(sent_messages) == 1
            assert sent_messages[0]["type"] == "error"
            assert "Handler error" in sent_messages[0]["error"]

    @pytest.mark.asyncio
    async def test_stop_sends_shutdown_ack(self):
        """WHEN stop is called
        THEN should send shutdown acknowledgment"""
        handler = IpcHandler()
        sent_messages = []

        # Capture sent messages
        async def mock_send(msg):
            sent_messages.append(msg)
        handler.send_message = mock_send

        await handler.stop()

        assert len(sent_messages) == 1
        assert sent_messages[0]["type"] == "shutdown_ack"
        assert "stats" in sent_messages[0]


class TestIpcStatistics:
    """Test IPC statistics tracking"""

    def test_get_stats_returns_copy(self):
        """WHEN get_stats is called
        THEN should return a copy of stats"""
        handler = IpcHandler()

        stats1 = handler.get_stats()
        stats1["modified"] = True

        stats2 = handler.get_stats()
        assert "modified" not in stats2

        # Original stats should be unchanged
        assert "modified" not in handler.stats


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
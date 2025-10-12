"""
IPC Handler for communication with Rust parent process
MVP1 Real STT - Production Implementation

Requirements:
- STT-REQ-007: IPC Protocol Extension (Backward Compatible)
- STT-REQ-007.1: Add new fields (confidence, language, processing_time_ms)
- STT-REQ-007.2: Version field support
- STT-REQ-007.3: Unknown field handling (forward compatibility)
"""

import asyncio
import json
import sys
import logging
from typing import Dict, Any, Optional, Callable, Awaitable
from asyncio import StreamReader, StreamWriter
import time

logger = logging.getLogger(__name__)


class IpcProtocolError(Exception):
    """IPC protocol specific errors"""
    pass


class IpcTimeoutError(Exception):
    """IPC timeout errors"""
    pass


class IpcHandler:
    """
    Handler for IPC communication via stdin/stdout.

    Design principles:
    - Non-blocking async I/O
    - JSON message format with newline delimiter
    - Timeout protection
    - Buffer overflow prevention
    - Graceful error handling
    """

    # Protocol version (STT-REQ-007.2)
    PROTOCOL_VERSION = "1.0"

    # Buffer limits to prevent overflow
    MAX_MESSAGE_SIZE = 1024 * 1024  # 1MB max per message
    READ_BUFFER_SIZE = 8192  # 8KB chunks

    # Timeout settings
    DEFAULT_TIMEOUT_SEC = 10.0

    def __init__(
        self,
        message_handler: Optional[Callable[[Dict[str, Any]], Awaitable[None]]] = None,
        timeout: float = DEFAULT_TIMEOUT_SEC
    ):
        """
        Initialize IPC handler.

        Args:
            message_handler: Optional async callback for handling received messages
            timeout: Default timeout for I/O operations in seconds
        """
        self.message_handler = message_handler
        self.timeout = timeout
        self._running = False
        self._reader: Optional[StreamReader] = None
        self._writer: Optional[StreamWriter] = None
        self._buffer = bytearray()

        # Statistics for monitoring
        self.stats = {
            "messages_sent": 0,
            "messages_received": 0,
            "errors": 0,
            "timeouts": 0
        }

        logger.info(f"IpcHandler initialized with timeout={timeout}s")

    async def send_message(self, message: Dict[str, Any]) -> None:
        """
        Send a message to the Rust parent process via stdout.

        Args:
            message: Dictionary to send as JSON

        Raises:
            IpcProtocolError: If message is invalid or too large
            IpcTimeoutError: If send times out
        """
        try:
            # Add protocol version if not present (STT-REQ-007.2)
            if "version" not in message:
                message["version"] = self.PROTOCOL_VERSION

            # Add timestamp for latency tracking
            if "timestamp" not in message:
                message["timestamp"] = int(time.time() * 1000)

            # Serialize to JSON with compact encoding
            json_str = json.dumps(message, separators=(',', ':'))

            # Check message size
            if len(json_str) > self.MAX_MESSAGE_SIZE:
                raise IpcProtocolError(
                    f"Message too large: {len(json_str)} bytes > {self.MAX_MESSAGE_SIZE} bytes"
                )

            # Add newline delimiter
            data = (json_str + '\n').encode('utf-8')

            # Write to stdout with timeout
            sys.stdout.buffer.write(data)
            sys.stdout.buffer.flush()

            self.stats["messages_sent"] += 1
            logger.debug(f"Sent message: type={message.get('type')}, size={len(data)} bytes")

        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Failed to send message: {e}")
            raise IpcProtocolError(f"Send failed: {e}") from e

    async def receive_message(self) -> Optional[Dict[str, Any]]:
        """
        Receive a message from the Rust parent process via stdin.

        Returns:
            Parsed message dictionary or None if no complete message available

        Raises:
            IpcProtocolError: If message is malformed
            IpcTimeoutError: If receive times out
        """
        try:
            # Read from stdin with timeout
            line = await asyncio.wait_for(
                self._read_line_async(),
                timeout=self.timeout
            )

            if not line:
                return None

            # Parse JSON
            message = json.loads(line)

            # Validate message structure
            if not isinstance(message, dict):
                raise IpcProtocolError(f"Invalid message format: expected dict, got {type(message)}")

            # Log version mismatch as warning but continue (STT-REQ-007.3)
            if "version" in message:
                msg_version = message["version"]
                if msg_version != self.PROTOCOL_VERSION:
                    logger.warning(
                        f"Protocol version mismatch: received {msg_version}, expected {self.PROTOCOL_VERSION}"
                    )

            self.stats["messages_received"] += 1
            logger.debug(f"Received message: type={message.get('type')}, version={message.get('version')}")

            return message

        except asyncio.TimeoutError:
            self.stats["timeouts"] += 1
            logger.warning(f"Receive timeout after {self.timeout}s")
            raise IpcTimeoutError(f"Receive timeout after {self.timeout}s")
        except json.JSONDecodeError as e:
            self.stats["errors"] += 1
            logger.error(f"Invalid JSON received: {e}")
            raise IpcProtocolError(f"Invalid JSON: {e}") from e
        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Failed to receive message: {e}")
            raise IpcProtocolError(f"Receive failed: {e}") from e

    async def _read_line_async(self) -> Optional[str]:
        """
        Asynchronously read a line from stdin.

        Returns:
            Line string without trailing newline, or None if EOF
        """
        loop = asyncio.get_event_loop()

        # Use ThreadPoolExecutor for blocking I/O
        line_bytes = await loop.run_in_executor(
            None,
            sys.stdin.buffer.readline
        )

        if not line_bytes:
            return None

        # Decode and strip newline
        return line_bytes.decode('utf-8').rstrip('\n\r')

    async def start(self) -> None:
        """
        Start the IPC handler event loop.

        Continuously receives and processes messages until stopped.
        """
        if self._running:
            logger.warning("IpcHandler already running")
            return

        self._running = True
        logger.info("IpcHandler started")

        try:
            while self._running:
                try:
                    # Receive next message
                    message = await self.receive_message()

                    if message is None:
                        # EOF reached, clean shutdown
                        logger.info("IPC input closed, shutting down")
                        break

                    # Process message if handler provided
                    if self.message_handler:
                        try:
                            await self.message_handler(message)
                        except Exception as e:
                            logger.error(f"Message handler error: {e}")
                            # Send error response
                            await self.send_message({
                                "type": "error",
                                "error": str(e),
                                "original_message_type": message.get("type")
                            })

                except IpcTimeoutError:
                    # Timeout is normal during idle, continue
                    continue
                except IpcProtocolError as e:
                    # Protocol error, log but continue
                    logger.error(f"Protocol error: {e}")
                    continue

        finally:
            self._running = False
            logger.info(f"IpcHandler stopped. Stats: {self.stats}")

    async def stop(self) -> None:
        """Stop the IPC handler gracefully."""
        logger.info("Stopping IpcHandler")
        self._running = False

        # Send shutdown acknowledgment
        try:
            await self.send_message({
                "type": "shutdown_ack",
                "stats": self.stats
            })
        except Exception as e:
            logger.error(f"Failed to send shutdown acknowledgment: {e}")

    def get_stats(self) -> Dict[str, Any]:
        """Get IPC statistics for monitoring."""
        return self.stats.copy()

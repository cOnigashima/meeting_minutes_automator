#!/usr/bin/env python3
"""
Python Sidecar Process - MVP1 Real STT Implementation
Task 4.3: AudioPipeline + IpcHandler Integration

Handles stdin/stdout JSON IPC communication with Tauri backend.
Orchestrates VAD → AudioPipeline → STT flow for real audio processing.

Related Requirements:
- STT-REQ-003.6: Final text generation on speech segment completion
- STT-REQ-003.7: Partial text generation during speech (1s interval)
- STT-REQ-003.8: Partial text with is_final=False
- STT-REQ-003.9: Final text with is_final=True
"""

import sys
import asyncio
import logging
from typing import Dict, Any

from stt_engine.ipc_handler import IpcHandler
from stt_engine.audio_pipeline import AudioPipeline
from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector
from stt_engine.transcription.whisper_client import WhisperSTTEngine

# ログ設定
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr  # stdoutはIPCに使うのでstderrに出力
)
logger = logging.getLogger(__name__)


class AudioProcessor:
    """
    Audio processing orchestrator for VAD → Pipeline → STT flow.

    This class integrates all components for real audio processing:
    - VoiceActivityDetector: Speech detection
    - WhisperSTTEngine: Transcription
    - AudioPipeline: Coordination between VAD and STT
    - IpcHandler: Communication with Rust backend
    """

    def __init__(self):
        """Initialize audio processing components."""
        logger.info("Initializing AudioProcessor...")

        # Initialize components (STT-REQ-003.1, STT-REQ-002.1)
        self.vad = VoiceActivityDetector(sample_rate=16000, aggressiveness=2)
        self.stt_engine = WhisperSTTEngine(auto_select_model=True)
        self.pipeline = AudioPipeline(vad=self.vad, stt_engine=self.stt_engine)
        self.ipc = None

        logger.info("AudioProcessor initialized successfully")

    async def handle_message(self, msg: Dict[str, Any]) -> None:
        """
        Handle incoming IPC messages from Rust backend.

        Args:
            msg: IPC message dictionary

        Message types:
        - process_audio: Process audio frames through VAD→Pipeline→STT
        - ping: Health check (respond with pong)
        - shutdown: Graceful shutdown
        """
        msg_type = msg.get('type')
        msg_id = msg.get('id', 'unknown')

        try:
            if msg_type == 'process_audio':
                await self._handle_process_audio(msg)

            elif msg_type == 'ping':
                await self.ipc.send_message({
                    'type': 'pong',
                    'id': msg_id
                })

            elif msg_type == 'shutdown':
                logger.info("Shutdown requested")
                await self.ipc.stop()

            else:
                logger.warning(f"Unknown message type: {msg_type}")
                await self.ipc.send_message({
                    'type': 'error',
                    'id': msg_id,
                    'error': f"Unknown message type: {msg_type}"
                })

        except Exception as e:
            logger.error(f"Error handling message: {e}", exc_info=True)
            await self.ipc.send_message({
                'type': 'error',
                'id': msg_id,
                'error': str(e)
            })

    async def _handle_process_audio(self, msg: Dict[str, Any]) -> None:
        """
        Process audio data and return SINGLE response (MVP0 compatible).

        This implementation maintains Request-Response protocol from MVP0:
        - 1 process_audio request → 1 response with final transcription
        - Intermediate events (speech_start, partial_text) are logged but not sent
        - Real-time partial text delivery will be implemented in Task 7 (IPC Extension)

        Related Requirements:
        - STT-REQ-007.1: Maintain existing message format (Request-Response)
        - STT-REQ-003.6: Final text on speech end
        - STT-REQ-003.9: Final text with is_final=True

        Args:
            msg: IPC message with audio_data field
        """
        msg_id = msg.get('id', 'unknown')
        audio_data = msg.get('audio_data', [])

        if not audio_data:
            logger.warning("Empty audio_data received")
            # Send empty response for empty audio
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': None
            })
            return

        # Convert audio data (u8 array from Rust) to bytes
        audio_bytes = bytes(audio_data)

        # Split into 10ms frames (STT-REQ-003.2)
        frames = self.vad.split_into_frames(audio_bytes)

        # Process all frames and collect events (no immediate send)
        events = []
        for frame in frames:
            # Use process_audio_frame_with_partial for partial text support
            result = await self.pipeline.process_audio_frame_with_partial(frame)

            if result:
                events.append(result)
                event_type = result.get('event')
                logger.debug(f"VAD event collected: {event_type}")

        # Send ONLY final transcription result (if any)
        final_event = next(
            (e for e in reversed(events) if e.get('event') == 'final_text'),
            None
        )

        if final_event:
            transcription = final_event['transcription']
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': {
                    'text': transcription['text'],
                    'is_final': transcription['is_final'],
                    'confidence': transcription.get('confidence', 0.0),
                    'language': transcription.get('language', 'ja'),
                    'processing_time_ms': transcription.get('processing_time_ms', 0),
                }
            })
            logger.info(f"Sent final transcription: {transcription['text'][:50]}...")
        else:
            # No transcription yet - send empty response
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': None
            })
            logger.debug("No final transcription yet, sent empty response")


async def main():
    """
    Main entry point for Python sidecar process.

    Initializes AudioProcessor and IpcHandler, then starts IPC event loop.
    """
    try:
        # Initialize audio processor
        processor = AudioProcessor()

        # Initialize IPC handler with message callback
        processor.ipc = IpcHandler(message_handler=processor.handle_message)

        # Send ready signal to Rust backend
        await processor.ipc.send_message({
            'type': 'ready',
            'message': 'Python sidecar ready (MVP1 Real STT)'
        })

        logger.info("Starting IPC event loop...")

        # Start IPC event loop (blocks until shutdown)
        await processor.ipc.start()

        logger.info("IPC event loop stopped")

    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt, shutting down...")

    except Exception as e:
        logger.error(f"Fatal error in main: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())

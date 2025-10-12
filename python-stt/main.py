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

        # Initialize ResourceMonitor (Task 5.2, STT-REQ-006)
        from stt_engine.resource_monitor import ResourceMonitor
        self.resource_monitor = ResourceMonitor()
        self.resource_monitor.current_model = self.stt_engine.model_size
        self.resource_monitor.initial_model = self.stt_engine.model_size

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
            # Send empty response for empty audio (MVP0 compatible)
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'text': None
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
            # Send response with flat structure (MVP0 compatible + MVP1 extensions)
            # STT-REQ-007.2/007.3: text field at root level for backward compatibility
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'text': transcription['text'],  # MVP0 compatible (root level)
                'is_final': transcription['is_final'],  # MVP1 extension
                'confidence': transcription.get('confidence', 0.0),  # MVP1 extension
                'language': transcription.get('language', 'ja'),  # MVP1 extension
                'processing_time_ms': transcription.get('processing_time_ms', 0),  # MVP1 extension
            })
            logger.info(f"Sent final transcription: {transcription['text'][:50]}...")
        else:
            # No transcription yet - send empty response
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'text': None
            })
            logger.debug("No final transcription yet, sent empty response")


    async def _handle_model_downgrade(self, old_model: str, new_model: str) -> None:
        """
        Handle model downgrade callback from ResourceMonitor (Task 5.2, STT-REQ-006.9).

        This method:
        1. Triggers WhisperSTTEngine to load the new model
        2. Updates ResourceMonitor's current_model (only on success)
        3. Sends IPC notification to UI

        Args:
            old_model: Previous model size
            new_model: New (downgraded) model size

        Raises:
            Exception: Re-raises any exception from load_model() to signal failure
        """
        logger.info(f"Model downgrade triggered: {old_model} → {new_model}")

        try:
            # Load new model in WhisperSTTEngine
            await self.stt_engine.load_model(new_model)

            # Update ResourceMonitor state (only after successful load)
            self.resource_monitor.current_model = new_model

            # Send IPC notification to UI (STT-REQ-006.9)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'event': 'model_change',
                    'old_model': old_model,
                    'new_model': new_model,
                    'reason': 'cpu_high' if self.resource_monitor.cpu_high_start_time else 'memory_high'
                })
                logger.info(f"Sent model_change IPC notification: {old_model} → {new_model}")

        except Exception as e:
            logger.error(f"Failed to handle model downgrade: {e}", exc_info=True)
            # Don't update current_model - it stays at old_model (automatic rollback)
            # Re-raise to signal failure to monitoring loop
            raise

    async def _handle_upgrade_proposal(self, current_model: str, proposed_model: str) -> None:
        """
        Handle upgrade proposal callback from ResourceMonitor (Task 5.2, STT-REQ-006.10).

        This method sends IPC notification to UI proposing model upgrade.
        Actual upgrade is performed only after user approval (Task 5.3).

        Args:
            current_model: Current model size
            proposed_model: Proposed (upgraded) model size
        """
        logger.info(f"Upgrade proposal: {current_model} → {proposed_model}")

        try:
            # Send IPC notification to UI (STT-REQ-006.10)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'event': 'upgrade_proposal',
                    'current_model': current_model,
                    'proposed_model': proposed_model,
                    'message': f'Resources have recovered. Upgrade to {proposed_model}?'
                })
                logger.info(f"Sent upgrade_proposal IPC notification: {current_model} → {proposed_model}")

        except Exception as e:
            logger.error(f"Failed to send upgrade proposal: {e}", exc_info=True)

    async def _handle_pause_recording(self) -> None:
        """
        Handle recording pause callback from ResourceMonitor (Task 5.2, STT-REQ-006.11).

        This method sends IPC notification to UI when tiny model is insufficient.
        Actual recording pause is handled by the UI/Rust backend.
        """
        logger.warning("Recording pause requested: tiny model insufficient for current resources")

        try:
            # Send IPC notification to UI (STT-REQ-006.11)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'event': 'recording_paused',
                    'reason': 'insufficient_resources',
                    'message': 'System resources are critically low. Recording paused.'
                })
                logger.info("Sent recording_paused IPC notification")

        except Exception as e:
            logger.error(f"Failed to send recording pause notification: {e}", exc_info=True)


async def main():
    """
    Main entry point for Python sidecar process.

    Initializes AudioProcessor and IpcHandler, then starts IPC event loop.
    """
    processor = None
    monitoring_task = None

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

        # Start resource monitoring loop (Task 5.2, STT-REQ-006)
        logger.info("Starting resource monitoring loop...")
        monitoring_task = asyncio.create_task(
            processor.resource_monitor.start_monitoring(
                interval_seconds=30.0,  # STT-NFR-001.6
                on_downgrade=processor._handle_model_downgrade,
                on_upgrade_proposal=processor._handle_upgrade_proposal,
                on_pause_recording=processor._handle_pause_recording
            )
        )

        logger.info("Starting IPC event loop...")

        # Start IPC event loop (blocks until shutdown)
        await processor.ipc.start()

        logger.info("IPC event loop stopped")

    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt, shutting down...")

    except Exception as e:
        logger.error(f"Fatal error in main: {e}", exc_info=True)
        sys.exit(1)

    finally:
        # Cleanup: Stop monitoring loop
        if processor and processor.resource_monitor:
            logger.info("Stopping resource monitoring loop...")
            await processor.resource_monitor.stop_monitoring()

        # Wait for monitoring task to complete
        if monitoring_task:
            try:
                await asyncio.wait_for(monitoring_task, timeout=2.0)
            except asyncio.TimeoutError:
                logger.warning("Monitoring task did not stop within timeout")
                monitoring_task.cancel()


if __name__ == "__main__":
    asyncio.run(main())

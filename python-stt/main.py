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
import time
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

        Message types (Task 7.1.5 - New IPC Protocol):
        - request (new): Generic request with method field (STT-REQ-007.1)
          - method=process_audio: Process audio frames through VAD→Pipeline→STT
          - method=process_audio_stream: Real-time event streaming (Task 7.1.6)
          - method=approve_upgrade: User-approved model upgrade
        - process_audio (legacy): Direct process_audio for backward compatibility
        - approve_upgrade (legacy): Direct approve_upgrade for backward compatibility
        - ping: Health check (respond with pong)
        - shutdown: Graceful shutdown
        """
        msg_type = msg.get('type')
        msg_id = msg.get('id', 'unknown')

        try:
            # New format: type="request" + method field (STT-REQ-007.1)
            if msg_type == 'request':
                method = msg.get('method')
                params = msg.get('params', {})

                if method == 'process_audio':
                    # Extract audio_data from params
                    msg_with_audio = {'id': msg_id, 'audio_data': params.get('audio_data')}
                    await self._handle_process_audio(msg_with_audio)

                elif method == 'process_audio_stream':
                    # Task 7.1.6: Real-time event streaming
                    msg_with_audio = {'id': msg_id, 'audio_data': params.get('audio_data')}
                    await self._handle_process_audio_stream(msg_with_audio)

                elif method == 'approve_upgrade':
                    # Extract target_model from params (new format)
                    params = msg.get('params', {})
                    msg_with_target = {'id': msg_id, 'target_model': params.get('target_model')}
                    await self._handle_approve_upgrade(msg_with_target)

                elif method == 'stop_processing':
                    # Legacy compatibility: stop_processing converted from LegacyIpcMessage::StopProcessing
                    # In new protocol, stop is handled by Rust side, so just acknowledge
                    logger.debug("⚠️ Received legacy stop_processing (converted from old IPC)")
                    await self.ipc.send_message({
                        'type': 'response',
                        'id': msg_id,
                        'version': '1.0',
                        'result': {'status': 'acknowledged'}
                    })

                else:
                    logger.warning(f"Unknown request method: {method}")
                    await self.ipc.send_message({
                        'type': 'error',
                        'id': msg_id,
                        'errorCode': 'UNKNOWN_METHOD',
                        'errorMessage': f"Unknown request method: {method}",
                        'recoverable': True
                    })

            # Legacy format: type="process_audio" (backward compatibility)
            elif msg_type == 'process_audio':
                logger.debug("⚠️ Received legacy format: type='process_audio' (deprecated)")
                await self._handle_process_audio(msg)

            elif msg_type == 'approve_upgrade':
                logger.debug("⚠️ Received legacy format: type='approve_upgrade' (deprecated)")
                await self._handle_approve_upgrade(msg)

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
                    'errorCode': 'UNKNOWN_TYPE',
                    'errorMessage': f"Unknown message type: {msg_type}",
                    'recoverable': True
                })

        except Exception as e:
            logger.error(f"Error handling message: {e}", exc_info=True)
            await self.ipc.send_message({
                'type': 'error',
                'id': msg_id,
                'errorCode': 'INTERNAL_ERROR',
                'errorMessage': str(e),
                'recoverable': False
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
            # Send empty response for empty audio (Task 7.1.5 new format)
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': {
                    'text': '',
                    'is_final': True,
                    'confidence': None,
                    'language': None,
                    'processing_time_ms': None,
                    'model_size': None
                }
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
            # Task 7.1.5: Send response with nested result structure (STT-REQ-007.2)
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': {  # Nested structure (new format)
                    'text': transcription['text'],
                    'is_final': transcription['is_final'],
                    'confidence': transcription.get('confidence'),  # None if not available
                    'language': transcription.get('language'),
                    'processing_time_ms': transcription.get('processing_time_ms'),
                    'model_size': self.stt_engine.model_size  # STT-REQ-007.2
                }
            })
            logger.info(f"Sent final transcription: {transcription['text'][:50]}...")
        else:
            # No transcription yet - send empty response
            await self.ipc.send_message({
                'id': msg_id,
                'type': 'response',
                'version': '1.0',
                'result': {
                    'text': '',
                    'is_final': True,
                    'confidence': None,
                    'language': None,
                    'processing_time_ms': None,
                    'model_size': None
                }
            })
            logger.debug("No final transcription yet, sent empty response")

    async def _handle_process_audio_stream(self, msg: Dict[str, Any]) -> None:
        """
        Process audio data with REAL-TIME event streaming (Task 7.1.6).

        This implementation sends multiple IPC events as audio is processed:
        - speech_start: When speech begins
        - partial_text: Intermediate transcription results (is_final=False)
        - final_text: Complete transcription after speech ends (is_final=True)
        - speech_end: Immediately after final_text

        Related Requirements:
        - STT-REQ-003.7: Partial text generation during speech (1s interval)
        - STT-REQ-003.8: Partial text with is_final=False
        - STT-REQ-007.1: New endpoint (existing process_audio unchanged)

        Args:
            msg: IPC message with audio_data field
        """
        msg_id = msg.get('id', 'unknown')
        audio_data = msg.get('audio_data', [])

        if not audio_data:
            logger.warning("Empty audio_data received for stream")
            return

        # Convert audio data (u8 array from Rust) to bytes
        audio_bytes = bytes(audio_data)

        # Split into 10ms frames (STT-REQ-003.2)
        frames = self.vad.split_into_frames(audio_bytes)

        # Track whether any speech was detected in this request
        speech_detected = False

        # P1 FIX: Process frames WITHOUT artificial sleep
        # AudioPipeline now uses frame-count based partial timing (100 frames = 1 second)
        # instead of wall-clock time, eliminating the need for asyncio.sleep(0.01).
        # Performance: 2 min recording now processes in seconds instead of 2 min.
        for frame in frames:
            # Use process_audio_frame_with_partial for partial text support
            result = await self.pipeline.process_audio_frame_with_partial(frame)

            if result:
                speech_detected = True  # Mark that speech was detected
                event_type = result.get('event')
                logger.debug(f"Stream event: {event_type}")

                # Immediately send event to Rust (real-time streaming)
                # CRITICAL: Match ipc_protocol.rs IpcMessage::Event schema
                # Schema: { type: "event", version: "1.0", eventType: "...", data: {...} }
                if event_type == 'speech_start':
                    await self.ipc.send_message({
                        'type': 'event',
                        'version': '1.0',
                        'eventType': 'speech_start',
                        'data': {
                            'requestId': msg_id,
                            'timestamp': result.get('timestamp')
                        }
                    })

                elif event_type == 'partial_text':
                    transcription = result['transcription']
                    # Task 11.1: Include latency_metrics for E2E validation
                    data = {
                        'requestId': msg_id,
                        'text': transcription['text'],
                        'is_final': False,  # STT-REQ-003.8
                        'confidence': transcription.get('confidence'),
                        'language': transcription.get('language'),
                        'processing_time_ms': transcription.get('processing_time_ms'),
                        'model_size': self.stt_engine.model_size
                    }
                    # Include latency_metrics if available (Task 11.1)
                    if 'latency_metrics' in result:
                        data['latency_metrics'] = result['latency_metrics']

                    await self.ipc.send_message({
                        'type': 'event',
                        'version': '1.0',
                        'eventType': 'partial_text',
                        'data': data
                    })

                elif event_type == 'final_text':
                    transcription = result['transcription']
                    # Send final_text event
                    # Task 11.1: Include latency_metrics for E2E validation
                    data = {
                        'requestId': msg_id,
                        'text': transcription['text'],
                        'is_final': True,  # STT-REQ-003.9
                        'confidence': transcription.get('confidence'),
                        'language': transcription.get('language'),
                        'processing_time_ms': transcription.get('processing_time_ms'),
                        'model_size': self.stt_engine.model_size
                    }
                    # Include latency_metrics if available (Task 11.1)
                    if 'latency_metrics' in result:
                        data['latency_metrics'] = result['latency_metrics']

                    await self.ipc.send_message({
                        'type': 'event',
                        'version': '1.0',
                        'eventType': 'final_text',
                        'data': data
                    })

                    # FIXED: Immediately send speech_end after final_text
                    # AudioPipeline._handle_speech_end() never returns speech_end event
                    # when STT engine is active (it returns final_text instead).
                    # We must derive and send speech_end here to satisfy the API contract.
                    await self.ipc.send_message({
                        'type': 'event',
                        'version': '1.0',
                        'eventType': 'speech_end',
                        'data': {
                            'requestId': msg_id,
                            'timestamp': int(time.time() * 1000)  # Current timestamp
                        }
                    })
                    logger.debug(f"Sent speech_end after final_text for {msg_id}")

                elif event_type == 'speech_end':
                    # This branch is only hit when STT engine is disabled
                    # (AudioPipeline._handle_speech_end L258-260)
                    # In production with Whisper, this never happens.
                    await self.ipc.send_message({
                        'type': 'event',
                        'version': '1.0',
                        'eventType': 'speech_end',
                        'data': {
                            'requestId': msg_id,
                            'timestamp': result.get('timestamp', int(time.time() * 1000))
                        }
                    })

                elif event_type == 'error':
                    # P0 FIX: Handle error events to prevent Rust-side hang
                    # Without this, Rust's receive_message() will block forever
                    # CRITICAL: IpcMessage::Error requires 'id' field (ipc_protocol.rs L104)
                    error_msg = result.get('message', 'Unknown error')
                    logger.error(f"AudioPipeline error: {error_msg}")
                    await self.ipc.send_message({
                        'type': 'error',
                        'id': msg_id,  # FIXED: id field is mandatory for IpcMessage::Error
                        'version': '1.0',
                        'errorCode': 'AUDIO_PIPELINE_ERROR',
                        'errorMessage': error_msg,
                        'recoverable': True
                    })
                    # Exit loop after sending error
                    break

                else:
                    # Unknown event type - log and continue
                    logger.warning(f"Unknown event type: {event_type}")

        # CRITICAL FIX: VAD-based no_speech detection (ADR-009)
        # Check VAD state instead of just event emission to prevent false positives.
        #
        # Problem scenario (P1 bug):
        # 1. User is talking continuously
        # 2. Previous request sent partial_text, reset _frame_count_since_partial
        # 3. This request processes 30-80 frames, no new partial yet
        # 4. result is None → speech_detected stays False
        # 5. OLD CODE: Sends no_speech even though user is still talking!
        #
        # Solution: Check VAD state (is_in_speech, has_buffered_speech)
        # Only send no_speech if VAD confirms silence.
        if not speech_detected:
            # Check VAD state to confirm silence (ADR-009 requirement)
            if not self.pipeline.is_in_speech() and not self.pipeline.has_buffered_speech():
                logger.debug(f"No speech detected (VAD confirmed silence) for {msg_id}")
                await self.ipc.send_message({
                    'type': 'event',
                    'version': '1.0',
                    'eventType': 'no_speech',
                    'data': {
                        'requestId': msg_id,
                        'timestamp': int(time.time() * 1000)
                    }
                })
            else:
                # Speech in progress but no event yet - DO NOT send no_speech
                # Rust's Receiver Task will keep waiting for next event (ADR-009)
                logger.debug(f"Speech in progress (VAD active, no event yet) for {msg_id}")

        logger.info(f"Stream processing complete for request {msg_id}")


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
            # Returns actual loaded model (may differ due to bundled fallback)
            actual_model = await self.stt_engine.load_model(new_model)

            # Update ResourceMonitor state with ACTUAL loaded model
            self.resource_monitor.current_model = actual_model

            # Send IPC notification to UI (STT-REQ-006.9, STT-REQ-007.1 Event format)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'version': '1.0',
                    'eventType': 'model_change',
                    'data': {
                        'old_model': old_model,
                        'new_model': actual_model,  # Actual loaded model
                        'requested_model': new_model,  # What was requested
                        'reason': 'cpu_high' if self.resource_monitor.cpu_high_start_time else 'memory_high'
                    }
                })
                logger.info(f"Sent model_change IPC notification: {old_model} → {actual_model}")

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
            # Send IPC notification to UI (STT-REQ-006.10, STT-REQ-007.1 Event format)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'version': '1.0',
                    'eventType': 'upgrade_proposal',
                    'data': {
                        'current_model': current_model,
                        'proposed_model': proposed_model,
                        'message': f'Resources have recovered. Upgrade to {proposed_model}?'
                    }
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
            # Send IPC notification to UI (STT-REQ-006.11, STT-REQ-007.1 Event format)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'event',
                    'version': '1.0',
                    'eventType': 'recording_paused',
                    'data': {
                        'reason': 'insufficient_resources',
                        'message': 'System resources are critically low. Recording paused.'
                    }
                })
                logger.info("Sent recording_paused IPC notification")

        except Exception as e:
            logger.error(f"Failed to send recording pause notification: {e}", exc_info=True)

    async def _handle_approve_upgrade(self, msg: Dict[str, Any]) -> None:
        """
        Handle user-approved model upgrade (Task 5.4, STT-REQ-006.12).

        This method:
        1. Loads the approved target model via WhisperSTTEngine
        2. Updates ResourceMonitor's current_model (only on success)
        3. Sends success/failure IPC notification to UI

        Args:
            msg: IPC message containing 'target_model' field

        Expected message format:
        {
            'type': 'approve_upgrade',
            'id': 'msg-id',
            'target_model': 'small'  # The approved model size
        }
        """
        msg_id = msg.get('id', 'unknown')
        target_model = msg.get('target_model')

        if not target_model:
            logger.error("approve_upgrade message missing 'target_model' field")
            if self.ipc:
                # STT-REQ-007.5: Unified error format
                await self.ipc.send_message({
                    'type': 'error',
                    'id': msg_id,
                    'version': '1.0',
                    'errorCode': 'MISSING_PARAMETER',
                    'errorMessage': "Missing 'target_model' field in approve_upgrade request",
                    'recoverable': True
                })
            return

        old_model = self.resource_monitor.current_model
        logger.info(f"User approved upgrade: {old_model} → {target_model}")

        try:
            # Load new model in WhisperSTTEngine
            # Returns actual loaded model (may differ from target_model due to bundled fallback)
            actual_model = await self.stt_engine.load_model(target_model)

            # Update ResourceMonitor state with ACTUAL loaded model (STT-REQ-006.9/006.12)
            self.resource_monitor.current_model = actual_model

            # Check if fallback occurred
            upgrade_succeeded = (actual_model == target_model)

            # Send IPC messages (STT-REQ-006.12, STT-REQ-007.1, STT-REQ-007.2)
            if self.ipc:
                # 1. id付きレスポンス（STT-REQ-007.1: Request-Response契約準拠、STT-REQ-007.2: ネスト構造）
                await self.ipc.send_message({
                    'id': msg_id,
                    'type': 'response',
                    'version': '1.0',
                    'result': {  # Nested structure (new format)
                        'success': upgrade_succeeded,
                        'old_model': old_model,
                        'new_model': actual_model,  # Actual loaded model
                        'requested_model': target_model,  # What user requested
                        'fallback_occurred': not upgrade_succeeded
                    }
                })
                logger.info(f"Sent approve_upgrade response: {old_model} → {actual_model} (requested: {target_model})")

                # 2. イベント通知（UI通知用、オプショナル、STT-REQ-007.1 Event format）
                if upgrade_succeeded:
                    event_type = 'upgrade_success'
                    message = f'Model upgraded successfully to {actual_model}'
                else:
                    event_type = 'upgrade_fallback'
                    message = f'Requested {target_model} not available, using {actual_model} instead'

                await self.ipc.send_message({
                    'type': 'event',
                    'version': '1.0',
                    'eventType': event_type,
                    'data': {
                        'old_model': old_model,
                        'new_model': actual_model,
                        'requested_model': target_model,
                        'message': message
                    }
                })
                logger.info(f"Sent {event_type} event notification")

        except Exception as e:
            logger.error(f"Failed to handle approved upgrade: {e}", exc_info=True)
            # Don't update current_model - it stays at old_model (automatic rollback)

            # Send failure IPC notification to UI (STT-REQ-007.5)
            if self.ipc:
                await self.ipc.send_message({
                    'type': 'error',
                    'id': msg_id,
                    'version': '1.0',
                    'errorCode': 'MODEL_LOAD_ERROR',
                    'errorMessage': f"Failed to upgrade model: {str(e)}",
                    'recoverable': False  # Model loading failure is not recoverable
                })


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

        # CRITICAL: Initialize WhisperSTTEngine before sending ready signal
        # Without this, transcribe() will raise "WhisperSTTEngine not initialized"
        # Decision: Fail fast on initialization errors (no graceful degradation)
        logger.info("Initializing WhisperSTTEngine (may take a few seconds)...")
        try:
            await processor.stt_engine.initialize()
            logger.info(f"WhisperSTTEngine initialized with model: {processor.stt_engine.model_size}")

            # Sync ResourceMonitor.current_model after initialization
            # (model_size may have changed due to bundled fallback)
            # IMPORTANT: Keep initial_model unchanged - it represents the resource-based
            # recommendation and serves as the upgrade ceiling (STT-REQ-006.10/006.12)
            processor.resource_monitor.current_model = processor.stt_engine.model_size
            logger.info(f"ResourceMonitor.current_model synced to: {processor.stt_engine.model_size}")
            logger.info(f"ResourceMonitor.initial_model (upgrade ceiling): {processor.resource_monitor.initial_model}")
        except Exception as e:
            logger.error(f"Failed to initialize WhisperSTTEngine: {e}", exc_info=True)
            logger.error("Cannot start sidecar without STT engine. Aborting.")
            # Do NOT send ready signal - let Rust side detect failure
            sys.exit(1)

        # Send ready signal to Rust backend (only after successful initialization)
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

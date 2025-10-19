"""
Audio Processing Pipeline Orchestrator
MVP1 Real STT - Decoupled Architecture

This module orchestrates VAD and STT components in a loosely coupled manner.
VAD focuses only on speech detection, STT focuses only on transcription,
and this pipeline coordinates between them.

Requirements:
- Separation of concerns between VAD and STT
- Flexible pipeline configuration
- Error handling and recovery
"""

import asyncio
import logging
from typing import Optional, AsyncGenerator, Dict, Any, List
from dataclasses import dataclass
from enum import Enum
import time

logger = logging.getLogger(__name__)


class SegmentType(Enum):
    """Types of audio segments"""
    SPEECH_START = "speech_start"
    SPEECH_PARTIAL = "speech_partial"
    SPEECH_END = "speech_end"


@dataclass
class AudioSegment:
    """
    Represents an audio segment detected by VAD.

    Attributes:
        type: Type of segment (start, partial, end)
        audio_data: Raw audio bytes
        duration_ms: Duration in milliseconds
        timestamp_ms: Timestamp when segment was detected
    """
    type: SegmentType
    audio_data: bytes
    duration_ms: int
    timestamp_ms: int = None

    def __post_init__(self):
        if self.timestamp_ms is None:
            self.timestamp_ms = int(time.time() * 1000)


@dataclass
class TranscriptionResult:
    """
    Result from STT engine.

    Attributes:
        text: Transcribed text
        is_final: Whether this is final or partial transcription
        confidence: Confidence score (0-1)
        language: Detected language
        processing_time_ms: Time taken for transcription
    """
    text: str
    is_final: bool
    confidence: float = 0.0
    language: str = "ja"
    processing_time_ms: int = 0


class AudioPipeline:
    """
    Orchestrates audio processing pipeline.

    This class decouples VAD and STT components, allowing them to be
    independently developed, tested, and replaced.

    Design principles:
    - VAD only detects speech segments
    - STT only transcribes audio
    - Pipeline handles coordination and error recovery
    - Components communicate via data structures, not direct calls
    """

    def __init__(
        self,
        vad=None,
        stt_engine=None,
        sample_rate: int = 16000
    ):
        """
        Initialize audio pipeline.

        Args:
            vad: Voice Activity Detector instance (optional)
            stt_engine: STT Engine instance (optional)
            sample_rate: Audio sample rate in Hz
        """
        self.vad = vad
        self.stt_engine = stt_engine
        self.sample_rate = sample_rate

        # Pipeline state
        self._running = False
        self._current_speech_buffer = bytearray()
        self._speech_start_time = None
        self._last_partial_time = None
        self._frame_count_since_partial = 0  # P1 fix: Frame-based partial timing

        # Task 11.1: Latency measurement for NFR-001.1 validation
        self._speech_start_timestamp_ms = None  # VAD speech_start detection time
        self._speech_end_timestamp_ms = None    # VAD speech_end detection time

        # Configuration
        self.partial_interval_ms = 1000  # 1 second between partial results
        self.partial_interval_frames = 100  # P1 fix: 100 frames = 1 second (10ms/frame)

        # Statistics
        self.stats = {
            "segments_processed": 0,
            "transcriptions_generated": 0,
            "errors": 0
        }

        logger.info(
            f"AudioPipeline initialized: "
            f"vad={'enabled' if vad else 'disabled'}, "
            f"stt={'enabled' if stt_engine else 'disabled'}"
        )

    async def process_audio_frame(self, audio_frame: bytes) -> Optional[Dict[str, Any]]:
        """
        Process a single audio frame through the pipeline.

        This is the main entry point for audio processing. It:
        1. Passes frame to VAD for speech detection
        2. Manages speech buffers
        3. Triggers STT when appropriate
        4. Returns transcription results

        Args:
            audio_frame: Raw audio frame (10ms, 16kHz, mono)

        Returns:
            Dict with event type and data, or None if no event
        """
        if not self.vad:
            logger.warning("VAD not configured, cannot process audio")
            return None

        # Step 1: VAD processing (speech detection only)
        vad_result = self.vad.process_frame(audio_frame)

        if not vad_result:
            return None

        # Step 2: Handle VAD events
        event_type = vad_result.get('event')

        if event_type == 'speech_start':
            pre_roll_data = vad_result.get('pre_roll')
            timestamp_ms = vad_result.get('timestamp_ms')  # Task 11.1
            return await self._handle_speech_start(pre_roll_data, timestamp_ms)

        elif event_type == 'speech_end':
            segment_data = vad_result.get('segment', {})
            timestamp_ms = vad_result.get('timestamp_ms')  # Task 11.1
            return await self._handle_speech_end(segment_data, timestamp_ms)

        return None

    async def process_audio_frame_with_partial(
        self,
        audio_frame: bytes
    ) -> Optional[Dict[str, Any]]:
        """
        Enhanced audio processing with partial transcription support.

        This method extends basic processing to include:
        - Partial transcriptions during speech
        - Frame-count based triggering (100 frames = 1 second)

        Args:
            audio_frame: Raw audio frame

        Returns:
            Dict with transcription result or None
        """
        if not self.vad:
            return None

        # VAD processing
        vad_result = self.vad.process_frame(audio_frame)

        # Track speech state
        if self.vad.is_in_speech:
            # Accumulate audio for partial transcription
            self._current_speech_buffer.extend(audio_frame)

            # P1 FIX: Increment frame counter (instead of checking wall-clock time)
            self._frame_count_since_partial += 1

            # Check if we should generate partial transcription
            if self._should_generate_partial():
                return await self._generate_partial_transcription()

        # Handle VAD events
        if vad_result:
            event_type = vad_result.get('event')

            if event_type == 'speech_start':
                pre_roll_data = vad_result.get('pre_roll')
                timestamp_ms = vad_result.get('timestamp_ms')  # Task 11.1
                return await self._handle_speech_start(pre_roll_data, timestamp_ms)

            elif event_type == 'speech_end':
                segment_data = vad_result.get('segment', {})
                timestamp_ms = vad_result.get('timestamp_ms')  # Task 11.1
                return await self._handle_speech_end(segment_data, timestamp_ms)

        return None

    def _should_generate_partial(self) -> bool:
        """
        Determine if partial transcription should be generated.

        P1 FIX: Frame-count based instead of wall-clock time.
        This avoids the need for asyncio.sleep() in the caller,
        preventing performance regression (2 min recording = 2 min processing).

        Task 11.2: Early trigger for first partial to meet NFR-001.1 (<500ms).
        - First partial: 10 frames (100ms) for fast initial response
        - Subsequent partials: 100 frames (1 second) for normal cadence

        Returns:
            True if partial should be generated
        """
        if not self._speech_start_time:
            return False

        # Task 11.2: Early trigger for first partial (NFR-001.1 compliance)
        # First partial needs fast response (<500ms end-to-end)
        # - 10 frames (100ms) audio accumulation
        # - ~1.5s Whisper processing
        # - Total: ~1.6s (still exceeds 500ms, but 4x improvement from 6.5s)
        is_first_partial = self._last_partial_time is None
        
        if is_first_partial:
            # First partial: trigger after 10 frames (100ms)
            if self._frame_count_since_partial >= 10:
                self._frame_count_since_partial = 0
                return True
        else:
            # Subsequent partials: trigger after 100 frames (1 second)
            if self._frame_count_since_partial >= self.partial_interval_frames:
                self._frame_count_since_partial = 0
                return True

        return False

    async def _handle_speech_start(
        self,
        pre_roll: Optional[bytes] = None,
        timestamp_ms: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Handle speech start event.

        Args:
            pre_roll: Optional pre-roll audio data from VAD (P0.5 FIX)
            timestamp_ms: VAD detection timestamp for latency measurement (Task 11.1)

        Returns:
            Speech start event dict
        """
        self._current_speech_buffer = bytearray()

        # ✅ P0.5 FIX: Seed with pre-roll if available
        # This ensures partial_text includes all leading frames
        if pre_roll:
            self._current_speech_buffer.extend(pre_roll)
            logger.info(f"Seeded speech buffer with {len(pre_roll)} bytes pre-roll ({len(pre_roll) // 320} frames)")

        self._speech_start_time = int(time.time() * 1000)
        self._last_partial_time = None
        self._frame_count_since_partial = 0  # P1 fix: Reset frame counter

        # Task 11.1: Record VAD detection timestamp for latency measurement
        self._speech_start_timestamp_ms = timestamp_ms if timestamp_ms else self._speech_start_time

        logger.info(f"Speech started in pipeline at VAD timestamp {self._speech_start_timestamp_ms}")

        return {
            'event': 'speech_start',
            'timestamp': self._speech_start_time
        }

    async def _handle_speech_end(
        self,
        segment_data: Dict,
        timestamp_ms: Optional[int] = None
    ) -> Optional[Dict[str, Any]]:
        """
        Handle speech end event and generate final transcription.

        Args:
            segment_data: Segment data from VAD
            timestamp_ms: VAD detection timestamp for latency measurement (Task 11.1)

        Returns:
            Final transcription result or None
        """
        # Task 11.1: Record VAD speech_end detection timestamp
        self._speech_end_timestamp_ms = timestamp_ms if timestamp_ms else int(time.time() * 1000)

        if not self.stt_engine:
            logger.warning("STT engine not configured, cannot transcribe")
            return {
                'event': 'speech_end',
                'segment': segment_data
            }

        try:
            # Get audio data from segment
            audio_data = segment_data.get('audio_data', b'')

            if not audio_data:
                logger.warning("No audio data in segment")
                return None

            # Transcribe with STT engine (final)
            start_time = time.time()

            transcription = await self.stt_engine.transcribe(
                audio_data,
                sample_rate=self.sample_rate,
                is_final=True
            )

            processing_time_ms = int((time.time() - start_time) * 1000)

            # Add processing time to result
            if isinstance(transcription, dict):
                transcription['processing_time_ms'] = processing_time_ms

            # Task 11.1: Calculate end-to-end latency (VAD speech_end → final_text delivery)
            # Target: < 2000ms (STT-NFR-001 implied requirement)
            current_time_ms = int(time.time() * 1000)
            end_to_end_latency_ms = current_time_ms - self._speech_end_timestamp_ms

            # Update statistics
            self.stats["transcriptions_generated"] += 1

            # Reset state
            self._current_speech_buffer = bytearray()
            self._speech_start_time = None
            self._last_partial_time = None

            # Task 11.1: Log latency metrics (structured logging for analysis)
            logger.info(
                f"Final transcription generated: "
                f"text='{transcription.get('text', '')[:50]}...', "
                f"whisper_time={processing_time_ms}ms, "
                f"end_to_end_latency={end_to_end_latency_ms}ms "
                f"(target: <2000ms, {'✅ PASS' if end_to_end_latency_ms < 2000 else '❌ FAIL'})"
            )

            return {
                'event': 'final_text',
                'transcription': transcription,
                'segment': segment_data,
                'latency_metrics': {
                    'whisper_processing_ms': processing_time_ms,
                    'end_to_end_latency_ms': end_to_end_latency_ms,
                    'vad_speech_end_timestamp_ms': self._speech_end_timestamp_ms,
                    'delivery_timestamp_ms': current_time_ms
                }
            }

        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Failed to generate final transcription: {e}")
            return {
                'event': 'error',
                'error': str(e),
                'segment': segment_data
            }

    async def _generate_partial_transcription(self) -> Optional[Dict[str, Any]]:
        """
        Generate partial transcription for accumulated speech.

        Returns:
            Partial transcription result or None
        """
        if not self.stt_engine or not self._current_speech_buffer:
            return None

        try:
            # Convert buffer to bytes
            audio_data = bytes(self._current_speech_buffer)

            # Transcribe with STT engine (partial)
            start_time = time.time()

            transcription = await self.stt_engine.transcribe(
                audio_data,
                sample_rate=self.sample_rate,
                is_final=False
            )

            processing_time_ms = int((time.time() - start_time) * 1000)

            # Add processing time
            if isinstance(transcription, dict):
                transcription['processing_time_ms'] = processing_time_ms

            # Task 11.1/11.2: Calculate end-to-end latency (VAD speech_start → **first** partial_text delivery)
            # Target: < 3000ms (STT-NFR-001.7 per ADR-017)
            # Note: Only measure latency for the FIRST partial in an utterance.
            # Subsequent partials measure incremental response (last_partial → current_partial).
            current_time_ms = int(time.time() * 1000)

            is_first_partial = self._last_partial_time is None

            if is_first_partial:
                # First partial: measure from speech_start
                end_to_end_latency_ms = current_time_ms - self._speech_start_timestamp_ms if self._speech_start_timestamp_ms else 0
                latency_type = "first_partial"
                # ADR-017: First partial target is <3000ms
                target_ms = 3000
                sla_status = f"{'✅ PASS' if end_to_end_latency_ms < target_ms else '❌ FAIL'}"
            else:
                # Subsequent partials: measure from last partial (incremental latency)
                end_to_end_latency_ms = current_time_ms - self._last_partial_time
                latency_type = "incremental"
                # Incremental partials are informational only (not SLA)
                target_ms = None
                sla_status = "ℹ️ INFO"

            # Update last partial time
            self._last_partial_time = current_time_ms

            # Update statistics
            self.stats["transcriptions_generated"] += 1

            # Task 11.1/11.2: Log latency metrics (structured logging for analysis)
            if is_first_partial:
                logger.info(
                    f"Partial transcription generated ({latency_type}): "
                    f"text='{transcription.get('text', '')[:50]}...', "
                    f"whisper_time={processing_time_ms}ms, "
                    f"end_to_end_latency={end_to_end_latency_ms}ms "
                    f"(SLA target: <{target_ms}ms per ADR-017, {sla_status})"
                )
            else:
                logger.info(
                    f"Partial transcription generated ({latency_type}): "
                    f"text='{transcription.get('text', '')[:50]}...', "
                    f"whisper_time={processing_time_ms}ms, "
                    f"incremental_latency={end_to_end_latency_ms}ms ({sla_status})"
                )

            return {
                'event': 'partial_text',
                'transcription': transcription,
                'latency_metrics': {
                    'whisper_processing_ms': processing_time_ms,
                    'end_to_end_latency_ms': end_to_end_latency_ms,
                    'vad_speech_start_timestamp_ms': self._speech_start_timestamp_ms,
                    'delivery_timestamp_ms': current_time_ms,
                    'is_first_partial': is_first_partial  # Flag for analysis
                }
            }

        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Failed to generate partial transcription: {e}")
            return None

    async def process_audio_stream(
        self,
        audio_stream: AsyncGenerator[bytes, None]
    ) -> AsyncGenerator[Dict[str, Any], None]:
        """
        Process continuous audio stream.

        Args:
            audio_stream: Async generator yielding audio frames

        Yields:
            Transcription results and events
        """
        self._running = True

        try:
            async for audio_frame in audio_stream:
                if not self._running:
                    break

                result = await self.process_audio_frame_with_partial(audio_frame)

                if result:
                    yield result

        finally:
            self._running = False
            logger.info(f"Pipeline stopped. Stats: {self.stats}")

    def stop(self):
        """Stop the pipeline."""
        self._running = False
        logger.info("Pipeline stop requested")

    def get_stats(self) -> Dict[str, Any]:
        """Get pipeline statistics."""
        return self.stats.copy()

    def is_in_speech(self) -> bool:
        """
        Check if currently in speech state (VAD detected voice).
        
        Returns:
            bool: True if VAD is currently detecting speech, False otherwise.
        
        Requirements:
            - ADR-009: VAD-based no_speech detection
            - Prevents false no_speech events during utterance
        """
        # Check if VAD is in speech state using actual VoiceActivityDetector field
        return self.vad.is_in_speech

    def has_buffered_speech(self) -> bool:
        """
        Check if there are speech frames buffered for STT processing.
        
        Returns:
            bool: True if speech buffer contains audio data, False otherwise.
        
        Requirements:
            - ADR-009: VAD-based no_speech detection
            - Prevents false no_speech when frames are queued for STT
        """
        return len(self._current_speech_buffer) > 0

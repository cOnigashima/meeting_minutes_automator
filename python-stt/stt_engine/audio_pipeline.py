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

        # Configuration
        self.partial_interval_ms = 1000  # 1 second between partial results

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
            return await self._handle_speech_start()

        elif event_type == 'speech_end':
            segment_data = vad_result.get('segment', {})
            return await self._handle_speech_end(segment_data)

        return None

    async def process_audio_frame_with_partial(
        self,
        audio_frame: bytes
    ) -> Optional[Dict[str, Any]]:
        """
        Enhanced audio processing with partial transcription support.

        This method extends basic processing to include:
        - Partial transcriptions during speech
        - Time-based triggering (1 second intervals)

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

            # Check if we should generate partial transcription
            if self._should_generate_partial():
                return await self._generate_partial_transcription()

        # Handle VAD events
        if vad_result:
            event_type = vad_result.get('event')

            if event_type == 'speech_start':
                return await self._handle_speech_start()

            elif event_type == 'speech_end':
                segment_data = vad_result.get('segment', {})
                return await self._handle_speech_end(segment_data)

        return None

    def _should_generate_partial(self) -> bool:
        """
        Determine if partial transcription should be generated.

        Returns:
            True if partial should be generated
        """
        if not self._speech_start_time:
            return False

        current_time = int(time.time() * 1000)

        # Initialize last partial time if needed
        if self._last_partial_time is None:
            self._last_partial_time = self._speech_start_time

        # Check if interval has elapsed
        time_since_last = current_time - self._last_partial_time
        return time_since_last >= self.partial_interval_ms

    async def _handle_speech_start(self) -> Dict[str, Any]:
        """
        Handle speech start event.

        Returns:
            Speech start event dict
        """
        self._current_speech_buffer = bytearray()
        self._speech_start_time = int(time.time() * 1000)
        self._last_partial_time = None

        logger.info("Speech started in pipeline")

        return {
            'event': 'speech_start',
            'timestamp': self._speech_start_time
        }

    async def _handle_speech_end(self, segment_data: Dict) -> Optional[Dict[str, Any]]:
        """
        Handle speech end event and generate final transcription.

        Args:
            segment_data: Segment data from VAD

        Returns:
            Final transcription result or None
        """
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

            # Update statistics
            self.stats["transcriptions_generated"] += 1

            # Reset state
            self._current_speech_buffer = bytearray()
            self._speech_start_time = None
            self._last_partial_time = None

            logger.info(
                f"Final transcription generated: "
                f"text='{transcription.get('text', '')[:50]}...', "
                f"time={processing_time_ms}ms"
            )

            return {
                'event': 'final_text',
                'transcription': transcription,
                'segment': segment_data
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

            # Update last partial time
            self._last_partial_time = int(time.time() * 1000)

            # Update statistics
            self.stats["transcriptions_generated"] += 1

            logger.debug(
                f"Partial transcription generated: "
                f"text='{transcription.get('text', '')[:50]}...', "
                f"time={processing_time_ms}ms"
            )

            return {
                'event': 'partial_text',
                'transcription': transcription
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
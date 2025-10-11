"""
Transcription module for audio-to-text conversion.

This module contains the WhisperSTTEngine implementation using faster-whisper.
"""

from .whisper_client import WhisperSTTEngine, ModelSize

__all__ = ["WhisperSTTEngine", "ModelSize"]

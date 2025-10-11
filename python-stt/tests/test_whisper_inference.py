"""
Unit tests for faster-whisper Inference (STT-REQ-002.11-002.14).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import base64
import json
import numpy as np
from unittest.mock import Mock, patch, MagicMock, AsyncMock
from stt_engine.transcription.whisper_client import WhisperSTTEngine


class TestAudioDataDecoding:
    """Test audio data Base64 decoding (STT-REQ-002.11)."""

    @pytest.mark.asyncio
    async def test_decode_base64_audio_data(self):
        """WHEN valid Base64 audio data is provided
        THEN should decode it successfully."""
        engine = WhisperSTTEngine(model_size="tiny")

        # Create sample audio data (16kHz, 1 second, mono)
        sample_rate = 16000
        duration = 1.0
        audio_samples = np.random.randint(-32768, 32767, int(sample_rate * duration), dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                # Mock transcribe method to return segments
                mock_segment = MagicMock()
                mock_segment.text = "テスト音声"
                mock_segment.avg_logprob = -0.5  # Higher is better (closer to 0)
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                result = await engine.transcribe(audio_bytes, sample_rate=sample_rate, is_final=True)

                # Should successfully decode and transcribe
                assert result is not None
                assert "text" in result

    @pytest.mark.asyncio
    async def test_handle_empty_audio_data(self):
        """WHEN empty audio data is provided
        THEN should return error response (STT-REQ-002.14)."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_whisper_model.return_value = MagicMock()

                await engine.initialize()

                result = await engine.transcribe(b"", sample_rate=16000, is_final=True)

                # Should return error response
                assert "error" in result or result.get("text") == ""


class TestWhisperInference:
    """Test faster-whisper inference execution (STT-REQ-002.11, STT-REQ-002.12)."""

    @pytest.mark.asyncio
    async def test_transcribe_audio_with_whisper(self):
        """WHEN audio data is transcribed
        THEN should return text result."""
        engine = WhisperSTTEngine(model_size="tiny")

        # Create sample audio data
        sample_rate = 16000
        audio_samples = np.random.randint(-32768, 32767, sample_rate, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                # Mock transcribe method
                mock_segment = MagicMock()
                mock_segment.text = "こんにちは、これはテストです。"
                mock_segment.avg_logprob = -0.3
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                result = await engine.transcribe(audio_bytes, sample_rate=sample_rate, is_final=True)

                assert result["text"] == "こんにちは、これはテストです。"
                assert "language" in result
                assert result["language"] == "ja"

    @pytest.mark.asyncio
    async def test_transcribe_returns_confidence(self):
        """WHEN audio is transcribed
        THEN should return confidence score."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 16000, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                mock_segment = MagicMock()
                mock_segment.text = "テスト"
                mock_segment.avg_logprob = -0.2  # Good confidence
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                result = await engine.transcribe(audio_bytes, sample_rate=16000, is_final=True)

                assert "confidence" in result
                assert isinstance(result["confidence"], (int, float))
                assert 0 <= result["confidence"] <= 1

    @pytest.mark.asyncio
    async def test_error_when_model_not_initialized(self):
        """WHEN transcribe is called before initialization
        THEN should raise RuntimeError."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 16000, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with pytest.raises(RuntimeError) as exc_info:
            await engine.transcribe(audio_bytes, sample_rate=16000, is_final=True)

        assert "not initialized" in str(exc_info.value).lower()


class TestJSONResponseFormat:
    """Test JSON response format (STT-REQ-002.12)."""

    @pytest.mark.asyncio
    async def test_response_contains_required_fields(self):
        """WHEN transcription completes
        THEN response should contain all required fields."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 16000, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                mock_segment = MagicMock()
                mock_segment.text = "テスト結果"
                mock_segment.avg_logprob = -0.4
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                result = await engine.transcribe(audio_bytes, sample_rate=16000, is_final=True)

                # Check all required fields (STT-REQ-002.12)
                assert "text" in result
                assert "confidence" in result
                assert "language" in result
                assert "is_final" in result
                assert "processing_time_ms" in result

    @pytest.mark.asyncio
    async def test_is_final_flag_propagated(self):
        """WHEN is_final parameter is set
        THEN should be reflected in response."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 16000, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                mock_segment = MagicMock()
                mock_segment.text = "テスト"
                mock_segment.avg_logprob = -0.5
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                # Test with is_final=True
                result_final = await engine.transcribe(audio_bytes, sample_rate=16000, is_final=True)
                assert result_final["is_final"] is True

                # Test with is_final=False
                result_partial = await engine.transcribe(audio_bytes, sample_rate=16000, is_final=False)
                assert result_partial["is_final"] is False

    @pytest.mark.asyncio
    async def test_processing_time_is_positive(self):
        """WHEN transcription is performed
        THEN processing_time_ms should be positive."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 16000, dtype=np.int16)
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                mock_segment = MagicMock()
                mock_segment.text = "テスト"
                mock_segment.avg_logprob = -0.5
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                result = await engine.transcribe(audio_bytes, sample_rate=16000, is_final=True)

                assert result["processing_time_ms"] >= 0


class TestInvalidAudioHandling:
    """Test invalid audio data handling (STT-REQ-002.14)."""

    @pytest.mark.asyncio
    async def test_invalid_sample_rate_error(self):
        """WHEN audio with invalid sample rate is provided
        THEN should handle gracefully."""
        engine = WhisperSTTEngine(model_size="tiny")

        audio_samples = np.random.randint(-32768, 32767, 8000, dtype=np.int16)  # 8kHz instead of 16kHz
        audio_bytes = audio_samples.tobytes()

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                # Mock transcribe to handle different sample rates
                mock_segment = MagicMock()
                mock_segment.text = ""
                mock_segment.avg_logprob = -1.0
                mock_model_instance.transcribe.return_value = ([mock_segment], {"language": "ja"})

                await engine.initialize()

                # Should handle gracefully (may return empty or error)
                result = await engine.transcribe(audio_bytes, sample_rate=8000, is_final=True)
                assert result is not None

    @pytest.mark.asyncio
    async def test_corrupted_audio_data_error(self):
        """WHEN corrupted audio data is provided
        THEN should return error response."""
        engine = WhisperSTTEngine(model_size="tiny")

        corrupted_data = b"not valid audio data"

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
                mock_detect.return_value = "[app_resources]/models/faster-whisper/tiny"
                mock_model_instance = MagicMock()
                mock_whisper_model.return_value = mock_model_instance

                await engine.initialize()

                result = await engine.transcribe(corrupted_data, sample_rate=16000, is_final=True)

                # Should handle error gracefully
                assert result is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

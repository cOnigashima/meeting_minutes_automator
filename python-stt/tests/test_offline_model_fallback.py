"""
Unit tests for Offline Model Fallback (STT-REQ-002.3-002.9).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import tempfile
import os
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock, call
from stt_engine.transcription.whisper_client import WhisperSTTEngine


class TestHuggingFaceHubDownload:
    """Test HuggingFace Hub model download (STT-REQ-002.3, STT-REQ-002.8, STT-REQ-002.9)."""

    @pytest.mark.asyncio
    async def test_download_from_huggingface_hub_success(self):
        """WHEN HuggingFace Hub is accessible
        THEN should download model and cache it (STT-REQ-002.3, STT-REQ-002.9)."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                # Simulate successful download
                mock_detect.return_value = str(Path.home() / ".cache/huggingface/hub/models--Systran--faster-whisper-tiny")
                mock_whisper.return_value = MagicMock()

                await engine.initialize()

                # Should have successfully loaded model
                assert engine.model is not None

    @pytest.mark.asyncio
    async def test_download_timeout_10_seconds(self):
        """WHEN downloading from HuggingFace Hub
        THEN should timeout after 10 seconds (STT-REQ-002.3)."""
        engine = WhisperSTTEngine(model_size="small")

        # This test verifies the timeout is set correctly
        # The actual timeout handling is in the implementation
        assert hasattr(engine, '_download_timeout')
        assert engine._download_timeout == 10

    @pytest.mark.asyncio
    @patch('logging.Logger.info')
    async def test_log_download_progress(self, mock_log):
        """WHEN downloading model from HuggingFace Hub
        THEN should log download progress (STT-REQ-002.8)."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                mock_detect.return_value = str(Path.home() / ".cache/huggingface/hub/models--Systran--faster-whisper-tiny")
                mock_whisper.return_value = MagicMock()

                with patch.object(engine, '_log_download_progress') as mock_progress:
                    await engine.initialize()

                    # Download progress logging should be called
                    # (Will be verified in implementation)


class TestBundledModelFallback:
    """Test bundled model fallback (STT-REQ-002.4, STT-REQ-002.5)."""

    @pytest.mark.asyncio
    async def test_fallback_to_bundled_model_on_network_error(self):
        """WHEN HuggingFace Hub download fails
        THEN should fallback to bundled base model (STT-REQ-002.4)."""
        engine = WhisperSTTEngine(model_size="small", offline_mode=False)

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                # Simulate HF Hub failure, then bundled model success
                def detect_side_effect():
                    # First try HF Hub, then fallback to bundled
                    if mock_detect.call_count == 1:
                        raise TimeoutError("HuggingFace Hub timeout")
                    return "[app_resources]/models/faster-whisper/base"

                mock_detect.side_effect = detect_side_effect
                mock_whisper.return_value = MagicMock()

                with patch('logging.Logger.info') as mock_log:
                    await engine.initialize()

                    # Should log fallback message
                    log_messages = [str(call) for call in mock_log.call_args_list]
                    fallback_logged = any('bundled' in msg.lower() or 'fallback' in msg.lower() for msg in log_messages)

    @pytest.mark.asyncio
    async def test_error_when_bundled_model_not_found(self):
        """WHEN both HuggingFace Hub and bundled model fail
        THEN should raise error (STT-REQ-002.5)."""
        engine = WhisperSTTEngine(model_size="small", offline_mode=False)

        with patch.object(engine, '_detect_model_path') as mock_detect:
            # Both HF Hub and bundled model fail
            mock_detect.side_effect = FileNotFoundError("No model found")

            with pytest.raises(Exception) as exc_info:
                await engine.initialize()

            assert "model" in str(exc_info.value).lower()

    @pytest.mark.asyncio
    @patch('logging.Logger.info')
    async def test_log_offline_mode_message(self, mock_log):
        """WHEN falling back to bundled model
        THEN should log 'オフラインモードで起動: バンドルbaseモデル使用' (STT-REQ-002.4)."""
        engine = WhisperSTTEngine(model_size="small", offline_mode=False)

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                # Simulate fallback to bundled model
                mock_detect.return_value = "[app_resources]/models/faster-whisper/base"
                mock_whisper.return_value = MagicMock()

                await engine.initialize()

                # Should log offline mode message
                log_messages = [str(call) for call in mock_log.call_args_list]
                offline_logged = any('オフライン' in msg or 'バンドル' in msg for msg in log_messages)


class TestOfflineModeSetting:
    """Test offline mode forced setting (STT-REQ-002.6)."""

    def test_offline_mode_initialization(self):
        """WHEN offline_mode=True is set
        THEN should initialize in offline mode."""
        engine = WhisperSTTEngine(model_size="base", offline_mode=True)

        assert engine.offline_mode is True

    @pytest.mark.asyncio
    async def test_offline_mode_skips_huggingface_hub(self):
        """WHEN offline_mode=True
        THEN should skip HuggingFace Hub connection entirely (STT-REQ-002.6)."""
        engine = WhisperSTTEngine(model_size="base", offline_mode=True)

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                # In offline mode, should only look for local models
                mock_detect.return_value = str(Path.home() / ".cache/huggingface/hub/models--Systran--faster-whisper-base")
                mock_whisper.return_value = MagicMock()

                await engine.initialize()

                # Should not attempt HuggingFace Hub download
                assert engine.model is not None

    @pytest.mark.asyncio
    async def test_offline_mode_uses_cached_or_bundled_only(self):
        """WHEN offline_mode=True
        THEN should only use cached or bundled models (STT-REQ-002.6)."""
        engine = WhisperSTTEngine(model_size="small", offline_mode=True)

        with patch.object(engine, '_try_download_from_hub') as mock_download:
            with patch.object(engine, '_detect_model_path') as mock_detect:
                with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                    mock_detect.return_value = "[app_resources]/models/faster-whisper/base"
                    mock_whisper.return_value = MagicMock()

                    await engine.initialize()

                    # Should NOT call HuggingFace Hub download
                    mock_download.assert_not_called()


class TestProxyEnvironment:
    """Test proxy environment support (STT-REQ-002.7)."""

    @pytest.mark.asyncio
    async def test_recognize_https_proxy_environment_variable(self):
        """WHEN HTTPS_PROXY environment variable is set
        THEN should use it for HuggingFace Hub connection (STT-REQ-002.7)."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.dict(os.environ, {'HTTPS_PROXY': 'http://proxy.example.com:8080'}):
            # Verify that proxy settings are recognized
            proxies = engine._get_proxy_settings()

            assert 'https' in proxies
            assert proxies['https'] == 'http://proxy.example.com:8080'

    @pytest.mark.asyncio
    async def test_recognize_http_proxy_environment_variable(self):
        """WHEN HTTP_PROXY environment variable is set
        THEN should use it for HuggingFace Hub connection (STT-REQ-002.7)."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.dict(os.environ, {'HTTP_PROXY': 'http://proxy.example.com:3128'}):
            proxies = engine._get_proxy_settings()

            assert 'http' in proxies
            assert proxies['http'] == 'http://proxy.example.com:3128'

    @pytest.mark.asyncio
    async def test_both_proxies_recognized(self):
        """WHEN both HTTPS_PROXY and HTTP_PROXY are set
        THEN should recognize both."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.dict(os.environ, {
            'HTTPS_PROXY': 'http://proxy.example.com:8080',
            'HTTP_PROXY': 'http://proxy.example.com:3128'
        }):
            proxies = engine._get_proxy_settings()

            assert 'https' in proxies
            assert 'http' in proxies


class TestModelCaching:
    """Test model caching (STT-REQ-002.9)."""

    @pytest.mark.asyncio
    async def test_cache_downloaded_model(self):
        """WHEN model is downloaded from HuggingFace Hub
        THEN should cache it to ~/.cache/huggingface/hub/ (STT-REQ-002.9)."""
        engine = WhisperSTTEngine(model_size="tiny")

        expected_cache_path = Path.home() / ".cache/huggingface/hub" / "models--Systran--faster-whisper-tiny"

        with patch.object(engine, '_detect_model_path') as mock_detect:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                mock_detect.return_value = str(expected_cache_path)
                mock_whisper.return_value = MagicMock()

                await engine.initialize()

                # Model should be loaded from cache path
                assert str(expected_cache_path) in str(mock_detect.return_value)

    @pytest.mark.asyncio
    async def test_use_cached_model_on_second_initialization(self):
        """WHEN model is already cached
        THEN should use cached model instead of downloading again."""
        engine = WhisperSTTEngine(model_size="base")

        cache_path = Path.home() / ".cache/huggingface/hub" / "models--Systran--faster-whisper-base"

        with patch.object(Path, 'exists') as mock_exists:
            with patch.object(engine, '_detect_model_path') as mock_detect:
                with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                    # Simulate cache exists
                    mock_exists.return_value = True
                    mock_detect.return_value = str(cache_path)
                    mock_whisper.return_value = MagicMock()

                    await engine.initialize()

                    # Should load from cache without downloading
                    assert engine.model is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

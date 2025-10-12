"""
Unit tests for WhisperSTTEngine (STT-REQ-002).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import tempfile
import json
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock
from stt_engine.transcription.whisper_client import WhisperSTTEngine, ModelSize


class TestWhisperSTTEngineInitialization:
    """Test WhisperSTTEngine initialization and model detection (STT-REQ-002.1, STT-REQ-002.10)."""

    def test_whisper_engine_initialization_with_default_model(self):
        """WHEN WhisperSTTEngine is initialized with default settings
        THEN it should set model_size to 'small' and model should be None initially."""
        engine = WhisperSTTEngine()

        assert engine.model_size == "small"
        assert engine.model is None
        assert engine.model_path is None

    def test_whisper_engine_initialization_with_custom_model_size(self):
        """WHEN WhisperSTTEngine is initialized with a custom model size
        THEN it should use the specified model size."""
        engine = WhisperSTTEngine(model_size="base")

        assert engine.model_size == "base"

    @pytest.mark.asyncio
    async def test_model_detection_priority_user_config(self):
        """WHEN user has specified a custom model path in config
        THEN WhisperSTTEngine should prioritize that path (STT-REQ-002.1 priority 1)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            user_model_path = Path(tmpdir) / "custom_model"
            user_model_path.mkdir()

            engine = WhisperSTTEngine()

            with patch('pathlib.Path.exists', return_value=True):
                with patch('pathlib.Path.expanduser', return_value=Path(tmpdir) / ".config" / "meeting-minutes-automator" / "whisper_model_path"):
                    with patch('builtins.open', create=True) as mock_open:
                        mock_open.return_value.__enter__.return_value.read.return_value = str(user_model_path)

                        detected_path = engine._detect_model_path()

                        assert detected_path == str(user_model_path)

    @pytest.mark.asyncio
    async def test_model_detection_priority_huggingface_cache(self):
        """WHEN user config doesn't exist but HuggingFace cache exists
        THEN WhisperSTTEngine should use HF cache (STT-REQ-002.1 priority 2)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            hf_cache_path = Path(tmpdir) / ".cache" / "huggingface" / "hub" / "models--Systran--faster-whisper-small"
            hf_cache_path.mkdir(parents=True)

            engine = WhisperSTTEngine(model_size="small")

            # Create a custom Path subclass that overrides exists()
            original_exists = Path.exists

            def custom_exists(self):
                path_str = str(self)
                if "whisper_model_path" in path_str:
                    return False  # User config doesn't exist
                elif "huggingface" in path_str and "faster-whisper-small" in path_str:
                    return True  # HF cache exists
                return original_exists(self)

            with patch.object(Path, 'exists', custom_exists):
                with patch.object(Path, 'home', return_value=Path(tmpdir)):
                    detected_path = engine._detect_model_path()

                    assert "huggingface" in detected_path
                    assert "faster-whisper-small" in detected_path

    @pytest.mark.asyncio
    async def test_model_detection_priority_bundled_fallback(self):
        """WHEN neither user config nor HF cache exists
        THEN WhisperSTTEngine should fallback to bundled model (STT-REQ-002.1 priority 3)."""
        engine = WhisperSTTEngine(model_size="base")

        # Mock all paths to not exist (will fallback to bundled model)
        with patch.object(Path, 'exists', return_value=False):
            detected_path = engine._detect_model_path()

            assert "[app_resources]" in detected_path
            assert "models/faster-whisper/base" in detected_path

    @pytest.mark.asyncio
    async def test_initialize_outputs_ready_message(self):
        """WHEN WhisperSTTEngine.initialize() completes successfully
        THEN it should output 'whisper_model_ready' to stdout (STT-REQ-002.10)."""
        engine = WhisperSTTEngine()

        with patch.object(engine, '_detect_model_path', return_value="/mock/path"):
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                mock_whisper.return_value = MagicMock()

                with patch('sys.stdout.write') as mock_stdout:
                    await engine.initialize()

                    # Check that stdout was written with ready message
                    calls = [str(call) for call in mock_stdout.call_args_list]
                    ready_message_found = any('whisper_model_ready' in call for call in calls)

                    assert ready_message_found, "Expected 'whisper_model_ready' message in stdout"

    @pytest.mark.asyncio
    async def test_initialize_loads_faster_whisper_model(self):
        """WHEN WhisperSTTEngine.initialize() is called
        THEN it should load the faster-whisper model."""
        engine = WhisperSTTEngine(model_size="tiny")

        with patch.object(engine, '_detect_model_path', return_value="/mock/tiny"):
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                mock_model = MagicMock()
                mock_whisper.return_value = mock_model

                await engine.initialize()

                assert engine.model is not None
                mock_whisper.assert_called_once()


class TestWhisperSTTEngineModelTypes:
    """Test WhisperSTTEngine model size validation."""

    def test_valid_model_sizes(self):
        """WHEN WhisperSTTEngine is initialized with valid model sizes
        THEN it should accept them without error."""
        valid_sizes: list[ModelSize] = ["tiny", "base", "small", "medium", "large-v3"]

        for size in valid_sizes:
            engine = WhisperSTTEngine(model_size=size)
            assert engine.model_size == size

    def test_model_size_type_hint(self):
        """Verify ModelSize Literal type includes all expected values."""
        from typing import get_args
        from stt_engine.transcription.whisper_client import ModelSize

        expected_sizes = {"tiny", "base", "small", "medium", "large-v3"}
        actual_sizes = set(get_args(ModelSize))

        assert actual_sizes == expected_sizes


class TestWhisperSTTEngineModelSwitching:
    """Test WhisperSTTEngine dynamic model switching (Task 5.2, STT-REQ-006.9)."""

    @pytest.mark.asyncio
    async def test_load_model_rollback_on_failure(self):
        """
        STT-REQ-006.9: load_model() should rollback on failure

        GIVEN WhisperSTTEngine with a loaded model
        WHEN load_model() fails to load new model
        THEN old model should be restored and transcribe() should still work
        """
        from stt_engine.transcription.whisper_client import WhisperSTTEngine, WhisperModel
        from unittest.mock import patch, MagicMock

        # Initialize with 'tiny' model
        engine = WhisperSTTEngine(model_size='tiny')

        # Manually set up a mock model to simulate already-loaded state
        mock_old_model = MagicMock(spec=WhisperModel)
        engine.model = mock_old_model
        engine.model_path = "fake/path/to/tiny"

        # Verify initialization succeeded
        assert engine.model is not None, "Initial model should be loaded"

        # Save old model reference
        old_model = engine.model
        old_model_size = engine.model_size
        old_model_path = engine.model_path

        # Mock WhisperModel to raise exception on construction
        with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper_model:
            mock_whisper_model.side_effect = RuntimeError("Mock model load failure")

            # Try to switch to 'base' model (should fail and rollback)
            with pytest.raises(RuntimeError, match="Mock model load failure"):
                await engine.load_model('base')

            # Verify rollback: old model, size, and path restored
            assert engine.model is old_model, "Model should be restored to old model"
            assert engine.model_size == old_model_size, f"model_size should be {old_model_size}"
            assert engine.model_path == old_model_path, "model_path should be restored"

            # Verify engine still works with old model
            # (Don't actually call transcribe() as it requires real audio data,
            #  just verify model is not None)
            assert engine.model is not None, "Model should not be None after rollback"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

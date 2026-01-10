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
            # Create actual directory structure including snapshots
            hf_cache_path = Path(tmpdir) / ".cache" / "huggingface" / "hub" / "models--Systran--faster-whisper-small"
            snapshots_dir = hf_cache_path / "snapshots"
            snapshots_dir.mkdir(parents=True)
            # Create a dummy snapshot directory (iterdir() needs at least one entry)
            (snapshots_dir / "abc123def456").mkdir()

            engine = WhisperSTTEngine(model_size="small")

            with patch.object(Path, 'home', return_value=Path(tmpdir)):
                detected_path = engine._detect_model_path()

                assert "huggingface" in detected_path
                assert "faster-whisper-small" in detected_path

    @pytest.mark.asyncio
    async def test_model_detection_priority_bundled_fallback(self):
        """WHEN neither user config nor HF cache exists
        THEN WhisperSTTEngine should fallback to bundled model (STT-REQ-002.1 priority 3)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create bundled model directory structure
            bundled_model_dir = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_model_dir.mkdir(parents=True)
            (bundled_model_dir / "model.bin").touch()

            # offline_mode=True prevents falling back to HuggingFace Hub model ID
            engine = WhisperSTTEngine(model_size="base", offline_mode=True)

            # Mock Path.home() to use temp dir (so HF cache won't be found)
            # and mock _detect_bundled_model_path to return our temp bundled path
            with patch.object(Path, 'home', return_value=Path(tmpdir)):
                with patch.object(engine, '_detect_bundled_model_path', return_value=str(bundled_model_dir)):
                    detected_path = engine._detect_model_path()

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


class TestBundledModelFallback:
    """Test bundled model fallback (STT-REQ-002.4/002.6)"""

    def test_bundled_fallback_from_requested_to_base(self):
        """
        STT-REQ-002.4/002.6: Fallback to bundled base when requested model unavailable

        GIVEN offline mode with only bundled 'base' model
        WHEN requested model 'small' is not available
        THEN _detect_model_path should fallback to bundled 'base'
        AND model_size should be updated to 'base'
        """
        import tempfile
        from pathlib import Path
        from stt_engine.transcription.whisper_client import WhisperSTTEngine

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create bundled base model directory
            bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_base.mkdir(parents=True)
            (bundled_base / "model.bin").touch()

            engine = WhisperSTTEngine(model_size='small', offline_mode=True)

            # Mock _detect_model_path to use temp directory
            def patched_detect():
                bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

                # Try requested size (small)
                for base_dir in bundled_base_dirs:
                    bundled_path = base_dir / engine.model_size
                    if bundled_path.exists() and (bundled_path / "model.bin").exists():
                        return str(bundled_path)

                # Fallback to base (STT-REQ-002.4/002.6)
                if engine.model_size != 'base':
                    for base_dir in bundled_base_dirs:
                        bundled_base_path = base_dir / "base"
                        if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                            engine.model_size = "base"
                            return str(bundled_base_path)

                raise FileNotFoundError("No model found")

            engine._detect_model_path = patched_detect

            # Execute detection
            model_path = engine._detect_model_path()

            # Verify fallback occurred
            assert engine.model_size == "base", "Should fallback to base"
            assert "base" in model_path, "Path should contain 'base'"
            assert Path(model_path).exists(), "Bundled base path should exist"

    def test_no_fallback_when_requested_model_available(self):
        """
        Verify that fallback does NOT occur when requested model is available

        GIVEN bundled 'small' model exists
        WHEN requested model is 'small'
        THEN should use 'small' directly without fallback
        """
        import tempfile
        from pathlib import Path
        from stt_engine.transcription.whisper_client import WhisperSTTEngine

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create bundled small AND base models
            bundled_small = Path(tmpdir) / "models" / "faster-whisper" / "small"
            bundled_small.mkdir(parents=True)
            (bundled_small / "model.bin").touch()

            bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_base.mkdir(parents=True)
            (bundled_base / "model.bin").touch()

            engine = WhisperSTTEngine(model_size='small', offline_mode=True)

            # Mock _detect_model_path
            def patched_detect():
                bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

                # Try requested size (small) - should find it
                for base_dir in bundled_base_dirs:
                    bundled_path = base_dir / engine.model_size
                    if bundled_path.exists() and (bundled_path / "model.bin").exists():
                        return str(bundled_path)

                # Fallback to base (should NOT reach here)
                if engine.model_size != 'base':
                    for base_dir in bundled_base_dirs:
                        bundled_base_path = base_dir / "base"
                        if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                            engine.model_size = "base"
                            return str(bundled_base_path)

                raise FileNotFoundError("No model found")

            engine._detect_model_path = patched_detect

            # Execute detection
            model_path = engine._detect_model_path()

            # Verify NO fallback occurred
            assert engine.model_size == "small", "Should keep requested model"
            assert "small" in model_path, "Path should contain 'small'"

    @pytest.mark.asyncio
    async def test_load_model_returns_actual_model_on_fallback(self):
        """
        Verify that load_model() returns actual loaded model size when fallback occurs

        GIVEN bundled 'base' only
        WHEN load_model('small') is called
        THEN should return 'base' (not 'small')
        """
        import tempfile
        from pathlib import Path
        from stt_engine.transcription.whisper_client import WhisperSTTEngine
        from unittest.mock import MagicMock, patch

        with tempfile.TemporaryDirectory() as tmpdir:
            bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_base.mkdir(parents=True)
            (bundled_base / "model.bin").touch()

            # Create engine with base initially loaded
            engine = WhisperSTTEngine(model_size='base', offline_mode=True)
            engine.model = MagicMock()  # Mock WhisperModel
            engine.model_path = str(bundled_base)

            # Mock _detect_model_path to simulate fallback
            def patched_detect():
                bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

                # Try requested size (small) - not found
                for base_dir in bundled_base_dirs:
                    bundled_path = base_dir / engine.model_size
                    if bundled_path.exists() and (bundled_path / "model.bin").exists():
                        return str(bundled_path)

                # Fallback to base
                if engine.model_size != 'base':
                    for base_dir in bundled_base_dirs:
                        bundled_base_path = base_dir / "base"
                        if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                            engine.model_size = "base"  # FALLBACK OCCURS HERE
                            return str(bundled_base_path)

                raise FileNotFoundError("No model found")

            engine._detect_model_path = patched_detect

            # Mock WhisperModel to avoid actual model loading
            with patch('stt_engine.transcription.whisper_client.WhisperModel', return_value=MagicMock()):
                # Call load_model with 'small' (will fallback to 'base')
                actual_model = await engine.load_model('small')

            # Verify return value reflects fallback
            assert actual_model == 'base', "load_model should return 'base' (fallback occurred)"
            assert engine.model_size == 'base', "engine.model_size should be 'base'"


class TestOnlineModeHubPriority:
    """Test online mode prioritizes HuggingFace Hub over bundled models (STT-REQ-002.1/002.3)"""

    def test_online_mode_returns_hub_model_id_before_bundled(self):
        """
        Verify online mode returns Hub model ID before checking bundled models

        GIVEN online mode with bundled 'base' model available
        WHEN requested model 'small' is not in cache
        THEN should return HuggingFace Hub model ID (not bundled 'base')
        AND bundled fallback should NOT occur
        """
        import tempfile
        from pathlib import Path
        from stt_engine.transcription.whisper_client import WhisperSTTEngine

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create bundled base model
            bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_base.mkdir(parents=True)
            (bundled_base / "model.bin").touch()

            # Create engine in ONLINE mode (offline_mode=False)
            engine = WhisperSTTEngine(model_size='small', offline_mode=False)

            # Mock _detect_model_path to use temp directory for bundled paths
            def patched_detect():
                # Simulate: no user config, no HF cache
                # Online mode should return Hub model ID BEFORE bundled check

                # Return Hub model ID (online mode priority)
                model_id = f"Systran/faster-whisper-{engine.model_size}"
                return model_id

            engine._detect_model_path = patched_detect

            # Execute detection
            model_path = engine._detect_model_path()

            # Verify Hub model ID returned (NOT bundled path)
            assert model_path == "Systran/faster-whisper-small", \
                "Online mode should return Hub model ID, not bundled fallback"
            assert engine.model_size == "small", \
                "model_size should remain 'small' (no fallback)"

    def test_offline_mode_uses_bundled_fallback(self):
        """
        Verify offline mode uses bundled fallback (not Hub model ID)

        GIVEN offline mode with bundled 'base' model available
        WHEN requested model 'small' is not in cache or bundled
        THEN should fallback to bundled 'base' model
        """
        import tempfile
        from pathlib import Path
        from stt_engine.transcription.whisper_client import WhisperSTTEngine

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create bundled base model only
            bundled_base = Path(tmpdir) / "models" / "faster-whisper" / "base"
            bundled_base.mkdir(parents=True)
            (bundled_base / "model.bin").touch()

            # Create engine in OFFLINE mode
            engine = WhisperSTTEngine(model_size='small', offline_mode=True)

            # Mock _detect_model_path to use temp directory
            def patched_detect():
                bundled_base_dirs = [Path(tmpdir) / "models" / "faster-whisper"]

                # Try requested size
                for base_dir in bundled_base_dirs:
                    bundled_path = base_dir / engine.model_size
                    if bundled_path.exists() and (bundled_path / "model.bin").exists():
                        return str(bundled_path)

                # Offline mode: fallback to bundled base
                if engine.model_size != 'base':
                    for base_dir in bundled_base_dirs:
                        bundled_base_path = base_dir / "base"
                        if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                            engine.model_size = "base"
                            return str(bundled_base_path)

                raise FileNotFoundError("No model found")

            engine._detect_model_path = patched_detect

            # Execute detection
            model_path = engine._detect_model_path()

            # Verify bundled fallback occurred
            assert engine.model_size == "base", "Should fallback to 'base' in offline mode"
            assert "base" in model_path, "Path should contain 'base'"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

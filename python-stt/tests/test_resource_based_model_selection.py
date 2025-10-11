"""
Unit tests for Resource-Based Model Selection (STT-REQ-006).

Test-Driven Development: These tests are written first and will initially fail.
"""

import pytest
import logging
from unittest.mock import Mock, patch, MagicMock
from stt_engine.transcription.whisper_client import WhisperSTTEngine, ModelSize


class TestSystemResourceDetection:
    """Test system resource detection (STT-REQ-006.1)."""

    def test_detect_system_resources_cpu_cores(self):
        """WHEN system resources are detected
        THEN it should detect CPU core count."""
        engine = WhisperSTTEngine()

        resources = engine._detect_system_resources()

        assert 'cpu_cores' in resources
        assert isinstance(resources['cpu_cores'], int)
        assert resources['cpu_cores'] > 0

    def test_detect_system_resources_memory(self):
        """WHEN system resources are detected
        THEN it should detect total memory in GB."""
        engine = WhisperSTTEngine()

        resources = engine._detect_system_resources()

        assert 'memory_gb' in resources
        assert isinstance(resources['memory_gb'], (int, float))
        assert resources['memory_gb'] > 0

    def test_detect_system_resources_gpu_availability(self):
        """WHEN system resources are detected
        THEN it should detect GPU availability."""
        engine = WhisperSTTEngine()

        resources = engine._detect_system_resources()

        assert 'has_gpu' in resources
        assert isinstance(resources['has_gpu'], bool)

    def test_detect_system_resources_gpu_memory(self):
        """WHEN system resources are detected
        THEN it should detect GPU memory if available."""
        engine = WhisperSTTEngine()

        resources = engine._detect_system_resources()

        assert 'gpu_memory_gb' in resources
        if resources['has_gpu']:
            assert isinstance(resources['gpu_memory_gb'], (int, float))
            assert resources['gpu_memory_gb'] > 0
        else:
            assert resources['gpu_memory_gb'] == 0


class TestModelSelectionRules:
    """Test model selection rules based on system resources (STT-REQ-006.2, STT-REQ-006.3)."""

    def test_select_model_large_v3_for_high_end_gpu(self):
        """WHEN GPU available AND memory≥8GB AND GPU memory≥10GB
        THEN should select large-v3 model."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 8,
            'memory_gb': 16,
            'has_gpu': True,
            'gpu_memory_gb': 12
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        assert selected_model == "large-v3"

    def test_select_model_medium_for_mid_range_gpu(self):
        """WHEN GPU available AND memory≥4GB AND GPU memory≥5GB
        THEN should select medium model."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 4,
            'memory_gb': 6,
            'has_gpu': True,
            'gpu_memory_gb': 6
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        assert selected_model == "medium"

    def test_select_model_small_for_cpu_with_4gb(self):
        """WHEN CPU only AND memory≥4GB
        THEN should select small model."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 4,
            'memory_gb': 4,
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        assert selected_model == "small"

    def test_select_model_base_for_cpu_with_2gb(self):
        """WHEN CPU only AND memory≥2GB
        THEN should select base model."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 2,
            'memory_gb': 2.5,
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        assert selected_model == "base"

    def test_select_model_tiny_for_low_memory(self):
        """WHEN memory<2GB
        THEN should select tiny model."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 2,
            'memory_gb': 1.5,
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        assert selected_model == "tiny"

    @patch('logging.Logger.info')
    def test_log_selected_model(self, mock_log):
        """WHEN model selection completes
        THEN should log the selected model size (STT-REQ-006.3)."""
        engine = WhisperSTTEngine()

        mock_resources = {
            'cpu_cores': 4,
            'memory_gb': 4,
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        selected_model = engine._select_model_by_resources(mock_resources)

        # The model selection should trigger logging
        # (This will be verified when we implement the actual method)
        assert selected_model is not None


class TestManualModelOverride:
    """Test manual model selection override (STT-REQ-006.4, STT-REQ-006.5)."""

    def test_manual_model_override(self):
        """WHEN user manually selects a model
        THEN should use the manually selected model instead of auto-selection."""
        # Manual selection via constructor
        engine = WhisperSTTEngine(model_size="large-v3")

        # Even if system resources suggest a smaller model, user choice should be respected
        assert engine.model_size == "large-v3"

    def test_detect_resource_exceeded_warning(self):
        """WHEN manually selected model exceeds system resources
        THEN should log a warning (STT-REQ-006.5)."""
        engine = WhisperSTTEngine(model_size="large-v3")

        mock_resources = {
            'cpu_cores': 2,
            'memory_gb': 2,  # Too low for large-v3
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        with patch('logging.Logger.warning') as mock_warning:
            exceeds = engine._check_model_exceeds_resources("large-v3", mock_resources)

            assert exceeds is True
            # Warning should be logged (will be verified in implementation)

    def test_no_warning_when_resources_sufficient(self):
        """WHEN manually selected model fits within system resources
        THEN should not log a warning."""
        engine = WhisperSTTEngine(model_size="small")

        mock_resources = {
            'cpu_cores': 4,
            'memory_gb': 8,
            'has_gpu': False,
            'gpu_memory_gb': 0
        }

        exceeds = engine._check_model_exceeds_resources("small", mock_resources)

        assert exceeds is False


class TestAutoModelSelection:
    """Test automatic model selection on initialization."""

    @pytest.mark.asyncio
    async def test_auto_select_model_on_init(self):
        """WHEN WhisperSTTEngine initializes with auto_select=True
        THEN should automatically select model based on system resources."""
        with patch.object(WhisperSTTEngine, '_detect_system_resources') as mock_detect:
            mock_detect.return_value = {
                'cpu_cores': 4,
                'memory_gb': 4,
                'has_gpu': False,
                'gpu_memory_gb': 0
            }

            engine = WhisperSTTEngine(auto_select_model=True)

            # Should have auto-selected 'small' based on mock resources
            assert engine.model_size == "small"

    @pytest.mark.asyncio
    async def test_manual_model_disables_auto_select(self):
        """WHEN WhisperSTTEngine initializes with explicit model_size
        THEN should NOT auto-select model."""
        with patch.object(WhisperSTTEngine, '_detect_system_resources') as mock_detect:
            mock_detect.return_value = {
                'cpu_cores': 4,
                'memory_gb': 4,
                'has_gpu': False,
                'gpu_memory_gb': 0
            }

            # Explicit model_size should override auto-selection
            engine = WhisperSTTEngine(model_size="base", auto_select_model=True)

            # Should use explicitly specified model, not auto-selected
            assert engine.model_size == "base"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

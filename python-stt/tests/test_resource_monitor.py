"""
Unit Tests: ResourceMonitor - リソース監視と動的モデルダウングレード

TDD Approach: RED → GREEN → REFACTOR

Related Requirements:
- STT-REQ-006.1: システムリソース検出（CPU、メモリ、GPU）
- STT-REQ-006.2: モデル選択ルール適用
- STT-REQ-006.3: モデル選択のログ記録
- STT-REQ-006.6-006.8: 動的ダウングレード（30秒間隔監視）
"""

import pytest
from unittest.mock import MagicMock, patch


class TestResourceDetection:
    """Test resource detection on startup (STT-REQ-006.1)"""

    def test_detect_cpu_cores(self):
        """
        STT-REQ-006.1: CPU cores detection

        GIVEN ResourceMonitor initialization
        WHEN detect_resources() is called
        THEN CPU core count should be detected correctly
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        resources = monitor.detect_resources()

        assert 'cpu_cores' in resources
        assert isinstance(resources['cpu_cores'], int)
        assert resources['cpu_cores'] > 0

    def test_detect_total_memory(self):
        """
        STT-REQ-006.1: Total memory detection

        GIVEN ResourceMonitor initialization
        WHEN detect_resources() is called
        THEN Total memory in GB should be detected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        resources = monitor.detect_resources()

        assert 'total_memory_gb' in resources
        assert isinstance(resources['total_memory_gb'], (int, float))
        assert resources['total_memory_gb'] > 0

    def test_detect_gpu_availability(self):
        """
        STT-REQ-006.1: GPU availability detection

        GIVEN ResourceMonitor initialization
        WHEN detect_resources() is called
        THEN GPU availability should be detected (True/False)
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        resources = monitor.detect_resources()

        assert 'gpu_available' in resources
        assert isinstance(resources['gpu_available'], bool)


class TestModelSelection:
    """Test model selection rules (STT-REQ-006.2)"""

    def test_select_large_v3_with_gpu_and_high_memory(self):
        """
        STT-REQ-006.2: large-v3 selection with GPU + 8GB+ memory

        GIVEN GPU available AND system memory ≥ 8GB
        WHEN select_model() is called
        THEN large-v3 should be selected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        # Mock resources
        resources = {
            'cpu_cores': 8,
            'total_memory_gb': 16,
            'gpu_available': True,
            'gpu_memory_gb': 12
        }

        model = monitor.select_model(resources)
        assert model == 'large-v3'

    def test_select_medium_with_gpu_and_moderate_memory(self):
        """
        STT-REQ-006.2: medium selection with GPU + 4-8GB memory

        GIVEN GPU available AND system memory ≥ 4GB AND < 8GB
        WHEN select_model() is called
        THEN medium should be selected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        resources = {
            'cpu_cores': 4,
            'total_memory_gb': 6,
            'gpu_available': True,
            'gpu_memory_gb': 6
        }

        model = monitor.select_model(resources)
        assert model == 'medium'

    def test_select_small_with_cpu_and_4gb_memory(self):
        """
        STT-REQ-006.2: small selection with CPU + 4GB memory

        GIVEN CPU only AND system memory ≥ 4GB
        WHEN select_model() is called
        THEN small should be selected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        resources = {
            'cpu_cores': 4,
            'total_memory_gb': 8,
            'gpu_available': False,
            'gpu_memory_gb': 0
        }

        model = monitor.select_model(resources)
        assert model == 'small'

    def test_select_base_with_cpu_and_2gb_memory(self):
        """
        STT-REQ-006.2: base selection with CPU + 2-4GB memory

        GIVEN CPU only AND system memory ≥ 2GB AND < 4GB
        WHEN select_model() is called
        THEN base should be selected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        resources = {
            'cpu_cores': 2,
            'total_memory_gb': 3,
            'gpu_available': False,
            'gpu_memory_gb': 0
        }

        model = monitor.select_model(resources)
        assert model == 'base'

    def test_select_tiny_with_low_memory(self):
        """
        STT-REQ-006.2: tiny selection with < 2GB memory

        GIVEN system memory < 2GB
        WHEN select_model() is called
        THEN tiny should be selected
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        resources = {
            'cpu_cores': 2,
            'total_memory_gb': 1.5,
            'gpu_available': False,
            'gpu_memory_gb': 0
        }

        model = monitor.select_model(resources)
        assert model == 'tiny'


class TestResourceMonitoring:
    """Test resource monitoring loop (STT-REQ-006.6-006.8)"""

    @pytest.mark.asyncio
    async def test_monitor_memory_usage(self):
        """
        STT-REQ-006.6: Memory usage monitoring

        GIVEN ResourceMonitor running
        WHEN get_current_memory_usage() is called
        THEN Current memory usage in GB should be returned
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        memory_gb = monitor.get_current_memory_usage()

        assert isinstance(memory_gb, (int, float))
        assert memory_gb >= 0

    @pytest.mark.asyncio
    async def test_monitor_cpu_usage(self):
        """
        STT-REQ-006.7: CPU usage monitoring

        GIVEN ResourceMonitor running
        WHEN get_current_cpu_usage() is called
        THEN Current CPU usage percentage should be returned
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        cpu_percent = monitor.get_current_cpu_usage()

        assert isinstance(cpu_percent, (int, float))
        assert 0 <= cpu_percent <= 100


class TestDynamicDowngrade:
    """Test dynamic model downgrade (STT-REQ-006.7-006.9)"""

    def test_should_downgrade_for_high_cpu_usage(self):
        """
        STT-REQ-006.7: Downgrade decision for high CPU usage

        GIVEN CPU usage >= 85% sustained for 60 seconds
        WHEN should_downgrade_cpu() is called
        THEN downgrade should be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'

        # Simulate 60 seconds of high CPU usage
        should_downgrade = monitor.should_downgrade_cpu(cpu_percent=90, duration_seconds=65)

        assert should_downgrade is True

    def test_should_not_downgrade_for_short_cpu_spike(self):
        """
        STT-REQ-006.7: No downgrade for short CPU spikes

        GIVEN CPU usage >= 85% for < 60 seconds
        WHEN should_downgrade_cpu() is called
        THEN downgrade should NOT be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'

        # Simulate short CPU spike (30 seconds)
        should_downgrade = monitor.should_downgrade_cpu(cpu_percent=90, duration_seconds=30)

        assert should_downgrade is False

    def test_should_downgrade_for_high_memory_usage(self):
        """
        STT-REQ-006.8: Immediate downgrade to base for 4GB+ memory usage

        GIVEN Memory usage >= 4GB
        WHEN should_downgrade_memory() is called
        THEN immediate downgrade to 'base' should be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'

        # Simulate 4.5GB memory usage
        should_downgrade, target_model = monitor.should_downgrade_memory(memory_gb=4.5)

        assert should_downgrade is True
        assert target_model == 'base'

    def test_get_next_model_in_downgrade_sequence(self):
        """
        STT-REQ-006.6: Downgrade sequence (large → medium → small → base → tiny)

        GIVEN current model
        WHEN get_downgrade_target() is called
        THEN next model in sequence should be returned
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        # Test sequence
        assert monitor.get_downgrade_target('large-v3') == 'medium'
        assert monitor.get_downgrade_target('medium') == 'small'
        assert monitor.get_downgrade_target('small') == 'base'
        assert monitor.get_downgrade_target('base') == 'tiny'
        assert monitor.get_downgrade_target('tiny') is None  # Already at minimum

    def test_downgrade_model_changes_current_model(self):
        """
        STT-REQ-006.9: Model downgrade execution

        GIVEN current model is 'large-v3'
        WHEN downgrade_model() is called
        THEN current model should change to 'medium'
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'

        old_model, new_model = monitor.downgrade_model()

        assert old_model == 'large-v3'
        assert new_model == 'medium'
        assert monitor.current_model == 'medium'

    def test_cannot_downgrade_from_tiny(self):
        """
        STT-REQ-006.11: Cannot downgrade below tiny

        GIVEN current model is 'tiny'
        WHEN downgrade_model() is called
        THEN None should be returned (no further downgrade possible)
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'tiny'

        result = monitor.downgrade_model()

        assert result is None
        assert monitor.current_model == 'tiny'


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

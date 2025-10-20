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
        STT-REQ-006.8: Immediate downgrade to base for critical memory usage

        GIVEN Memory usage >= 4GB
        WHEN should_downgrade_memory() is called
        THEN immediate downgrade to 'base' should be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'

        # Simulate 4.5GB memory usage (critical, >= 4GB)
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


class TestUINotificationAndUpgrade:
    """Test UI notification and upgrade proposal (STT-REQ-006.9-006.12)"""

    def test_create_downgrade_notification(self):
        """
        STT-REQ-006.9: Create downgrade notification message

        GIVEN model downgrade occurs
        WHEN create_downgrade_notification() is called
        THEN notification message should be generated
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        notification = monitor.create_downgrade_notification(
            old_model='large-v3',
            new_model='medium'
        )

        assert notification is not None
        assert notification['type'] == 'model_change'
        assert notification['message'] == 'モデル変更: large-v3 → medium'
        assert notification['old_model'] == 'large-v3'
        assert notification['new_model'] == 'medium'

    def test_should_propose_upgrade_when_resources_recovered(self):
        """
        STT-REQ-006.10: Propose upgrade when resources recovered

        GIVEN memory < 2GB AND CPU < 50% sustained for 5 minutes
        WHEN should_propose_upgrade() is called
        THEN upgrade should be proposed
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'large-v3'

        # Simulate resource recovery (5 minutes = 300 seconds)
        should_propose = monitor.should_propose_upgrade(
            memory_gb=1.5,
            cpu_percent=45,
            duration_seconds=305
        )

        assert should_propose is True

    def test_should_not_propose_upgrade_before_5_minutes(self):
        """
        STT-REQ-006.10: No upgrade proposal before 5 minutes

        GIVEN memory < 2GB AND CPU < 50% but < 5 minutes
        WHEN should_propose_upgrade() is called
        THEN upgrade should NOT be proposed
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'large-v3'

        # Simulate short recovery period (2 minutes)
        should_propose = monitor.should_propose_upgrade(
            memory_gb=1.5,
            cpu_percent=45,
            duration_seconds=120
        )

        assert should_propose is False

    def test_should_not_propose_upgrade_when_high_cpu(self):
        """
        STT-REQ-006.10: No upgrade proposal when CPU still high

        GIVEN memory < 2GB but CPU >= 50%
        WHEN should_propose_upgrade() is called
        THEN upgrade should NOT be proposed
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'large-v3'

        # Simulate high CPU (65%)
        should_propose = monitor.should_propose_upgrade(
            memory_gb=1.5,
            cpu_percent=65,
            duration_seconds=305
        )

        assert should_propose is False

    def test_create_upgrade_proposal_notification(self):
        """
        STT-REQ-006.10: Create upgrade proposal notification

        GIVEN upgrade proposal is triggered
        WHEN create_upgrade_proposal_notification() is called
        THEN proposal notification should be generated
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        notification = monitor.create_upgrade_proposal_notification(
            current_model='small',
            target_model='medium'
        )

        assert notification is not None
        assert notification['type'] == 'upgrade_proposal'
        assert 'small' in notification['message']
        assert 'medium' in notification['message']
        assert notification['current_model'] == 'small'
        assert notification['target_model'] == 'medium'

    def test_get_upgrade_target(self):
        """
        STT-REQ-006.12: Upgrade sequence (tiny → base → small → medium → large)

        GIVEN current model
        WHEN get_upgrade_target() is called
        THEN next higher model in sequence should be returned
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.initial_model = 'large-v3'

        # Test upgrade sequence
        assert monitor.get_upgrade_target('tiny') == 'base'
        assert monitor.get_upgrade_target('base') == 'small'
        assert monitor.get_upgrade_target('small') == 'medium'
        assert monitor.get_upgrade_target('medium') == 'large-v3'
        assert monitor.get_upgrade_target('large-v3') is None  # Already at maximum

    def test_get_upgrade_target_respects_initial_model(self):
        """
        STT-REQ-006.12: Upgrade should not exceed initial_model

        GIVEN initial_model = small
        WHEN get_upgrade_target() is called
        THEN should not return models larger than small
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.initial_model = 'small'

        # MODEL_SEQUENCE = ['large-v3', 'medium', 'small', 'base', 'tiny']
        # index:             0          1         2        3       4
        # Smaller index = larger model

        # Upgrades that should work (staying within small ceiling)
        assert monitor.get_upgrade_target('tiny') == 'base'  # 4→3 OK
        assert monitor.get_upgrade_target('base') == 'small'  # 3→2 OK (at ceiling)

        # Upgrades that should be blocked (would exceed initial_model)
        assert monitor.get_upgrade_target('small') is None  # 2→1 would be medium, exceeds small

    def test_upgrade_model_changes_current_model(self):
        """
        STT-REQ-006.12: Model upgrade execution

        GIVEN current model is 'small'
        WHEN upgrade_model() is called
        THEN current model should change to 'medium'
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'large-v3'

        old_model, new_model = monitor.upgrade_model()

        assert old_model == 'small'
        assert new_model == 'medium'
        assert monitor.current_model == 'medium'

    def test_cannot_upgrade_above_initial_model(self):
        """
        STT-REQ-006.12: Cannot upgrade beyond initial model

        GIVEN current model is same as initial model
        WHEN upgrade_model() is called
        THEN None should be returned (no upgrade possible)
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'
        monitor.initial_model = 'large-v3'

        result = monitor.upgrade_model()

        assert result is None
        assert monitor.current_model == 'large-v3'

    def test_should_pause_recording_when_tiny_and_insufficient(self):
        """
        STT-REQ-006.11: Pause recording when tiny model is insufficient

        GIVEN current model is 'tiny' AND resources still insufficient
        WHEN should_pause_recording() is called
        THEN recording pause should be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'tiny'

        # Simulate high memory usage (>= 4GB) on tiny model
        should_pause = monitor.should_pause_recording(
            current_model='tiny',
            memory_gb=4.5,
            cpu_percent=90
        )

        assert should_pause is True

    def test_should_not_pause_recording_when_not_tiny(self):
        """
        STT-REQ-006.11: No pause when not on tiny model

        GIVEN current model is not 'tiny'
        WHEN should_pause_recording() is called
        THEN recording pause should NOT be recommended
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()
        monitor.current_model = 'small'

        # Even with high resource usage
        should_pause = monitor.should_pause_recording(
            current_model='small',
            memory_gb=4.5,
            cpu_percent=90
        )

        assert should_pause is False

    def test_create_recording_pause_notification(self):
        """
        STT-REQ-006.11: Create recording pause notification

        GIVEN recording pause is triggered
        WHEN create_recording_pause_notification() is called
        THEN pause notification should be generated
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        notification = monitor.create_recording_pause_notification()

        assert notification is not None
        assert notification['type'] == 'recording_paused'
        assert notification['message'] == 'システムリソース不足のため録音を一時停止しました'
        assert notification['reason'] == 'insufficient_resources'


class TestMonitoringLoop:
    """Test monitoring loop functionality (STT-NFR-001.6, STT-NFR-005.4)"""

    @pytest.mark.asyncio
    async def test_monitor_loop_executes_periodically(self):
        """
        STT-NFR-001.6, STT-NFR-005.4: Monitor loop executes every 30 seconds

        GIVEN ResourceMonitor with monitoring loop
        WHEN monitor_loop() is running
        THEN resources should be checked periodically
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import AsyncMock, MagicMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'medium'
        monitor.initial_model = 'large-v3'

        # Mock callbacks
        on_downgrade = AsyncMock()
        on_upgrade_proposal = AsyncMock()
        on_pause_recording = AsyncMock()

        # Run monitoring loop for short duration
        task = asyncio.create_task(monitor.start_monitoring(
            interval_seconds=0.1,  # Fast interval for testing
            on_downgrade=on_downgrade,
            on_upgrade_proposal=on_upgrade_proposal,
            on_pause_recording=on_pause_recording
        ))

        # Let it run long enough for multiple cycles
        # With 0.1s interval: cycle1 at 0s, cycle2 at 0.1s, cycle3 at 0.2s
        await asyncio.sleep(0.5)

        # Stop monitoring
        await monitor.stop_monitoring()
        await task

        # Verify monitoring occurred (at least 3 checks with 0.1s interval in 0.5s)
        assert monitor.monitoring_cycle_count >= 3

    @pytest.mark.asyncio
    async def test_cpu_high_duration_tracking(self):
        """
        STT-REQ-006.7: Track CPU high duration automatically

        GIVEN ResourceMonitor with monitoring loop
        WHEN CPU >= 85% for multiple cycles
        THEN cpu_high_start_time should be tracked
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'
        monitor.initial_model = 'large-v3'

        # Mock CPU to always return high usage, memory to be low (avoid memory-based downgrade)
        with patch('psutil.cpu_percent', return_value=90), \
             patch('psutil.virtual_memory') as mock_mem:

            # Set memory low enough to avoid memory-based downgrade (< 3GB)
            mock_mem.return_value.percent = 30
            mock_mem.return_value.used = 2.0 * (1024 ** 3)  # 2GB (< 3GB threshold)
            mock_mem.return_value.available = 4 * (1024 ** 3)  # Phase 1.1: _update_state_machine needs available

            on_downgrade = AsyncMock()

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=on_downgrade
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            # Verify cpu_high_start_time was set
            assert monitor.cpu_high_start_time is not None

    @pytest.mark.asyncio
    async def test_low_resource_duration_tracking(self):
        """
        STT-REQ-006.10: Track low resource duration for upgrade proposal

        GIVEN ResourceMonitor with monitoring loop
        WHEN memory < 2GB AND CPU < 50% for multiple cycles
        THEN low_resource_start_time should be tracked
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'large-v3'

        # Mock resources to show recovery
        with patch('psutil.cpu_percent', return_value=30), \
             patch('psutil.virtual_memory') as mock_mem:

            mock_mem.return_value.percent = 40
            mock_mem.return_value.used = 1.5 * (1024 ** 3)  # 1.5GB (low memory)
            mock_mem.return_value.available = 4 * (1024 ** 3)

            on_upgrade_proposal = AsyncMock()

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_upgrade_proposal=on_upgrade_proposal
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            # Verify low_resource_start_time was set
            assert monitor.low_resource_start_time is not None

    @pytest.mark.asyncio
    async def test_memory_downgrade_triggered_immediately(self):
        """
        STT-REQ-006.8: Memory downgrade triggers immediately

        GIVEN ResourceMonitor with high memory usage (>= 4GB)
        WHEN monitoring loop detects high memory
        THEN downgrade callback should be invoked immediately
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'
        monitor.initial_model = 'large-v3'

        # Mock memory to always return high usage
        with patch('psutil.virtual_memory') as mock_mem, \
             patch('psutil.cpu_percent', return_value=50):

            mock_mem.return_value.percent = 92  # High percentage (for logging)
            mock_mem.return_value.used = 4.5 * (1024 ** 3)  # 4.5GB (critical, >= 4GB)
            mock_mem.return_value.available = 0.5 * (1024 ** 3)  # Phase 1.1: _update_state_machine needs available

            on_downgrade = AsyncMock()

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=on_downgrade
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            # Verify downgrade was called (immediately, not after 60 seconds)
            assert on_downgrade.called
            # Should downgrade to 'base' immediately
            call_args = on_downgrade.call_args
            assert call_args[0][1] == 'base'  # new_model should be 'base'

    @pytest.mark.asyncio
    async def test_memory_downgrade_skips_when_already_base(self):
        """
        Regression: skip repeated base reloads when already on base model

        GIVEN ResourceMonitor already running the base model with high memory usage
        WHEN monitoring loop checks resources
        THEN downgrade callback should NOT be invoked again
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'base'
        monitor.initial_model = 'large-v3'

        with patch('psutil.virtual_memory') as mock_mem, \
             patch('psutil.cpu_percent', return_value=55):

            mock_mem.return_value.percent = 90
            mock_mem.return_value.used = 4.5 * (1024 ** 3)

            on_downgrade = AsyncMock()

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=on_downgrade
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            assert on_downgrade.call_count == 0

    @pytest.mark.asyncio
    async def test_downgrade_triggered_after_60_seconds_high_cpu(self):
        """
        STT-REQ-006.7: Downgrade after 60 seconds of high CPU

        GIVEN ResourceMonitor with high CPU for 60+ seconds
        WHEN monitoring loop detects sustained high CPU
        THEN downgrade callback should be invoked
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'
        monitor.initial_model = 'large-v3'

        # Mock CPU to always return high usage
        with patch('psutil.cpu_percent', return_value=90):
            on_downgrade = AsyncMock()

            # Simulate 60 seconds elapsed by manipulating cpu_high_start_time
            import time
            monitor.cpu_high_start_time = time.time() - 65

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=on_downgrade
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            # Verify downgrade was called
            assert on_downgrade.called

    @pytest.mark.asyncio
    async def test_downgrade_failure_does_not_update_current_model(self):
        """
        Task 5.2: Downgrade failure should not change current_model

        GIVEN ResourceMonitor with high memory usage
        WHEN downgrade callback fails (raises exception)
        THEN current_model should remain unchanged (rollback)
        """
        from stt_engine.resource_monitor import ResourceMonitor
        from unittest.mock import patch, AsyncMock
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'large-v3'
        monitor.initial_model = 'large-v3'

        # Mock high memory usage (>= 4GB triggers immediate downgrade)
        with patch('psutil.cpu_percent', return_value=50), \
             patch('psutil.virtual_memory') as mock_mem:

            mock_mem.return_value.percent = 92
            mock_mem.return_value.used = 4.5 * (1024 ** 3)  # 4.5GB
            mock_mem.return_value.available = 0.5 * (1024 ** 3)  # Phase 1.1: _update_state_machine needs available

            # Mock callback that fails
            async def failing_downgrade(old_model, new_model):
                raise RuntimeError("Mock downgrade failure")

            on_downgrade = AsyncMock(side_effect=failing_downgrade)

            task = asyncio.create_task(monitor.start_monitoring(
                interval_seconds=0.1,
                on_downgrade=on_downgrade
            ))

            await asyncio.sleep(0.25)
            await monitor.stop_monitoring()
            await task

            # Verify downgrade was attempted
            assert on_downgrade.called

            # CRITICAL: current_model should NOT be changed (rollback)
            assert monitor.current_model == 'large-v3', \
                "current_model should remain unchanged after downgrade failure"


class TestAppMemoryMonitoring:
    """Test application-specific memory monitoring (Fix for system-wide memory bug)"""

    def test_app_memory_not_system_memory(self):
        """
        Verify that memory monitoring uses app RSS, not system-wide memory

        GIVEN ResourceMonitor with app-specific memory tracking
        WHEN get_current_memory_usage() is called
        THEN returned value should be app memory (much smaller than system memory)
        """
        from stt_engine.resource_monitor import ResourceMonitor
        import psutil

        monitor = ResourceMonitor()
        app_memory = monitor.get_current_memory_usage()

        # Get system-wide memory for comparison
        system_memory_gb = psutil.virtual_memory().used / (1024**3)

        # App memory should be much smaller than system memory
        assert app_memory < system_memory_gb, \
            f"App memory ({app_memory:.2f}GB) should be < system memory ({system_memory_gb:.2f}GB)"

        # Python sidecar should typically use < 2GB
        assert app_memory < 2.0, \
            f"Python sidecar memory ({app_memory:.2f}GB) seems too high (expected < 2GB)"

    def test_should_downgrade_memory_with_app_thresholds(self):
        """
        STT-REQ-006.8: Memory downgrade uses app-specific thresholds (2.0GB/1.5GB)

        GIVEN app memory monitoring
        WHEN app memory reaches 2.0GB (critical) or 1.5GB (warning)
        THEN appropriate downgrade should be triggered
        """
        from stt_engine.resource_monitor import ResourceMonitor

        monitor = ResourceMonitor()

        # Test critical threshold (>= 2.0GB -> immediate downgrade to base)
        should_downgrade, target_model = monitor.should_downgrade_memory(2.5)
        assert should_downgrade is True
        assert target_model == 'base', "Should immediately downgrade to base at 2.5GB"

        # Test warning threshold (>= 1.5GB -> gradual downgrade)
        should_downgrade, target_model = monitor.should_downgrade_memory(1.7)
        assert should_downgrade is True
        assert target_model is None, "Should gradually downgrade at 1.7GB"

        # Test safe level (< 1.5GB -> no downgrade)
        should_downgrade, target_model = monitor.should_downgrade_memory(1.0)
        assert should_downgrade is False
        assert target_model is None, "Should not downgrade at 1.0GB"

    def test_monitor_cycle_uses_app_memory(self):
        """
        Verify that _monitor_cycle() uses app-specific memory, not system-wide

        GIVEN ResourceMonitor with monitoring enabled
        WHEN _monitor_cycle() executes
        THEN it should check app memory (via get_current_memory_usage)
        AND not trigger false positives from system-wide memory usage
        """
        from stt_engine.resource_monitor import ResourceMonitor
        import asyncio

        monitor = ResourceMonitor()
        monitor.current_model = 'small'
        monitor.initial_model = 'small'

        downgrade_called = False
        async def mock_downgrade(old, new):
            nonlocal downgrade_called
            downgrade_called = True

        # Run one monitoring cycle
        async def test():
            await monitor._monitor_cycle(
                on_downgrade=mock_downgrade,
                on_upgrade_proposal=None,
                on_pause_recording=None
            )

        asyncio.run(test())

        # With typical Python sidecar memory (< 1.5GB), downgrade should NOT be triggered
        # (Unlike system-wide memory which would often be > 3GB)
        assert not downgrade_called, \
            "Downgrade should not trigger with typical app memory usage"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

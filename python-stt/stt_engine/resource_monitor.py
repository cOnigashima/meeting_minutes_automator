"""
ResourceMonitor - システムリソース監視と動的モデルダウングレード

Related Requirements:
- STT-REQ-006.1: システムリソース検出（CPU、メモリ、GPU）
- STT-REQ-006.2: モデル選択ルール適用
- STT-REQ-006.3: モデル選択のログ記録
- STT-REQ-006.6-006.12: 動的ダウングレードとアップグレード提案
- STT-NFR-001.6: 30秒間隔の監視ループ
- STT-NFR-005.4: 30秒間隔のDEBUGログ出力

Implementation (P13-PREP-001 完了 2025-10-20):
- Hierarchical state machine: monitoring → degraded → recovering
- Debounce logic: 60s minimum between downgrades
- Recovery counter: 10 consecutive low-resource samples (5min @ 30s interval)
- IPC integration: model_change events via IpcHandler.send_message()
"""

import logging
import psutil
import asyncio
import time
import os
from typing import Dict, Any, Optional, Callable, Awaitable

logger = logging.getLogger(__name__)


class ResourceMonitor:
    """
    システムリソース監視とWhisperモデル選択管理

    Responsibilities:
    - 起動時のシステムリソース検出（CPU、メモリ、GPU）
    - リソースに基づくWhisperモデル自動選択
    - 実行中のリソース監視（30秒間隔）
    - リソース制約時の動的モデルダウングレード
    - リソース回復時のアップグレード提案
    """

    # Model downgrade sequence (STT-REQ-006.6)
    MODEL_SEQUENCE = ['large-v3', 'medium', 'small', 'base', 'tiny']

    def __init__(self, stt_engine=None, ipc_handler=None):
        """
        Initialize ResourceMonitor with dependencies.
        
        Args:
            stt_engine: WhisperSTTEngine instance for model switching
            ipc_handler: IpcHandler instance for event notifications
        """
        # Model tracking
        self.current_model = None
        self.initial_model = None
        self.resources = None
        
        # Legacy timer fields (kept for backward compatibility with existing tests)
        self.cpu_high_start_time = None
        self.low_resource_start_time = None
        
        # Phase 1.1: Hierarchical state management (STT-REQ-006.6-006.12)
        from collections import deque
        self.cpu_samples = deque(maxlen=2)  # CPU history (2 samples @ 30s = 60s window)
        self.recovery_sample_count = 0      # Recovery state counter (need 10 = 5min)
        self.last_downgrade_timestamp = 0.0 # For debounce (60s minimum)
        
        # State machine: "monitoring" | "degraded" | "recovering"
        self.monitoring_state = "monitoring"

        # Monitoring loop state
        self.monitoring_task = None
        self.monitoring_running = False
        self.monitoring_cycle_count = 0

        # Process handle for app-specific memory tracking
        self.process = psutil.Process(os.getpid())
        
        # Dependencies (Phase 1.1)
        self.stt_engine = stt_engine  # For load_model() calls
        self.ipc = ipc_handler         # For event notifications

        logger.info("ResourceMonitor initialized with state machine")

    async def set_model_level(self, target_model: str) -> str:
        """
        Programmatic model switching API (Phase 1.1 - FIXED: string-based).
        
        Coordinates with WhisperSTTEngine to ensure state consistency.
        Emits IPC event via proper protocol channel.
        
        Args:
            target_model: Target model size ("tiny", "base", "small", "medium", "large-v3")
            
        Returns:
            str: Actual model size loaded (may differ due to bundled fallback)
            
        Raises:
            ValueError: If stt_engine/ipc not configured or invalid model size
            Exception: If model loading fails
            
        Example:
            >>> monitor = ResourceMonitor(stt_engine, ipc_handler)
            >>> actual_model = await monitor.set_model_level("base")
            >>> print(actual_model)  # "base" or fallback
        """
        if self.stt_engine is None:
            raise ValueError("stt_engine not configured - cannot switch model")
        if self.ipc is None:
            raise ValueError("ipc_handler not configured - cannot emit events")
        
        # Validate model size against MODEL_SEQUENCE
        valid_models = self.MODEL_SEQUENCE
        if target_model not in valid_models:
            raise ValueError(
                f"Invalid model size '{target_model}'. "
                f"Must be one of {valid_models}"
            )
        
        old_model = self.current_model
        
        try:
            # Call WhisperSTTEngine.load_model() to perform actual switch
            # This ensures monitor state stays in sync with loaded model
            actual_model = await self.stt_engine.load_model(target_model)
            
            # Update monitor state ONLY after successful load
            self.current_model = actual_model
            
            # Emit IPC event via proper protocol (not direct stdout!)
            await self.ipc.send_message({
                "type": "event",
                "eventType": "model_change",
                "data": {
                    "old_model": old_model or "unknown",
                    "new_model": actual_model,
                    "reason": "manual_switch",
                    "target_model": target_model
                }
            })
            
            logger.info(f"Model switched: {old_model} -> {actual_model} (target={target_model})")
            return actual_model
            
        except Exception as e:
            logger.error(f"Model switch failed: {e}")
            raise

    def _should_debounce_downgrade(self) -> bool:
        """
        Check if downgrade should be debounced (Phase 1.1).
        
        Implements STT-REQ-006.6 60-second minimum interval between downgrades
        to prevent chattering when metrics oscillate around threshold.
        
        Returns:
            bool: True if downgrade should be skipped due to debounce
        """
        import time
        elapsed = time.time() - self.last_downgrade_timestamp
        
        if elapsed < 60.0:
            logger.debug(f"Downgrade debounced: {elapsed:.1f}s since last (need 60s)")
            return True
        
        return False

    def _update_state_machine(self, cpu_usage: float, memory_available_gb: float) -> None:
        """
        Update hierarchical state machine (Phase 1.1).
        
        State transitions:
        - monitoring: Normal operation, current_model == initial_model
        - degraded: Downgraded due to high resources, monitoring for recovery
        - recovering: Low resources sustained, counting samples (need 10 = 5min)
        
        Implements STT-REQ-006.6-006.12 recovery logic.
        
        Args:
            cpu_usage: Current CPU usage percentage
            memory_available_gb: Available memory in GB
        """
        # Add current CPU sample to history
        self.cpu_samples.append(cpu_usage)
        
        # Check if resources are low (recovery condition)
        cpu_low = cpu_usage < 50.0
        memory_ok = memory_available_gb >= 2.0
        resources_recovered = cpu_low and memory_ok
        
        if self.monitoring_state == "monitoring":
            # Normal state - check if we need to downgrade
            if cpu_usage >= 85.0:
                # High CPU detected - will trigger downgrade in _monitor_cycle
                logger.debug(f"State: monitoring -> CPU high ({cpu_usage:.1f}%)")
        
        elif self.monitoring_state == "degraded":
            # Degraded state - check for recovery
            if resources_recovered:
                # Start recovery counter
                self.recovery_sample_count += 1
                logger.debug(f"Recovery sample {self.recovery_sample_count}/10")
                
                if self.recovery_sample_count >= 10:
                    # 10 consecutive low samples (5 minutes @ 30s interval)
                    self.monitoring_state = "recovering"
                    logger.info("State: degraded -> recovering (10 samples of low resources)")
            else:
                # Resources still high - reset counter
                if self.recovery_sample_count > 0:
                    logger.debug(f"Recovery interrupted (CPU={cpu_usage:.1f}%, Mem={memory_available_gb:.2f}GB)")
                self.recovery_sample_count = 0
        
        elif self.monitoring_state == "recovering":
            # Recovery state - upgrade proposal will be sent in _monitor_cycle
            # Reset to monitoring after upgrade
            pass

    def detect_resources(self) -> Dict[str, Any]:
        """
        Detect system resources on startup (STT-REQ-006.1)

        Returns:
            Dict containing:
            - cpu_cores: int
            - total_memory_gb: float
            - gpu_available: bool
            - gpu_memory_gb: float
        """
        cpu_cores = psutil.cpu_count(logical=True)
        total_memory_bytes = psutil.virtual_memory().total
        total_memory_gb = total_memory_bytes / (1024 ** 3)

        # GPU detection (simplified - real implementation would use torch.cuda)
        gpu_available = False
        gpu_memory_gb = 0.0

        try:
            import torch
            if torch.cuda.is_available():
                gpu_available = True
                gpu_memory_gb = torch.cuda.get_device_properties(0).total_memory / (1024 ** 3)
        except ImportError:
            pass  # torch not installed, GPU not available

        resources = {
            'cpu_cores': cpu_cores,
            'total_memory_gb': total_memory_gb,
            'gpu_available': gpu_available,
            'gpu_memory_gb': gpu_memory_gb
        }

        logger.info(f"Detected resources: {resources}")
        self.resources = resources
        return resources

    def select_model(self, resources: Dict[str, Any]) -> str:
        """
        Select Whisper model based on system resources (STT-REQ-006.2)

        Model selection rules:
        - GPU + 8GB+ memory + 10GB+ GPU mem → large-v3
        - GPU + 4GB+ memory + 5GB+ GPU mem → medium
        - CPU + 4GB+ memory → small
        - CPU + 2GB+ memory → base
        - < 2GB memory → tiny

        Args:
            resources: Dict with cpu_cores, total_memory_gb, gpu_available, gpu_memory_gb

        Returns:
            Model name: 'large-v3' | 'medium' | 'small' | 'base' | 'tiny'
        """
        memory_gb = resources['total_memory_gb']
        gpu_available = resources['gpu_available']
        gpu_memory_gb = resources.get('gpu_memory_gb', 0.0)

        # GPU-based selection
        if gpu_available:
            if memory_gb >= 8 and gpu_memory_gb >= 10:
                model = 'large-v3'
            elif memory_gb >= 4 and gpu_memory_gb >= 5:
                model = 'medium'
            else:
                # Fallback to CPU selection
                model = self._select_cpu_model(memory_gb)
        else:
            # CPU-only selection
            model = self._select_cpu_model(memory_gb)

        logger.info(f"Selected model: {model} (memory: {memory_gb:.1f}GB, GPU: {gpu_available})")
        self.current_model = model
        self.initial_model = model
        return model

    def _select_cpu_model(self, memory_gb: float) -> str:
        """
        Select model for CPU-only systems

        Args:
            memory_gb: Total system memory in GB

        Returns:
            Model name
        """
        if memory_gb >= 4:
            return 'small'
        elif memory_gb >= 2:
            return 'base'
        else:
            return 'tiny'

    def get_current_memory_usage(self) -> float:
        """
        Get current application memory usage in GB (STT-REQ-006.6)

        Returns:
            Current application memory usage in GB (RSS - Resident Set Size)
        """
        memory_info = self.process.memory_info()
        app_memory_gb = memory_info.rss / (1024 ** 3)
        return app_memory_gb

    def get_current_cpu_usage(self) -> float:
        """
        Get current CPU usage percentage (STT-REQ-006.7)

        Returns:
            CPU usage percentage (0-100)

        Note: Uses interval=None for non-blocking behavior.
        Returns average CPU usage since last call.
        """
        cpu_percent = psutil.cpu_percent(interval=None)
        return cpu_percent

    def should_downgrade_cpu(self, cpu_percent: float, duration_seconds: float) -> bool:
        """
        Check if CPU-based downgrade is needed (STT-REQ-006.7)

        Args:
            cpu_percent: Current CPU usage percentage
            duration_seconds: Duration of high CPU usage

        Returns:
            True if downgrade is needed
        """
        # CPU >= 85% sustained for 60+ seconds
        if cpu_percent >= 85 and duration_seconds >= 60:
            return True
        return False

    def should_downgrade_memory(self, memory_gb: float) -> tuple[bool, Optional[str]]:
        """
        Check if memory-based downgrade is needed (STT-REQ-006.8)

        Args:
            memory_gb: Current application memory usage in GB (not system-wide)

        Returns:
            Tuple of (should_downgrade, target_model)
            - should_downgrade: True if downgrade is needed
            - target_model: 'base' for immediate downgrade, or None for gradual

        Note: Thresholds are for application memory usage (RSS), not system-wide.
              Whisper models typically use:
              - tiny: ~100MB, base: ~200MB, small: ~500MB, medium: ~1.5GB, large-v3: ~3GB
        """
        # App memory >= 2.0GB: immediate downgrade to base (critical threshold)
        # This indicates the model itself is consuming excessive memory
        if memory_gb >= 2.0:
            logger.warning(f"Application memory critically high: {memory_gb:.2f}GB (>= 2.0GB)")
            return (True, 'base')

        # App memory >= 1.5GB: gradual downgrade (1 step)
        # Prevents reaching critical threshold
        if memory_gb >= 1.5:
            logger.warning(f"Application memory high: {memory_gb:.2f}GB (>= 1.5GB)")
            return (True, None)  # Gradual downgrade (1 step)

        return (False, None)

    def get_downgrade_target(self, current_model: str) -> Optional[str]:
        """
        Get next model in downgrade sequence (STT-REQ-006.6)

        Downgrade sequence: large-v3 → medium → small → base → tiny

        Args:
            current_model: Current model name

        Returns:
            Next model in sequence, or None if already at minimum
        """
        try:
            current_index = self.MODEL_SEQUENCE.index(current_model)
            if current_index < len(self.MODEL_SEQUENCE) - 1:
                return self.MODEL_SEQUENCE[current_index + 1]
            else:
                return None  # Already at tiny
        except ValueError:
            logger.warning(f"Unknown model: {current_model}")
            return None

    def downgrade_model(self) -> Optional[tuple[str, str]]:
        """
        Execute model downgrade (STT-REQ-006.9)

        Returns:
            Tuple of (old_model, new_model), or None if cannot downgrade
        """
        old_model = self.current_model
        new_model = self.get_downgrade_target(old_model)

        if new_model is None:
            logger.warning(f"Cannot downgrade from {old_model} (already at minimum)")
            return None

        self.current_model = new_model
        logger.info(f"Model downgraded: {old_model} → {new_model}")
        return (old_model, new_model)

    def create_downgrade_notification(self, old_model: str, new_model: str) -> Dict[str, Any]:
        """
        Create UI notification for model downgrade (STT-REQ-006.9)

        Args:
            old_model: Previous model name
            new_model: New model name

        Returns:
            Notification dictionary with type, message, old_model, new_model
        """
        notification = {
            'type': 'model_change',
            'message': f'モデル変更: {old_model} → {new_model}',
            'old_model': old_model,
            'new_model': new_model
        }
        logger.info(f"Created downgrade notification: {notification['message']}")
        return notification

    def should_propose_upgrade(self, memory_gb: float, cpu_percent: float, duration_seconds: float) -> bool:
        """
        Check if upgrade proposal is needed (STT-REQ-006.10)

        Conditions:
        - Memory < 2GB AND CPU < 50% sustained for 5 minutes (300 seconds)

        Args:
            memory_gb: Current memory usage in GB
            cpu_percent: Current CPU usage percentage
            duration_seconds: Duration of low resource usage

        Returns:
            True if upgrade should be proposed
        """
        # Check resource conditions
        if memory_gb >= 2.0 or cpu_percent >= 50:
            return False

        # Check duration (5 minutes = 300 seconds)
        if duration_seconds < 300:
            return False

        # Don't propose upgrade if already at initial model
        if self.current_model == self.initial_model:
            return False

        return True

    def create_upgrade_proposal_notification(self, current_model: str, target_model: str) -> Dict[str, Any]:
        """
        Create UI notification for upgrade proposal (STT-REQ-006.10)

        Args:
            current_model: Current model name
            target_model: Proposed upgrade target model

        Returns:
            Notification dictionary with type, message, current_model, target_model
        """
        notification = {
            'type': 'upgrade_proposal',
            'message': f'リソースが回復しました。モデルを {current_model} から {target_model} にアップグレードしますか？',
            'current_model': current_model,
            'target_model': target_model
        }
        logger.info(f"Created upgrade proposal notification: {notification['message']}")
        return notification

    def get_upgrade_target(self, current_model: str) -> Optional[str]:
        """
        Get next model in upgrade sequence (STT-REQ-006.12)

        Upgrade sequence: tiny → base → small → medium → large-v3

        Args:
            current_model: Current model name

        Returns:
            Next model in sequence, or None if already at maximum or above initial_model
        """
        try:
            current_index = self.MODEL_SEQUENCE.index(current_model)
            if current_index > 0:
                target_model = self.MODEL_SEQUENCE[current_index - 1]

                # Don't upgrade beyond initial_model
                if self.initial_model:
                    initial_index = self.MODEL_SEQUENCE.index(self.initial_model)
                    # MODEL_SEQUENCE is reverse order (large-v3=0, tiny=4)
                    # Smaller index = larger model
                    # Block if target_index < initial_index (would be larger model)
                    if current_index - 1 < initial_index:
                        return None  # Would exceed initial model

                return target_model
            else:
                return None  # Already at large-v3
        except ValueError:
            logger.warning(f"Unknown model: {current_model}")
            return None

    def upgrade_model(self) -> Optional[tuple[str, str]]:
        """
        Execute model upgrade (STT-REQ-006.12)

        Returns:
            Tuple of (old_model, new_model), or None if cannot upgrade
        """
        old_model = self.current_model
        new_model = self.get_upgrade_target(old_model)

        if new_model is None:
            logger.warning(f"Cannot upgrade from {old_model} (already at maximum or initial model)")
            return None

        self.current_model = new_model
        logger.info(f"Model upgraded: {old_model} → {new_model}")
        return (old_model, new_model)

    def should_pause_recording(self, current_model: str, memory_gb: float, cpu_percent: float) -> bool:
        """
        Check if recording should be paused (STT-REQ-006.11)

        Conditions:
        - Current model is 'tiny' AND resources still insufficient
        - App memory usage >= 2.0GB OR CPU >= 85%

        Args:
            current_model: Current model name
            memory_gb: Current application memory usage in GB (not system-wide)
            cpu_percent: Current CPU usage percentage (0-100)

        Returns:
            True if recording should be paused
        """
        # Only pause if already on tiny model
        if current_model != 'tiny':
            return False

        # Check if resources are still insufficient
        # App memory usage >= 2.0GB (critical threshold for application)
        if memory_gb >= 2.0:
            logger.error(f"Tiny model still uses {memory_gb:.2f}GB (>= 2.0GB), pausing recording")
            return True

        # CPU >= 85% (downgrade threshold)
        if cpu_percent >= 85:
            logger.error(f"CPU still at {cpu_percent:.1f}% (>= 85%) with tiny model, pausing recording")
            return True

        return False

    def create_recording_pause_notification(self) -> Dict[str, Any]:
        """
        Create UI notification for recording pause (STT-REQ-006.11)

        Returns:
            Notification dictionary with type, message, reason
        """
        notification = {
            'type': 'recording_paused',
            'message': 'システムリソース不足のため録音を一時停止しました',
            'reason': 'insufficient_resources'
        }
        logger.warning(f"Created recording pause notification: {notification['message']}")
        return notification

    async def start_monitoring(
        self,
        interval_seconds: float = 30.0,
        on_downgrade: Optional[Callable[[str, str], Awaitable[None]]] = None,
        on_upgrade_proposal: Optional[Callable[[str, str], Awaitable[None]]] = None,
        on_pause_recording: Optional[Callable[[], Awaitable[None]]] = None
    ):
        """
        Start monitoring loop (STT-NFR-001.6, STT-NFR-005.4)

        Monitors CPU/memory every `interval_seconds` (default 30s) and triggers:
        - Downgrade after CPU >= 85% for 60+ seconds
        - Upgrade proposal after resources recovered for 5+ minutes
        - Recording pause when tiny model is insufficient

        Args:
            interval_seconds: Monitoring interval (default 30s)
            on_downgrade: Async callback for downgrade: (old_model, new_model) -> None
            on_upgrade_proposal: Async callback for upgrade proposal: (current, target) -> None
            on_pause_recording: Async callback for recording pause: () -> None
        """
        self.monitoring_running = True
        self.monitoring_cycle_count = 0

        logger.info(f"Starting resource monitoring loop (interval={interval_seconds}s)")

        while self.monitoring_running:
            try:
                await self._monitor_cycle(on_downgrade, on_upgrade_proposal, on_pause_recording)
                self.monitoring_cycle_count += 1
            except Exception as e:
                logger.error(f"Error in monitoring cycle: {e}", exc_info=True)

            # Wait for next cycle
            await asyncio.sleep(interval_seconds)

    async def stop_monitoring(self):
        """Stop monitoring loop"""
        self.monitoring_running = False
        logger.info(f"Stopped resource monitoring (total cycles: {self.monitoring_cycle_count})")

    async def _monitor_cycle(
        self,
        on_downgrade: Optional[Callable[[str, str], Awaitable[None]]],
        on_upgrade_proposal: Optional[Callable[[str, str], Awaitable[None]]],
        on_pause_recording: Optional[Callable[[], Awaitable[None]]]
    ):
        """
        Execute one monitoring cycle (Phase 1.1 enhanced)

        Checks CPU/memory usage and triggers appropriate actions.
        Integrates hierarchical state machine and debounce logic.
        """
        # Get current resource usage
        cpu_percent = self.get_current_cpu_usage()
        app_memory_gb = self.get_current_memory_usage()  # Application memory (RSS)

        # System memory for informational logging
        system_memory = psutil.virtual_memory()
        system_memory_percent = system_memory.percent
        system_memory_available_gb = system_memory.available / (1024 ** 3)

        # Phase 1.1: Update state machine (hierarchical tracking)
        self._update_state_machine(cpu_percent, system_memory_available_gb)

        # Log current status (STT-NFR-005.4)
        logger.debug(
            f"Resource monitoring: CPU={cpu_percent:.1f}%, "
            f"App Memory={app_memory_gb:.2f}GB, "
            f"System Memory={system_memory_percent:.1f}%, "
            f"Model={self.current_model}, State={self.monitoring_state}"
        )

        # Track CPU high duration (STT-REQ-006.7)
        current_time = time.time()
        if cpu_percent >= 85:
            if self.cpu_high_start_time is None:
                self.cpu_high_start_time = current_time
                logger.debug(f"CPU high started: {cpu_percent:.1f}%")
        else:
            # Reset if CPU drops below threshold
            if self.cpu_high_start_time is not None:
                logger.debug("CPU high ended")
            self.cpu_high_start_time = None

        # Track low resource duration for upgrade proposal (STT-REQ-006.10)
        # Use app memory < 500MB (not system-wide) as low resource indicator
        if app_memory_gb < 0.5 and cpu_percent < 50:  # Resource recovery conditions
            if self.low_resource_start_time is None:
                self.low_resource_start_time = current_time
                logger.debug("Low resource period started")
        else:
            # Reset if resources go high again
            if self.low_resource_start_time is not None:
                logger.debug("Low resource period ended")
            self.low_resource_start_time = None

        # Check for memory-based downgrade trigger (STT-REQ-006.8)
        should_downgrade_mem, target_model = self.should_downgrade_memory(app_memory_gb)
        if should_downgrade_mem:
            # Skip downgrade if already at tiny model (cannot downgrade further)
            if self.current_model == 'tiny':
                logger.debug("Already at tiny model, cannot downgrade further for memory")
            elif target_model == 'base':
                # Immediate downgrade to base (app memory >= 2.0GB)
                logger.error(f"Critical app memory usage {app_memory_gb:.2f}GB (>= 2.0GB), forcing downgrade to base")
                old_model = self.current_model
                # Don't update current_model here - let callback handle it after success
                if old_model == 'base':
                    logger.debug("Already at base model, skipping repeated memory downgrade")
                else:
                    if on_downgrade:
                        await on_downgrade(old_model, 'base')
                        # Phase 1.1 FIX: Update state machine (指摘2対応)
                        self.last_downgrade_timestamp = current_time
                        self.monitoring_state = "degraded"
                        self.recovery_sample_count = 0  # 指摘3対応: リカバリーカウンターリセット
                    # Reset CPU timer as we just downgraded
                    self.cpu_high_start_time = None
            else:
                # Gradual downgrade (app memory >= 1.5GB)
                logger.warning(f"High app memory usage {app_memory_gb:.2f}GB (>= 1.5GB), triggering gradual downgrade")
                old_model = self.current_model
                new_model = self.get_downgrade_target(old_model)
                # Don't update current_model here - let callback handle it after success
                if new_model and on_downgrade:
                    await on_downgrade(old_model, new_model)
                    # Phase 1.1 FIX: Update state machine (指摘2対応)
                    self.last_downgrade_timestamp = current_time
                    self.monitoring_state = "degraded"
                    self.recovery_sample_count = 0  # 指摘3対応: リカバリーカウンターリセット
                # Reset CPU timer as we just downgraded
                self.cpu_high_start_time = None

        # Check for CPU-based downgrade trigger (STT-REQ-006.7, Phase 1.1 with debounce)
        elif self.cpu_high_start_time is not None:
            cpu_high_duration = current_time - self.cpu_high_start_time
            if cpu_high_duration >= 60:  # 60 seconds sustained
                # Phase 1.1: Debounce check (prevent chattering)
                if self._should_debounce_downgrade():
                    logger.debug("CPU downgrade skipped due to debounce")
                    # Don't reset timer - let it accumulate for next cycle
                else:
                    logger.warning(f"CPU high for {cpu_high_duration:.1f}s, triggering downgrade")
                    old_model = self.current_model
                    new_model = self.get_downgrade_target(old_model)
                    # Don't update current_model here - let callback handle it after success
                    if new_model and on_downgrade:
                        await on_downgrade(old_model, new_model)
                        # Update debounce timestamp after successful downgrade
                        self.last_downgrade_timestamp = current_time
                        # Transition to degraded state
                        self.monitoring_state = "degraded"
                        self.recovery_sample_count = 0  # 指摘3対応: リカバリーカウンターリセット
                    # Reset timer after downgrade
                    self.cpu_high_start_time = None

        # Check for upgrade proposal (STT-REQ-006.10)
        if self.low_resource_start_time is not None:
            low_resource_duration = current_time - self.low_resource_start_time
            if low_resource_duration >= 300:  # 5 minutes
                # Propose upgrade to initial_model if set and higher than current
                target = None
                if self.initial_model and self.initial_model != self.current_model:
                    # Check if initial_model is higher in hierarchy
                    models = ['tiny', 'base', 'small', 'medium', 'large-v3']
                    try:
                        current_idx = models.index(self.current_model)
                        initial_idx = models.index(self.initial_model)
                        if initial_idx > current_idx:
                            target = self.initial_model
                    except ValueError:
                        pass

                if not target:
                    # Fallback to one-step upgrade
                    target = self.get_upgrade_target(self.current_model)

                if target and on_upgrade_proposal:
                    logger.info(f"Resource recovered for {low_resource_duration:.1f}s, proposing upgrade to {target}")
                    await on_upgrade_proposal(self.current_model, target)
                    # Phase 1.1: Reset state machine after upgrade proposal
                    self.monitoring_state = "monitoring"
                    self.recovery_sample_count = 0
                # Reset timer after proposal (only propose once per recovery period)
                self.low_resource_start_time = None

        # Check for recording pause (STT-REQ-006.11)
        if self.current_model == 'tiny':
            if self.should_pause_recording('tiny', app_memory_gb, cpu_percent):
                logger.error("Tiny model insufficient, pausing recording")
                if on_pause_recording:
                    await on_pause_recording()


if __name__ == "__main__":
    # Quick test
    monitor = ResourceMonitor()
    resources = monitor.detect_resources()
    model = monitor.select_model(resources)
    print(f"Resources: {resources}")
    print(f"Selected model: {model}")

"""
ResourceMonitor - システムリソース監視と動的モデルダウングレード

Related Requirements:
- STT-REQ-006.1: システムリソース検出（CPU、メモリ、GPU）
- STT-REQ-006.2: モデル選択ルール適用
- STT-REQ-006.3: モデル選択のログ記録
- STT-REQ-006.6-006.12: 動的ダウングレードとアップグレード提案
"""

import logging
import psutil
from typing import Dict, Any, Optional

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

    def __init__(self):
        """Initialize ResourceMonitor"""
        self.current_model = None
        self.initial_model = None
        self.resources = None
        self.cpu_high_start_time = None  # Track when CPU went high
        logger.info("ResourceMonitor initialized")

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
        Get current memory usage in GB (STT-REQ-006.6)

        Returns:
            Current memory usage in GB
        """
        memory_info = psutil.virtual_memory()
        used_memory_gb = memory_info.used / (1024 ** 3)
        return used_memory_gb

    def get_current_cpu_usage(self) -> float:
        """
        Get current CPU usage percentage (STT-REQ-006.7)

        Returns:
            CPU usage percentage (0-100)
        """
        cpu_percent = psutil.cpu_percent(interval=1)
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
            memory_gb: Current memory usage in GB

        Returns:
            Tuple of (should_downgrade, target_model)
            - should_downgrade: True if downgrade is needed
            - target_model: 'base' for immediate downgrade, or None
        """
        # Memory >= 4GB: immediate downgrade to base
        if memory_gb >= 4.0:
            return (True, 'base')

        # Memory >= 3GB: gradual downgrade (1 step)
        if memory_gb >= 3.0:
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


if __name__ == "__main__":
    # Quick test
    monitor = ResourceMonitor()
    resources = monitor.detect_resources()
    model = monitor.select_model(resources)
    print(f"Resources: {resources}")
    print(f"Selected model: {model}")

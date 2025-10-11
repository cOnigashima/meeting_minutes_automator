"""
WhisperSTTEngine implementation using faster-whisper.

This module provides the core STT (Speech-To-Text) functionality using the
faster-whisper library, which is a CTranslate2-optimized version of OpenAI Whisper.

Requirements:
- STT-REQ-002: faster-whisper Integration (Offline-First)
- STT-REQ-002.1: Model detection priority (user config → HF cache → bundled)
- STT-REQ-002.10: Output "whisper_model_ready" message to stdout
"""

import sys
import json
import logging
import psutil
from pathlib import Path
from typing import Literal, Optional, Dict, Union
from faster_whisper import WhisperModel

# Type definition for model sizes (STT-REQ-002.2)
ModelSize = Literal["tiny", "base", "small", "medium", "large-v3"]

logger = logging.getLogger(__name__)


class WhisperSTTEngine:
    """
    WhisperSTTEngine handles audio transcription using faster-whisper.

    Attributes:
        model_size: The Whisper model size to use
        model: The loaded WhisperModel instance (None until initialized)
        model_path: The detected or configured model path
    """

    def __init__(
        self,
        model_size: Optional[ModelSize] = None,
        auto_select_model: bool = False
    ):
        """
        Initialize WhisperSTTEngine with specified or auto-selected model size.

        Args:
            model_size: Model size to use. If None and auto_select_model=True,
                       will auto-select based on system resources.
            auto_select_model: If True, automatically select model based on
                              system resources (unless model_size is explicitly provided)
        """
        self.model: Optional[WhisperModel] = None
        self.model_path: Optional[str] = None

        # Auto-select model if enabled and no explicit model_size provided
        if auto_select_model and model_size is None:
            resources = self._detect_system_resources()
            self.model_size = self._select_model_by_resources(resources)
            logger.info(f"Auto-selected model based on system resources: {self.model_size}")
        elif model_size is not None:
            self.model_size = model_size
            # Check if manually selected model exceeds resources (STT-REQ-006.5)
            if auto_select_model:
                resources = self._detect_system_resources()
                if self._check_model_exceeds_resources(model_size, resources):
                    logger.warning(
                        f"Manually selected model '{model_size}' may exceed system resources. "
                        f"Consider using a smaller model for better performance."
                    )
        else:
            # Default to 'small' if no model specified and auto_select disabled
            self.model_size = "small"

        logger.info(f"WhisperSTTEngine initialized with model_size={self.model_size}")

    def _detect_system_resources(self) -> Dict[str, Union[int, float, bool]]:
        """
        Detect system resources (STT-REQ-006.1).

        Returns:
            Dict with keys:
                - cpu_cores: Number of CPU cores
                - memory_gb: Total memory in GB
                - has_gpu: Whether GPU is available
                - gpu_memory_gb: GPU memory in GB (0 if no GPU)
        """
        # Detect CPU cores
        cpu_cores = psutil.cpu_count(logical=True) or 1

        # Detect total memory in GB
        memory_info = psutil.virtual_memory()
        memory_gb = memory_info.total / (1024 ** 3)  # Convert bytes to GB

        # Detect GPU availability
        has_gpu = False
        gpu_memory_gb = 0

        try:
            import torch
            if torch.cuda.is_available():
                has_gpu = True
                # Get GPU memory in GB
                gpu_memory_gb = torch.cuda.get_device_properties(0).total_memory / (1024 ** 3)
        except ImportError:
            # torch not available, assume no GPU
            pass
        except Exception as e:
            logger.debug(f"GPU detection failed: {e}")

        resources = {
            'cpu_cores': cpu_cores,
            'memory_gb': round(memory_gb, 2),
            'has_gpu': has_gpu,
            'gpu_memory_gb': round(gpu_memory_gb, 2)
        }

        logger.info(f"Detected system resources: {resources}")
        return resources

    def _select_model_by_resources(self, resources: Dict[str, Union[int, float, bool]]) -> ModelSize:
        """
        Select optimal Whisper model based on system resources (STT-REQ-006.2, STT-REQ-006.3).

        Args:
            resources: System resource information from _detect_system_resources()

        Returns:
            ModelSize: Selected model size

        Selection rules:
            - GPU + 8GB RAM + 10GB VRAM → large-v3 (highest accuracy)
            - GPU + 4GB RAM + 5GB VRAM → medium (balanced)
            - CPU + 4GB RAM → small (realistic CPU inference limit)
            - CPU + 2GB RAM → base (low resource)
            - RAM < 2GB → tiny (minimal operation guarantee)
        """
        memory_gb = resources['memory_gb']
        has_gpu = resources['has_gpu']
        gpu_memory_gb = resources['gpu_memory_gb']

        selected_model: ModelSize

        if has_gpu and memory_gb >= 8 and gpu_memory_gb >= 10:
            selected_model = "large-v3"
        elif has_gpu and memory_gb >= 4 and gpu_memory_gb >= 5:
            selected_model = "medium"
        elif not has_gpu and memory_gb >= 4:
            selected_model = "small"
        elif memory_gb >= 2:
            selected_model = "base"
        else:
            selected_model = "tiny"

        logger.info(f"Selected model '{selected_model}' based on resources: "
                    f"CPU={not has_gpu}, Memory={memory_gb}GB, GPU Memory={gpu_memory_gb}GB")

        return selected_model

    def _check_model_exceeds_resources(
        self,
        model_size: ModelSize,
        resources: Dict[str, Union[int, float, bool]]
    ) -> bool:
        """
        Check if manually selected model exceeds system resources (STT-REQ-006.5).

        Args:
            model_size: The manually selected model size
            resources: System resource information

        Returns:
            bool: True if model exceeds resources, False otherwise
        """
        memory_gb = resources['memory_gb']
        has_gpu = resources['has_gpu']
        gpu_memory_gb = resources['gpu_memory_gb']

        # Define minimum resource requirements for each model
        model_requirements = {
            "large-v3": {"memory_gb": 8, "gpu_memory_gb": 10, "requires_gpu": True},
            "medium": {"memory_gb": 4, "gpu_memory_gb": 5, "requires_gpu": True},
            "small": {"memory_gb": 4, "gpu_memory_gb": 0, "requires_gpu": False},
            "base": {"memory_gb": 2, "gpu_memory_gb": 0, "requires_gpu": False},
            "tiny": {"memory_gb": 1, "gpu_memory_gb": 0, "requires_gpu": False}
        }

        requirements = model_requirements.get(model_size, {})

        # Check if resources are insufficient
        if requirements.get("requires_gpu", False) and not has_gpu:
            logger.warning(
                f"Model '{model_size}' requires GPU, but no GPU detected. "
                f"Performance may be significantly degraded."
            )
            return True

        if memory_gb < requirements.get("memory_gb", 0):
            logger.warning(
                f"Model '{model_size}' requires {requirements['memory_gb']}GB RAM, "
                f"but only {memory_gb}GB available."
            )
            return True

        if has_gpu and gpu_memory_gb < requirements.get("gpu_memory_gb", 0):
            logger.warning(
                f"Model '{model_size}' requires {requirements['gpu_memory_gb']}GB VRAM, "
                f"but only {gpu_memory_gb}GB available."
            )
            return True

        return False

    def _detect_model_path(self) -> str:
        """
        Detect model path following priority order (STT-REQ-002.1):
        1. User-specified path (~/.config/meeting-minutes-automator/whisper_model_path)
        2. HuggingFace Hub cache (~/.cache/huggingface/hub/models--Systran--faster-whisper-*)
        3. Bundled model ([app_resources]/models/faster-whisper/base)

        Returns:
            str: Detected model path

        Raises:
            FileNotFoundError: If no model path is found
        """
        # Priority 1: User-specified path
        user_config_path = Path.home() / ".config" / "meeting-minutes-automator" / "whisper_model_path"
        if user_config_path.exists():
            with open(user_config_path, 'r') as f:
                custom_path = f.read().strip()
                if Path(custom_path).exists():
                    logger.info(f"Using user-specified model path: {custom_path}")
                    return custom_path

        # Priority 2: HuggingFace Hub cache
        hf_cache_base = Path.home() / ".cache" / "huggingface" / "hub"
        model_name_in_cache = f"models--Systran--faster-whisper-{self.model_size}"
        hf_model_path = hf_cache_base / model_name_in_cache

        if hf_model_path.exists():
            logger.info(f"Using HuggingFace cache model: {hf_model_path}")
            return str(hf_model_path)

        # Priority 3: Bundled model (fallback to base model)
        bundled_path = f"[app_resources]/models/faster-whisper/base"
        logger.info(f"Falling back to bundled model: {bundled_path}")
        return bundled_path

    async def initialize(self) -> None:
        """
        Initialize the WhisperSTTEngine by loading the faster-whisper model.

        This method:
        1. Detects the model path using priority order
        2. Loads the faster-whisper model
        3. Outputs "whisper_model_ready" message to stdout (STT-REQ-002.10)

        Raises:
            Exception: If model loading fails
        """
        try:
            # Detect model path
            self.model_path = self._detect_model_path()
            logger.info(f"Detected model path: {self.model_path}")

            # Load faster-whisper model
            # Note: In actual implementation, we would load the real model
            # For now, we create a mock model to pass tests
            logger.info(f"Loading faster-whisper model: {self.model_size}")

            # TODO: Actual model loading will be implemented in Task 3.3 and 3.4
            # For now, we simulate model loading for testing purposes
            self.model = WhisperModel(
                self.model_path,
                device="cpu",
                compute_type="int8"
            )

            logger.info("WhisperModel loaded successfully")

            # Output ready message to stdout (STT-REQ-002.10)
            ready_message = json.dumps({
                "type": "event",
                "event": "whisper_model_ready",
                "model_size": self.model_size,
                "model_path": self.model_path
            })
            sys.stdout.write(f"{ready_message}\n")
            sys.stdout.flush()

            logger.info("WhisperSTTEngine initialization complete")

        except Exception as e:
            logger.error(f"Failed to initialize WhisperSTTEngine: {e}")
            raise

    async def transcribe(self, audio_data: bytes, sample_rate: int = 16000, is_final: bool = False) -> dict:
        """
        Transcribe audio data to text (placeholder for Task 3.4).

        Args:
            audio_data: Raw audio data as bytes
            sample_rate: Audio sample rate (default: 16000 Hz)
            is_final: Whether this is a final transcription or partial

        Returns:
            dict: Transcription result with text, confidence, language, etc.

        Note:
            Full implementation will be done in Task 3.4 (faster-whisper推論機能)
        """
        if self.model is None:
            raise RuntimeError("WhisperSTTEngine not initialized. Call initialize() first.")

        # Placeholder implementation
        logger.debug(f"Transcribe called: sample_rate={sample_rate}, is_final={is_final}")

        return {
            "text": "",
            "confidence": 0.0,
            "language": "ja",
            "is_final": is_final,
            "processing_time_ms": 0
        }

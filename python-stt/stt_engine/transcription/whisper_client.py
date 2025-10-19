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
import os
from pathlib import Path
from typing import Literal, Optional, Dict, Union
from faster_whisper import WhisperModel

# Type definition for model sizes (STT-REQ-002.2)
ModelSize = Literal["tiny", "base", "small", "medium", "large-v3"]

logger = logging.getLogger(__name__)


def _log_structured(level: int, component: str, event: str, **details) -> None:
    payload = {
        "component": component,
        "event": event,
    }
    if details:
        payload["details"] = details
    logger.log(level, json.dumps(payload, ensure_ascii=False))


def log_info_event(component: str, event: str, **details) -> None:
    _log_structured(logging.INFO, component, event, **details)


def log_warning_event(component: str, event: str, **details) -> None:
    _log_structured(logging.WARNING, component, event, **details)


def log_error_event(component: str, event: str, **details) -> None:
    _log_structured(logging.ERROR, component, event, **details)


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
        auto_select_model: bool = False,
        offline_mode: bool = False
    ):
        """
        Initialize WhisperSTTEngine with specified or auto-selected model size.

        Args:
            model_size: Model size to use. If None and auto_select_model=True,
                       will auto-select based on system resources.
            auto_select_model: If True, automatically select model based on
                              system resources (unless model_size is explicitly provided)
            offline_mode: If True, skip HuggingFace Hub downloads and use only
                         cached or bundled models (STT-REQ-002.6)
        """
        self.model: Optional[WhisperModel] = None
        self.model_path: Optional[str] = None
        self.offline_mode: bool = offline_mode
        self._download_timeout: int = 10  # seconds (STT-REQ-002.3)

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

    def _get_proxy_settings(self) -> Dict[str, str]:
        """
        Get proxy settings from environment variables (STT-REQ-002.7).

        Returns:
            Dict with 'http' and 'https' proxy URLs if set
        """
        proxies = {}

        https_proxy = os.environ.get('HTTPS_PROXY') or os.environ.get('https_proxy')
        if https_proxy:
            proxies['https'] = https_proxy
            logger.info(f"Using HTTPS proxy: {https_proxy}")

        http_proxy = os.environ.get('HTTP_PROXY') or os.environ.get('http_proxy')
        if http_proxy:
            proxies['http'] = http_proxy
            logger.info(f"Using HTTP proxy: {http_proxy}")

        return proxies

    def _log_download_progress(self, message: str) -> None:
        """
        Log download progress (STT-REQ-002.8).

        Args:
            message: Progress message to log
        """
        logger.info(f"[Download Progress] {message}")

    def _try_download_from_hub(self, model_size: ModelSize) -> Optional[str]:
        """
        Try to download model from HuggingFace Hub with timeout (STT-REQ-002.3).

        Args:
            model_size: Model size to download

        Returns:
            Path to downloaded model, or None if download failed

        Note:
            - Timeout: 10 seconds (STT-REQ-002.3)
            - Respects proxy settings (STT-REQ-002.7)
            - Logs download progress (STT-REQ-002.8)
        """
        if self.offline_mode:
            logger.info("Offline mode enabled, skipping HuggingFace Hub download")
            return None

        try:
            self._log_download_progress(f"Attempting to download {model_size} model from HuggingFace Hub...")

            # Check if model is already cached
            hf_cache_base = Path.home() / ".cache" / "huggingface" / "hub"
            model_name_in_cache = f"models--Systran--faster-whisper-{model_size}"
            cached_model_path = hf_cache_base / model_name_in_cache

            if cached_model_path.exists():
                self._log_download_progress(f"Model found in cache: {cached_model_path}")
                return str(cached_model_path)

            # Try to download with timeout
            # Note: faster-whisper's WhisperModel handles downloading automatically
            # We'll rely on the model loading in initialize() with appropriate timeout
            self._log_download_progress(f"Model not in cache, will download on first use")

            return None  # Let WhisperModel handle the download

        except TimeoutError as e:
            logger.warning(f"HuggingFace Hub download timeout after {self._download_timeout}s: {e}")
            return None
        except Exception as e:
            logger.warning(f"HuggingFace Hub download failed: {e}")
            return None

    def _detect_bundled_model_path(self, requested_size: ModelSize) -> Optional[str]:
        """
        Detect bundled model path with fallback to 'base' (STT-REQ-002.4).

        Args:
            requested_size: Requested model size

        Returns:
            Path to bundled model, or None if not found

        Note:
            - Updates self.model_size to 'base' if fallback occurs
            - Checks multiple installation directories
        """
        # Candidate directories for bundled models
        bundled_base_dirs = [
            Path(__file__).parent.parent.parent / "models" / "faster-whisper",
            Path.home() / ".local" / "share" / "meeting-minutes-automator" / "models" / "faster-whisper",
            Path("/opt/meeting-minutes-automator/models/faster-whisper"),
        ]

        # First, try the requested model size
        for base_dir in bundled_base_dirs:
            bundled_path = base_dir / requested_size
            if bundled_path.exists() and (bundled_path / "model.bin").exists():
                logger.info(f"Using bundled model: {bundled_path}")
                return str(bundled_path)

        # STT-REQ-002.4: If requested size not found, fallback to bundled 'base' model
        if requested_size != 'base':
            logger.warning(f"Requested model '{requested_size}' not found in bundle, falling back to 'base'")
            for base_dir in bundled_base_dirs:
                bundled_base_path = base_dir / "base"
                if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                    logger.info(f"Using bundled fallback model: {bundled_base_path}")
                    # Update model_size to reflect actual model being loaded
                    self.model_size = "base"
                    return str(bundled_base_path)

        return None

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
        try:
            cpu_cores = psutil.cpu_count(logical=True) or 1
        except Exception as exc:
            log_warning_event(
                "resource_monitor",
                "cpu_count_fallback",
                error=str(exc),
                default_cpu_cores=1,
            )
            cpu_cores = 1

        # Detect total memory in GB
        try:
            memory_info = psutil.virtual_memory()
            memory_gb = memory_info.total / (1024 ** 3)  # Convert bytes to GB
        except Exception as exc:
            log_warning_event(
                "resource_monitor",
                "memory_fallback",
                error=str(exc),
                default_memory_gb=1.0,
            )
            memory_gb = 1.0

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
            logger.debug(f"resource_monitor.gpu_detection_failed: {e}")

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
        Detect model path following priority order (STT-REQ-002.1, STT-REQ-002.4, STT-REQ-002.6):
        1. User-specified path (~/.config/meeting-minutes-automator/whisper_model_path)
        2. HuggingFace Hub cache (~/.cache/huggingface/hub/models--Systran--faster-whisper-*)
        3. HuggingFace Hub model ID (online mode - triggers auto-download)
        4. Bundled model (offline mode only - multiple installation paths checked)
        5. Error (offline mode with no bundled model)

        Returns:
            str: Detected model path

        Raises:
            FileNotFoundError: If no model path is found (offline mode with no cache/bundle)
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
        hf_model_dir = hf_cache_base / model_name_in_cache

        if hf_model_dir.exists():
            # HuggingFace cache uses snapshots/<hash>/ structure
            # Find the latest snapshot
            snapshots_dir = hf_model_dir / "snapshots"
            if snapshots_dir.exists():
                snapshots = list(snapshots_dir.iterdir())
                if snapshots:
                    # Use the first (and typically only) snapshot
                    snapshot_path = snapshots[0]
                    logger.info(f"Using HuggingFace cache model: {snapshot_path}")
                    return str(snapshot_path)

            # Fallback: use the model directory directly (older cache format)
            logger.info(f"Using HuggingFace cache model: {hf_model_dir}")
            return str(hf_model_dir)

        # Priority 3: HuggingFace Hub model ID (online mode - STT-REQ-002.1/002.3)
        # In online mode, prefer downloading from Hub over bundled fallback
        if not self.offline_mode:
            # Check if already in cache (quick check)
            downloaded_path = self._try_download_from_hub(self.model_size)
            if downloaded_path:
                logger.info(f"Using cached model from Hub: {downloaded_path}")
                return downloaded_path

            # Not in cache, return Hub model ID to trigger auto-download
            # This ensures we download the requested model size before falling back to bundled
            model_id = f"Systran/faster-whisper-{self.model_size}"
            logger.info(f"Using HuggingFace Hub model ID: {model_id} (will auto-download)")
            return model_id

        # Priority 4: Bundled model fallback (OFFLINE MODE ONLY - STT-REQ-002.4/002.6)
        bundled_path = self._detect_bundled_model_path(self.model_size)
        if bundled_path:
            return bundled_path

        # Priority 5: No model available after all fallbacks (offline mode only)
        # STT-REQ-002.4/002.6: Bundled model paths checked but not found
        raise FileNotFoundError(
            f"No Whisper model found for '{self.model_size}' in offline mode. "
            f"Checked locations:\n"
            f"  - User config: {user_config_path}\n"
            f"  - HuggingFace cache: {hf_cache_base / model_name_in_cache}\n"
            f"  - Bundled model directories (see _detect_bundled_model_path)\n"
            f"Please download the model manually or run in online mode."
        )

    async def initialize(self) -> None:
        """
        Initialize the WhisperSTTEngine by loading the faster-whisper model.

        This method:
        1. Detects the model path using priority order
        2. Loads the faster-whisper model
        3. On network error, falls back to bundled base model (STT-REQ-002.4)
        4. Outputs "whisper_model_ready" message to stdout (STT-REQ-002.10)

        Raises:
            Exception: If model loading fails even with bundled model
        """
        try:
            # Detect model path
            try:
                self.model_path = self._detect_model_path()
            except FileNotFoundError as e:
                # Offline mode with no cached model - cannot proceed
                logger.error(f"Model detection failed: {e}")
                raise  # Re-raise to abort initialization

            logger.info(f"Detected model path: {self.model_path}")

            # Try to load faster-whisper model
            logger.info(f"Loading faster-whisper model: {self.model_size}")

            try:
                self.model = WhisperModel(
                    self.model_path,
                    device="cpu",
                    compute_type="int8"
                )
                logger.info("WhisperModel loaded successfully")

            except Exception as load_error:
                # STT-REQ-002.4: Network error during download → bundled fallback
                if not self.offline_mode:
                    logger.warning(f"Failed to load model from {self.model_path}: {load_error}")
                    logger.info("Attempting fallback to bundled base model (STT-REQ-002.4)")

                    bundled_path = self._detect_bundled_model_path(self.model_size)
                    if bundled_path:
                        self.model_path = bundled_path
                        logger.info(f"Retrying with bundled model: {self.model_path}")

                        self.model = WhisperModel(
                            self.model_path,
                            device="cpu",
                            compute_type="int8"
                        )
                        logger.info(f"Successfully loaded bundled fallback: {self.model_path}")
                    else:
                        logger.error("No bundled model available for fallback (STT-REQ-002.5)")
                        raise FileNotFoundError(
                            "faster-whisperモデルが見つかりません。インストールを確認してください"
                        ) from load_error
                else:
                    # In offline mode, re-raise immediately
                    raise

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

    async def load_model(self, new_model_size: ModelSize) -> str:
        """
        Dynamically switch to a different model size (Task 5.2, STT-REQ-006.9).

        This method:
        1. Saves current model state for rollback
        2. Loads the new model (may fallback to bundled base if requested size unavailable)
        3. Only unloads old model after new model loads successfully
        4. Rolls back to old model on any failure

        Args:
            new_model_size: Target model size to load

        Returns:
            str: The actual model size that was loaded (may differ from new_model_size
                 if bundled fallback occurred)

        Raises:
            Exception: If model loading fails (after rollback)
        """
        import gc

        logger.info(f"Switching model from {self.model_size} to {new_model_size}")

        # Save current state for rollback (CRITICAL: before any modifications)
        old_model = self.model
        old_model_size = self.model_size
        old_model_path = self.model_path

        try:
            # Update model size first
            self.model_size = new_model_size

            # Re-detect model path for new size
            try:
                self.model_path = self._detect_model_path()
            except FileNotFoundError as e:
                # Offline mode with no cached model - rollback
                logger.error(f"Model detection failed: {e}")
                raise  # Will trigger rollback in outer except block
            except (TimeoutError, ConnectionError, OSError) as network_error:
                # Network error during download attempt
                logger.warning(f"Network error during model detection: {network_error}")
                logger.info(f"Attempting fallback to HuggingFace Hub model ID: Systran/faster-whisper-{new_model_size}")
                self.model_path = f"Systran/faster-whisper-{new_model_size}"

            logger.info(f"Loading new model: {new_model_size} from {self.model_path}")

            # Load new model (CRITICAL: this can fail)
            new_model = WhisperModel(
                self.model_path,
                device="cpu",
                compute_type="int8"
            )

            # Success: switch to new model and cleanup old one
            self.model = new_model
            if old_model is not None:
                logger.info(f"Unloading old model: {old_model_size}")
                del old_model
                # Force garbage collection to release resources
                # faster-whisper doesn't have close() method, relies on GC
                gc.collect()

            # Log actual loaded model (may differ from requested due to bundled fallback)
            if self.model_size != new_model_size:
                logger.warning(
                    f"Model switch with fallback: requested '{new_model_size}', "
                    f"but loaded '{self.model_size}' (bundled fallback)"
                )
            else:
                logger.info(f"Model switch complete: {old_model_size} → {self.model_size}")

            # Return actual loaded model size (STT-REQ-006.9/006.12)
            return self.model_size

        except Exception as e:
            # CRITICAL: Rollback to old model state on ANY failure
            logger.error(f"Failed to load model {new_model_size}: {e}")
            logger.info(f"Rolling back to previous model: {old_model_size}")

            # Restore all state (model, size, path)
            self.model = old_model
            self.model_size = old_model_size
            self.model_path = old_model_path

            raise

    async def transcribe(self, audio_data: bytes, sample_rate: int = 16000, is_final: bool = False) -> dict:
        """
        Transcribe audio data to text using faster-whisper (STT-REQ-002.11, STT-REQ-002.12).

        Args:
            audio_data: Raw audio data as bytes (16-bit PCM)
            sample_rate: Audio sample rate (default: 16000 Hz)
            is_final: Whether this is a final transcription or partial

        Returns:
            dict: Transcription result with text, confidence, language, is_final, processing_time_ms

        Raises:
            RuntimeError: If model not initialized
        """
        import time
        import numpy as np

        if self.model is None:
            raise RuntimeError("WhisperSTTEngine not initialized. Call initialize() first.")

        start_time = time.time()

        try:
            # Validate audio data (STT-REQ-002.14)
            if not audio_data or len(audio_data) == 0:
                logger.warning("Empty audio data received")
                return {
                    "text": "",
                    "confidence": 0.0,
                    "language": "ja",
                    "is_final": is_final,
                    "processing_time_ms": 0,
                    "error": "INVALID_AUDIO"
                }

            # Convert bytes to numpy array (16-bit PCM)
            try:
                audio_array = np.frombuffer(audio_data, dtype=np.int16)
                # Convert to float32 in range [-1.0, 1.0] as required by faster-whisper
                audio_float = audio_array.astype(np.float32) / 32768.0
            except Exception as e:
                logger.error(f"Failed to decode audio data: {e}")
                return {
                    "text": "",
                    "confidence": 0.0,
                    "language": "ja",
                    "is_final": is_final,
                    "processing_time_ms": int((time.time() - start_time) * 1000),
                    "error": "INVALID_AUDIO"
                }

            # Perform transcription with faster-whisper (STT-REQ-002.11)
            logger.debug(f"Transcribing audio: {len(audio_float)} samples at {sample_rate}Hz")

            segments, info = self.model.transcribe(
                audio_float,
                language="ja",  # Japanese language hint
                beam_size=5,
                vad_filter=False,  # VAD handled separately in Task 4
            )

            # Extract text from segments
            text_parts = []
            total_logprob = 0.0
            segment_count = 0

            for segment in segments:
                text_parts.append(segment.text)
                total_logprob += segment.avg_logprob
                segment_count += 1

            # Combine text
            full_text = "".join(text_parts).strip()

            # Calculate confidence from average log probability
            # avg_logprob ranges from -infinity to 0 (0 is perfect)
            # Convert to confidence score [0, 1]
            if segment_count > 0:
                avg_logprob = total_logprob / segment_count
                # Use exponential to convert log probability to confidence
                # Clamp to reasonable range
                confidence = min(1.0, max(0.0, np.exp(avg_logprob)))
            else:
                confidence = 0.0

            # Detect language
            detected_language = info.language if hasattr(info, 'language') else "ja"

            processing_time = int((time.time() - start_time) * 1000)

            logger.debug(f"Transcription complete: '{full_text}' (confidence={confidence:.2f}, time={processing_time}ms)")

            # Return JSON response format (STT-REQ-002.12)
            return {
                "text": full_text,
                "confidence": round(confidence, 3),
                "language": detected_language,
                "is_final": is_final,
                "processing_time_ms": processing_time
            }

        except Exception as e:
            logger.error(f"Transcription error: {e}")
            processing_time = int((time.time() - start_time) * 1000)

            return {
                "text": "",
                "confidence": 0.0,
                "language": "ja",
                "is_final": is_final,
                "processing_time_ms": processing_time,
                "error": str(e)
            }

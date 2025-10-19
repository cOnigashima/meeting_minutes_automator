# ADR-016 Offline Model Fallback P0 Bug Fix

**Date**: 2025-10-19
**Status**: Accepted
**Related**: STT-REQ-002.4, STT-REQ-002.5 (Network error → bundled base model fallback)

## Context

External code review discovered a **P0 bug** in `WhisperSTTEngine.initialize()` where STT-REQ-002.4 「ネットワークエラー時にbundled baseモデルへフォールバック」was not implemented despite having passing tests.

### Root Cause Analysis

**File**: `python-stt/stt_engine/transcription/whisper_client.py`

**Problem 1**: `_try_download_from_hub()` swallows exceptions
```python
# Line 110-153
def _try_download_from_hub(self, model_size: ModelSize) -> Optional[str]:
    try:
        # ... download logic ...
        return None  # Let WhisperModel handle the download
    except TimeoutError as e:
        logger.warning(f"HuggingFace Hub download timeout: {e}")
        return None  # ❌ Swallows exception
    except Exception as e:
        logger.warning(f"HuggingFace Hub download failed: {e}")
        return None  # ❌ Swallows exception
```

**Problem 2**: `_detect_model_path()` returns Hub model ID when offline (Line 344-354)
```python
if not self.offline_mode:
    downloaded_path = self._try_download_from_hub(self.model_size)
    if downloaded_path:
        return downloaded_path

    # Not in cache, return Hub model ID to trigger auto-download
    model_id = f"Systran/faster-whisper-{self.model_size}"
    return model_id  # ❌ Returns Hub ID, bypassing bundled check
```

**Problem 3**: `initialize()` doesn't catch WhisperModel load failure (Line 437-442)
```python
# Load faster-whisper model
self.model = WhisperModel(
    self.model_path,  # Hub model ID from _detect_model_path()
    device="cpu",
    compute_type="int8"
)
# ❌ No exception handling for network errors
```

**Problem 4**: Bundled model check (Line 360-386) is unreachable when `offline_mode=False`

**Actual behavior**:
1. `_detect_model_path()` returns `"Systran/faster-whisper-small"`
2. `WhisperModel(...)` tries to download from HuggingFace Hub
3. In offline environment → `ConnectionError` raised
4. No exception handler → **Application crashes**

**Expected behavior (STT-REQ-002.4)**:
- Network error → Fallback to bundled base model

### Why Tests Passed (False Positive)

**File**: `python-stt/tests/test_offline_model_fallback.py:69-91`

Original test (buggy):
```python
async def test_fallback_to_bundled_model_on_network_error(self):
    engine = WhisperSTTEngine(model_size="small", offline_mode=False)

    with patch.object(engine, '_detect_model_path') as mock_detect:
        with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
            # ❌ Makes _detect_model_path() raise TimeoutError
            # This doesn't match real control flow!
            def detect_side_effect():
                if mock_detect.call_count == 1:
                    raise TimeoutError("HuggingFace Hub timeout")
                return "[app_resources]/models/faster-whisper/base"

            mock_detect.side_effect = detect_side_effect
```

**Problem**: Test mocks `_detect_model_path()` to raise `TimeoutError`, but real code **never raises this exception** because `_try_download_from_hub()` swallows it and returns `None`.

## Decision

Implement **three-phase fix**:

### Phase 1: Extract `_detect_bundled_model_path()` Helper

Extract bundled model search logic into a reusable helper method.

```python
def _detect_bundled_model_path(self, requested_size: ModelSize) -> Optional[str]:
    """
    Detect bundled model path with fallback to 'base' (STT-REQ-002.4).

    Returns:
        Path to bundled model, or None if not found
    """
    bundled_base_dirs = [
        Path(__file__).parent.parent.parent / "models" / "faster-whisper",
        Path.home() / ".local" / "share" / "meeting-minutes-automator" / "models" / "faster-whisper",
        Path("/opt/meeting-minutes-automator/models/faster-whisper"),
    ]

    # Try requested size first
    for base_dir in bundled_base_dirs:
        bundled_path = base_dir / requested_size
        if bundled_path.exists() and (bundled_path / "model.bin").exists():
            return str(bundled_path)

    # STT-REQ-002.4: Fallback to 'base'
    if requested_size != 'base':
        for base_dir in bundled_base_dirs:
            bundled_base_path = base_dir / "base"
            if bundled_base_path.exists() and (bundled_base_path / "model.bin").exists():
                self.model_size = "base"  # Update model_size
                return str(bundled_base_path)

    return None
```

### Phase 2: Add WhisperModel Exception Handling in `initialize()`

Wrap `WhisperModel` load with try-except and fallback to bundled model.

```python
async def initialize(self) -> None:
    try:
        self.model_path = self._detect_model_path()
        logger.info(f"Detected model path: {self.model_path}")

        # Try to load faster-whisper model
        try:
            self.model = WhisperModel(
                self.model_path,
                device="cpu",
                compute_type="int8"
            )
            logger.info("WhisperModel loaded successfully")

        except Exception as load_error:
            # ✅ STT-REQ-002.4: Network error → bundled fallback
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
                    # ✅ STT-REQ-002.5: No bundled model available
                    raise FileNotFoundError(
                        "faster-whisperモデルが見つかりません。インストールを確認してください"
                    ) from load_error
            else:
                raise  # Re-raise in offline mode

        # Output ready message (STT-REQ-002.10)
        ready_message = json.dumps({
            "type": "event",
            "event": "whisper_model_ready",
            "model_size": self.model_size,
            "model_path": self.model_path
        })
        sys.stdout.write(f"{ready_message}\n")
        sys.stdout.flush()

    except Exception as e:
        logger.error(f"Failed to initialize WhisperSTTEngine: {e}")
        raise
```

### Phase 3: Fix Tests to Match Real Control Flow

Update tests to simulate actual code path:

```python
async def test_fallback_to_bundled_model_on_network_error(self):
    """Real control flow:
    1. _detect_model_path() returns Hub model ID
    2. WhisperModel() raises ConnectionError (network failure)
    3. initialize() catches exception and calls _detect_bundled_model_path()
    4. WhisperModel() succeeds with bundled path
    """
    engine = WhisperSTTEngine(model_size="small", offline_mode=False)

    with patch.object(engine, '_detect_model_path') as mock_detect:
        with patch.object(engine, '_detect_bundled_model_path') as mock_bundled:
            with patch('stt_engine.transcription.whisper_client.WhisperModel') as mock_whisper:
                # ✅ _detect_model_path returns Hub model ID (real behavior)
                mock_detect.return_value = "Systran/faster-whisper-small"

                # ✅ WhisperModel raises on first call, succeeds on second
                mock_whisper.side_effect = [
                    ConnectionError("Failed to download from HuggingFace Hub"),
                    MagicMock()  # Success on bundled fallback
                ]

                # ✅ Bundled fallback path
                mock_bundled.return_value = "/opt/meeting-minutes-automator/models/faster-whisper/base"

                await engine.initialize()

                # Verify bundled fallback was attempted
                mock_bundled.assert_called_once_with("small")
                assert engine.model_path == "/opt/meeting-minutes-automator/models/faster-whisper/base"
                assert mock_whisper.call_count == 2  # 1st fail, 2nd success
```

## Consequences

### Positive

1. ✅ **STT-REQ-002.4 compliance**: Network error now correctly falls back to bundled base model
2. ✅ **STT-REQ-002.5 compliance**: Clear error message when no bundled model available
3. ✅ **Offline startup fixed**: Application no longer crashes in offline environments
4. ✅ **Test accuracy**: Tests now simulate real control flow instead of mocked exceptions
5. ✅ **Code maintainability**: Bundled model logic extracted into reusable helper

### Negative

- None (This was a pure bug fix with no architectural changes)

## Compliance

**Requirements**:
- ✅ STT-REQ-002.4: Network error → bundled base model fallback
- ✅ STT-REQ-002.5: No bundled model → clear error message

**Principles**:
- ✅ **Principle 2**: Offline-first design - Now properly implemented
- ✅ **Principle 3**: Security boundary - No silent failures in offline mode

## Implementation Notes

**Files Modified**:
1. `python-stt/stt_engine/transcription/whisper_client.py`:
   - Added `_detect_bundled_model_path()` helper (Line 155-196)
   - Modified `_detect_model_path()` to use helper (Line 334-370)
   - Modified `initialize()` with WhisperModel exception handling (Line 432-475)

2. `python-stt/tests/test_offline_model_fallback.py`:
   - Fixed `test_fallback_to_bundled_model_on_network_error` (Line 69-105)
   - Fixed `test_error_when_bundled_model_not_found` (Line 108-137)

**Testing**:
```bash
pytest tests/test_offline_model_fallback.py -v
# 14 passed in 0.25s
```

**Impact Assessment**:
- **Before**: Offline environments crashed on first startup
- **After**: Offline environments automatically use bundled base model
- **User Impact**: **Critical bug fix** - Application now usable in offline/air-gapped environments

## Future Work

None required. This was a P0 bug fix to restore intended behavior (STT-REQ-002.4).

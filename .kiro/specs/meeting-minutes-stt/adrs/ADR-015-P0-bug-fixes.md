# ADR-015 P0 Bug Fixes (Round 2)

**Date**: 2025-10-14
**Status**: Completed
**Related**: ADR-013 Sidecar Full-Duplex IPC Final Design

## Context

After initial ADR-013 implementation (Phase 1-4), external code review identified two critical P0 bugs that violated core design contracts:

1. **P0-2**: Ring buffer partial write corruption (0% frame loss contract violation)
2. **P0-1**: VAD state check AttributeError (potential crash)

These bugs were discovered through rigorous code review before production deployment.

## P0-2: Ring Buffer Partial Write Prevention

### Problem

**File**: `src-tauri/src/ring_buffer.rs:104-122`

Original implementation allowed partial writes when buffer was nearly full:

```rust
pub fn push_from_callback(
    producer: &mut ringbuf::HeapProd<u8>,
    data: &[u8],
) -> (usize, BufferLevel) {
    // Try to push all data
    let pushed = producer.push_slice(data);  // ← Allows partial writes

    // Check AFTER write (too late!)
    if pushed < data.len() {
        return (pushed, BufferLevel::Overflow);  // ← Frame already corrupted
    }
    // ...
}
```

**Impact**:
- When buffer is 95% full, 320-byte frame might only write 100 bytes
- Remaining 220 bytes are **silently discarded**
- This violates ADR-013's "0% frame loss" contract
- Corrupted frames cause audio artifacts and transcription errors

### Solution

Check free space **before** writing to prevent partial writes:

```rust
pub fn push_from_callback(
    producer: &mut ringbuf::HeapProd<u8>,
    data: &[u8],
) -> (usize, BufferLevel) {
    // CRITICAL: Check free space BEFORE writing
    if producer.vacant_len() < data.len() {
        // Reject entire frame (no partial write)
        return (0, BufferLevel::Overflow);
    }

    // Now we know entire frame fits
    let pushed = producer.push_slice(data);
    debug_assert_eq!(pushed, data.len(), "Partial write should never occur");
    // ...
}
```

**Verification**:
- All 11 ring buffer unit tests pass
- 4/4 E2E tests pass (including `test_60s_continuous_speech_zero_frame_loss`)
- `debug_assert_eq!` ensures partial writes never occur in debug builds

## P0-1: VAD State Check AttributeError

### Problem

**File**: `python-stt/stt_engine/audio_pipeline.py:424-425`

Original implementation referenced non-existent fields:

```python
def is_in_speech(self) -> bool:
    # Check if VAD is in speech state
    return self.vad.silence_duration == 0 and self.vad.speech_active
    #          ^^^ AttributeError         ^^^ AttributeError
```

**Root Cause**:
- `VoiceActivityDetector` class (L53-56) exposes: `is_in_speech`, `speech_frames`, `silence_frames`
- Referenced fields `silence_duration` and `speech_active` do not exist
- Would cause `AttributeError` crash when `pipeline.is_in_speech()` is called

**Impact**:
- Potential Python sidecar crash during silence detection
- No graceful degradation (immediate crash)
- Not caught by E2E tests (code path not exercised)

### Solution

Use actual `VoiceActivityDetector` field:

```python
def is_in_speech(self) -> bool:
    # Check if VAD is in speech state using actual field
    return self.vad.is_in_speech
```

**Verification**:
- Static analysis confirms `VoiceActivityDetector.is_in_speech` exists (L53)
- Field is updated by `process_frame()` method (L152, L177)
- Semantically equivalent to original intent (currently detecting speech)

## Test Results

### Before Fix
- **Risk**: Partial writes allowed, AttributeError on silence detection
- **Frame Loss**: Potential corruption on buffer near-full conditions

### After Fix
- **Ring Buffer Tests**: 11/11 passed ✅
- **E2E Tests**: 4/4 passed (72s total) ✅
  - Test 1: 5s Python hang → buffer overflow (6.0s)
  - Test 2: 60s continuous → 0% frame loss (6000 frames)
  - Test 3: VAD active → 0 false no_speech (1000 frames)
  - Test 4: Parallel execution → no deadlock (500 frames)

## Decision

✅ **Accept both fixes** — Critical bugs that violate ADR-013 contracts.

### Rationale

1. **P0-2** (Partial write):
   - Directly violates "0% frame loss" Success Criteria
   - Simple fix with clear semantics (check before write)
   - No performance impact (single branch check)

2. **P0-1** (AttributeError):
   - Latent crash bug (not yet triggered in tests)
   - One-line fix using correct field
   - No semantic change (same intent)

## Consequences

### Positive
- ✅ 0% frame loss guarantee maintained
- ✅ Crash risk eliminated
- ✅ All Success Criteria met
- ✅ No performance regression

### Negative
- ⚠️ E2E tests did not catch P0-1 (silence detection path not covered)
- ⚠️ Code review was essential for finding these bugs

## Action Items

1. ✅ Apply P0-2 fix (ring buffer)
2. ✅ Apply P0-1 fix (VAD state)
3. ✅ Verify all tests pass
4. ⏳ Consider adding E2E test for silence detection path (future work)
5. ⏳ Update Requirement Traceability Matrix

---

# P0 Bug Fixes - Round 2 (Offline Mode & Resource Monitoring)

**Date**: 2025-10-18
**Status**: Completed
**Related**: STT-REQ-002.4/002.6, STT-REQ-006.9/006.12, ADR-002

## Context

After completing MVP1 Task 10 (Resource Monitoring), external code review identified 5 critical P0 bugs violating offline-first and resource-based model selection contracts:

1. **Bug 1**: Offline mode bundled model fallback not implemented
2. **Bug 2**: System-wide memory usage causing false downgrades
3. **Bug 3**: Initial model upgrade ceiling bug after bundled fallback
4. **Bug 4**: NameError in offline fallback error message
5. **Bug 5**: Silent fallback in `load_model()` causing system/UI state mismatch

## Bug 1: Offline Mode Bundled Model Fallback

### Problem

**File**: `python-stt/stt_engine/transcription/whisper_client.py:341-365`

Original implementation only searched for requested model size, causing startup failure when model unavailable in offline mode:

```python
# Priority 3: Bundled model
for base_dir in bundled_base_dirs:
    bundled_path = base_dir / self.model_size
    if bundled_path.exists():
        return str(bundled_path)

# No fallback → sys.exit(1) if not found
```

**Impact**:
- Violates STT-REQ-002.4/002.6 (offline-first principle)
- User requests 'small' model but only 'base' is bundled → startup failure
- No graceful degradation in offline environments

### Solution

Implement 2-stage fallback: requested size → bundled 'base' → error

```python
# First, try requested model size
for base_dir in bundled_base_dirs:
    bundled_path = base_dir / self.model_size
    if bundled_path.exists() and (bundled_path / "model.bin").exists():
        return str(bundled_path)

# STT-REQ-002.4/002.6: Fallback to bundled 'base'
if self.model_size != 'base':
    logger.warning(f"Requested model '{self.model_size}' not found, falling back to 'base'")
    for base_dir in bundled_base_dirs:
        bundled_base_path = base_dir / "base"
        if bundled_base_path.exists():
            self.model_size = "base"  # Update to reflect actual model
            return str(bundled_base_path)
```

## Bug 2: System-Wide Memory Causing False Downgrades

### Problem

**File**: `python-stt/stt_engine/resource_monitor.py:150-159`

Original implementation used system-wide memory (typically >3GB), not app-specific:

```python
def get_current_memory_usage(self) -> float:
    memory = psutil.virtual_memory()
    return memory.used / (1024 ** 3)  # System-wide (>3GB typical)
```

**Impact**:
- With 4GB/3GB thresholds, always triggered unnecessary downgrades
- Whisper 'small' model uses ~500MB, but system shows 3.5GB → false downgrade to 'base'
- Violates STT-REQ-006.6 (resource-based model selection accuracy)

### Solution

Use application-specific RSS (Resident Set Size):

```python
def __init__(self):
    self.process = psutil.Process(os.getpid())  # App process handle

def get_current_memory_usage(self) -> float:
    memory_info = self.process.memory_info()
    return memory_info.rss / (1024 ** 3)  # App memory only
```

Adjusted thresholds for app memory: 2.0GB/1.5GB (from 4GB/3GB).

## Bug 3: Initial Model Upgrade Ceiling Bug

### Problem

**File**: `python-stt/main.py:657-663`

Original implementation updated both `current_model` and `initial_model` after bundled fallback:

```python
await processor.stt_engine.initialize()
# Bundled fallback: small → base
processor.resource_monitor.initial_model = processor.stt_engine.model_size  # 'base'
processor.resource_monitor.current_model = processor.stt_engine.model_size  # 'base'
```

**Impact**:
- `initial_model` represents resource-based recommendation (upgrade ceiling)
- Setting it to 'base' after fallback prevents future upgrades to 'small' when network recovers
- Violates STT-REQ-006.10/006.12 (dynamic model upgrade capability)

### Solution

Only update `current_model`, preserve `initial_model` as upgrade ceiling:

```python
await processor.stt_engine.initialize()
# IMPORTANT: Keep initial_model unchanged (upgrade ceiling)
processor.resource_monitor.current_model = processor.stt_engine.model_size  # 'base'
# initial_model remains 'small' (resource-based recommendation)
```

## Bug 4: NameError in Offline Fallback Error Message

### Problem

**File**: `python-stt/stt_engine/transcription/whisper_client.py:388`

Referenced undefined variable `bundled_model_paths` (should be `bundled_base_dirs`):

```python
raise FileNotFoundError(
    f"  - Bundled base dirs: {', '.join(str(d) for d in bundled_model_paths)}\n"
    #                                                      ^^^ NameError
)
```

**Impact**:
- Would cause crash before displaying helpful error message
- Intended graceful `FileNotFoundError` becomes unexpected `NameError`

### Solution

Use correct variable name:

```python
f"  - Bundled base dirs: {', '.join(str(d) for d in bundled_base_dirs)}\n"
```

## Bug 5: Silent Fallback in load_model()

### Problem

**File**: `python-stt/stt_engine/transcription/whisper_client.py:453-472`, `main.py:585-629`

Original `load_model()` returned `void`, causing system/UI state mismatch:

```python
async def load_model(self, new_model_size: ModelSize) -> None:  # Returns void
    # ...
    # Bundled fallback may change self.model_size
```

Caller assumed success:

```python
await self.stt_engine.load_model('small')  # Returns nothing
# Assume 'small' was loaded, but actually 'base' due to fallback
self.resource_monitor.current_model = 'small'  # WRONG!

await self.ipc.send_message({
    'result': {'new_model': 'small'}  # UI shows 'small', but system uses 'base'
})
```

**Impact**:
- UI shows "Upgraded to small" but system uses 'base'
- ResourceMonitor and actual model out of sync
- User cannot trust upgrade notifications

### Solution

Return actual loaded model size and add `upgrade_fallback` event:

```python
async def load_model(self, new_model_size: ModelSize) -> str:  # Return actual model
    # ...
    return self.model_size  # May differ from new_model_size due to fallback
```

Caller uses returned value:

```python
actual_model = await self.stt_engine.load_model('small')
self.resource_monitor.current_model = actual_model  # Correct state

upgrade_succeeded = (actual_model == target_model)

if upgrade_succeeded:
    await self.ipc.send_message({
        'type': 'event',
        'eventType': 'upgrade_success',
        'data': {'old_model': 'base', 'new_model': 'small'}
    })
else:
    await self.ipc.send_message({
        'type': 'event',
        'eventType': 'upgrade_fallback',  # NEW EVENT
        'data': {
            'old_model': 'base',
            'new_model': 'base',
            'requested_model': 'small',
            'message': 'Requested small not available, using base instead'
        }
    })
```

### IPC Protocol Extension

**New Event**: `upgrade_fallback`

```typescript
interface UpgradeFallbackEvent {
  type: 'event';
  version: '1.0';
  eventType: 'upgrade_fallback';
  data: {
    old_model: ModelSize;
    new_model: ModelSize;        // Actual loaded model (fallback)
    requested_model: ModelSize;  // What user requested
    message: string;             // Human-readable explanation
  };
}
```

**Response Format Update**:

```typescript
interface ApproveUpgradeResponse {
  id: string;
  type: 'response';
  version: '1.0';
  result: {
    success: boolean;
    old_model: ModelSize;
    new_model: ModelSize;        // Actual loaded model
    requested_model: ModelSize;  // What user requested
    fallback_occurred: boolean;  // NEW FIELD
  };
}
```

## Test Results

### Unit Tests
- `test_resource_monitor.py::TestAppMemoryMonitoring`: 3/3 passed ✅
  - App memory < system memory verification
  - 2.0GB/1.5GB threshold tests
  - No false positive downgrades
- `test_whisper_client.py::TestBundledModelFallback`: 3/3 passed ✅
  - small → base fallback
  - No fallback when model available
  - `load_model()` return value correctness

### E2E Tests
- `test_upgrade_fallback.py`: 2/2 passed ✅
  - Offline bundled base only: fallback + IPC `upgrade_fallback` event
  - Model available: successful upgrade + IPC `upgrade_success` event

## Decision

✅ **Accept all 5 fixes** — Critical bugs violating offline-first and resource monitoring contracts.

### Rationale

1. **Bug 1** (Bundled fallback): Enables offline operation (STT-REQ-002.4/002.6)
2. **Bug 2** (App memory): Prevents false downgrades, improves accuracy (STT-REQ-006.6)
3. **Bug 3** (Upgrade ceiling): Preserves upgrade path after online recovery (STT-REQ-006.10)
4. **Bug 4** (NameError): Improves error message reliability
5. **Bug 5** (Silent fallback): Ensures system/UI state consistency (STT-REQ-006.12)

## Consequences

### Positive
- ✅ Offline-first principle maintained (ADR-002)
- ✅ Accurate resource-based model selection
- ✅ Transparent upgrade failure reporting to UI
- ✅ System state always matches UI display
- ✅ Graceful degradation in resource-constrained environments

### Negative
- ⚠️ `load_model()` API change (void → str) - internal only, no breaking changes
- ⚠️ New IPC event type requires UI handling (future work)

## Action Items

1. ✅ Apply all 5 bug fixes
2. ✅ Add unit tests (6 tests total)
3. ✅ Add E2E tests (2 tests total)
4. ✅ Update ADR-002 (bundled model distribution strategy)
5. ✅ Document IPC `upgrade_fallback` event
6. ⏳ Implement UI handler for `upgrade_fallback` event (Phase 3.5)

## References

- ADR-013: Sidecar Full-Duplex IPC Final Design
- BLOCK-004: ADR-013 Implementation tracking (spec.json)
- Code Review: External analysis (2025-10-14)
- ADR-002: Model Distribution Strategy (updated 2025-10-18)
- STT-REQ-002.4/002.6: Offline bundled model fallback
- STT-REQ-006.6/006.9/006.10/006.12: Resource-based model selection
- Code Review Round 2: External analysis (2025-10-18)

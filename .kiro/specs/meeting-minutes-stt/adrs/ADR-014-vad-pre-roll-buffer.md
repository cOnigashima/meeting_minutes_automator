# ADR-014 VAD Pre-roll Buffer for Leading Frame Preservation

**Date**: 2025-10-19
**Status**: Accepted
**Related**: STT-REQ-003.4 (Speech onset detection), NFR-001.1 (Partial text response time < 0.5s)

## Context

After MVP1 Task 10.1 E2E test validation, critical analysis identified a P0 bug in `VoiceActivityDetector.process_frame()` where **the first 300ms (30 frames) of speech were systematically discarded**, causing:

1. **User-facing transcription errors**: "こんにちは" → "んにちは" (missing "こ")
2. **Whisper degraded accuracy**: Leading phonemes missing from input audio
3. **STT-REQ-003.4 contract violation**: Speech onset threshold (0.3s) became audio loss period instead of detection threshold

### Root Cause Analysis

**File**: `python-stt/stt_engine/transcription/voice_activity_detector.py:139-188`

Original implementation (buggy):

```python
def process_frame(self, frame: bytes) -> dict:
    if not self.is_in_speech:
        if is_speech_frame:
            self.speech_frames += 1
            if self.speech_frames >= self.speech_onset_threshold:  # 30 frames
                self.is_in_speech = True
                self.current_segment = []  # ❌ BUG: Clear buffer
                self.current_segment.append(frame)  # ❌ Only frame 30 added
                return {'event': 'speech_start'}
```

**Why E2E tests passed initially**:
- Test fixtures (`test_audio_short.wav`) contained ~500ms leading silence padding
- Real-world users start speaking immediately without padding (0ms latency)
- Bug was masked by test data characteristics, not actual user behavior

### Impact Assessment

| Scenario | Leading Silence | Audio Loss | Result |
|----------|----------------|------------|--------|
| **Test fixture** | 500ms padding | 300ms from padding | ✅ Passed (false negative) |
| **Real user** | 0ms (immediate speech) | 300ms from actual speech | ❌ First syllable missing |

**Reproduction**:
- User says "会議を始めます" starting at t=0
- Frames 0-29 (0-290ms) detected as speech but **not accumulated**
- Frame 30 triggers `speech_start` but buffer is cleared
- Whisper receives frames 30+ only
- Transcription result: "議を始めます" (missing "会")

## P0.5 Bug Discovery (Post-P0 Fix)

After P0 fix implementation, external code review identified a **secondary regression** where `speech_end` (final_text) correctly preserved all 30 leading frames, but `partial_text` still lost them.

**Root Cause**: `AudioPipeline._current_speech_buffer` was reset to empty on `speech_start` (audio_pipeline.py:249), and only accumulated frames **after** `is_in_speech=True` (audio_pipeline.py:189). The VAD's 30-frame pre-roll never reached the partial transcription buffer.

**Impact**:
- ✅ **speech_end** (final_text): Complete audio preserved (P0 fix working)
- ❌ **partial_text**: Missing 30 leading frames (real-time UX degraded)

| Event Type | Buffer Source | Leading 300ms | Status |
|------------|--------------|---------------|--------|
| **speech_end** (final) | `vad_result['segment']['audio_data']` | ✅ Included | P0 Fixed |
| **partial_text** | `self._current_speech_buffer` | ❌ Missing | **P0.5 Bug** |

**User Impact**: Users saw leading syllables disappear in real-time transcription ("こんにちは" → "んにちは") despite final transcript being correct.

## Decision

Implement **two-phase fix**:

### Phase 1 (P0): VAD Ring Buffer Pre-roll
Implement ring buffer pre-roll pattern from webrtcvad official examples (`vad_collector` function).

### Architecture

```python
from collections import deque

class VoiceActivityDetector:
    def __init__(self):
        # Ring buffer with maxlen=30 (auto-discards oldest frame)
        self.pre_roll_buffer = deque(maxlen=self.speech_onset_threshold)

    def process_frame(self, frame: bytes) -> dict:
        if not self.is_in_speech:
            # Always push to pre-roll ring buffer
            self.pre_roll_buffer.append(frame)

            if is_speech_frame:
                self.speech_frames += 1
                if self.speech_frames >= self.speech_onset_threshold:
                    self.is_in_speech = True
                    # ✅ Transfer pre-roll buffer to current_segment
                    self.current_segment = list(self.pre_roll_buffer)
                    self.pre_roll_buffer.clear()
                    return {'event': 'speech_start'}
```

### Phase 2 (P0.5): AudioPipeline Pre-roll Seeding

Extend `speech_start` event to include `pre_roll` field, allowing AudioPipeline to seed `_current_speech_buffer` for partial transcriptions.

**VAD Side** (voice_activity_detector.py:168-179):
```python
# P0.5 FIX: Provide pre-roll audio to AudioPipeline
pre_roll_audio = b''.join(self.current_segment)
return {
    'event': 'speech_start',
    'pre_roll': pre_roll_audio  # ✅ New field
}
```

**AudioPipeline Side** (audio_pipeline.py:237-257):
```python
async def _handle_speech_start(
    self,
    pre_roll: Optional[bytes] = None  # ✅ New parameter
) -> Dict[str, Any]:
    self._current_speech_buffer = bytearray()

    # ✅ P0.5 FIX: Seed with pre-roll if available
    if pre_roll:
        self._current_speech_buffer.extend(pre_roll)
```

### Alternatives Considered (P0.5 Phase)

| Alternative | Pros | Cons | Decision |
|-------------|------|------|----------|
| **1. speech_start event field (chosen)** | ✅ Minimal change<br>✅ Clear responsibility separation<br>✅ Backward compatible | None | **Accepted** |
| **2. Expose current_segment property** | ✅ No event structure change | ❌ Encapsulation violation<br>❌ Timing issues | Rejected |
| **3. Pipeline reads VAD buffer directly** | ✅ No data duplication | ❌ Large refactoring<br>❌ Out of MVP1 scope | Deferred to MVP2 |

## Consequences

### Positive

1. **✅ Complete audio preservation**: All 30 leading frames transferred to Whisper
2. **✅ Transcription accuracy improvement**: Leading phonemes no longer lost
3. **✅ Minimal memory overhead**: 9.6KB per session (30 frames × 320 bytes)
4. **✅ Follows industry pattern**: Matches webrtcvad official `vad_collector` implementation
5. **✅ Backward compatible**: No API changes, existing tests updated with correct expectations

### Implementation Validation

**TDD Cycle Phase 1 (P0 Fix: VAD Pre-roll)**:

1. **RED**: Created `TestPreRollBufferIntegrity` with 2 failing tests
   - `test_speech_onset_preserves_leading_frames`: Expected 30, got 1 frame
   - `test_pre_roll_buffer_content_integrity`: Expected 30 unique frames, got 2

2. **GREEN**: Implemented deque pre-roll buffer
   - Both new tests passed
   - `test_segment_contains_duration`: Updated expected value 710ms → 1000ms

3. **REFACTOR**: All 19 VAD unit tests passed

**TDD Cycle Phase 2 (P0.5 Fix: AudioPipeline Seeding)**:

1. **RED**: Created `TestPartialTextPreRollIntegrity` with 1 failing test
   - `test_partial_text_includes_pre_roll_frames`: Expected 16000 bytes (50 frames), got 6400 bytes (20 frames)
   - **30 frames missing** from partial transcription buffer

2. **GREEN**: Added `pre_roll` field to speech_start event
   - AudioPipeline seeds `_current_speech_buffer` with pre-roll
   - Test passed: 16000 bytes received by STT engine

3. **REFACTOR**: All tests passed
   - VAD: 19/19 tests ✅
   - AudioPipeline: 11/11 tests ✅
   - E2E: 1/1 test ✅ (28.07s, 17 events received)

### Metrics

| Metric | P0 Before | P0 After | P0.5 After |
|--------|-----------|----------|------------|
| **speech_end frame preservation** | 3% (1/30) | 100% (30/30) ✅ | 100% (30/30) ✅ |
| **partial_text frame preservation** | 0% (0/30) | 0% (0/30) ❌ | 100% (30/30) ✅ |
| **Audio loss (final)** | 290ms | 0ms ✅ | 0ms ✅ |
| **Audio loss (partial)** | 290ms | 290ms ❌ | 0ms ✅ |
| **Memory overhead** | 0 | 9.6KB | 9.6KB |
| **Unit tests passing** | 89% (17/19) | 100% (19/19) | 100% (30/30) |

### Negative

- **Minimal memory cost**: 9.6KB per active speech detection session
  - Total sessions expected: 1-2 concurrent
  - Total overhead: ~20KB (acceptable for audio processing)

## Compliance

**Requirements**:
- ✅ STT-REQ-003.4: Speech onset detection (0.3s continuous speech) - now preserves all 30 frames
- ✅ NFR-001.1: Partial text response time < 0.5s - requires full audio without loss

**Principles**:
- ✅ **Principle 1**: Process boundary isolation - VAD internal implementation, no external API impact
- ✅ **Principle 3**: Security boundary - Ring buffer is bounded (maxlen=30), no unbounded growth

## Implementation Notes

**Files Modified (P0 Phase)**:
1. `python-stt/stt_engine/transcription/voice_activity_detector.py:27-71` - Added `self.pre_roll_buffer = deque(maxlen=30)`
2. `python-stt/stt_engine/transcription/voice_activity_detector.py:161-179` - Transferred pre-roll to `current_segment` on onset
3. `python-stt/tests/test_voice_activity_detector.py:449-530` - Added 2 new tests, updated 1 expectation

**Files Modified (P0.5 Phase)**:
1. `python-stt/stt_engine/transcription/voice_activity_detector.py:168-179` - Added `pre_roll` field to speech_start event
2. `python-stt/stt_engine/audio_pipeline.py:157-159, 205-207` - Extract `pre_roll` from event and pass to handler
3. `python-stt/stt_engine/audio_pipeline.py:237-257` - Added `pre_roll` parameter and seeding logic
4. `python-stt/tests/test_audio_pipeline.py:253-352` - Added 1 regression test
5. `python-stt/tests/test_voice_activity_detector.py:494-495` - Updated expectation for `pre_roll` field

**Testing**:
- `pytest tests/test_voice_activity_detector.py`: 19/19 passed ✅
- `pytest tests/test_audio_pipeline.py`: 11/11 passed ✅
- `cargo test --test stt_e2e_test`: 1/1 passed (28.07s) ✅

**References**:
- webrtcvad official example: `vad_collector` function using `collections.deque(maxlen=num_padding_frames)`
- GitHub: https://github.com/wiseman/py-webrtcvad/blob/master/example.py

## Future Work

None required for MVP1. Deferred enhancements:

1. **Configurable pre-roll duration**: Allow tuning 0.3s threshold per use case
2. **AudioPipeline-level buffering**: Separate VAD event detection from audio buffering (MVP2)
3. **Performance profiling**: Verify 9.6KB overhead acceptable on embedded systems (if targeting Raspberry Pi)

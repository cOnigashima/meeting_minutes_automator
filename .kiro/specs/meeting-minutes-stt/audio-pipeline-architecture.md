# Audio Pipeline Architecture

> **Status**: Living Document
> **Last Updated**: 2026-01-09
> **Related**: ADR-013 (Full-Duplex IPC), STT-REQ-007 (IPC Protocol), meeting-minutes-stt-multi-input (Input Mixer)

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AUDIO PIPELINE                                  │
└─────────────────────────────────────────────────────────────────────────────┘

┌────────────┐    ┌──────────────┐    ┌──────────────┐    ┌─────────────┐    ┌─────────────────┐
│  OS Audio  │───▶│  cpal Stream │───▶│ Input Mixer  │───▶│ Ring Buffer │───▶│ Batch Sender   │
│ (N devs)   │    │  Thread(s)   │    │ (16kHz mono) │    │ (160KB/5s)  │    │ Task (tokio)   │
└────────────┘    └──────────────┘    └──────────────┘    └─────────────┘    └────────┬────────┘
                        │                      │                                    │
                        │ f32→i16変換           │ 10ms frame align / gain / mix     │ JSON/stdin
                        │ ch downmix + resample │                                    ▼
                        │ リサンプリング         ┌─────────────────────┐
                        ▼                      │   Python Sidecar    │
                  ┌───────────┐                │   ┌─────────────┐   │
                  │ Watchdog  │                │   │ VAD (webrtc)│   │
                  │ (1200ms)  │                │   └──────┬──────┘   │
                  └───────────┘                │          ▼          │
                                               │   ┌─────────────┐   │
                                               │   │ Whisper STT │   │
                                               │   └──────┬──────┘   │
                                               │          │          │
                                               │   ┌──────▼──────┐   │
                                               │   │ Resource    │   │
                                               │   │ Monitor     │   │
                                               │   └─────────────┘   │
                                               └──────────┬──────────┘
                                                          │ JSON/stdout
                                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           IPC Reader Task (tokio)                            │
│  ┌───────────────┐    ┌──────────────────┐    ┌────────────────────────┐   │
│  │ JSON Parse    │───▶│ Confidence Filter│───▶│ Tauri emit + WebSocket │   │
│  │               │    │ (≥50%)           │    │                        │   │
│  └───────────────┘    └──────────────────┘    └────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                                                │
                                                                ▼
                                                     ┌─────────────────┐
                                                     │  React UI       │
                                                     │  (App.tsx)      │
                                                     └─────────────────┘
```

---

## 2. Layer Details

### 2.1 Audio Capture Layer (Rust/cpal)

| Component | File | Responsibility |
|-----------|------|----------------|
| AudioDeviceAdapter | `audio_device_adapter.rs` | OS-specific audio abstraction |
| Stream Thread | cpal | Real-time callback (300-500Hz) |
| Watchdog Thread | `audio_device_adapter.rs` | Stall detection (1200ms) |
| Device Polling | `audio_device_adapter.rs` | Device connection monitoring (3s) |

**Resampling Process** (L164-181):
- Input: 48kHz f32 mono
- Output: 16kHz i16 mono (for Whisper)
- Method: Averaging downsampling (3 samples → 1 sample)

**Data Rate**:
```
OS Audio:     48kHz × 4 bytes = 192 KB/s (f32 mono)
After Resamp: 16kHz × 2 bytes = 32 KB/s (i16 mono)
```

### 2.2 Input Mixer Layer (Multi-Input Extension)

| Component | File | Responsibility |
|-----------|------|----------------|
| Input Mixer | (planned) | 複数入力の時間整列・ゲイン調整・ミックス |
| Per-Input Buffer | (planned) | 入力ごとのフレームバッファ |

**Notes**:
- 単一入力の場合は実質パススルー（互換維持）
- 詳細は `meeting-minutes-stt-multi-input` spec を参照

### 2.3 Ring Buffer Layer

| Property | Value |
|----------|-------|
| Capacity | 160KB (5 seconds) |
| Design | SPSC (Single Producer Single Consumer) |
| Lock | Lock-free (ringbuf crate) |
| Level Monitoring | Normal (0-50%) / Warn (50-70%) / Critical (70-100%) / Overflow |

**File**: `ring_buffer.rs`

**API**:
```rust
// Create shared ring buffer
let ring_buffer = new_shared_ring_buffer(); // Arc<Mutex<HeapRb<u8>>>

// Producer (callback side) - use try_lock() for non-blocking
push_audio_drop_oldest(&mut rb, &data) -> (pushed, dropped, BufferLevel)

// Consumer (sender task side)
pop_audio(&mut rb, &mut buf) -> usize
```

**Overflow Behavior** (Drop-Oldest Strategy):
- When buffer full: **Drop oldest data** to make room for new
- New data is always accepted (real-time priority)
- Returns: (bytes_pushed, bytes_dropped, BufferLevel)
- **Benefit**: Latest audio is always preserved for live transcription

### 2.4 IPC Communication Layer (stdin/stdout)

| Design | Details |
|--------|---------|
| Protocol | JSON line-delimited (ADR-013) |
| Separation | `take_stdin()` / `take_stdout()` for full isolation |
| Mutex | Separate for stdin and stdout |
| Timeout | stdin write: 10 seconds |
| Cancellation | CancellationToken (tokio_util) |

**Message Format** (STT-REQ-007):
```json
// Request (Rust → Python)
{
  "id": "audio-1704067200000",
  "version": "1.0",
  "method": "process_audio_stream",
  "params": { "audio_data": [...] }
}

// Event (Python → Rust)
{
  "type": "event",
  "version": "1.0",
  "eventType": "partial_text",
  "data": {
    "text": "こんにちは",
    "confidence": 0.85,
    "language": "ja"
  }
}
```

### 2.5 Python STT Layer

| Component | File | Responsibility |
|-----------|------|----------------|
| IpcHandler | `ipc_handler.py` | stdin read loop |
| AudioPipeline | `audio_pipeline.py` | Frame buffering |
| VoiceActivityDetector | `voice_activity_detector.py` | VAD (webrtcvad) |
| WhisperClient | `whisper_client.py` | Whisper inference |
| ResourceMonitor | `resource_monitor.py` | Dynamic model switching |

**VAD Configuration**:
- Frame size: 10ms (320 bytes)
- Speech start: 30 consecutive frames (0.3s) with voice
- Speech end: 50 consecutive frames (0.5s) silence
- Pre-roll: 30 frames (0.3s) buffer

**Whisper Inference Timing**:
- First partial: After 10 frames (100ms)
- Subsequent partials: Every 100 frames (1 second)
- Final text: On VAD speech_end

**Resource Monitoring Thresholds**:
| Condition | Action |
|-----------|--------|
| CPU ≥85% (60s sustained) | 1-step downgrade |
| Memory ≥1.5GB | 1-step downgrade |
| Memory ≥2.0GB | Immediate downgrade to base |

**Model Sequence**: `large-v3 → medium → small → base → tiny`

### 2.6 UI Update Flow

| Processing | Details |
|------------|---------|
| Confidence Filter | Skip if <50% (Whisper hallucination prevention) |
| Tauri emit | `"transcription"` event |
| WebSocket | Chrome extension broadcast |
| React state | `transcriptions[]` (max 50 entries) |

---

## 3. Data Flow Rates

```
┌─────────────────────────────────────────────────────────────────┐
│                    RATE ANALYSIS                                 │
└─────────────────────────────────────────────────────────────────┘

Stage 1: OS → cpal
  Rate: 48 kHz × 4 bytes = 192 KB/s

Stage 2: cpal → Input Mixer (after per-input resampling/downmix)
  Rate: 16 kHz × 2 bytes = 32 KB/s

Stage 3: Input Mixer → Ring Buffer
  Rate: 16 kHz × 2 bytes = 32 KB/s

Stage 4: Ring Buffer → Python (batch sender)
  Batch size: ~8 KB
  Interval: 250 ms
  Effective rate: 32 KB/s (matches Stage 3)

Stage 5: Python Processing
  VAD: <1 ms per frame (10ms audio)
  Whisper: 100ms - 3s per inference (variable)

Stage 6: Python → Rust (IPC response)
  Event size: ~200-500 bytes
  Frequency: 1-5 events per second
```

---

## 4. Thread/Task Boundaries

```
┌──────────────────────────────────────────────────────────────────┐
│ NATIVE THREADS (JoinHandle via cpal)                             │
└──────────────────────────────────────────────────────────────────┘

1. Audio Stream Thread
   │ Constraint: Real-time (<10μs)
   │ Task: cpal stream.play() + callback invocation
   └─ Callback → Ring Buffer push (lock-free)

2. Watchdog Thread
   │ Interval: 250ms
   │ Threshold: 1200ms no callback
   └─ Event: AudioDeviceEvent::Stalled

3. Device Polling Thread
   │ Interval: 3 seconds
   └─ Event: AudioDeviceEvent::DeviceGone

4. Input Mixer Thread (planned)
   │ Task: per-input buffer → time align → gain/mix → ring buffer
   └─ Note: meeting-minutes-stt-multi-input spec


┌──────────────────────────────────────────────────────────────────┐
│ TOKIO ASYNC TASKS (tokio::spawn)                                 │
└──────────────────────────────────────────────────────────────────┘

5. IPC Reader Task
   │ Exclusive: sidecar_stdout
   │ Task: read_line() loop, JSON parse, event route
   └─ Cancellation: CancellationToken

6. Audio Sender Task (Batch Sender)
   │ Exclusive: Ring Buffer consumer, sidecar_stdin
   │ Task: pop from buffer → batch → serialize → write
   │ Interval: 250ms
   └─ Cancellation: CancellationToken

7. Device Event Monitor Task
   │ Task: Wait for AudioDeviceEvent → emit to frontend
   └─ Auto-reconnect on DeviceGone
```

---

## 5. Known Issues and Solutions

### 5.1 Buffer Overflow (CRITICAL) - RESOLVED

**Problem**: Unbounded MPSC channel caused memory growth when Python was slow.

**Solution**: Ring Buffer integration
- Fixed 160KB capacity
- Oldest data overwritten when full
- Lock-free for real-time safety

### 5.2 Rate Mismatch (HIGH) - MITIGATED

**Problem**: Rust produces 32KB/s, Python consumes at variable rate.

**Mitigation**:
- Ring Buffer absorbs burst
- 5-second buffer provides 5s of tolerance
- BufferLevel monitoring for early warning

### 5.3 Model Switch Stall (MEDIUM) - MONITORING

**Problem**: During `load_model()`, Python stops reading stdin.

**Current State**: Ring Buffer prevents data loss during short stalls.

**Future**: Consider background model loading.

---

## 6. Configuration Constants

### Rust Side (`ring_buffer.rs`)
```rust
pub const SAMPLE_RATE: usize = 16000;      // 16 kHz
pub const CHANNELS: usize = 1;              // mono
pub const BYTES_PER_SAMPLE: usize = 2;      // 16-bit PCM
pub const BUFFER_SECS: usize = 5;           // 5 seconds
pub const BUFFER_CAPACITY: usize = 160_000; // bytes
```

### Batch Sender (`commands.rs`)
```rust
const MIN_BATCH_BYTES: usize = 4000;        // 125ms minimum
const BATCH_INTERVAL_MS: u64 = 250;         // 250ms timer
const WRITE_TIMEOUT_SECS: u64 = 10;         // stdin timeout
```

### Python Side (`voice_activity_detector.py`)
```python
FRAME_DURATION_MS = 10      # 10ms frames
SPEECH_START_FRAMES = 30    # 0.3s to start
SPEECH_END_FRAMES = 50      # 0.5s to end
PRE_ROLL_FRAMES = 30        # 0.3s pre-roll
```

### Confidence Filter (`commands.rs`)
```rust
const MIN_CONFIDENCE: f64 = 0.50;  // 50% threshold
```

---

## 7. Monitoring Points

| Metric | Source | Purpose |
|--------|--------|---------|
| BufferLevel | Ring Buffer | Early overflow warning |
| batch_count | Sender Task | Throughput tracking |
| processing_time_ms | Python | Whisper latency |
| confidence | Python | Quality metric |
| model_change | Python | Resource adaptation |

---

## 8. Related Documents

- **ADR-013**: Sidecar Full-Duplex IPC Final Design
- **STT-REQ-007**: IPC Protocol Requirements
- **ring_buffer.rs**: Ring buffer implementation
- **audio_device_adapter.rs**: Audio capture implementation
- **commands.rs**: Tauri command handlers

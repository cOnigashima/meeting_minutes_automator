# Task 2.5: Device Disconnection Detection - Design Decisions

## Context

Task 2.5 (デバイス切断検出と自動再接続機能) では、当初の単純なアプローチ（cpalのエラーコールバックのみ）から、より堅牢なイベント駆動アーキテクチャに方針変更しました。

## Requirements

- **STT-REQ-004.9**: デバイス切断イベントを検出し、エラーログを記録
- **STT-REQ-004.10**: ユーザーに「音声デバイスが切断されました」通知を表示し、録音を停止
- **STT-REQ-004.11**: 5秒後に自動再接続を試行（最大3回）- **Phase外として延期**

## Initial Approach Issues

当初の単純なアプローチには3つの致命的な問題がありました：

### 1. StreamError::DeviceNotAvailable依存の危険性
- CoreAudio/PulseAudioは**無音になるだけ**でエラーコールバックを呼ばない場合がある
- エラーコールバックだけでは検出漏れが発生する

### 2. Arc<Mutex<bool>>だけでは回復不能
- `std::thread::park()`でストリームスレッドがブロックされたまま
- フラグをfalseにしてもストリームは生き続ける
- リソースリークが発生

### 3. トレイトAPI変更のBreaking Change
- 戻り値型を変更すると破壊的変更になる
- MVP0のFakeAudioDeviceとの互換性が失われる

### 4. MVP1仕様違反
- ログ出力だけではSTT-REQ-004.10を満たさない
- Tauri UIへの通知が必須

## Adopted Solution: Proposal A

ユーザーから提供された「Proposal A」を採用し、以下の堅牢なアーキテクチャを実装しました。

### Core Design Principles

#### 1. Event-Driven Architecture
```rust
pub enum AudioDeviceEvent {
    StreamError(String),           // cpalエラーコールバック
    Stalled { elapsed_ms: u64 },   // Liveness watchdog検出
    DeviceGone { device_id: String }, // デバイスポーリング検出
}

pub type AudioEventSender = mpsc::Sender<AudioDeviceEvent>;
pub type AudioEventReceiver = mpsc::Receiver<AudioDeviceEvent>;
```

#### 2. Triple Detection Mechanism

**a) Stream Error Callback (cpalネイティブ)**
- cpalのエラーコールバックでStreamErrorイベント送信
- 即座に検出できる場合に有効

**b) Liveness Watchdog**
- **チェック間隔**: 250ms
- **ストール閾値**: 1200ms（最後のコールバックから経過時間）
- 音声コールバックが呼ばれなくなったら`Stalled`イベント送信
- 無音になるケースを確実に検出

**c) Device Polling**
- **ポーリング間隔**: 3秒
- `cpal::Host::input_devices()`でデバイス存在確認
- デバイスが消えたら`DeviceGone`イベント送信
- 物理切断を確実に検出

#### 3. Reliable Cleanup with Shutdown Channels

```rust
pub struct CoreAudioAdapter {
    stream_thread: Option<JoinHandle<()>>,
    watchdog_handle: Option<JoinHandle<()>>,
    polling_handle: Option<JoinHandle<()>>,
    
    stream_shutdown_tx: Option<mpsc::Sender<()>>,
    watchdog_shutdown_tx: Option<mpsc::Sender<()>>,
    polling_shutdown_tx: Option<mpsc::Sender<()>>,
    
    last_callback: Arc<Mutex<Instant>>,
    event_tx: Option<AudioEventSender>,
}
```

**Key Points:**
- **Streamはスレッドローカルに保持**: Sync制約回避
- **3つの独立したshutdownチャネル**: 各スレッドを確実に終了
- **std::thread::park()を使わない**: `shutdown_rx.recv()`でブロック、確実にクリーンアップ

#### 4. Tauri UI Integration

```rust
// commands.rs
async fn monitor_audio_events(app: AppHandle) {
    let state = app.state::<AppState>();
    let rx = state.take_audio_event_rx().unwrap();
    
    while let Ok(event) = rx.recv() {
        match event {
            AudioDeviceEvent::DeviceGone { device_id } => {
                app.emit("audio-device-error", json!({
                    "type": "device_gone",
                    "message": "音声デバイスが切断されました",
                    "device_id": device_id,
                })).ok();
            }
            // ... other events
        }
    }
}
```

- **app.emit()**: フロントエンドへのイベント通知
- **STT-REQ-004.10準拠**: ユーザー通知を実装

### Implementation Phases

1. **Phase 1**: イベント定義（AudioDeviceEvent enum）
2. **Phase 2**: CoreAudioAdapter再設計（macOS）
3. **Phase 3**: Tauri UI通知統合
4. **Phase 4**: WasapiAdapter/AlsaAdapter対応（Windows/Linux）
5. **Phase 5**: ユニットテスト作成

### Cross-Platform Consistency

**macOS (CoreAudio)**:
```rust
impl AudioDeviceAdapter for CoreAudioAdapter {
    fn check_permission(&self) -> Result<()> { /* ... */ }
    fn start_recording_with_callback(...) { /* watchdog + polling */ }
    fn stop_recording(&mut self) { /* 3チャネルshutdown */ }
}
```

**Windows (WASAPI)**: 同じパターン  
**Linux (ALSA)**: 同じパターン

すべてのOSで一貫したイベント駆動アーキテクチャを実装。

### Testing Strategy

```rust
#[test]
fn test_audio_device_event_enum() { /* イベント型テスト */ }

#[test]
fn test_event_channel_send_receive() { /* mpsc動作確認 */ }

#[test]
#[cfg(target_os = "macos")]
fn test_core_audio_adapter_permission_check() { /* 許可確認 */ }
```

**結果**: 全36テスト合格 ✅

## Deferred Work

**STT-REQ-004.11（自動再接続ロジック）**は別タスクに延期：
- 現在の実装はイベント**検出・通知基盤**を提供
- 再接続ロジックは上位レイヤーで実装予定
- 5秒タイマー、最大3回リトライは将来のタスク

## Key Takeaways

1. **エラーコールバックだけでは不十分**: Liveness watchdogが必須
2. **多層防御**: Stream Error + Watchdog + Polling の3段構え
3. **信頼性の高いクリーンアップ**: Shutdownチャネルパターン
4. **API非破壊**: 新メソッド追加のみ（`set_event_sender`）
5. **クロスプラットフォーム一貫性**: 全OSで同じアーキテクチャ

## Files Modified

- `src-tauri/src/audio_device_adapter.rs` - コア実装
- `src-tauri/src/commands.rs` - UI通知統合
- `src-tauri/src/state.rs` - イベントチャネル管理
- `src-tauri/tests/unit/audio/test_device_adapter.rs` - テスト

## References

- Original discussion: Session continuation with Proposal A
- Requirements: `.kiro/specs/meeting-minutes-stt/requirements.md` (STT-REQ-004.9/10/11)
- Tasks: `.kiro/specs/meeting-minutes-stt/tasks.md` (Task 2.5)

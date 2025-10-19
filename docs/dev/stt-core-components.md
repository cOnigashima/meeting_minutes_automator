# STT Core Components (Rust)

最終更新: 2025-10-19  
対象ディレクトリ: `src-tauri/`

## オーディオ処理基盤

- `AudioRingBuffer` / `BufferLevel` — 5 秒分のオーディオを保持するロックレスバッファ。`try_send_frame` / `try_recv` で Python 側とのバックプレッシャを制御する。
- `FakeAudioDevice` / `AudioDevice` — MVP0 で利用するフェイクデバイス実装。`generate_dummy_data` で 100ms 間隔の PCM を提供し、`enumerate_devices_static()` で UI 向けのデバイス一覧 (`Vec<AudioDeviceInfo>`) を即時返却する。
- `AudioDeviceAdapter` / `CoreAudioAdapter` / `WasapiAdapter` — 実音声デバイスの列挙・監視・録音を担う。`set_audio_event_channel` でデバイス切断イベントを Tauri 側へ通知する。
- `set_audio_device`, `set_audio_event_channel`, `set_ipc_event_channel`, `set_python_sidecar`, `set_websocket_server`, `subscribe_ipc_events`, `set_selected_device_id`, `get_selected_device_id` — `AppState` に対する依存注入・ユーザー選択状態管理ヘルパー。
- `should_stop` / `stop_flag` — 録音停止条件を示すフラグセット。
- `LogEntry` / `LogLevel` / `with_details` / `with_message` — 構造化ログのエントリを表現し、`get_process_id` や `from_occupancy` などのヘルパーと組み合わせて診断イベントを記録する。

## セッション管理とストレージ

- `LocalStorageService` / `SessionHandle` — 録音セッションの生成 (`begin_session`)、ディスク容量チェック (`check_disk_space` / `is_critical`) を実装。
- `LoadedSession` / `SessionMetadata` — 保存済みセッションのメタデータとリプレイに使用する構造体。
- `TranscriptionEvent` / `transcript_writer` / `audio_writer` — `transcription.jsonl` と `audio.wav` をストリーム出力するためのユーティリティ。
- `save_metadata`, `create_audio_adapter`, `generate_session_id`, `validate_architecture`, `validate_python_version` — ローカルストレージ初期化時のサニティチェック群。
- `pop_for_writer`, `pcm_f32_to_i16_bytes` — Python サイドカーへ渡す前に PCM データをシリアライズするための補助関数。

## IPC / WebSocket 連携

- `PythonSidecarManager` / `PythonSidecarError` / `PythonDetectionError` — Python プロセスの検出・起動・エラーハンドリングを担当。`wait_for_ready`, `send_message`, `receive_message`, `force_close_stdin` を公開。
- `IpcMessage` / `VersionCompatibility` / `TranscriptionResult` / `as_transcription_result` — Rust から Python へ送受信する JSON スキーマを定義。
- `WebSocketMessage` / `WebSocketServer` — Chrome 拡張との WebSocket 通信を統括。`broadcast` で複数クライアントに配信する。
- `to_protocol_message` / `create_audio_adapter` — IPC 経由で Python 側に送る構造体を Rust の `TranscriptionResult` へ変換する。

## テスト支援

- `measure_partial_text_latency`, `measure_final_text_latency`, `monitor_cpu_usage_during_recording`, `monitor_memory_usage_long_running` — 統合テストで利用するメトリクス計測ヘルパー。
- `list_audio_devices`, `verify_local_storage_session`, `verify_partial_final_text_distribution` — 検証シナリオや QA ノートで利用する補助関数。

これらのコンポーネントが、`docs/uml/` のリングバッファ図や `.kiro/specs/meeting-minutes-stt/design.md` に記載されたアーキテクチャを支えている。新しいモジュールを追加する場合は、ここに概要を追記し、乖離検知ツール（`docs_crawler.py`）からも追跡できるようにする。

### デバイス選択ハンドリング例

```rust
use crate::audio::FakeAudioDevice;
use crate::state::AppState;

fn select_device(state: &AppState) -> anyhow::Result<()> {
    let devices = FakeAudioDevice::enumerate_devices_static()?;
    if let Some(first) = devices.first() {
        state.set_selected_device_id(first.id.clone());
    }

    if let Some(active) = state.get_selected_device_id() {
        println!("using device: {active}");
    }

    Ok(())
}
```

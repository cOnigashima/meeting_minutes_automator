# STT Audio Pipeline (Python)

最終更新: 2025-10-19  
対象モジュール: `python-stt/stt_engine/`

## 主なコンポーネント

- `SegmentType` / `AudioSegment` — VAD が検出した区間を表現するデータ構造。`speech_start`, `speech_partial`, `speech_end` の種別を `SegmentType` で管理する。
- `TranscriptionResult` — Whisper からの推論結果を保持。`text`, `is_final`, `confidence`, `language`, `processing_time_ms` をフィールドに持つ。
- `AudioPipeline` — `VoiceActivityDetector` と `WhisperSTTEngine` を束ねるオーケストレーター。`process_audio_frame` / `process_audio_frame_with_partial` でストリーム処理を行う。
- `AudioProcessor` — `AudioPipeline`, `IpcHandler`, `ResourceMonitor` を初期化し、`process_audio_stream` リクエストの実行を担うエントリポイント。
- `IpcHandler`, `IpcTimeoutError`, `IpcProtocolError` — Python サイドカーと Rust の間で JSON メッセージ（`process_audio_stream`, `model_change` など）を送受信するためのユーティリティ。
- `LifecycleManager` — サイドカー起動・シャットダウンの制御、`analyze_metrics` / `parse_metrics_from_log` へのメトリクス連携処理。

これらのクラス／関数は `.kiro/specs/meeting-minutes-stt/design.md` の音声パイプライン設計や `docs/uml/meeting-minutes-stt` の図解と対応しており、`docs_crawler.py` による乖離検知対象でもある。変更時はこのドキュメントとスナップショットを更新し、仕様との整合を維持する。

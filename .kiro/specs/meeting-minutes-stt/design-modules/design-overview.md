# Technical Design Document

## Overview

**目的**: meeting-minutes-stt (MVP1) は、meeting-minutes-core (Walking Skeleton) で確立した3プロセスアーキテクチャ上に実際の音声処理機能を実装します。Fake実装を実音声処理に置き換え、faster-whisperによる高精度文字起こしとwebrtcvadによる音声活動検出を実現します。

**ユーザー**: 会議参加者やファシリテーターが、実用可能なローカル音声文字起こし機能を活用します。

**インパクト**: Fakeデータから実音声処理への移行により、プロダクション環境で使用可能な文字起こし機能が実現されます。

### Goals

- ローカル環境での高精度リアルタイム音声認識 (faster-whisper統合)
- **オフラインファースト**: ネットワーク接続なしでの完全動作保証
- リソースベースモデル選択と動的ダウングレードによる安定性確保
- クロスプラットフォーム音声デバイス対応 (macOS、Windows、Linux)
- meeting-minutes-coreとの完全な後方互換性維持

### Non-Goals

- Google Docs連携 (MVP2 meeting-minutes-docs-syncで実装)
- LLM要約生成 (MVP3 meeting-minutes-llmで実装)
- UIの本格的洗練 (MVP3で実施)
- 話者分離 (speaker diarization) - 将来検討事項
- リアルタイム翻訳機能 - 将来検討事項

---

## Diagram Checklist

本MVP1実装では、以下の図版を作成し `docs/uml/meeting-minutes-stt/` に配置します:

- ✅ `cmp/CMP-001_STT-Audio-Processing-Pipeline.puml`: コンポーネント図（RealAudioDevice, VoiceActivityDetector, WhisperSTTEngine, ResourceMonitor, LocalStorageService）
- ✅ `seq/SEQ-001_Audio-Recording-to-Transcription.puml`: 音声録音→VAD→STT→保存の完全シーケンス
- ✅ `seq/SEQ-002_Offline-Model-Fallback.puml`: オフラインモデルフォールバックフロー
- ✅ `seq/SEQ-003_Dynamic-Model-Downgrade.puml`: 動的モデルダウングレードフロー
- ✅ `cls/CLS-001_Audio-Device-Adapter.puml`: AudioDeviceAdapter trait と OS別実装

全図版は `#[[file:docs/uml/meeting-minutes-stt/<カテゴリ>/ID_Title.puml]]` 形式で参照されます。

---

## Non-Functional Requirements

### ログ運用方針

**目的**: STT処理のトラブルシューティングとパフォーマンス分析を迅速化するため、詳細メタデータを含む構造化ログを収集します。

**方針** (`.kiro/steering/principles.md` の非機能ベースライン原則に準拠):
- INFO/DEBUG/ERROR レベル運用と構造化JSON出力
- 全ログレコードに `session_id` / `component` / `event` / `duration_ms` を付与
- PIIマスク: 音声データのバイナリ内容はログに記録しない (メタデータのみ)
- DEBUGレベルは開発環境のみで有効化

**STT固有のログ要件** (STT-NFR-005):
1. **faster-whisperモデルロード中**: ダウンロード進捗を5秒間隔でINFOレベル記録
2. **音声処理エラー**: エラーメッセージ、スタックトレース、コンテキストをERRORレベル記録
3. **VAD処理**: 発話開始/終了イベントをINFOレベル記録
4. **リソース監視**: メモリ使用量、CPU使用率を30秒間隔でDEBUGレベル記録

**実装タスク**:
- `python-stt/stt_engine/logging.py`: `structlog` 構成で非同期ファイルハンドラ追加
- Rust側: `tracing`/`tracing-subscriber` で JSON フォーマッタ実装 (meeting-minutes-core継承)

---


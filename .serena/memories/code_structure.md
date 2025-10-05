# Codebase Structure

## Current State (Specification Phase)
プロジェクトは仕様フェーズにあり、実装コードベースはまだ作成されていません。

### 現在存在するディレクトリ
```
meeting-minutes-automator/
├── .kiro/                    # Kiro仕様駆動開発ディレクトリ
│   ├── steering/             # プロジェクトガイダンス文書（4ファイル）
│   └── specs/                # 機能仕様書（umbrella + 3 sub-specs）
├── docs/                     # プロジェクトドキュメント
│   ├── uml/                  # PlantUMLアーキテクチャ図
│   └── dev/                  # 開発ガイドライン
├── scripts/                  # 開発・ビルドスクリプト
├── .pre-commit-config.yaml   # Pre-commitフック設定
├── .claude/                  # Claude Code設定
└── CLAUDE.md                 # プロジェクト指示書
```

## Target Structure (実装完了時)

### Tauri Application (src-tauri/)
- `src/main.rs`: エントリーポイント
- `src/commands/`: Tauriコマンド実装（audio, websocket, settings）
- `src/services/`: ビジネスロジック
  - `audio_device_adapter.rs`: OS固有音声API抽象化
  - `python_sidecar_manager.rs`: Pythonプロセスライフサイクル管理
  - `websocket_service.rs`: WebSocket通信サービス
- `src/models/`: データ構造定義
- `src/database/`: データベース関連

### Frontend (src/)
- `components/`: 再利用可能UIコンポーネント
  - `audio/`: 音声関連（AudioControls, AudioVisualizer, DeviceSelector）
  - `transcription/`: 文字起こし関連
  - `settings/`: 設定関連
- `pages/`: ページレベルコンポーネント
- `hooks/`: カスタムReactフック（useAudioCapture, useWebSocket, useTranscription）
- `stores/`: Zustand状態管理
- `types/`: TypeScript型定義

### Chrome Extension (chrome-extension/)
- `src/background/`: Service Worker
- `src/content/`: Content Scripts（Google Docs操作）
- `src/popup/`: ポップアップUI
- `src/options/`: 設定ページ
- `manifest.json`: Manifest V3設定

### Python Sidecar (python-stt/)
- `main.py`: エントリーポイント
- `stt_engine/app.py`: アプリケーションロジック
- `stt_engine/audio/`: 音声処理（VAD、前処理のみ。録音はRust側）
- `stt_engine/transcription/`: 文字起こし（whisper_client, streaming）
- `stt_engine/ipc/`: stdin/stdout JSONプロトコル
- `requirements.txt`: 本番依存関係

## Domain-Driven Design
- **Audio Domain**: 音声キャプチャ、VAD、前処理
- **Transcription Domain**: STT、後処理、精度管理
- **Summarization Domain**: 要約生成、キーポイント抽出
- **Communication Domain**: WebSocket、メッセージング、同期

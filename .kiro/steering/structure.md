# Project Structure

## Current State vs Target Structure

**📍 現在の状態**: Kiro仕様駆動開発の**実装フェーズ（Phase 3）**に入りました。meeting-minutes-core (Walking Skeleton/MVP0) の Task 1.1（プロジェクト基盤セットアップ）が完了し、実装コードベースの構築が開始されています。

### 現在存在するディレクトリ（Current State - 2025-10-05更新）

```
meeting-minutes-automator/
├── .kiro/                           # ✅ Kiro仕様駆動開発ディレクトリ
│   ├── steering/                    # ✅ プロジェクトガイダンス文書（4ファイル）
│   ├── specs/                       # ✅ 機能仕様書（umbrella + 5 sub-specs）
│   │   ├── meeting-minutes-automator/   # Umbrella spec
│   │   ├── meeting-minutes-core/        # MVP0 (Walking Skeleton) ✅
│   │   ├── meeting-minutes-stt/         # MVP1 (Real STT)
│   │   ├── meeting-minutes-docs-sync/   # MVP2 (Google Docs Sync)
│   │   ├── meeting-minutes-ci/          # Infrastructure (CI/CD) ✅
│   │   └── meeting-minutes-llm/         # MVP3 (LLM Summary)
│   └── research/                    # ✅ 技術調査資料
├── docs/                            # ✅ プロジェクトドキュメント
│   ├── uml/                         # ✅ PlantUMLアーキテクチャ図
│   └── dev/                         # ✅ 開発ガイドライン
│       ├── coding-standards.md      # ✅ コーディング規約
│       ├── spec-authoring.md        # ✅ 仕様作成ガイド
│       └── chrome-storage-best-practices.md  # ✅ Chrome Storage API使い方
├── scripts/                         # ✅ 開発・ビルドスクリプト
│   └── check_forbidden_imports.py   # ✅ 静的解析スクリプト
├── .pre-commit-config.yaml          # ✅ Pre-commitフック設定
├── .claude/                         # ✅ Claude Code設定
│   ├── commands/                    # ✅ カスタムコマンド
│   └── agents/                      # ✅ 専用エージェント
├── CLAUDE.md                        # ✅ プロジェクト指示書
├── src-tauri/                       # ✅ Tauriアプリケーションコア（Task 1.1で作成）
├── src/                             # ✅ フロントエンド（React）（Task 1.1で作成）
├── chrome-extension/                # ✅ Chrome拡張機能（Task 1.1で作成）
├── python-stt/                      # ✅ Pythonサイドカー（Task 1.1で作成）
├── package.json                     # ✅ Node.js依存関係
├── tsconfig.json                    # ✅ TypeScript設定
└── vite.config.ts                   # ✅ Viteビルド設定
```

### 未作成のディレクトリ（今後の実装で作成予定）

```
meeting-minutes-automator/
├── tests/                           # 🔵 テストスイート（Task 1.3以降で作成）
│   ├── unit/                        # ユニットテスト
│   ├── integration/                 # 統合テスト
│   └── e2e/                         # E2Eテスト
└── target/                          # 🔵 Rustビルド出力（自動生成）
```

### Spec-Driven Development Approach

本プロジェクトは、**実装前に詳細な仕様と設計を策定する**アプローチを採用しています:

1. **Phase 1: Steering** ✅ 完了
   - 製品方針、技術スタック、構造、設計原則の確定
   - 4つのsteering documents作成完了

2. **Phase 2: Specification** ✅ 完了
   - meeting-minutes-core (MVP0): Implementation Complete ✅
   - meeting-minutes-stt (MVP1): Design Validated（Tasks生成待ち）
   - meeting-minutes-docs-sync (MVP2): Design Generated（検証待ち）
   - meeting-minutes-ci (Infrastructure): Spec Initialized ✅

3. **Phase 3: Implementation** ✅ MVP0完了（2025-10-10）
   - Walking Skeleton実装（MVP0）完了 ✅
   - E2Eフロー検証完了 ✅
   - 主要ADR作成完了:
     - ADR-004: Chrome拡張WebSocket管理（Content Script方式）
     - ADR-005: chrome.storage.local状態管理メカニズム
   - ドキュメント整備:
     - chrome-storage-best-practices.md 作成
     - mvp0-known-issues.md 作成
   - 次のステップ: MVP1 (Real STT) タスク生成

**参照**:
- プロジェクト開発ガイドライン: [docs/dev/coding-standards.md](../docs/dev/coding-standards.md)
- 仕様作成ガイド: [docs/dev/spec-authoring.md](../docs/dev/spec-authoring.md)

---

## Root Directory Organization (Target Structure)

**注意**: 以下は実装完了時の目標構造です。

```
meeting-minutes-automator/
├── .kiro/                           # Kiro仕様駆動開発ディレクトリ
│   ├── steering/                    # プロジェクトガイダンス文書
│   └── specs/                       # 機能仕様書
├── src-tauri/                       # Tauriアプリケーションコア
│   ├── src/                         # Rustソースコード
│   ├── Cargo.toml                   # Rust依存関係
│   └── tauri.conf.json             # Tauri設定
├── src/                             # フロントエンド（React）
│   ├── components/                  # UIコンポーネント
│   ├── pages/                       # ページコンポーネント
│   ├── hooks/                       # カスタムReactフック
│   ├── stores/                      # Zustand状態管理
│   └── types/                       # TypeScript型定義
├── chrome-extension/                # Chrome拡張機能
│   ├── src/                         # 拡張ソースコード
│   ├── manifest.json               # Manifest V3設定
│   └── build/                       # ビルド出力
├── python-stt/                      # Pythonサイドカー
│   ├── stt_engine/                  # 音声処理エンジン
│   ├── requirements.txt             # Python依存関係
│   └── main.py                      # エントリーポイント
├── tests/                           # テストスイート
│   ├── unit/                        # ユニットテスト
│   ├── integration/                 # 統合テスト
│   └── e2e/                         # E2Eテスト
├── docs/                            # プロジェクトドキュメント
├── scripts/                         # 開発・ビルドスクリプト
└── README.md                        # プロジェクト概要
```

## Subdirectory Structures

### Tauriアプリケーション (`src-tauri/`)

```
src-tauri/
├── src/
│   ├── main.rs                      # アプリケーションエントリーポイント
│   ├── lib.rs                       # ライブラリルート
│   ├── commands/                    # Tauriコマンド実装
│   │   ├── mod.rs
│   │   ├── audio.rs                 # 音声関連コマンド
│   │   ├── websocket.rs             # WebSocket管理
│   │   └── settings.rs              # 設定管理
│   ├── services/                    # ビジネスロジック
│   │   ├── mod.rs
│   │   ├── audio_device_adapter.rs  # OS固有音声API抽象化層
│   │   ├── audio_stream_bridge.rs   # Python IPC通信層
│   │   ├── python_sidecar_manager.rs # Pythonプロセスライフサイクル管理
│   │   ├── websocket_service.rs     # WebSocket通信サービス
│   │   └── storage_service.rs       # データ永続化サービス
│   ├── models/                      # データ構造定義
│   │   ├── mod.rs
│   │   ├── audio.rs                 # 音声関連データ型
│   │   ├── transcription.rs         # 文字起こし型
│   │   └── session.rs               # セッション型
│   ├── utils/                       # ユーティリティ関数
│   │   ├── mod.rs
│   │   ├── audio_utils.rs           # 音声処理ヘルパー
│   │   └── error_handling.rs        # エラー処理
│   └── database/                    # データベース関連
│       ├── mod.rs
│       ├── migrations/              # マイグレーションスクリプト
│       └── models.rs                # データベースモデル
├── Cargo.toml                       # 依存関係とメタデータ
├── tauri.conf.json                  # Tauri設定ファイル
└── build.rs                         # ビルドスクリプト
```

### フロントエンド (`src/`)

```
src/
├── components/                      # 再利用可能UIコンポーネント
│   ├── common/                      # 汎用コンポーネント
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   └── Modal.tsx
│   ├── audio/                       # 音声関連コンポーネント
│   │   ├── AudioControls.tsx        # 録音制御
│   │   ├── AudioVisualizer.tsx      # 音声波形表示
│   │   └── DeviceSelector.tsx       # デバイス選択
│   ├── transcription/               # 文字起こし関連
│   │   ├── TranscriptionDisplay.tsx # テキスト表示
│   │   ├── PartialText.tsx          # 部分結果表示
│   │   └── SummaryPanel.tsx         # 要約パネル
│   └── settings/                    # 設定関連
│       ├── SettingsPanel.tsx
│       └── PreferencesForm.tsx
├── pages/                           # ページレベルコンポーネント
│   ├── MainPage.tsx                 # メインページ
│   ├── SettingsPage.tsx             # 設定ページ
│   └── HistoryPage.tsx              # 履歴ページ
├── hooks/                           # カスタムReactフック
│   ├── useAudioCapture.ts           # 音声キャプチャフック
│   ├── useWebSocket.ts              # WebSocket通信フック
│   ├── useTranscription.ts          # 文字起こしフック
│   └── useSettings.ts               # 設定管理フック
├── stores/                          # Zustand状態管理
│   ├── audioStore.ts                # 音声状態
│   ├── transcriptionStore.ts        # 文字起こし状態
│   ├── settingsStore.ts             # 設定状態
│   └── sessionStore.ts              # セッション状態
├── types/                           # TypeScript型定義
│   ├── audio.ts                     # 音声関連型
│   ├── transcription.ts             # 文字起こし型
│   ├── websocket.ts                 # WebSocket型
│   └── settings.ts                  # 設定型
├── utils/                           # ユーティリティ関数
│   ├── formatters.ts                # データフォーマッター
│   ├── validators.ts                # バリデーター
│   └── constants.ts                 # 定数定義
├── styles/                          # スタイルファイル
│   ├── globals.css                  # グローバルスタイル
│   └── components.css               # コンポーネントスタイル
├── main.tsx                         # Reactエントリーポイント
└── App.tsx                          # アプリケーションルート
```

### Chrome拡張 (`chrome-extension/`)

```
chrome-extension/
├── src/
│   ├── background/                  # Service Worker
│   │   ├── background.ts            # バックグラウンドスクリプト
│   │   ├── websocket-client.ts      # WebSocket通信
│   │   └── message-handler.ts       # メッセージハンドリング
│   ├── content/                     # Content Scripts
│   │   ├── content.ts               # Google Docsページ操作
│   │   ├── docs-injector.ts         # テキスト挿入処理
│   │   └── page-detector.ts         # ページ検出
│   ├── popup/                       # ポップアップUI
│   │   ├── Popup.tsx                # メインポップアップ
│   │   ├── components/              # ポップアップ用コンポーネント
│   │   └── popup.html               # ポップアップHTML
│   ├── options/                     # 設定ページ
│   │   ├── Options.tsx              # 設定画面
│   │   └── options.html             # 設定HTML
│   ├── types/                       # 拡張用型定義
│   │   ├── chrome.ts                # Chrome API型
│   │   ├── docs.ts                  # Google Docs関連型
│   │   └── messages.ts              # メッセージ型
│   └── utils/                       # ユーティリティ
│       ├── google-auth.ts           # Google認証
│       ├── docs-api.ts              # Docs API操作
│       └── storage.ts               # ストレージ操作
├── public/                          # 静的ファイル
│   ├── icons/                       # アイコンファイル
│   └── _locales/                    # 国際化ファイル
├── manifest.json                    # Manifest V3設定
└── build/                           # ビルド出力ディレクトリ
```

### Pythonサイドカー (`python-stt/`)

```
python-stt/
├── main.py                          # エントリーポイント（実行用ラッパー）
├── stt_engine/                      # パッケージルート
│   ├── __init__.py
│   ├── app.py                       # アプリケーションロジック（メイン処理）
│   ├── lifecycle.py                 # プロセスライフサイクル管理（NEW）
│   ├── audio/                       # 音声処理モジュール
│   │   ├── __init__.py
│   │   ├── vad.py                   # 音声活動検出
│   │   └── preprocessing.py         # 音声前処理（正規化、ノイズ除去）
│   │   # 注意: 音声録音はRust側のAudioDeviceAdapterが担当
│   ├── transcription/               # 文字起こしモジュール
│   │   ├── __init__.py
│   │   ├── whisper_client.py        # faster-whisper操作
│   │   ├── streaming.py             # ストリーミング処理
│   │   └── post_processing.py       # 後処理
│   ├── summarization/               # 要約モジュール
│   │   ├── __init__.py
│   │   ├── extractive.py            # 抽出型要約
│   │   ├── generative.py            # 生成型要約
│   │   └── key_points.py            # キーポイント抽出
│   ├── ipc/                         # プロセス間通信（NEW）
│   │   ├── __init__.py
│   │   ├── protocol.py              # stdin/stdout JSONプロトコル
│   │   ├── health_check.py          # ヘルスチェック機構
│   │   └── message_handler.py       # メッセージディスパッチ
│   ├── adapters/                    # 外部依存抽象化層（NEW）
│   │   ├── __init__.py
│   │   ├── stt_adapter.py           # STT Engine抽象化
│   │   ├── llm_adapter.py           # LLM API抽象化
│   │   └── storage_adapter.py       # ストレージ抽象化
│   └── utils/                       # ユーティリティ
│       ├── __init__.py
│       ├── config.py                # 設定管理
│       ├── logging.py               # ログ処理
│       └── error_handling.py        # エラー処理
├── tests/                           # Pythonテスト
│   ├── test_audio.py
│   ├── test_vad.py
│   ├── test_transcription.py
│   └── test_integration.py
├── requirements.txt                 # 本番依存関係
├── requirements-dev.txt             # 開発依存関係
└── setup.py                         # パッケージ設定
```

## Code Organization Patterns

### Domain-Driven Design (DDD)

**ドメイン境界の明確化**:
- **Audio Domain**: 音声キャプチャ、VAD、前処理
- **Transcription Domain**: STT、後処理、精度管理
- **Summarization Domain**: 要約生成、キーポイント抽出
- **Communication Domain**: WebSocket、メッセージング、同期

**レイヤードアーキテクチャ**:
```
Presentation Layer    (UI Components, Controllers)
    ↓
Application Layer     (Use Cases, Services)
    ↓
Domain Layer         (Business Logic, Entities)
    ↓
Infrastructure Layer  (Database, External APIs)
```

### Dependency Injection Pattern

**Rust側（依存性注入）**:
```rust
// services/mod.rs
pub struct ServiceContainer {
    pub audio_service: Arc<AudioService>,
    pub websocket_service: Arc<WebSocketService>,
    pub storage_service: Arc<StorageService>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        let storage = Arc::new(StorageService::new());
        let audio = Arc::new(AudioService::new(storage.clone()));
        let websocket = Arc::new(WebSocketService::new());

        Self {
            audio_service: audio,
            websocket_service: websocket,
            storage_service: storage,
        }
    }
}
```

**React側（カスタムフック）**:
```typescript
// hooks/useServices.ts
export const useServices = () => {
  const audioService = useAudioCapture();
  const transcriptionService = useTranscription();
  const websocketService = useWebSocket();

  return {
    audioService,
    transcriptionService,
    websocketService,
  };
};
```

## File Naming Conventions

### Rust Naming

- **Modules**: `snake_case` (e.g., `audio_service.rs`)
- **Structs**: `PascalCase` (e.g., `AudioSession`)
- **Functions**: `snake_case` (e.g., `start_recording`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_SAMPLE_RATE`)

### TypeScript/React Naming

- **Components**: `PascalCase` (e.g., `AudioControls.tsx`)
- **Hooks**: `camelCase` with `use` prefix (e.g., `useAudioCapture.ts`)
- **Types**: `PascalCase` (e.g., `AudioConfig`)
- **Utilities**: `camelCase` (e.g., `formatDuration.ts`)

### Python Naming

- **Modules**: `snake_case` (e.g., `audio_capture.py`)
- **Classes**: `PascalCase` (e.g., `WhisperClient`)
- **Functions**: `snake_case` (e.g., `process_audio_chunk`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `SAMPLE_RATE`)

## Import Organization

### Rust Import Ordering

```rust
// 1. Standard library imports
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crate imports
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

// 3. Internal module imports
use crate::models::AudioSession;
use crate::services::AudioService;

// 4. Local imports
use super::utils::format_duration;
```

### TypeScript Import Ordering

```typescript
// 1. React and React ecosystem
import React, { useState, useEffect } from 'react';

// 2. External libraries
import { invoke } from '@tauri-apps/api/tauri';
import clsx from 'clsx';

// 3. Internal utilities and hooks
import { useAudioCapture } from '@/hooks/useAudioCapture';
import { formatDuration } from '@/utils/formatters';

// 4. Type imports (separated)
import type { AudioConfig, AudioDevice } from '@/types/audio';

// 5. Relative imports
import './AudioControls.css';
```

### Python Import Ordering

```python
# 1. Standard library
import asyncio
import logging
from typing import Optional, List

# 2. Third-party packages
import numpy as np
import sounddevice as sd
from faster_whisper import WhisperModel

# 3. Local application imports
from .audio.vad import VoiceActivityDetector
from .utils.config import get_config
```

## Key Architectural Principles

### 1. Separation of Concerns

**責任の明確な分離**:
- **UI Layer**: ユーザーインタラクションのみ
- **Service Layer**: ビジネスロジックとデータ処理
- **Data Layer**: 永続化とデータアクセス
- **Communication Layer**: プロセス間通信

### 2. Asynchronous-First Design

**非同期パターンの統一**:
- **Rust**: `async/await` with `tokio`
- **TypeScript**: `Promise` based APIs
- **Python**: `asyncio` for I/O operations

### 3. Error Handling Strategy

**Result型による明示的エラー処理**:
```rust
type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Audio device error: {0}")]
    AudioDevice(String),
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("STT processing error: {0}")]
    SttProcessing(String),
}
```

### 4. Configuration Management

**環境別設定の階層化**:
```
config/
├── default.json          # デフォルト設定
├── development.json       # 開発環境
├── production.json        # 本番環境
└── local.json            # ローカル上書き（git無視）
```

### 5. Testing Strategy

**テストピラミッド構造**:
- **Unit Tests**: 各モジュールの単体機能
- **Integration Tests**: コンポーネント間連携
- **E2E Tests**: ユーザーワークフロー全体
- **Performance Tests**: レスポンス時間と負荷

### 6. Documentation Strategy

**生きたドキュメントの維持**:
- **Code Comments**: 複雑なロジックの説明
- **API Documentation**: 自動生成によるAPI仕様
- **Architecture Decision Records (ADR)**: 重要な設計決定の記録
- **User Guides**: 機能別の操作ガイド
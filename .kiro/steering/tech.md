# Technology Stack

## Architecture

### High-Level System Design

**ハイブリッドアーキテクチャ**: Tauriデスクトップアプリケーション + Chrome拡張の連携システム

```
┌─────────────────────────────────┐    ┌──────────────────────────┐
│        Tauriデスクトップアプリ         │    │      Chrome拡張          │
│  ┌─────────────┐ ┌─────────────┐ │    │  ┌─────────────────────┐ │
│  │   Frontend  │ │   Backend   │ │    │  │    Popup UI         │ │
│  │   (React)   │ │   (Rust)    │ │◄──►│  │    Content Script   │ │
│  └─────────────┘ └─────────────┘ │    │  │    Background SW    │ │
│         │              │         │    │  └─────────────────────┘ │
│         │       ┌─────────────┐  │    └──────────────────────────┘
│         └───────┤ Python STT  │  │                   │
│                 │  サイドカー   │  │                   │
│                 └─────────────┘  │                   ▼
└─────────────────────────────────┘         ┌──────────────────┐
                   │                        │   Google Docs    │
                   ▼                        │      API         │
        ┌─────────────────────┐             └──────────────────┘
        │   ローカルストレージ    │
        │    (SQLite)        │
        └─────────────────────┘
```

### Core Principles

- **ローカルファースト**: プライバシー保護のため音声処理は原則ローカル実行
- **リアルタイム性**: 0.5秒以内の応答時間を目標とした非同期パイプライン
- **モジュラー設計**: 各コンポーネントの独立性と交換可能性
- **クロスプラットフォーム**: macOS、Windows、Linux統一体験

## Frontend

### デスクトップアプリケーション (Tauri)

**Framework**: Tauri 2.0
- **理由**: Electronと比較して90%小さいバンドルサイズ、高いセキュリティ、ネイティブパフォーマンス
- **バージョン**: 2.0以降（Raw Payloads対応）

**Frontend Framework**: React 18+ with TypeScript
```json
{
  "react": "^18.0.0",
  "typescript": "^5.0.0",
  "@types/react": "^18.0.0"
}
```

**状態管理**: 現行MVPでは `useState` ベースのシンプル構成  
- **方針**: グローバル状態が必要になったタイミングで Zustand などを導入し、導入時は ADR で目的と影響をレビューする。

**UI Library**: プレーンCSS + `App.css`  
- **理由**: MVP1 では録音ボタン中心の最小UIを提供。Tailwind / shadcn は今後のUI拡張時に検討する。

### Chrome拡張

**Manifest Version**: V3 (必須)
```json
{
  "manifest_version": 3,
  "minimum_chrome_version": "116"
}
```

**Frontend Framework**: 現状はプレーン TypeScript + DOM API  
- **Popup**: まだ未実装（MVP2 でReactベースのUIを導入予定）  
- **Content Script**: Google Meetページ上で WebSocket 管理・表示ログ出力（ADR-004）  
- **Service Worker**: Manifest V3 制約下での最小メッセージリレーのみ実装

**アーキテクチャ決定**:
- **[ADR-004: Chrome Extension WebSocket Management](../.kiro/specs/meeting-minutes-core/adrs/ADR-004-chrome-extension-websocket-management.md)**
  - **決定**: Content ScriptでWebSocket接続を管理（Service Worker方式を却下）
  - **理由**: MV3のService Worker 30秒制限回避、タブ単位の状態管理、接続永続性
  - **影響**: WebSocketクライアントはContent Scriptに実装、状態はchrome.storage.localで共有

**状態管理メカニズム**:
- **[ADR-005: State Management Mechanism](../.kiro/specs/meeting-minutes-core/adrs/ADR-005-state-management-mechanism.md)**
  - **決定**: chrome.storage.localを中心とした3層状態管理（Presentation / Bridge / Persistence）
  - **重要**: ドット記法による部分更新は不可能 → オブジェクト全体を更新
  - **パターン**: イミュータブル更新（既存取得→スプレッド演算子→全体保存）
  - **参照**: [chrome-storage-best-practices.md](../../docs/dev/chrome-storage-best-practices.md)

## Backend

### Core Backend (Rust)

**Runtime**: Tauri 2.0 Core
```toml
[dependencies]
# セキュリティ: api-allは使用せず、必要な機能のみを列挙
tauri = { version = "2.0", features = [
    "protocol-asset",     # アセット配信
    "window-create",      # ウィンドウ管理
    "fs-read-file",       # ファイル読み込み
    "fs-write-file",      # ファイル書き込み
    "dialog-open",        # ファイル選択ダイアログ
    "dialog-save",        # 保存ダイアログ
    "notification",       # システム通知
    "clipboard-write-text", # クリップボード書き込み
] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

**セキュリティ設定の理由**:
- `api-all`は開発時は便利だが、プロダクションでは攻撃面を広げる
- Tauriのセキュリティモデルは「必要最小限の権限」を推奨
- 各機能は`tauri.conf.json`の`allowlist`とも連動させる必要あり

**WebSocket Server**: tokio-tungstenite
```toml
tokio-tungstenite = "0.20"
```

**永続化**: ファイルベースのローカルセッションディレクトリ  
- `AppData/recordings/<session_id>/` 配下に `audio.wav` / `transcription.jsonl` / `session.json` を保存（Task 6.x）。  
- 将来的にSQLite等へ移行する場合は ADR で判断する。

**音声処理インターフェース**: Rust subprocess → Python (stdin/stdout JSON IPC)

### Python Sidecar Lifecycle Management

- **起動**: `PythonSidecarManager::start()` が Python 実行ファイルを検出し、`python-stt/main.py` を `-u`（行バッファ）付きで起動。  
- **レディシグナル**: Whisper モデル初期化後、Python 側は `{"type":"ready","message":"Python sidecar ready (MVP1 Real STT)"}` を標準出力に送信し、Rust 側 `wait_for_ready()` がハンドシェイクを完了する。  
- **終了処理**: `PythonSidecarManager::shutdown()` が `{"type":"shutdown"}` を送信し、3秒待機後にプロセスを回収。`Drop` 実装で異常終了時もクリーンアップ。  

#### IPCメッセージ（行区切り JSON）

- **音声ストリーム要求**（Rust → Python）
```json
{
  "id": "chunk-1739954160123",
  "type": "request",
  "version": "1.0",
  "method": "process_audio_stream",
  "params": {
    "audio_data": [0, 0, 12, 255, ...]
  }
}
```
  - `audio_data` は 16kHz 10ms フレーム（320byte）をそのまま `Vec<u8>` として送信（Base64 ではない）。

- **イベント通知**（Python → Rust）
```json
{"type":"event","version":"1.0","eventType":"speech_start","data":{"requestId":"chunk-1739954160123","timestamp":1739954160456}}
{"type":"event","version":"1.0","eventType":"partial_text","data":{"requestId":"chunk-1739954160123","text":"hello","is_final":false,"confidence":0.62,"language":"en","processing_time_ms":312,"model_size":"small"}}
{"type":"event","version":"1.0","eventType":"final_text","data":{"requestId":"chunk-1739954160123","text":"hello world","is_final":true,"confidence":0.79,"language":"en","processing_time_ms":812,"model_size":"small"}}
{"type":"event","version":"1.0","eventType":"speech_end","data":{"requestId":"chunk-1739954160123","timestamp":1739954161820}}
{"type":"event","version":"1.0","eventType":"model_change","data":{"old_model":"small","new_model":"tiny","reason":"cpu_high"}}
```

- **エラー通知**（Python → Rust）
```json
{
  "type": "error",
  "id": "chunk-1739954160123",
  "version": "1.0",
  "errorCode": "AUDIO_PIPELINE_ERROR",
  "errorMessage": "webrtcvad returned invalid frame length",
  "recoverable": true
}
```

- **レスポンス互換性**: IPCプロトコルはセマンティックバージョニング（ADR-003）により後方互換性を保証。`IpcMessage`でバージョンチェック（major不一致→エラー、minor不一致→警告）を実施。

#### Backpressure & Monitoring（ADR-013）
- 音声送信用の `tokio::sync::mpsc` と 5 秒リングバッファを採用し、Python 側の処理遅延時に `no_speech` / タイムアウトを Rust 側へ通知。
- ResourceMonitor は CPU/メモリ監視を 30 秒周期で実行し、ダウングレード提案・強制停止・アップグレード提案を IPC イベントで送信する。

### Audio Processing Backend (Python)

**Core Engine**: faster-whisper (CTranslate2最適化版)
```txt
faster-whisper>=0.10.0
```

**Voice Activity Detection**: webrtcvad
```txt
webrtcvad>=2.0.0
```

**Audio Processing**: numpy（音声データ処理用）
```txt
numpy>=1.24.0
```

**注意**: 音声録音（キャプチャ）はRust側の`AudioDeviceAdapter`が担当します。
Python側は音声データの前処理（正規化、ノイズ除去）とSTT処理のみを行います。

**Requirements.txt**:
```txt
faster-whisper>=0.10.0
webrtcvad>=2.0.0
numpy>=1.24.0
# 注意: asyncio と queue は Python 3.9+ の標準ライブラリのため記載不要
# PyPIの古いパッケージによる上書きを防ぐため、意図的に除外しています
# sounddevice, pyaudioは削除: 音声録音はRust側のAudioDeviceAdapterが担当
```

## Process Communication Architecture

### プロセス間通信の全体像

本システムは3つの独立したプロセスで構成されます:

1. **Tauri App (Rust)**: コア処理とプロセス調整
2. **Python Sidecar**: 音声処理専用プロセス
3. **Chrome Extension**: ブラウザUI and Google Docs統合

```
┌─────────────────────────────────────────────────────────────┐
│                         ユーザー                              │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┴─────────────────────┐
        │                                           │
        ▼                                           ▼
┌──────────────────┐                    ┌──────────────────────┐
│   Tauri App      │◄───WebSocket──────►│  Chrome Extension    │
│   (Rust)         │    (Port 9001-     │  (JavaScript)        │
│                  │     9100 range)     │                      │
└────────┬─────────┘                    └──────────────────────┘
         │                                          │
         │ stdin/stdout                             │
         │ JSON IPC                                 │ HTTPS
         ▼                                          ▼
┌──────────────────┐                    ┌──────────────────────┐
│  Python Sidecar  │                    │  Google Docs API     │
│  (faster-whisper)│                    │                      │
└──────────────────┘                    └──────────────────────┘
```

### プロセス異常終了時の回復シーケンス

#### Pythonサイドカークラッシュ

```
[検知] Rust: ヘルスチェック3回連続失敗
    ↓
[通知] UIに警告表示「音声処理一時停止中...」
    ↓
[回復] 自動再起動試行（最大3回）
    ↓
[成功] 音声キューからの処理再開
[失敗] ユーザーに手動再起動を促す + エラーログ記録
```

#### Tauriアプリクラッシュ

```
[検知] Chrome拡張: WebSocket切断
    ↓
[通知] 拡張ポップアップに「接続断」表示
    ↓
[回復] 指数バックオフで再接続試行（1秒、2秒、4秒、8秒...）
    ↓
[成功] キューイングされたメッセージを再送信
[失敗] 「Tauriアプリを再起動してください」メッセージ表示
```

#### Chrome拡張クラッシュ（タブ/Service Worker再起動）

```
[検知] Tauri: WebSocket接続切断
    ↓
[動作] 音声処理は継続（ローカル保存）
    ↓
[回復] 拡張再接続時に過去10分の履歴を配信
    ↓
[成功] リアルタイム配信再開
```

### プロセス起動順序と依存関係

**正常起動シーケンス**:
1. Tauriアプリ起動
2. Pythonサイドカー起動（10秒タイムアウト）
3. WebSocketサーバー起動（9001-9100ポートスキャン）
4. Chrome拡張接続待機

**依存関係ルール**:
- Tauriは単独で起動可能（Python待機なしモード）
- Pythonが起動しない場合、録音機能は無効化（UI無効表示）
- Chrome拡張は任意タイミングで接続可能（疎結合）

## Static Analysis Infrastructure

### Pre-Commit Hooks Configuration

**目的**: コミット前に自動的にコード品質チェックを実行し、設計原則違反を早期検出する。

**設定ファイル**: `.pre-commit-config.yaml`

**チェック項目**:
- **Forbidden Imports Check**: Python側での音声録音ライブラリ使用を禁止
  - 禁止ライブラリ: `sounddevice`, `pyaudio`, `soundfile`（read_write mode）
  - 理由: [ADR-001: 録音責務の一元化](.kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md) に基づき、音声録音はRust側`AudioDeviceAdapter`のみが担当
  - 実装: `scripts/check_forbidden_imports.py`

**実行タイミング**:
```bash
# 手動実行
pre-commit run --all-files

# Git commit時に自動実行
git commit -m "message"  # pre-commitフックが自動起動
```

### Architecture Decision Records (ADRs)

本プロジェクトでは、重要な技術的意思決定をADR（Architecture Decision Record）として文書化しています。

**作成済みADR**:

**meeting-minutes-core (MVP0)**:
- **[ADR-004: Chrome Extension WebSocket Management](../.kiro/specs/meeting-minutes-core/adrs/ADR-004-chrome-extension-websocket-management.md)**
  - **決定**: Content ScriptでWebSocket接続を管理（Service Worker方式を却下）
  - **理由**: Manifest V3のService Worker 30秒タイムアウト制限回避、タブ単位の状態管理、接続永続性
  - **影響**: WebSocketクライアントはContent Scriptに実装、状態共有はchrome.storage.local経由

- **[ADR-005: State Management Mechanism](../.kiro/specs/meeting-minutes-core/adrs/ADR-005-state-management-mechanism.md)**
  - **決定**: chrome.storage.localを中心とした3層状態管理（Presentation / Bridge / Persistence）
  - **理由**: Popup UIとContent Script間の疎結合、複数タブ状態の一元管理
  - **重要**: ドット記法は使用不可（`'docsSync.syncStatus'`は文字列キーになる）→オブジェクト全体更新
  - **影響**: イミュータブル更新パターンの採用、[chrome-storage-best-practices.md](../../docs/dev/chrome-storage-best-practices.md)の作成

**meeting-minutes-stt (MVP1)**:
- **[ADR-001: Recording Responsibility](../.kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md)**
  - **決定**: 音声録音はRust側`AudioDeviceAdapter`のみが担当し、Pythonサイドカーは録音を行わない
  - **理由**: レース条件防止、リソース競合回避、プロセス境界の明確化
  - **影響**: Python `requirements.txt`から`sounddevice`/`pyaudio`を削除、静的解析で使用を禁止

- **[ADR-002: Model Distribution Strategy](../.kiro/specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md)**
  - **決定**: ハイブリッド配布戦略（初回起動時にオンデマンドダウンロード + システム共有パス利用）
  - **理由**: インストーラサイズ削減（1.5GB→50MB）、複数バージョン共存、ユーザー選択の自由度
  - **影響**: `~/.cache/meeting-minutes/models/`に共有保存、初回起動時にネットワーク必要

- **[ADR-003: IPC Versioning](../.kiro/specs/meeting-minutes-stt/adrs/ADR-003-ipc-versioning.md)**
  - **決定**: セマンティックバージョニング + 後方互換性保証（マイナーバージョンアップは互換性維持）
  - **理由**: Rust/Pythonの独立更新を可能にし、段階的なロールアウトを実現
  - **影響**: メッセージにバージョンフィールド追加、バージョン不一致時のエラーハンドリング実装

**ADR参照の設計原則**:
- [Principle 1: プロセス境界の明確化](.kiro/steering/principles.md#1-プロセス境界の明確化原則) → ADR-001
- [Principle 5: ベンダーロックイン回避](.kiro/steering/principles.md#5-依存関係のベンダーロックイン回避原則) → ADR-002

### Development Phase Status

**現在の開発フェーズ**: MVP2 実装中（2025-10-21〜）

**完了したマイルストーン**:

- ✅ **MVP0 (meeting-minutes-core)**: 2025-10-10 完了
  - Walking Skeleton: Tauri + Python + Chrome拡張の最小疎通確認
  - Fake実装による3プロセスアーキテクチャ検証

- ✅ **MVP1 (meeting-minutes-stt)**: 2025-10-21 完了
  - Real STT: faster-whisper統合、webrtcvad統合
  - リソースベースモデル選択、音声デバイス管理
  - Ring Buffer drop-oldest戦略（2025-01-09追加: バッファオーバーフロー対策）
  - 267/285テスト合格（18件失敗はP2扱い）

**進行中**:

- 🔵 **MVP2 (meeting-minutes-docs-sync)**: Phase 5開始（2025-12-29〜）
  - Phase 0-4: 完了（OAuth 2.0認証、Google Docs API統合、SyncManager統合）
  - Phase 5: UAT実施 → フィードバック回収 → バグ修正/最適化

- 🔵 **meeting-minutes-ci**: 低優先度（requirements生成待ち）
  - GitHub Actions CI/CD、クロスプラットフォームテストマトリックス

**既知の課題（P2）**:
- テスト失敗: Python 17件 + Rust 1件（MVP2 Phase 0で対応検討）

**次のステップ**:
- docs-sync Phase 5 UAT完了
- テスト失敗対応（P2）

---

## Development Environment

### Required Tools

**Core Development**:
- **Node.js**: 18.0.0以降（Chrome拡張ビルド）
- **Rust**: 1.70.0以降（Tauriアプリケーション）
- **Python**: 3.9以降（音声処理エンジン）

**Platform-Specific Audio Dependencies**:
- **macOS**: BlackHole (ループバックオーディオ)
- **Windows**: WASAPI loopback（OS標準）
- **Linux**: PulseAudio/ALSA monitor

**Development Tools**:
```bash
# Rust development
cargo install tauri-cli
cargo install cargo-watch

# Node.js development
npm install -g pnpm
pnpm install

# Python development
pip install -r requirements.txt
pip install -r requirements-dev.txt
```

## Common Commands

### Development Workflow

**Tauriアプリケーション開発**:
```bash
# 開発モード起動
cargo tauri dev

# ビルド
cargo tauri build

# テスト実行
cargo test
```

**Chrome拡張開発**:
```bash
# 拡張ビルド
pnpm build:extension

# 開発モード（ウォッチ）
pnpm dev:extension

# テスト実行
pnpm test:extension
```

**Pythonサイドカー開発**:
```bash
# STTエンジン単体テスト
python -m pytest tests/test_stt.py

# VAD性能テスト
python -m pytest tests/test_vad.py

# パフォーマンステスト
python scripts/benchmark_audio.py
```

**統合テスト**:
```bash
# E2Eテスト実行
pnpm test:e2e

# パフォーマンステスト
pnpm test:performance
```

## Environment Variables

### Development Configuration

```env
# WebSocket通信設定
WEBSOCKET_PORT=9001
WEBSOCKET_HOST=localhost

# ログレベル設定
RUST_LOG=debug
PYTHON_LOG_LEVEL=INFO

# 音声処理設定
AUDIO_SAMPLE_RATE=16000
AUDIO_CHUNK_SIZE=320
VAD_AGGRESSIVENESS=2

# STTモデル設定
WHISPER_MODEL_SIZE=small
WHISPER_DEVICE=cpu
WHISPER_COMPUTE_TYPE=int8

# Google Docs API設定
GOOGLE_CLIENT_ID=your_client_id
GOOGLE_CLIENT_SECRET=your_client_secret

# 開発環境フラグ
TAURI_DEBUG=true
CHROME_EXTENSION_DEV=true
```

### Production Configuration

```env
# WebSocket通信設定
WEBSOCKET_PORT=9001
WEBSOCKET_HOST=127.0.0.1

# ログレベル設定
RUST_LOG=info
PYTHON_LOG_LEVEL=WARNING

# パフォーマンス最適化
WHISPER_MODEL_SIZE=base
WHISPER_DEVICE=cpu
WHISPER_COMPUTE_TYPE=int8

# セキュリティ設定
TAURI_DEBUG=false
CHROME_EXTENSION_DEV=false
```

## Port Configuration

### Standard Port Assignments

- **WebSocket Server**: 9001 (デフォルト)
- **Tauri Dev Server**: 1420 (開発時のみ)
- **Chrome Extension Dev**: 3000 (開発時のみ)

### Port Conflict Resolution

```rust
// 動的ポート割り当て（Rust側）
async fn find_available_port(start: u16) -> u16 {
    for port in start..start + 100 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    start // フォールバック
}
```

## External Dependencies

### Critical Dependencies

**Google APIs**:
- **Google Docs API v1**: ドキュメント操作
- **OAuth 2.0**: 認証フロー
- **Rate Limits**: 100 requests/100 seconds/user

**Audio Models**:
- **faster-whisper models**: HuggingFace Hubから自動ダウンロード
- **Model Sizes**: tiny (39MB) → large (1550MB)
- **Storage**: ~/.cache/huggingface/

**System Dependencies**:
- **音声ドライバ**: OS固有の音声システムアクセス
- **Chrome Browser**: 116以降（Manifest V3対応）

### Dependency Management

**Rust Dependencies**: Cargo.toml
```toml
[dependencies]
tauri = { version = "2.0", features = ["api-all"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
rusqlite = { version = "0.29", features = ["bundled"] }
tokio-tungstenite = "0.20"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Node.js Dependencies**: package.json
```json
{
  "dependencies": {
    "react": "^18.2.0",
    "typescript": "^5.0.0",
    "@types/chrome": "^0.0.245"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "vite": "^4.0.0",
    "vitest": "^0.34.0"
  }
}
```

## Performance Considerations

### Target Metrics

- **音声処理遅延**: 部分テキスト 0.5秒以内、確定テキスト 2秒以内
- **メモリ使用量**: 2時間録音で最大2GB
- **CPU使用率**: 継続的に50%以下
- **バッテリー消費**: ネイティブアプリレベルの効率性

### Optimization Strategies

**Rust最適化**:
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

**Python最適化**:
- **モデルキャッシュ**: 起動時ロードと常駐
- **バッファプール**: メモリアロケーション最適化
- **並列処理**: asyncio活用

**WebSocket最適化**:
- **メッセージ圧縮**: gzip/deflate適用
- **バックプレッシャー**: キュー制御
- **Keep-Alive**: 20秒間隔でのピング

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

**状態管理**: Zustand
- **理由**: 軽量性とTauriアプリの要件に最適

**UI Library**: shadcn/ui + Tailwind CSS
- **理由**: 一貫したデザインシステムとカスタマイズ性

### Chrome拡張

**Manifest Version**: V3 (必須)
```json
{
  "manifest_version": 3,
  "minimum_chrome_version": "116"
}
```

**Frontend Framework**: React + TypeScript
- **Popup**: 拡張のメインUI
- **Content Script**: Google Docsページ操作
- **Service Worker**: バックグラウンド処理とWebSocket管理

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

**Database**: SQLite with rusqlite
```toml
rusqlite = { version = "0.29", features = ["bundled"] }
```

**音声処理インターフェース**: Rust subprocess → Python (stdin/stdout JSON IPC)

### Python Sidecar Lifecycle Management

**プロセス起動とライフサイクル**:

Pythonサイドカープロセスは、Tauriアプリケーションの起動時に自動的に開始され、アプリ終了時に適切にクリーンアップされます。

#### 起動シーケンス

```rust
// Rust側（Tauri）
pub struct PythonSidecarManager {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl PythonSidecarManager {
    pub async fn start() -> Result<Self> {
        // 1. Pythonインタープリタのパス検出
        let python_path = detect_python_executable()?;

        // 2. サイドカースクリプトのパス解決（統一されたエントリーポイント）
        let script_path = resolve_sidecar_path("python-stt/main.py")?;

        // 3. プロセス起動
        let mut process = Command::new(python_path)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

        // 4. 初期化完了待機（タイムアウト10秒）
        let manager = Self { process, stdin, stdout };
        manager.wait_for_ready(Duration::from_secs(10)).await?;

        Ok(manager)
    }

    async fn wait_for_ready(&self, timeout: Duration) -> Result<()> {
        // "ready"メッセージの受信待機
    }
}
```

#### IPC通信プロトコル (stdin/stdout JSON)

**メッセージフォーマット**:
```json
{
  "id": "unique-message-id",
  "type": "request|response|event|error",
  "method": "transcribe|configure|health_check",
  "params": { ... },
  "timestamp": 1234567890
}
```

**通信フロー**:
1. **Rust → Python (Request)**: 音声データとメタデータをJSON + Base64でstdinに送信
2. **Python → Rust (Response)**: 文字起こし結果をJSON形式でstdoutに出力
3. **Python → Rust (Event)**: 部分結果や進捗通知を非同期イベントとして送信

**実装例**:
```rust
// Rust側: 音声データ送信
pub async fn send_audio_chunk(&mut self, chunk: &AudioChunk) -> Result<()> {
    let message = json!({
        "id": Uuid::new_v4().to_string(),
        "type": "request",
        "method": "transcribe",
        "params": {
            "audio_data": base64::encode(&chunk.data),
            "sample_rate": chunk.sample_rate,
            "is_final": chunk.is_final,
        },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    });

    writeln!(self.stdin, "{}", message.to_string())?;
    self.stdin.flush()?;
    Ok(())
}

// Rust側: 応答受信
pub async fn receive_response(&mut self) -> Result<TranscriptionResponse> {
    let mut line = String::new();
    self.stdout.read_line(&mut line)?;
    let response: TranscriptionResponse = serde_json::from_str(&line)?;
    Ok(response)
}
```

```python
# Python側: メインループ
async def main():
    await send_ready_signal()

    while True:
        try:
            # stdinから1行読み込み
            line = await asyncio.get_event_loop().run_in_executor(
                None, sys.stdin.readline
            )

            if not line:
                break

            message = json.loads(line)

            # メソッドディスパッチ
            if message["method"] == "transcribe":
                result = await handle_transcribe(message["params"])
            elif message["method"] == "health_check":
                result = {"status": "healthy"}

            # 応答送信
            response = {
                "id": message["id"],
                "type": "response",
                "result": result,
                "timestamp": time.time(),
            }
            print(json.dumps(response), flush=True)

        except Exception as e:
            error_response = {
                "id": message.get("id", "unknown"),
                "type": "error",
                "error": str(e),
            }
            print(json.dumps(error_response), flush=True)
```

#### ヘルスチェック機構

**定期的なヘルスチェック**:
- Rust側から5秒ごとに`health_check`リクエストを送信
- Python側が3秒以内に応答しない場合、プロセス異常と判断
- 3回連続失敗でプロセス再起動を試行

```rust
pub async fn health_check_loop(&mut self) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut failure_count = 0;

    loop {
        interval.tick().await;

        match self.send_health_check().await {
            Ok(_) => failure_count = 0,
            Err(_) => {
                failure_count += 1;
                if failure_count >= 3 {
                    log::error!("Python sidecar health check failed 3 times, restarting...");
                    self.restart().await?;
                    failure_count = 0;
                }
            }
        }
    }
}
```

#### クラッシュ時の自動再起動

**再起動ポリシー**:
- 初回失敗: 即座に再起動
- 2回目失敗: 5秒待機後に再起動
- 3回目失敗: 30秒待機後に再起動
- 4回目以降: ユーザー通知と手動再起動を促す

```rust
pub async fn restart(&mut self) -> Result<()> {
    // 1. 既存プロセスの終了
    self.shutdown().await?;

    // 2. 再起動試行
    *self = Self::start().await?;

    Ok(())
}

pub async fn shutdown(&mut self) -> Result<()> {
    // Graceful shutdown
    let _ = self.send_shutdown_signal().await;

    // 3秒待機
    tokio::time::sleep(Duration::from_secs(3)).await;

    // まだ生きている場合は強制終了
    if let Ok(None) = self.process.try_wait() {
        self.process.kill()?;
    }

    Ok(())
}
```

#### ゾンビプロセス防止

**プロセス監視とクリーンアップ**:
- Tauriアプリ終了時の`Drop` traitでの確実なクリーンアップ
- シグナルハンドラ（SIGTERM, SIGINT）での適切な終了処理
- プロセスIDの記録とOS再起動後の孤児プロセス検出

```rust
impl Drop for PythonSidecarManager {
    fn drop(&mut self) {
        // 同期的なクリーンアップ
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
```

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

**作成済みADR**（meeting-minutes-stt spec）:

- **[ADR-001: Recording Responsibility](.kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md)**
  - **決定**: 音声録音はRust側`AudioDeviceAdapter`のみが担当し、Pythonサイドカーは録音を行わない
  - **理由**: レース条件防止、リソース競合回避、プロセス境界の明確化
  - **影響**: Python `requirements.txt`から`sounddevice`/`pyaudio`を削除、静的解析で使用を禁止

- **[ADR-002: Model Distribution Strategy](.kiro/specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md)**
  - **決定**: ハイブリッド配布戦略（初回起動時にオンデマンドダウンロード + システム共有パス利用）
  - **理由**: インストーラサイズ削減（1.5GB→50MB）、複数バージョン共存、ユーザー選択の自由度
  - **影響**: `~/.cache/meeting-minutes/models/`に共有保存、初回起動時にネットワーク必要

- **[ADR-003: IPC Versioning](.kiro/specs/meeting-minutes-stt/adrs/ADR-003-ipc-versioning.md)**
  - **決定**: セマンティックバージョニング + 後方互換性保証（マイナーバージョンアップは互換性維持）
  - **理由**: Rust/Pythonの独立更新を可能にし、段階的なロールアウトを実現
  - **影響**: メッセージにバージョンフィールド追加、バージョン不一致時のエラーハンドリング実装

**ADR参照の設計原則**:
- [Principle 1: プロセス境界の明確化](.kiro/steering/principles.md#1-プロセス境界の明確化原則) → ADR-001
- [Principle 5: ベンダーロックイン回避](.kiro/steering/principles.md#5-依存関係のベンダーロックイン回避原則) → ADR-002

### Development Phase Status

**現在の開発フェーズ**: 仕様検証完了・実装準備中（Specification Phase）

**完了した活動**:
- ✅ 3プロセスアーキテクチャの詳細設計完了
- ✅ IPC通信プロトコル（stdin/stdout JSON）の確定
- ✅ WebSocketメッセージ型設計（Tagged Union）
- ✅ Pythonサイドカーライフサイクル管理仕様の策定
- ✅ 静的解析基盤の整備（pre-commit hooks、forbidden imports check）
- ✅ 主要な技術的意思決定のADR文書化

**未開始**:
- 🔵 実装コードベース（`src-tauri/`, `src/`, `chrome-extension/`, `python-stt/`）
- 🔵 依存関係ファイル（`Cargo.toml`, `package.json`, `requirements.txt`）
- 🔵 ビルド設定（`tauri.conf.json`, `vite.config.ts`）

**次のステップ**: meeting-minutes-core (MVP0) Walking Skeleton実装開始

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
# Meeting Minutes Automator

Google Meetの音声を自動で文字起こしし、議事録を生成するデスクトップアプリケーション。

## 🎯 Project Status

**Current Phase**: MVP1 Core Implementation Milestone ✅（2025-10-19時点）

### 完了した機能（MVP1 Core Implementation）

**基盤機能**（MVP0）:
- ✅ Tauri + Python + Chrome拡張の3プロセスアーキテクチャ
- ✅ Pythonサイドカープロセス管理（起動/終了/ヘルスチェック）
- ✅ JSON IPC通信（Rust ↔ Python、Line-Delimited JSON）
- ✅ WebSocketサーバー（Rust ↔ Chrome拡張、ポート9001）
- ✅ Chrome拡張スケルトン（Google Meetページで動作）

**STT機能**（MVP1）:
- ✅ **faster-whisper統合**: リアルタイム音声認識（tiny/base/small/medium/large-v3）
- ✅ **VAD統合**: webrtcvadによる音声活動検出（speech_start/speech_end）
- ✅ **部分テキスト/確定テキスト**: `isPartial`フラグ付き配信（<0.5s/<2s応答）
- ✅ **オフラインフォールバック**: HuggingFace Hub接続失敗時にbundled baseモデルへ自動切替
- ✅ **リソース監視**: CPU/メモリ使用率に応じた動的モデルダウングレード/アップグレード提案
- ✅ **音声デバイス管理**: CoreAudio/WASAPI/ALSA対応、デバイス切断検出・自動再接続
- ✅ **ローカルストレージ**: セッション別音声/文字起こし保存（audio.wav, transcription.jsonl, session.json）
- ✅ **UI拡張**: 音声デバイス選択、Whisperモデル選択、リソース情報表示

**テスト・品質保証**:
- ✅ Rust: 71テスト合格（E2Eテスト含む）
- ✅ Python: 143テスト合格（単体・統合テスト）
- ✅ E2Eテスト: Task 10.1緑化（VAD→STT完全フロー、23.49秒実行）
- ✅ 性能テスト: Task 10.6完了（IPC/Audio callback latency測定、全項目合格）
- ✅ 後方互換性テスト: Task 10.7完了（IPC 26テスト、WebSocket 6テスト、全合格）
- ✅ セキュリティテスト: Task 11.5完了（検証完了、修正はMVP2 Phase 0）

**性能指標** (2025-10-19測定、ADR-017基準):

| 項目 | 目標 | 実測値 | 合否 |
|------|------|--------|------|
| 部分テキストレイテンシ (初回) | <3000ms | 1830ms | ✅ PASS |
| 確定テキストレイテンシ | <2000ms | 1623ms | ✅ PASS |
| IPC latency (平均) | <5ms | 0.409ms | ✅ PASS |
| IPC latency (最大) | <5ms | 1.904ms | ✅ PASS |
| Audio callback (P99) | <10μs | 2.125μs | ✅ PASS |
| Audio callback (平均) | <10μs | 0.356μs | ✅ PASS |

詳細: [Task 10.6](.kiro/specs/meeting-minutes-stt/tasks/phase-13-verification.md#task-10-6)

**後方互換性テスト** (2025-10-19測定):

| カテゴリ | テスト数 | 合格 | カバレッジ要件 |
|----------|----------|------|----------------|
| IPC Protocol | 26 | 26 | STT-REQ-007.1-007.6, ADR-003 |
| WebSocket Extension | 6 | 6 | STT-REQ-008.1-008.3 |

詳細: [Task 10.7](.kiro/specs/meeting-minutes-stt/tasks/phase-13-verification.md#task-10-7)

**ドキュメント**:
- ✅ UML図5種類（コンポーネント、シーケンス×3、クラス）
- ✅ ADR 7件実装完了（ADR-001, 002, 003, 013, 014, 016, 017）
- ✅ MVP2申し送りドキュメント（検証負債・リスク宣言付き）

### 検証負債（MVP2 Phase 0で対応）

⚠️ **以下の検証が未完了です**（詳細は`.kiro/specs/meeting-minutes-stt/MVP2-HANDOFF.md`参照）:

- **Task 10.2-10.7**: Rust E2Eテスト未実装（Python単体テストは完了）
- **Task 11.3**: 長時間稼働安定性テスト（2時間録音）
- **SEC-001〜005**: セキュリティ修正5件（pip脆弱性、CSP設定、ファイル権限、TLS検証、cargo-audit）

### 次のフェーズ
- 📋 **MVP2 Phase 0**: 検証負債解消（Task 10.2-10.7、Task 11.3、SEC-001〜005）
- 📋 **MVP2**: Google Docs同期（OAuth 2.0、Named Range管理、オフライン同期）
- 📋 **MVP3**: LLM要約 + UI（プロダクション準備）

---

## 📚 ドキュメント

- **[ユーザーガイド](docs/user-guide.md)**: インストール、音声デバイス設定、faster-whisperモデル設定、トラブルシューティング
- **[アーキテクチャ図](docs/diagrams/)**: UML図5種類（コンポーネント、シーケンス×3、クラス）
- **[ADR実装レビュー](.kiro/specs/meeting-minutes-stt/adr-implementation-review.md)**: ADR-001〜017実装状況確認
- **[セキュリティテストレポート](.kiro/specs/meeting-minutes-stt/security-test-report.md)**: SEC-001〜005詳細
- **[MVP2申し送り](.kiro/specs/meeting-minutes-stt/MVP2-HANDOFF.md)**: MVP2 Phase 0ブロッカー、リスク宣言

## 📚 Architecture Decision Records
- ADR履歴の俯瞰: `.kiro/specs/meeting-minutes-stt/adrs/ADR-history.md`
- 最新IPC設計: `.kiro/specs/meeting-minutes-stt/adrs/ADR-013-sidecar-fullدuplex-final-design.md`  
  - 実装状況: ADR-013は2025-10-14に承認済み。stdin/stdout分離とバックプレッシャ制御の実装タスクはMVP1のSTT統合作業で追跡予定です。
- フォローアップ修正: `.kiro/specs/meeting-minutes-stt/adrs/ADR-013-P0-bug-fixes.md`

## 🏗️ Architecture

```
┌─────────────────┐       WebSocket        ┌──────────────────┐
│  Tauri App      │◄─────────────────────►│ Chrome Extension │
│  (Rust + React) │      (port 9001)       │  (Content Script)│
└────────┬────────┘                        └──────────────────┘
         │                                   Google Meet Page
         │ stdin/stdout
         │ JSON IPC
         │
    ┌────▼──────┐
    │  Python   │
    │  Sidecar  │
    │  (STT)    │
    └───────────┘
```

### コンポーネント

- **Tauri App** (Rust + React): メインアプリケーション、Pythonサイドカー/音声デバイス管理、WebSocketサーバー
- **Python Sidecar**: webrtcvad + faster-whisper によるリアルタイム文字起こしとモデル監視
- **Chrome Extension**: Google Meetページでの音声取得、部分/確定文字起こしの表示と状態保持

## 🚀 Quick Start

### 前提条件

- **Node.js**: 18.x以上
- **Rust**: 1.70以上（推奨: 1.85以上）
- **Python**: 3.9-3.12（64bit）
- **Chrome**: 最新版

### セットアップ

1. **リポジトリのクローン**
```bash
git clone https://github.com/yourusername/meeting-minutes-automator.git
cd meeting-minutes-automator
```

2. **依存関係のインストール**
```bash
# Node.js依存関係
npm install

# Rust依存関係（自動）
cd src-tauri
cargo build
cd ..
```

3. **Pythonスクリプトの確認**
```bash
# Python 3.9-3.12が利用可能か確認
python3 --version

# python-stt/main.pyが存在することを確認
ls python-stt/main.py
```

4. **Python仮想環境（必ず `.venv` を使用）**
```bash
cd python-stt
python3 -m venv .venv
source .venv/bin/activate  # Windowsは .venv\Scripts\activate
pip install -r requirements.txt      # faster-whisper / webrtcvad / numpy など本番依存
pip install -r requirements-dev.txt  # pytest など開発用依存
cd ..
```
> Rust側の `.cargo/config.toml` は `python-stt/.venv/bin/python` を指しています。  
> フォルダ名を `venv` などに変えるとテストが失敗するので、必ず `.venv` を使ってください。
> 初回の `WhisperSTTEngine` 利用時に Hugging Face からモデル（既定: `small`）のダウンロードが発生します。オフライン環境では事前にキャッシュを用意してください。

### 開発モードでの起動

```bash
npm run tauri dev
```

以下のログが表示されれば起動成功：
```
[Meeting Minutes] ✅ Python sidecar started
[Meeting Minutes] ✅ Python sidecar ready
[Meeting Minutes] ✅ FakeAudioDevice initialized
[Meeting Minutes] ✅ WebSocket server started on port 9001
```

Tauri UIウィンドウが自動で開きます（http://localhost:1420/）

## 🧪 E2Eテスト手順

MVP0で確立した疎通に加え、MVP1のリアルSTTストリームを検証するための手順です。

### 1. Chrome拡張の読み込み

1. Chromeで `chrome://extensions/` を開く
2. 右上の「デベロッパーモード」を有効化
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. `chrome-extension/` フォルダを選択
5. 「Meeting Minutes Automator」が有効になっていることを確認

### 2. Google Meetへアクセス

1. https://meet.google.com にアクセス（新しい会議を作成）
2. Chrome DevToolsを開く（F12キー）
3. Consoleタブを選択
4. 以下のログが表示されることを確認：
```
[Meeting Minutes] Content script loaded on Google Meet
[Meeting Minutes] ✅ Connected to WebSocket server on port 9001
[Meeting Minutes] 📦 Storage saved: {connectionStatus: 'connected', ...}
[Meeting Minutes] ✅ Connection established - Session: [UUID]
```

### 3. 録音開始テスト

1. **Tauri UIウィンドウ**で「Start Recording」ボタンをクリック
2. **Tauriコンソール**で以下のイベントログを確認（無音の場合は `🤫 No speech detected` が出力されます）
3. **Chrome DevTools Console**で WebSocket メッセージを確認  
   ```
   [Meeting Minutes] Received message: {type: 'transcription', text: '', isPartial: false, ...}
   ```
   `FakeAudioDevice` は無音データを生成するため、テキストは空文字列になります。これはハンドシェイク確認用の期待挙動です。

### 4. 録音停止テスト

1. **Tauri UIウィンドウ**で「Stop Recording」ボタンをクリック
2. **Chrome DevTools Console**でログ出力が停止することを確認

### 5. 実音声ストリームの検証（任意）

リアルSTTパイプラインと部分結果配信を確認するには以下のいずれかを実行してください。

- **Rust統合テスト**: `cd src-tauri && cargo test --test stt_e2e_test -- --nocapture`  
  Whisperモデルがダウンロードされ、`test_audio_short.wav` を用いた `partial_text` / `final_text` イベントを確認できます。
- **Python統合テスト**: `cd python-stt && .venv/bin/python -m pytest tests/test_audio_integration.py -k process_audio_stream -vv`  
  `process_audio_stream` ハンドラが `speech_start → partial_text → final_text → speech_end` を送出することを検証します。
- **手動検証**: `src-tauri/tests/fixtures/test_audio_short.wav` を再生しながら実マイクを `AudioDeviceAdapter` に接続する（UI統合が完了したブランチで有効）。

### 期待される動作

```
Tauri UI「Start Recording」
    ↓
FakeAudioDevice（既定）または AudioDeviceAdapter（実装中）が音声フレームを生成
    ↓
Rust → Python IPC: process_audio_stream リクエスト送信
    ↓
Python AudioPipeline: VAD → Whisper 推論 → 部分/確定テキスト生成
    ↓
Rust WebSocket: transcription イベント（isPartial / confidence / language / processingTimeMs 付き）を配信
    ↓
Chrome Extension: コンソールと `chrome.storage.local` にストリームを反映
```

## 🔧 トラブルシューティング

### Python sidecarが起動しない

**エラー**: `[Meeting Minutes] ❌ Failed to start Python sidecar`

**解決策**:
1. Python 3.9-3.12（64bit）がインストールされているか確認
```bash
python3 --version
python3 -c "import platform; print(platform.architecture())"
```

2. 環境変数でPythonパスを指定（オプション）
```bash
export APP_PYTHON=/path/to/python3.11
npm run tauri dev
```

### WebSocketサーバーが起動しない

**エラー**: `[Meeting Minutes] ❌ Failed to start WebSocket server`

**解決策**:
1. ポート9001-9100が他のプロセスで使用されていないか確認
```bash
lsof -i :9001-9100  # macOS/Linux
netstat -ano | findstr "9001"  # Windows
```

2. ファイアウォール設定でローカルホスト接続を許可

### Chrome拡張が接続できない

**エラー**: `WebSocket connection failed`

**解決策**:
1. Tauri appが起動していることを確認
2. WebSocketサーバーのログを確認（`port 9001` など）
3. Chrome拡張を再読み込み（`chrome://extensions/` → 🔄ボタン）
4. Google Meetページをリロード（F5）

### Rust toolchainのバージョンが古い

**エラー**: `feature 'edition2024' is required`

**解決策**:
```bash
# Rustをアップデート
rustup update stable

# または、npm経由でTauriを実行（推奨）
npm run tauri dev  # ✅ これで動作します
```

## 📁 Project Structure

```
meeting-minutes-automator/
├── src/                      # React frontend
│   ├── App.tsx              # メインUI（録音ボタン）
│   └── main.tsx
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── audio.rs                 # FakeAudioDevice（デフォルト開発用）
│   │   ├── audio_device_adapter.rs  # CoreAudio / WASAPI / ALSA 実装
│   │   ├── commands.rs              # IPCイベントストリーム → WebSocket配信
│   │   ├── ipc_protocol.rs          # プロトコル定義
│   │   ├── python_sidecar.rs        # サイドカープロセス管理
│   │   ├── websocket.rs             # WebSocketサーバー
│   │   └── state.rs                 # アプリケーション状態
│   ├── tests/                      # stt_e2e_test / audio_ipc_integration など
│   └── Cargo.toml
├── python-stt/                      # Python音声処理
│   ├── main.py                      # AudioProcessor（VAD→Whisper→IPC）
│   ├── stt_engine/
│   │   ├── audio_pipeline.py
│   │   ├── transcription/           # whisper_client / voice_activity_detector
│   │   ├── resource_monitor.py
│   │   └── ipc_handler.py
│   └── tests/                       # pytestベースの統合・単体テスト
├── chrome-extension/        # Chrome拡張
│   ├── manifest.json        # Manifest V3
│   ├── content-script.js    # WebSocketクライアント
│   └── service-worker.js    # バックグラウンド処理
└── .kiro/                   # 仕様ドキュメント
    ├── steering/            # プロジェクト指針
    └── specs/               # 機能仕様
        ├── meeting-minutes-automator/  # Umbrella spec
        ├── meeting-minutes-core/       # MVP0 (完成✅)
        ├── meeting-minutes-stt/        # MVP1 (予定)
        ├── meeting-minutes-docs-sync/  # MVP2 (予定)
        └── meeting-minutes-llm/        # MVP3 (予定)
```

## 📚 Documentation

- **仕様**: `.kiro/specs/meeting-minutes-core/`
  - `requirements.md`: 要件定義
  - `design.md`: 設計詳細
  - `tasks.md`: 実装タスク
- **開発ガイド**: `docs/dev/`
  - `coding-standards.md`: コーディング規約
  - `spec-authoring.md`: 仕様作成手順

## 🧑‍💻 Development

### コマンド

```bash
# 開発モード（ホットリロード）
npm run tauri dev

# プロダクションビルド
npm run tauri build

# Rustテスト
cd src-tauri
cargo test

# Pythonテスト
cd python-stt
pytest
```

### デバッグ

- **Rust**: `println!`, `dbg!`, `RUST_LOG=debug cargo run`
- **Python**: `print()` → Rust側のstdoutに出力
- **Chrome Extension**: DevTools Console（F12）

## 🤝 Contributing

本プロジェクトはKiro仕様駆動開発プロセスを採用しています。

1. `.kiro/specs/` で仕様を確認
2. 要件・設計・タスクに従って実装
3. TDDサイクル（ユニットテスト→実装→統合テスト）
4. コミットメッセージに要件IDを含める

## 📄 License

TBD

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) - クロスプラットフォームデスクトップアプリフレームワーク
- [faster-whisper](https://github.com/guillaumekln/faster-whisper) - 音声認識（MVP1で統合予定）
- [Google Meet](https://meet.google.com/) - オンライン会議プラットフォーム

# Meeting Minutes Automator

Google Meetの音声を自動で文字起こしし、議事録を生成するデスクトップアプリケーション。

## 🎯 Project Status

**Current Phase**: Walking Skeleton (MVP0) ✅ **完成**

全コンポーネント間のE2E疎通確認が完了し、後続MVP（STT、Docs同期、LLM要約）の実装基盤が確立されました。

### 完成した機能（MVP0）
- ✅ Tauri + Python + Chrome拡張の3プロセスアーキテクチャ
- ✅ Fake音声録音（100ms間隔でダミーデータ生成）
- ✅ Pythonサイドカープロセス管理（起動/終了/ヘルスチェック）
- ✅ JSON IPC通信（Rust ↔ Python）
- ✅ WebSocketサーバー（Rust ↔ Chrome拡張）
- ✅ Chrome拡張スケルトン（Google Meetページで動作）
- ✅ E2E疎通確認（録音→処理→配信→表示）

### 次のフェーズ
- 📋 MVP1: Real STT（faster-whisper統合、音声デバイス管理）
- 📋 MVP2: Google Docs同期（OAuth 2.0、Named Range管理）
- 📋 MVP3: LLM要約 + UI（プロダクション準備）

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

- **Tauri App** (Rust + React): メインアプリケーション、プロセス管理、WebSocketサーバー
- **Python Sidecar**: 音声処理（MVP0ではFake実装）
- **Chrome Extension**: Google Meetページでの音声取得、文字起こし結果表示

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

Walking Skeletonの全フローを手動で検証します。

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
2. **Chrome DevTools Console**（Google Meetのタブ）で、100ms間隔で以下が表示されることを確認：
```
[Meeting Minutes] Received message: {type: 'transcription', ...}
[Meeting Minutes] 📝 Transcription: This is a fake transcription result
```

### 4. 録音停止テスト

1. **Tauri UIウィンドウ**で「Stop Recording」ボタンをクリック
2. **Chrome DevTools Console**でログ出力が停止することを確認

### 期待される動作

```
Tauri UI「Start Recording」クリック
    ↓
FakeAudioDevice: 100ms間隔で16バイトダミーデータ生成
    ↓
Rust → Python IPC: process_audioメッセージ送信
    ↓
Python: "This is a fake transcription result" 返信
    ↓
Rust → Chrome WebSocket: transcriptionメッセージ配信
    ↓
Chrome Extension: コンソールに表示
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
│   │   ├── audio.rs         # FakeAudioDevice
│   │   ├── python_sidecar.rs # Pythonプロセス管理
│   │   ├── websocket.rs     # WebSocketサーバー
│   │   ├── commands.rs      # Tauriコマンド
│   │   ├── state.rs         # アプリケーション状態
│   │   └── lib.rs           # メインエントリーポイント
│   └── Cargo.toml
├── python-stt/              # Python音声処理
│   └── main.py              # IPC handler + Fake processor
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

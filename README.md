# Meeting Minutes Automator

Google Meetの音声をリアルタイムで文字起こしし、Google Docsに自動保存するデスクトップアプリ。

## Features

- **リアルタイム文字起こし** - faster-whisper + VADで高精度な音声認識
- **Google Docs同期** - 文字起こし結果を2秒以内に自動保存
- **オフライン対応** - ネットワーク切断時もキューに保存、復帰後に自動同期
- **マルチプラットフォーム** - macOS / Windows / Linux対応

## Architecture

```
┌──────────────────┐     WebSocket      ┌──────────────────┐
│   Tauri App      │◄──────────────────►│ Chrome Extension │
│  (Rust + React)  │    port 9001       │   (Manifest V3)  │
└────────┬─────────┘                    └────────┬─────────┘
         │ stdin/stdout                          │
         │ JSON IPC                              │ HTTPS
         ▼                                       ▼
┌──────────────────┐                    ┌──────────────────┐
│  Python Sidecar  │                    │   Google APIs    │
│  (STT Engine)    │                    │  OAuth + Docs    │
└──────────────────┘                    └──────────────────┘
```

## Quick Start

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Python 3.9-3.12
- Chrome

### Setup

```bash
# Clone
git clone https://github.com/anthropics/meeting-minutes-automator.git
cd meeting-minutes-automator

# Install dependencies
npm install

# Setup Python
cd python-stt
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
cd ..

# Run
npm run tauri dev
```

### Chrome Extension

1. `chrome://extensions/` を開く
2. デベロッパーモードを有効化
3. 「パッケージ化されていない拡張機能を読み込む」→ `chrome-extension/dist/` を選択

## Project Status

| Phase | Description | Status |
|-------|-------------|--------|
| MVP0 | Walking Skeleton | ✅ Complete |
| MVP1 | Real-time STT | ✅ Complete |
| MVP2 | Google Docs Sync | ✅ Complete |
| MVP3 | LLM Summarization | 📋 Planned |

### Test Coverage

```
Rust:     71 tests
Python:   143 tests
Chrome:   148 tests (unit) + 60 tests (E2E)
```

## Documentation

| Document | Description |
|----------|-------------|
| [User Guide](docs/user/google-docs-sync-guide.md) | Google Docs同期の使い方 |
| [Developer Guide](docs/dev/google-docs-api-integration.md) | API仕様、セットアップ手順 |
| [UAT Plan](docs/test/uat-plan.md) | ユーザー受け入れテスト計画 |
| [Release Notes](docs/release/RELEASE_NOTES_v0.2.0.md) | v0.2.0リリースノート |

## Development

```bash
# Dev mode
npm run tauri dev

# Build
npm run tauri build

# Test
cd src-tauri && cargo test
cd python-stt && pytest
cd chrome-extension && npm test
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop | Tauri 2.0 (Rust + React) |
| STT | faster-whisper + webrtcvad |
| Extension | Chrome MV3 + Playwright |
| API | Google Docs API v1 |

## License

TBD

---

Built with [Tauri](https://tauri.app/) and [Claude Code](https://claude.ai/code)

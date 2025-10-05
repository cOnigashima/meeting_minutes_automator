# Technology Stack

## Architecture
**ハイブリッドアーキテクチャ**: Tauriデスクトップアプリケーション + Chrome拡張の連携システム

## 3プロセス構成
1. **Tauri App (Rust)**: コア処理とプロセス調整
2. **Python Sidecar**: 音声処理専用プロセス（stdin/stdout JSON IPC）
3. **Chrome Extension**: ブラウザUI and Google Docs統合

## Core Technologies

### Tauri Application (Rust)
- **Framework**: Tauri 2.0 (Raw Payloads対応)
- **Runtime**: tokio (async/await)
- **Database**: SQLite with rusqlite
- **WebSocket**: tokio-tungstenite
- **重要**: `api-all`は禁止、必要最小限のfeaturesのみ使用

### Frontend (React + TypeScript)
- **Framework**: React 18+ with TypeScript 5.0+
- **State Management**: Zustand
- **UI Library**: shadcn/ui + Tailwind CSS
- **Build**: Vite

### Chrome Extension
- **Manifest Version**: V3 (必須)
- **Minimum Chrome Version**: 116
- **Frontend**: React + TypeScript
- **Communication**: WebSocket with Tauri App

### Python Sidecar (Audio Processing)
- **STT Engine**: faster-whisper (CTranslate2最適化版) >=0.10.0
- **VAD**: webrtcvad >=2.0.0
- **Audio Processing**: numpy >=1.24.0
- **重要**: 音声録音はRust側のみ。Python側は録音禁止（sounddevice, pyaudioは使用不可）

## Communication Architecture
- **Tauri ↔ Python**: stdin/stdout JSON IPC
- **Tauri ↔ Chrome Extension**: WebSocket (Port 9001-9100 range)
- **Chrome Extension ↔ Google Docs**: HTTPS REST API

## Development Tools
- **Node.js**: 18.0.0以降
- **Rust**: 1.70.0以降
- **Python**: 3.9以降
- **Package Managers**: pnpm (Node.js), cargo (Rust), pip (Python)

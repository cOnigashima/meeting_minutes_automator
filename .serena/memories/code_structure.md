# Codebase Structure

## Current State (Implementation Phase - MVP1)

### Completed Features
- **MVP0 (meeting-minutes-core)**: Walking Skeleton完成（2025-10-06完了）
  - 44テスト合格（unit 31, integration 5, e2e 8）
  - Fake実装による疎通確認完了
  - Tauri + Python + Chrome拡張の3プロセス連携確認

### In Progress
- **MVP1 (meeting-minutes-stt)**: Real STT実装中（タスク2.4完了 / 2025-10-10）
  - faster-whisper統合（計画中）
  - 音声デバイス管理（CoreAudio/WASAPI/ALSA実装済み）
  - ループバックデバイス対応（3プラットフォーム完了）
  - リソースベースモデル選択（計画中）

### Existing Codebase Structure

#### Rust / Tauri Application (src-tauri/src/)
```
src-tauri/src/
├── main.rs                  # Tauriエントリーポイント
├── lib.rs                   # ライブラリルート
├── audio.rs                 # 音声キャプチャ（FakeAudioDevice実装済み）
├── audio_device_adapter.rs  # OS固有音声API抽象化（CoreAudio/WASAPI/ALSA）
├── websocket.rs             # WebSocketサーバー（Chrome拡張連携）
├── python_sidecar.rs        # Pythonサイドカー管理
├── commands.rs              # Tauriコマンド定義
├── state.rs                 # アプリケーション状態管理
└── logger.rs                # ログ設定

src-tauri/tests/
├── unit_tests.rs            # ユニットテストエントリーポイント
├── integration_tests.rs     # 統合テストエントリーポイント
├── e2e_test.rs             # E2Eテスト
└── unit/
    ├── audio/              # 音声関連ユニットテスト
    ├── sidecar/            # Pythonサイドカー関連テスト
    └── websocket/          # WebSocket関連テスト
```

**実装済み機能**:
- `audio.rs`: FakeAudioDevice（MVP0）、AudioChunkCallback型
- `audio_device_adapter.rs`: OS別音声デバイス管理（MVP1タスク2.1-2.4完了）
  - CoreAudioAdapter (macOS)
  - WasapiAdapter (Windows)
  - AlsaAdapter (Linux)
  - ループバックデバイス検出（BlackHole/Stereo Mix/PulseAudio Monitor）
- `websocket.rs`: WebSocketサーバー（ポート9001、Chrome拡張連携）
- `python_sidecar.rs`: Pythonプロセスライフサイクル管理
- `commands.rs`: Tauriコマンド（audio_devices, start_recording等）

#### Python Sidecar (python-stt/)
```
python-stt/
├── main.py                 # エントリーポイント（stdin/stdout IPC）
└── stt_engine/
    ├── __init__.py
    ├── ipc_handler.py      # IPC通信ハンドラ
    ├── fake_processor.py   # Fake文字起こし実装（MVP0）
    └── lifecycle_manager.py # ライフサイクル管理

python-stt/tests/
└── test_integration.py     # 統合テスト
```

**実装済み機能**:
- `ipc_handler.py`: JSON-RPC風メッセージハンドリング
- `fake_processor.py`: Fake transcribe実装（MVP0）
- `lifecycle_manager.py`: プロセスライフサイクル管理

**未実装（MVP1予定）**:
- faster-whisper統合
- webrtcvad統合
- モデル管理（HuggingFace Hub + バンドル）

#### Chrome Extension (chrome-extension/)
```
chrome-extension/
├── manifest.json           # Manifest V3設定
├── content-script.js       # Content Script（WebSocket管理）
├── service-worker.js       # Service Worker（軽量メッセージ中継）
└── popup.html             # ポップアップUI
```

**実装済み機能**:
- Content Script中心のWebSocket管理（ADR-004決定）
- Google Meet SPA対応（URL監視、重複注入防止）
- Tauri AppへのWebSocket接続（ws://localhost:9001）

#### Frontend (src/)
```
src/
├── main.tsx               # Reactエントリーポイント
├── App.tsx                # メインアプリケーション
├── App.css                # スタイル
└── vite-env.d.ts          # Vite型定義
```

**実装済み機能**:
- 基本UI構造（MVP0）
- Tauriコマンド呼び出し統合

**未実装（MVP1以降予定）**:
- 音声デバイス選択UI
- リアルタイム文字起こし表示
- 設定画面

---

## Target Structure (実装完了時)

### Rust / Tauri Application (src-tauri/)
- `src/main.rs`: エントリーポイント
- `src/commands/`: Tauriコマンド実装（audio, websocket, settings）
- `src/services/`: ビジネスロジック
  - `audio_device_adapter.rs`: OS固有音声API抽象化（✅ 実装済み）
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
- `stores/`: 状態管理（現状はuseState/useReducer、必要に応じてZustand検討）
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

---

## Domain-Driven Design

### Audio Domain
- **責務**: 音声キャプチャ、デバイス管理
- **実装状況**: 
  - ✅ OS別デバイス列挙（CoreAudio/WASAPI/ALSA）
  - ✅ ループバックデバイス検出
  - ⚪ デバイス切断検出・再接続（タスク2.5）
  - ⚪ マイクアクセス許可確認（タスク2.6）

### Transcription Domain
- **責務**: STT、後処理、精度管理
- **実装状況**:
  - ✅ Fake実装（MVP0）
  - ⚪ faster-whisper統合（タスク3.1-3.4）
  - ⚪ VAD統合（タスク4.1-4.3）

### Communication Domain
- **責務**: WebSocket、メッセージング、同期
- **実装状況**:
  - ✅ WebSocketサーバー（Rust側）
  - ✅ Chrome拡張連携（Content Script管理）
  - ✅ Pythonサイドカー IPC（stdin/stdout JSON）

### Summarization Domain (MVP3予定)
- **責務**: 要約生成、キーポイント抽出
- **実装状況**: ⚪ 未着手

---

## ADRs (Architecture Decision Records)

### ADR-001: Recording Responsibility
- **決定**: 音声録音はRust側`AudioDeviceAdapter`のみが担当
- **理由**: レース条件防止、リソース競合回避
- **強制手段**: `scripts/check_forbidden_imports.py` + pre-commit hooks
- **実装状況**: ✅ 完了

### ADR-002: Model Distribution Strategy
- **決定**: ハイブリッド配布戦略（HuggingFace Hub + バンドルbaseモデル）
- **理由**: オフライン動作保証 + 初回起動時の柔軟性
- **実装状況**: ⚪ 設計完了、実装待ち（タスク3.3）

### ADR-003: IPC Versioning
- **決定**: セマンティックバージョニング + 後方互換性保証
- **実装状況**: ⚪ 設計完了、実装待ち

### ADR-004: Chrome Extension WebSocket Management
- **決定**: Content Script中心のWebSocket管理
- **理由**: MV3 Service Worker制約回避、接続安定性
- **実装状況**: ✅ 完了

---

## Testing Strategy

### Unit Tests (31 passed)
- `src-tauri/tests/unit/audio/`: 音声デバイス関連
- `src-tauri/tests/unit/sidecar/`: Pythonサイドカー関連
- `src-tauri/tests/unit/websocket/`: WebSocket関連

### Integration Tests (5 passed)
- `src-tauri/tests/integration/audio_ipc_integration.rs`: 音声 + IPC統合
- `src-tauri/tests/integration/websocket_integration.rs`: WebSocket統合

### E2E Tests (8 passed)
- `src-tauri/tests/e2e_test.rs`: 3プロセス連携E2Eテスト

### Test Coverage
- **目標**: ユニット80% / 統合主要シナリオ100%
- **現状**: MVP0で基準達成

---

## Known Issues and Limitations

詳細は `docs/mvp0-known-issues.md` 参照。

### MVP0 Limitations
- Fake実装のため実際の音声処理なし
- リトライロジック未実装（タスク4.2-4.3でMVP1対応予定）
- ヘルスチェック未実装

### Technical Debt
- Python側のモジュール構造（直接main.py実装、将来的にリファクタリング検討）
- Chrome拡張のエラーハンドリング強化
- UI/UXの改善（MVP1以降で対応）

---

## Next Steps

### MVP1 (meeting-minutes-stt) - 現在実装中
- [ ] タスク2.5: デバイス切断検出と自動再接続
- [ ] タスク2.6: マイクアクセス許可確認
- [ ] タスク3.x: faster-whisper統合
- [ ] タスク4.x: webrtcvad統合
- [ ] タスク5.x: リソースベースモデル選択

### MVP2 (meeting-minutes-docs-sync) - 計画中
- OAuth 2.0認証
- Google Docs API統合
- Named Range管理

### MVP3 (meeting-minutes-llm) - 計画中
- LLM要約機能
- プロダクション準備

---

## References

- **設計原則**: `.kiro/steering/principles.md`
- **技術スタック**: `.kiro/steering/tech.md`
- **コーディング規約**: `docs/dev/coding-standards.md`
- **MVP0既知の問題**: `docs/mvp0-known-issues.md`
- **ADR一覧**: `.kiro/specs/meeting-minutes-stt/adrs/`
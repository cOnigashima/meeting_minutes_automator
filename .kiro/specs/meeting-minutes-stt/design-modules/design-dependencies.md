## Dependencies

### Upstream Dependencies (Blocking)

本specの実装開始前に、以下の成果物が完了している必要があります:

- **meeting-minutes-core** (phase: design-validated以降):
  - **CORE-REQ-004**: IPC通信プロトコル v1.0 (stdin/stdout JSON)
  - **CORE-REQ-006**: WebSocketサーバー (ポート9001-9100)
  - **CORE-REQ-007**: Chrome拡張スケルトン (WebSocket接続機能)

**前提**: meeting-minutes-core/design.md で定義されたWebSocketMessage Tagged Union形式を基準とする

---

### External Dependencies

**Python依存関係** (`python-stt/requirements.txt`):

```
faster-whisper>=0.10.0
webrtcvad>=2.0.0
numpy>=1.24.0
psutil>=5.9.0          # システムリソース監視
structlog>=23.1.0      # 構造化ログ
aiofiles>=23.1.0       # 非同期ファイルI/O
```

**Rust依存関係** (`src-tauri/Cargo.toml`):

```toml
[dependencies]
tauri = { version = "2.0", features = [] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.20"  # WebSocketサーバー (meeting-minutes-core継承)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
cpal = "0.15"              # クロスプラットフォーム音声デバイス
```

---

### Internal Dependencies

- **meeting-minutes-core**: Walking Skeleton実装 (IPC通信、WebSocketサーバー、Chrome拡張スケルトン)
- **Umbrella Spec**: `.kiro/specs/meeting-minutes-automator` - 全体アーキテクチャリファレンス
- **Steering Documents**:
  - `principles.md`: プロセス境界の明確化原則、オフラインファースト原則、段階的リソース管理原則
  - `tech.md`: faster-whisper統合、webrtcvad統合の技術詳細
  - `structure.md`: Pythonモジュール構造 (`audio/`, `transcription/`, `ipc/`, `adapters/`)

---

## Requirement Traceability Matrix

本サブスペックとアンブレラ仕様 (meeting-minutes-automator) の要件対応表。

| STT ID | 要件概要 | アンブレラID | 実装コンポーネント | 備考 |
|--------|---------|-------------|-------------------|------|
| STT-REQ-001 | Real Audio Device Management | REQ-001.1 | RealAudioDevice (Rust) | マイク/ループバック録音 |
| STT-REQ-002 | faster-whisper Integration (Offline-First) | ARC-002.a, ARC-002.1 | WhisperSTTEngine (Python) | オフライン対応、モデルバンドル含む |
| STT-REQ-003 | webrtcvad Integration | ARC-002.c | VoiceActivityDetector (Python) | 音声活動検出 |
| STT-REQ-004 | Cross-Platform Audio Device Support | REQ-004, CON-001.c | AudioDeviceAdapter trait (Rust) | OS別音声デバイスアクセス |
| STT-REQ-005 | Local Storage | REQ-001.1.e, REQ-001.1.f | LocalStorageService (Rust) | 録音ファイル保存・ローテーション |
| STT-REQ-006 | Resource-Based Model Selection and Dynamic Downgrade | ARC-002.2, NFR-002.1 | ResourceMonitor (Python) | 起動時選択+動的ダウングレード |
| STT-REQ-007 | IPC Protocol Extension | REQ-005 | AudioStreamBridge (Rust) | Pythonサイドカー通信拡張 |
| STT-REQ-008 | WebSocket Message Extension | REQ-003.1 | WebSocketServer (Rust) | Chrome拡張連携メッセージ拡張 |
| STT-NFR-001 | Performance | NFR-001 | 全コンポーネント | リアルタイム性能要件 |
| STT-NFR-002 | Reliability | NFR-004 | WhisperSTTEngine, RealAudioDevice | 可用性・自動復旧 |
| STT-NFR-003 | Compatibility | REQ-004 | AudioDeviceAdapter (OS別実装) | クロスプラットフォーム動作 |
| STT-NFR-004 | Security | NFR-003 | WhisperSTTEngine, LocalStorageService | ローカル処理優先、改ざん検証 |
| STT-NFR-005 | Logging | - | 全コンポーネント | MVP1固有ログ要件 |

**上流依存**:
- **meeting-minutes-core**: CORE-REQ-004 (IPC通信プロトコルv1.0), CORE-REQ-006 (WebSocketサーバー), CORE-REQ-007 (Chrome拡張スケルトン)

**下流影響**:
- **meeting-minutes-docs-sync**: STT-REQ-008のWebSocketメッセージ形式を利用
- **meeting-minutes-llm**: STT-REQ-005のローカルストレージ (transcription.jsonl) を要約入力として利用

---


## Architecture

### High-Level Architecture

meeting-minutes-coreのWalking Skeletonアーキテクチャを継承し、Fake実装を実コンポーネントに置き換えます。

```
┌─────────────────────────────────────────────────────────┐
│             Tauri Desktop Application (Rust)            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────┐      ┌──────────────────┐        │
│  │ RealAudioDevice │──────│ AudioStreamBridge│        │
│  │ (OS Audio API)  │      │ (IPC通信層)       │        │
│  └─────────────────┘      └──────────┬───────┘        │
│                                      │                 │
│                            ┌─────────▼────────┐        │
│                            │ PythonSidecar    │        │
│                            │ Manager          │        │
│                            └─────────┬────────┘        │
│                                      │                 │
│  ┌───────────────────────────────────▼─────────────┐  │
│  │     Python Sidecar Process (音声処理)           │  │
│  │  ┌──────────────────┐  ┌─────────────────────┐ │  │
│  │  │ VoiceActivity    │──│ WhisperSTTEngine    │ │  │
│  │  │ Detector         │  │ (faster-whisper)    │ │  │
│  │  │ (webrtcvad)      │  └─────────────────────┘ │  │
│  │  └──────────────────┘                          │  │
│  │  ┌──────────────────┐                          │  │
│  │  │ ResourceMonitor  │ (起動時選択+動的ダウン   │  │
│  │  │                  │  グレード)               │  │
│  │  └──────────────────┘                          │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌─────────────────┐      ┌──────────────────┐    │
│  │ WebSocketServer │──────│ LocalStorage     │    │
│  │ (ポート9001-9100)│      │ Service          │    │
│  └────────┬────────┘      └──────────────────┘    │
└───────────┼─────────────────────────────────────────┘
            │
            │ WebSocket (JSON)
            ▼
┌─────────────────────────────────────────────────────────┐
│         Chrome Extension (Manifest V3)                  │
│  ┌───────────────┐      ┌────────────────┐            │
│  │ Content Script│      │ Service Worker │            │
│  │ (WebSocket管理)│─────│ (メッセージ中継)│            │
│  │ (chrome.storage)│    │                │            │
│  └───────────────┘      └────────────────┘            │
│       ↓                          ↓                      │
│  ┌──────────────────────────────────────┐              │
│  │     chrome.storage.local             │              │
│  │    (録音状態・接続状態の共有)         │              │
│  └──────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────┘
```

**主要な変更点** (meeting-minutes-core からの移行):
- `FakeAudioDevice` → `RealAudioDevice` (OS固有音声API統合)
- `FakeProcessor` → `VoiceActivityDetector` + `WhisperSTTEngine` (実音声処理)
- IPC通信プロトコルは維持 (後方互換性保証)
- WebSocketメッセージフォーマットを拡張 (confidence, language等の追加フィールド)
- **Chrome拡張アーキテクチャ**: Content ScriptがWebSocket管理を担当（**ADR-004採用**）
  - **WebSocket接続の永続化**: タブ存続期間中はContent Scriptが接続を維持（MV3の30秒制限回避）
  - **状態共有**: chrome.storage.local経由でPopup UI・複数タブ間の状態を同期（ADR-005パターン）
  - **Service Workerの役割**: 軽量メッセージ中継のみ（録音開始/停止コマンドのルーティング）
  - **参照**: `.kiro/specs/meeting-minutes-core/adrs/ADR-004-chrome-extension-websocket-management.md`、`.kiro/specs/meeting-minutes-core/adrs/ADR-005-state-management-mechanism.md`

### Technology Stack and Design Decisions

#### 音声処理層

**選定技術**:
- **音声認識**: faster-whisper ≥0.10.0 (CTranslate2最適化版)
- **音声活動検出**: webrtcvad ≥2.0.0
- **音声デバイスアクセス**: cpal (Rust crate) - クロスプラットフォーム対応
- **数値処理**: numpy ≥1.24.0 (Python)

**選定理由**:
- faster-whisperは2025年時点で最もCPU効率的なWhisper実装 (OpenAI Whisperと比較してCPU使用量50%削減)
- webrtcvadは16-bit mono PCMで低遅延VAD処理を実現
- cpalはmacOS/Windows/Linuxで統一的な音声デバイスアクセスを提供

**代替案考慮**:
- OpenAI Whisper (除外: CPU使用量が高い)
- SimulStreaming (2025年新技術、将来検討)

#### Rust音声処理層

**選定技術**:
- **音声デバイス**: cpal 0.15.x (クロスプラットフォーム音声I/O)
- **非同期処理**: tokio 1.x (meeting-minutes-core継承)
- **IPC通信**: stdin/stdout JSON (meeting-minutes-core継承)

#### 重要な設計決定

**決定1: 録音責務の一元化 (プロセス境界の明確化原則)**
- **決定**: 音声録音はRust側 `RealAudioDevice` のみが担当
- **背景**: Pythonサイドカーとの録音競合を防止し、レース条件を回避
- **制約**: Pythonサイドカーは録音を行わず、Rustから送信されたバイナリストリームの受信とSTT処理のみを実施
- **根拠**: `.kiro/steering/principles.md` プロセス境界の明確化原則 - 各プロセスは独立した責務を持ち、録音はRustプロセスに一元化

**録音責務の一元化の技術的実現**:

**静的解析による検証**:
- CI/CD パイプラインに Python コード静的解析を追加
- 禁止パッケージ検出: `sounddevice`, `pyaudio`, `portaudio`, `soundcard`, `PySndHdr`
- `flake8-forbidden-imports` プラグインで自動検証
- pre-commit フックでの自動チェック

**依存関係ロック**:
- `pip-compile` による許可リスト (allowlist) 方式
- `requirements.txt` に録音関連パッケージを含めない
- `requirements-lock.txt` で依存関係を固定

**実装例** (`.pre-commit-config.yaml`):
```yaml
- repo: local
  hooks:
    - id: check-python-audio-imports
      name: Check forbidden audio recording imports
      entry: python scripts/check_forbidden_imports.py
      language: system
      files: ^python-stt/.*\.py$
```

**違反時のエラーメッセージ**:
```
❌ Forbidden import detected: 'sounddevice' in python-stt/stt_engine/audio/capture.py
📖 Recording responsibility is exclusively handled by Rust AudioDeviceAdapter.
📄 See: .kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md
```

**参照**: `.kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md`

**決定2: オフラインファーストアーキテクチャ (オフラインファースト原則)**
- **決定**: ネットワーク依存機能を全てオプショナル化し、ローカル完結モードで動作保証
- **背景**: 企業ネットワークやネットワーク切断時でも実用可能なSTT機能が求められる
- **実装戦略**:
  1. モデル検出優先順位: ユーザー設定 → HuggingFace Hubキャッシュ → インストーラーバンドルモデル
  2. HuggingFace Hub接続タイムアウト: 10秒
  3. ダウンロード失敗時の自動フォールバック: バンドルbaseモデル使用
  4. プロキシ環境対応: `HTTPS_PROXY` / `HTTP_PROXY` 環境変数認識
- **トレードオフ**: インストーラーサイズ増加 (バンドルbaseモデル: 39MB) vs オフライン動作保証
- **根拠**: `.kiro/steering/principles.md` オフラインファースト原則 - Tier 1機能はネットワーク接続不要で完全動作

**決定3: リソースベースモデル選択と動的ダウングレード (段階的リソース管理原則)**
- **決定**: 起動時にシステムリソースを検出してWhisperモデルを自動選択し、実行中のリソース制約時に動的ダウングレード
- **背景**: リソース制約下でも安定動作を保証し、システムのフリーズを防止
- **実装戦略**:
  - **起動時モデル選択**: GPU利用可能+メモリ8GB以上 → large-v3、CPU+メモリ4GB以上 → small、etc.
  - **動的ダウングレード**: CPU 85%/60秒持続 → 1段階ダウングレード、メモリ4GB到達 → 即座にbaseモデル
  - **UI通知**: トースト通知で「モデル変更: {old} → {new}」を表示
- **トレードオフ**: 実装複雑性増加 vs リソース制約下での安定性向上
- **根拠**: `.kiro/steering/principles.md` 段階的リソース管理原則 - 警告閾値（黄色）、制限閾値（赤色）、強制停止閾値の3段階

**決定4: IPC通信プロトコルの後方互換性維持 (依存関係のベンダーロックイン回避原則)**
- **決定**: meeting-minutes-coreで確立したIPC通信プロトコルv1.0を維持し、新フィールドを追加拡張
- **背景**: Walking Skeleton実装との互換性を保ちつつ、実音声処理の追加情報を伝達
- **実装戦略**:
  - 既存メッセージ形式を維持: `{ id, type, result: { text, is_final } }`
  - 新フィールド追加: `confidence`, `language`, `processing_time_ms`, `model_size`
  - バージョンフィールド追加: `"version": "1.0"`
- **後方互換性保証**: meeting-minutes-core (Fake実装) は未知のフィールドを無視し、`text`フィールドのみを使用
- **根拠**: `.kiro/steering/principles.md` 依存関係のベンダーロックイン回避原則 - 外部サービスは全てAdapterパターンで抽象化

---


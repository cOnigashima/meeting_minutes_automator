# Technical Design Document - Master Index

> **注意**: このドキュメントはマスタードキュメントです。詳細な設計内容は各分割ドキュメントを参照してください。

## 概要

**目的**: meeting-minutes-stt (MVP1) は、meeting-minutes-core (Walking Skeleton) で確立した3プロセスアーキテクチャ上に実際の音声処理機能を実装します。Fake実装を実音声処理に置き換え、faster-whisperによる高精度文字起こしとwebrtcvadによる音声活動検出を実現します。

**ユーザー**: 会議参加者やファシリテーターが、実用可能なローカル音声文字起こし機能を活用します。

**インパクト**: Fakeデータから実音声処理への移行により、プロダクション環境で使用可能な文字起こし機能が実現されます。

---

## 分割ドキュメント一覧

本設計ドキュメントは、以下の9つの分割ドキュメントで構成されています。各ドキュメントは独立して読むことができます。

### 1. [Overview and Requirements](design-modules/design-overview.md)
**66行 | 推定読了時間: 3分**

プロジェクトの目的、Goals/Non-Goals、図版チェックリスト、非機能要件（ログ運用方針）を記載。

**含まれるセクション**:
- プロジェクト概要と目標
- 実装する図版のチェックリスト (PlantUML)
- 非機能要件: ログ運用方針 (STT-NFR-005準拠)

**推奨読者**: 全員（特にプロジェクトマネージャー、アーキテクト）

---

### 2. [Architecture](design-modules/design-architecture.md)
**168行 | 推定読了時間: 8分**

高レベルアーキテクチャ、技術スタック選定、4つの重要な設計決定 (ADR-001〜004) を記載。

**含まれるセクション**:
- High-Level Architecture（3プロセス構成図）
- Technology Stack and Design Decisions（faster-whisper、webrtcvad、cpal）
- 重要な設計決定:
  - **ADR-001**: 録音責務の一元化（Rust側AudioDeviceAdapterのみ）
  - **ADR-002**: オフラインファーストアーキテクチャ
  - **ADR-003**: リソースベースモデル選択と動的ダウングレード
  - **ADR-004**: IPC通信プロトコルの後方互換性維持

**推奨読者**: アーキテクト、シニアエンジニア、新規参加者（オンボーディング）

---

### 3. [System Flows](design-modules/design-flows.md)
**318行 | 推定読了時間: 15分**

4つの主要フローをMermaidシーケンス図と説明で記載。

**含まれるセクション**:
- **音声処理パイプライン全体フロー**: 録音開始→VAD→STT→保存→WebSocket配信
- **オフラインモデルフォールバックフロー**: HuggingFace Hub接続→ダウンロード失敗→バンドルbaseモデル
- **動的モデルダウングレードフロー**: リソース監視→CPU/メモリ閾値判定→モデル切り替え→UI通知
- **デバイス切断/再接続フロー**: 切断検出→ログ記録→ユーザー通知→自動再接続（最大3回）
- **Task 2.5 実装詳細**: イベント駆動デバイス監視アーキテクチャ（3層検出メカニズム）

**推奨読者**: エンジニア全員（特に統合テスト設計者、トラブルシューティング担当）

---

### 4. [Components and Interfaces](design-modules/design-components.md)
**809行 | 推定読了時間: 40分**

各コンポーネントの詳細設計、契約定義、依存関係を記載。最大のセクション。

**含まれるセクション**:
- **音声処理ドメイン**:
  - \`RealAudioDevice\` (Rust): OS固有音声API統合、AudioDeviceAdapter trait
  - \`AudioStreamBridge\` (Rust): Rust→Python間IPC転送
  - \`VoiceActivityDetector\` (Python): webrtcvad統合、発話セグメンテーション
  - \`WhisperSTTEngine\` (Python): faster-whisper統合、推論実行
  - \`ResourceMonitor\` (Python): リソース監視、動的モデルダウングレード
- **ストレージドメイン**:
  - \`LocalStorageService\` (Rust): 録音ファイル保存、セッション管理
- **通信ドメイン**:
  - \`WebSocketServer\` (Rust): Chrome拡張への配信
  - Chrome Extension: Content Script WebSocket管理 (ADR-004採用)

**推奨読者**: 実装担当エンジニア、コードレビュアー（セクションごとに部分読み推奨）

---

### 5. [Data Models](design-modules/design-data.md)
**241行 | 推定読了時間: 12分**

IPC通信プロトコル、WebSocketメッセージフォーマット、ローカルストレージスキーマを記載。

**含まれるセクション**:
- **IPC通信プロトコル v1.0** (stdin/stdout JSON):
  - 要求メッセージ: \`process_audio\`, \`list_devices\`
  - 応答メッセージ: 成功応答、エラー応答
  - 後方互換性: 新フィールド追加（\`confidence\`, \`language\`, \`processing_time_ms\`）
- **WebSocketメッセージフォーマット**:
  - \`transcription\` (部分/確定テキスト)
  - \`status\` (録音状態通知)
  - \`error\` (エラー通知)
- **ローカルストレージスキーマ**:
  - \`session.json\` (セッションメタデータ)
  - \`transcription.jsonl\` (JSON Lines形式)
  - \`audio.wav\` (16kHz mono PCM)

**推奨読者**: API統合担当者、テストエンジニア、ドキュメント作成者

---

### 6. [Error Handling](design-modules/design-error.md)
**276行 | 推定読了時間: 14分**

エラー分類、エラーコード定義、エラーハンドリング戦略を記載。

**含まれるセクション**:
- **エラー分類**:
  - Recoverable Error (部分テキスト生成失敗等)
  - Non-Recoverable Error (モデルロード失敗等)
  - System Error (IPC通信断絶等)
- **エラーコード定義**: \`AUDIO_DEVICE_ERROR\`, \`STT_INFERENCE_ERROR\`, \`IPC_ERROR\`, \`STORAGE_ERROR\`
- **エラーハンドリング戦略**:
  - リトライロジック: 指数バックオフ (1s, 2s, 4s, 8s, 16s max)
  - ユーザー通知: トースト通知 + エラーダイアログ
  - ログ記録: ERRORレベル + スタックトレース
- **例外契約**: 各コンポーネントのエラー返却保証

**推奨読者**: エラーハンドリング担当者、QAエンジニア、サポート担当

---

### 7. [Testing Strategy](design-modules/design-testing.md)
**157行 | 推定読了時間: 8分**

ユニットテスト、統合テスト、E2Eテストの方針とカバレッジ目標を記載。

**含まれるセクション**:
- **ユニットテスト**: カバレッジ80%以上、TDD実践
- **統合テスト**: 主要シナリオ100%、プロセス間通信検証
- **E2Eテスト**: 全要件カバー、クロスプラットフォーム検証
- **テストダブル戦略**: Mock/Stub/Fake使い分け
- **CI/CDパイプライン統合**: GitHub Actions自動テスト実行

**推奨読者**: テストエンジニア、QAリード、CI/CD担当者

---

### 8. [Dependencies and Traceability](design-modules/design-dependencies.md)
**85行 | 推定読了時間: 4分**

外部依存関係、内部依存関係、要件トレーサビリティマトリックスを記載。

**含まれるセクション**:
- **外部依存関係**:
  - faster-whisper ≥0.10.0
  - webrtcvad ≥2.0.0
  - numpy ≥1.24.0
  - cpal 0.15.x (Rust)
- **内部依存関係**:
  - meeting-minutes-core (IPC通信プロトコル v1.0, WebSocketサーバー)
  - Umbrella Spec (meeting-minutes-automator)
  - Steering Documents (tech.md, structure.md, principles.md)
- **要件トレーサビリティマトリックス**: design.md各セクション → requirements.md要件IDへのマッピング

**推奨読者**: プロジェクトマネージャー、アーキテクト、依存関係管理担当者

---

### 9. [Implementation Plan](design-modules/design-implementation.md)
**153行 | 推定読了時間: 8分**

実装タスク、TDD実装フロー、次のアクション、リビジョン履歴を記載。

**含まれるセクション**:
- **実装タスク**: 12セクション88タスク（tasks.mdと同期）
- **TDD実装フロー**: RED → GREEN → REFACTOR
- **重要な制約**: 録音責務一元化、オフラインファースト、後方互換性
- **テストカバレッジ目標**: ユニット80%以上、統合100%、E2E全要件
- **Next Actions**: 次に着手するタスクの明示
- **Revision History**: 設計変更履歴

**推奨読者**: 実装担当エンジニア、スプリントプランナー

---

## 使い方ガイド

### 初めて読む場合（新規参加者）
1. **[Overview and Requirements](design-modules/design-overview.md)** - プロジェクトの全体像を理解
2. **[Architecture](design-modules/design-architecture.md)** - アーキテクチャと重要な設計決定を理解
3. **[System Flows](design-modules/design-flows.md)** - 主要フローをシーケンス図で理解

### 実装担当者の場合
1. **[Implementation Plan](design-modules/design-implementation.md)** - 次のタスクを確認
2. **[Components and Interfaces](design-modules/design-components.md)** - 担当コンポーネントの詳細設計を確認
3. **[Data Models](design-modules/design-data.md)** - メッセージフォーマットとスキーマを確認
4. **[Error Handling](design-modules/design-error.md)** - エラーハンドリング方針を確認

### テストエンジニアの場合
1. **[Testing Strategy](design-modules/design-testing.md)** - テスト方針とカバレッジ目標を確認
2. **[System Flows](design-modules/design-flows.md)** - テストシナリオを理解
3. **[Error Handling](design-modules/design-error.md)** - 異常系テストケースを設計

### トラブルシューティングの場合
1. **[System Flows](design-modules/design-flows.md)** - 問題発生箇所のフローを確認
2. **[Error Handling](design-modules/design-error.md)** - エラーコードと対応方法を確認
3. **[Components and Interfaces](design-modules/design-components.md)** - 該当コンポーネントの契約定義を確認

---

## 更新履歴

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-12 | 2.0 | Claude Code | design.mdを9つの分割ドキュメントに分割（読みやすさ向上） |
| 2025-10-06 | 1.2 | Claude Code | ADR-004追加（Chrome拡張WebSocket管理: Content Script方式採用） |
| 2025-10-05 | 1.1 | Claude Code | System Flows追加、コンポーネント詳細設計追加 |
| 2025-10-02 | 1.0 | Claude Code | 初版作成（MVP1 Real STT設計） |

---

## 関連ドキュメント

- **要件定義**: [requirements.md](requirements.md)
- **実装タスク**: [tasks.md](tasks.md)
- **ADR一覧**: [adrs/](adrs/)
- **Steering Documents**: [.kiro/steering/](../../steering/)
- **Umbrella Spec**: [meeting-minutes-automator](../meeting-minutes-automator/)

---

## フィードバック

設計ドキュメントの改善提案や質問がある場合は、以下の方法でフィードバックをお願いします：
- GitHub Issue作成（設計提案）
- ADR作成提案（重要な技術的決定）
- ドキュメント修正Pull Request

**最終更新**: 2025年10月12日
**メンテナー**: Claude Code
**ステータス**: 🔵 Active Development (MVP1実装フェーズ)

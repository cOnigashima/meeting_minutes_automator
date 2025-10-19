# Project Structure

更新日: 2025-10-19  
対象フェーズ: MVP1 Real STT（コア機能完了、UI/Docs同期拡張中）

---

## 1. 現在のスナップショット

- ✅ **MVP0 (meeting-minutes-core)**: Walking Skeleton 完了。Fake 録音と 3 プロセス疎通は安定運用。
- ✅ **MVP1 (meeting-minutes-stt)**: AudioDeviceAdapter / VAD / Whisper / ローカルストレージ / IPC イベントストリームを実装済み。UI (Task 9.x) と Docs 連携前提のテスト自動化 (Task 10.x) が継続中。
- 🔵 **MVP2 (meeting-minutes-docs-sync)**: 設計完了。実装は MVP1 安定化後に着手。
- 🔵 **meeting-minutes-ci**: CI マトリクス設計を継続。現在は手動検証 + ローカルテストで代替。
- ⚪ **meeting-minutes-llm**: LLM 連携は MVP2 完遂後に要件定義を開始。

---

## 2. リポジトリ構成（2025-10-19 時点）

```
meeting-minutes-automator/
├── .kiro/
│   ├── steering/            # プロジェクトガイダンス（本ドキュメント等）
│   ├── specs/               # Umbrella + 各サブスペック + ADR
│   └── research/            # 技術調査メモ
├── docs/
│   ├── dev/                 # 開発ガイド（coding-standards, spec-authoring, chrome-storage...）
│   ├── mvp0-known-issues.md # レガシー課題一覧（随時更新）
│   └── platform-verification.md # プラットフォーム検証ログ
├── src-tauri/
│   ├── src/                 # Rust 実装（AudioDeviceAdapter, IPC, Storage, WebSocket 等）
│   └── tests/               # ユニット / 統合 / E2E テスト (`stt_e2e_test.rs` など)
├── python-stt/
│   ├── stt_engine/          # AudioPipeline, ResourceMonitor, Whisper クライアント
│   └── tests/               # pytest ベースの統合テスト群
├── chrome-extension/        # Manifest V3 拡張（content-script.js で WebSocket 管理）
├── src/                     # Tauri フロントエンド（MVP1 現在は最小 UI）
├── scripts/                 # 静的解析 / ビルド補助スクリプト
└── README.md, CLAUDE.md     # ルートのナビゲーションドキュメント
```

---

## 3. 仕様フェーズと実装状況

| Spec | フェーズ | 代表タスク | 備考 |
|------|---------|------------|------|
| meeting-minutes-core | Implementation Complete ✅ | Task 1.x | Walking Skeleton と基盤整備のみメンテナンス対応 |
| meeting-minutes-stt  | Implementation 🔄 | Task 2〜7 完了 / Task 9,10 継続 | Audioデバイス管理 + リアルSTT + ローカル保存を実装済み |
| meeting-minutes-docs-sync | Design Generated 🔵 | Task 1.x | OAuth / Docs API 設計完了。実装は MVP1 安定後 |
| meeting-minutes-ci | Spec Initialized 🔵 | Task 1.x | クロスプラットフォーム CI 設計（GitHub Actions）が進行中 |
| meeting-minutes-llm | Not Started ⚪ | - | MVP2 以降で着手 |

---

## 4. ディレクトリ別の責務

### `.kiro/`
- **steering/**: プロジェクト全体方針（product / tech / principles / structure）。
- **specs/**: MVP別の requirements / design / tasks と ADR。最新タスク進捗は `meeting-minutes-stt/tasks.md` 等を参照。

### `docs/`
- **dev/**: 開発フローに関するリファレンス。`coding-standards.md` は lint/format の最新ポリシーを反映。
- **platform-verification.md**: クロスプラットフォーム検証ログ。現在は macOS のリアル STT 検証結果を記録。
- **mvp0-known-issues.md**: MVP0 レビュー時に指摘された課題の追跡。解消済みの項目は適宜クローズ予定。

### `src-tauri/`
- `audio_device_adapter.rs`: CoreAudio / WASAPI / ALSA 実装とデバイス監視イベント。
- `commands.rs`: `process_audio_stream` を経由した Python 連携と WebSocket ブロードキャスト。
- `storage.rs`: セッションディレクトリ (`audio.wav` / `transcription.jsonl` / `session.json`) の管理。
- `tests/`: `stt_e2e_test.rs`, `audio_ipc_integration.rs` など、Python サイドカーを含む統合テスト。

### `python-stt/`
- `stt_engine/audio_pipeline.py`: VAD + Whisper の調停。partial/final イベント生成を担当。
- `stt_engine/resource_monitor.py`: CPU/メモリ監視とモデルダウングレード提案。
- `main.py`: IPC メッセージディスパッチ。`process_audio_stream` ハンドラと ResourceMonitor イベント通知を実装。
- `tests/`: `test_audio_integration.py`, `test_resource_monitor.py`, `test_offline_model_fallback.py` など。

### `chrome-extension/`
- `content-script.js`: ポートスキャン、WebSocket 再接続、`chrome.storage.local` 更新、コンソールログ表示。
- `service-worker.js`: Manifest V3 制約下の最小メッセージリレー。Docs 同期機能は MVP2 で拡張予定。

### `src/`
- 現在は MVP1 の最小 UI（録音開始/停止ボタン）のみ。デバイス選択や履歴 UI は Task 9.x で拡張予定。

---

## 5. 今後の構造アップデート予定

| 時期 | 予定 | 依存タスク |
|------|------|-----------|
| MVP1 UI 拡張 | `src/components/` / `src/hooks/` を追加し、デバイス選択 UI とリソース警告を表示 | Task 9.1, 9.2 |
| MVP2 Docs Sync | `chrome-extension/` に Popup UI と Docs 同期マネージャを追加。Tauri 側に OAuth Storage サービスを実装 | meeting-minutes-docs-sync Task 3.x |
| meeting-minutes-ci | `.github/workflows/` 追加、クロスプラットフォーム smoke テストを自動化 | meeting-minutes-ci Task 2.x |
| MVP3 LLM | `src-tauri/src/llm/`、`python-stt/stt_engine/summary/` などを新設予定 | meeting-minutes-llm Task TBD |

---

## 6. 参照ドキュメント

- 開発ガイドライン: [docs/dev/coding-standards.md](../../docs/dev/coding-standards.md)
- 仕様作成ガイド: [docs/dev/spec-authoring.md](../../docs/dev/spec-authoring.md)
- 最新ステータス: [README.md](../../README.md) / [docs/platform-verification.md](../../docs/platform-verification.md)

---

このドキュメントは MVP ごとのディレクトリ責務が変化したタイミングで更新します。更新の際は README・specs と整合が取れているかを必ず確認してください。

# Known Issues - MVP0 Walking Skeleton

## 概要

MVP0 Walking Skeleton完成後の専門家レビュー（Ask 8-9）で指摘された品質ギャップを記録します。

**レビュー日時**: 2025-10-05
**対象バージョン**: MVP0 (Walking Skeleton)
**ステータス**: 📋 記録済み（対応予定: MVP1以降 or 専用タスク）

---

## 指摘事項

### Ask 8: E2E疎通確認の不備

#### 8-1: Chrome拡張テストの欠如

**問題**:
- `src-tauri/tests/e2e_test.rs` はRust↔Python間のみテスト
- WebSocket → Chrome拡張の検証が自動化されていない
- AC-008.2 "Chrome Console output に Transcription: … が表示される" の自動検証が未実装

**影響範囲**:
- E2Eテスト（`test_recording_to_transcription_flow`）が完全なフローをカバーしていない
- 手動E2Eテストに依存（再現性・自動化の観点で脆弱）

**対応予定**:
- [ ] Puppeteer/Playwright を使用したChrome拡張の自動E2Eテスト追加
- [ ] WebSocket → Chrome拡張の疎通を含む完全なE2Eシナリオ実装
- [ ] CI/CD環境でのヘッドレステスト対応

**関連ファイル**:
- `src-tauri/tests/e2e_test.rs:8-154`
- `.kiro/specs/meeting-minutes-core/design.md:1717-1726` (E2E-1/2/3設計)

---

#### 8-2: Python依存の脆弱性

**問題**:
- E2Eテストが実行環境のPython 3.9+に依存
- CI環境や他の開発者のマシンでPythonバージョン不一致時に失敗する可能性
- `PythonSidecarManager` が実際のプロセスを起動するため、環境依存性が高い

**影響範囲**:
- CI/CD環境でのテスト失敗リスク
- 開発者オンボーディング時のセットアップ障壁

**対応予定**:
- [ ] Fake Sidecar実装（テスト用モックプロセス）
- [ ] 環境変数 `USE_FAKE_SIDECAR=true` での切り替え機構
- [ ] CI環境でのPythonバージョン固定（pyenv/asdf等）

**関連ファイル**:
- `src-tauri/tests/e2e_test.rs:101-154`
- `src-tauri/src/python_sidecar.rs` (実プロセス起動ロジック)

---

### Ask 9: 非機能要件の不備

#### 9-1: IPCレイテンシメトリクスの欠落

**問題**:
- AC-NFR-PERF.4 "IPC latency < 50ms (mean)" の計測ロジックが未実装
- `ipc_latency_ms` ログが存在しない（コードベース全体を検索しても見つからず）
- `scripts/performance_report.py` が集計するメトリクスが存在しない

**影響範囲**:
- パフォーマンステストが不完全（WebSocket latencyのみ計測）
- Rust ↔ Python IPC のボトルネック検出不可

**対応予定**:
- [ ] `PythonSidecarManager::send_message()` に送信タイムスタンプ記録
- [ ] `PythonSidecarManager::receive_message()` でレイテンシ計算
- [ ] `logger.rs` 経由で `ipc_latency_ms` メトリクス出力
- [ ] `scripts/performance_report.py` での集計確認

**関連ファイル**:
- `src-tauri/src/python_sidecar.rs` (IPC通信ロジック)
- `src-tauri/src/logger.rs` (メトリクス出力)
- `scripts/performance_report.py` (集計スクリプト)

---

#### 9-2: 構造化ログの未使用

**問題**:
- `src-tauri/src/logger.rs` は実装済みだが、実際には使用されていない
- 全コンポーネントで `println!` / `eprintln!` のまま
- AC-NFR-LOG.1〜3（JSON構造化ログ出力）が未達成

**影響範囲**:
- ログ解析・モニタリングが困難
- デバッグ効率の低下
- プロダクション運用時のトレーサビリティ欠如

**対応予定**:
- [ ] 全 `println!`/`eprintln!` を `log_info!`/`log_error!` 等に置き換え
- [ ] 主要イベント（start/stop recording, IPC送受信, WebSocket broadcast）のログ記録
- [ ] エラーハンドリング時の詳細ログ出力

**関連ファイル**:
- `src-tauri/src/logger.rs` (構造化ログモジュール)
- `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/websocket.rs` (ログ出力箇所)

---

#### 9-3: IPC JSONバリデーションの欠如

**問題**:
- AC-NFR-SEC.3 "IPC JSON message validation (field/size limits)" が未実装
- `PythonSidecarManager::receive_message()` が受信JSONを無条件でデシリアライズ
- 不正なJSONや巨大ペイロード攻撃への対策なし
- `scripts/security_test.py:63-84` がスキップ扱い

**影響範囲**:
- セキュリティ要件未達成
- DoS攻撃リスク（巨大JSONペイロード受信時のメモリ枯渇）

**対応予定**:
- [ ] IPC受信メッセージのサイズ制限（例: 1MB上限）
- [ ] 必須フィールド検証（`type`, `id` 等）
- [ ] スキーマバリデーション（serde_jsonでの型チェック強化）
- [ ] `scripts/security_test.py` での実テスト追加

**関連ファイル**:
- `src-tauri/src/python_sidecar.rs:receive_message()` (バリデーション追加箇所)
- `scripts/security_test.py` (セキュリティテストスクリプト)

---

#### 9-4: クロスプラットフォーム検証の欠如

**問題**:
- `docs/platform-verification.md` は存在するが、実証跡が不足
- macOS以外（Windows, Linux）での動作確認ログなし
- AC-NFR-COMP.1〜3（プラットフォーム互換性）の検証不完全

**影響範囲**:
- Windows/Linux環境での動作保証なし
- プラットフォーム固有のバグ発見遅延

**対応予定**:
- [ ] Windows 10+ での手動E2E実施（ログ記録）
- [ ] Ubuntu 20.04+ での手動E2E実施（ログ記録）
- [ ] `docs/platform-verification.md` への実証跡追記
- [ ] CI/CDマトリクステストでの自動化（`.kiro/specs/meeting-minutes-ci/` で対応）

**関連ファイル**:
- `docs/platform-verification.md` (検証ドキュメント)
- `.kiro/specs/meeting-minutes-ci/` (CI/CD仕様、マトリクステスト計画)

---

## 対応優先度

| 優先度 | 項目 | 理由 |
|-------|------|------|
| **High** | 9-1: IPCレイテンシメトリクス | パフォーマンス測定の基礎データ、MVP1で必須 |
| **High** | 9-2: 構造化ログ使用 | デバッグ・運用効率に直結、即座に対応可能 |
| **Medium** | 9-3: IPC JSONバリデーション | セキュリティ要件、MVP1でのリアルSTT前に必須 |
| **Medium** | 8-1: Chrome拡張E2Eテスト | 自動化の完全性、CI/CD構築時に対応 |
| **Low** | 8-2: Python依存の脆弱性 | 代替手段（Fake Sidecar）の実装コスト高、CI環境固定で対応可 |
| **Low** | 9-4: クロスプラットフォーム検証 | CI/CDマトリクステストで自動化予定 |

---

## MVP1 Traceability（引き継ぎ管理）

### Ask 8-1: Chrome拡張E2Eテストの欠如 → MVP1

**MVP1要件ID**: `STT-REQ-E2E-001` (Chrome拡張自動E2Eテスト)

**対応内容**:
- Puppeteer/Playwright による Chrome拡張自動テスト
- WebSocket → Chrome Console 出力の自動検証
- CI/CD環境でのヘッドレステスト

**ステータス**:
- [x] `meeting-minutes-stt/requirements.md` に要件追加（STT-REQ-E2E-001）
- [ ] `meeting-minutes-ci/design.md` に CI統合設計追加

---

### Ask 8-2: Python依存の脆弱性 → CI/CD spec

**CI/CD要件ID**: `CI-REQ-ENV-001` (Python環境固定)

**対応内容**:
- GitHub Actions での Python 3.9-3.12 マトリクステスト
- pyenv/asdf による環境固定
- Fake Sidecar 実装（テスト用モック、optional）

**ステータス**:
- [ ] `meeting-minutes-ci/requirements.md` に要件追加

---

### Ask 9-1: IPCレイテンシメトリクスの欠落 → MVP1

**MVP1要件ID**: `STT-REQ-IPC-004` (IPC latency monitoring)

**対応内容**:
- `PythonSidecarManager::send_message()` にタイムスタンプ記録
- `receive_message()` でレイテンシ計算
- `logger.rs` 経由で `ipc_latency_ms` メトリクス出力
- `scripts/performance_report.py` での集計

**ステータス**:
- [x] `meeting-minutes-stt/requirements.md` に要件追加（STT-REQ-IPC-004, IPC-005）
- [ ] `meeting-minutes-stt/design.md` に実装方針追加
- [ ] `meeting-minutes-stt/tasks.md` にタスク追加（`meeting-minutes-core` Task 4.2参照）

---

### Ask 9-2: 構造化ログの未使用 → MVP1

**MVP1要件ID**: `STT-REQ-LOG-001` (構造化ログ全面移行)

**対応内容**:
- 全 `println!`/`eprintln!` を `log_info!`/`log_error!` に置換
- 主要イベントのログ記録（start/stop, IPC, WebSocket broadcast）

**ステータス**:
- [x] `meeting-minutes-stt/requirements.md` に要件追加（STT-REQ-LOG-001）
- [ ] `meeting-minutes-stt/tasks.md` に Task 追加

---

### Ask 9-3: IPC JSONバリデーションの欠如 → MVP1

**MVP1要件ID**: `STT-REQ-SEC-001` (IPC message validation)

**対応内容**:
- IPC受信メッセージのサイズ制限（1MB上限）
- 必須フィールド検証（`type`, `id` 等）
- スキーマバリデーション（serde_json強化）

**ステータス**:
- [x] `meeting-minutes-stt/requirements.md` に要件追加（STT-REQ-SEC-001、Real STT前に必須）
- [ ] `meeting-minutes-stt/design.md` に実装方針追加

---

### Ask 9-4: クロスプラットフォーム検証の欠如 → CI/CD spec

**CI/CD要件ID**: `CI-REQ-MATRIX-001` (Cross-platform test matrix)

**対応内容**:
- Windows 10+ での手動E2E実施
- Ubuntu 20.04+ での手動E2E実施
- GitHub Actions マトリクステスト自動化

**ステータス**:
- [ ] `meeting-minutes-ci/requirements.md` に要件追加
- [ ] `meeting-minutes-ci/design.md` にマトリクス戦略追加

---

## 次のアクション

### Option A: 即座対応（MVP0完全化）
専門家指摘事項を `meeting-minutes-core` に Task 11 として追加し、完全化する

**メリット**:
- MVP0の品質を完全にしてからMVP1へ進める
- 技術的負債の早期解消

**デメリット**:
- MVP1（Real STT）への着手が遅延

---

### Option B: MVP1並行対応
MVP1開発中に並行して対応（リファクタリング枠）

**メリット**:
- MVP1機能開発を優先
- Real STT実装時に必要な品質改善を同時実施

**デメリット**:
- 並行作業による複雑性増加
- コンフリクトリスク

---

### Option C: CI/CD優先
`.kiro/specs/meeting-minutes-ci/` の要件定義を優先し、CI環境で自動検出

**メリット**:
- CI/CDパイプラインで自動検出・回帰防止
- インフラ整備を先行

**デメリット**:
- 問題が残ったままCI構築（"壊れたテスト"の自動化リスク）

---

## 推奨アプローチ

**Hybrid: 高優先度即対応 + CI/CD並行**

1. **即座対応（1-2日）**:
   - 9-1: IPCレイテンシメトリクス実装 ✅
   - 9-2: 構造化ログへ全面移行 ✅

2. **CI/CD構築（`.kiro/specs/meeting-minutes-ci/`）**:
   - 8-1: Chrome拡張E2Eテスト自動化をCI/CDタスクに含める
   - 9-4: プラットフォーム検証をCIマトリクスで実施

3. **MVP1並行対応**:
   - 9-3: IPC JSONバリデーション（Real STT前に必須）
   - 8-2: Python依存問題（CI環境固定で緩和）

---

## 関連ドキュメント

- **要件**: `.kiro/specs/meeting-minutes-core/requirements.md`
- **設計**: `.kiro/specs/meeting-minutes-core/design.md`
- **タスク**: `.kiro/specs/meeting-minutes-core/tasks.md`
- **CI/CD仕様**: `.kiro/specs/meeting-minutes-ci/` (新規作成)
- **プラットフォーム検証**: `docs/platform-verification.md`
- **セキュリティテスト**: `scripts/security_test.py`
- **パフォーマンステスト**: `scripts/performance_report.py`

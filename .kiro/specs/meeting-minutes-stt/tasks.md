# Implementation Tasks

## Overview

meeting-minutes-stt (MVP1 Core Implementation + Phase 13検証負債解消) は、meeting-minutes-core (Walking Skeleton) で確立した3プロセスアーキテクチャ上に実際の音声処理機能を実装し、本番リリース可能な状態にします。

**実装アプローチ**: TDD (Test-Driven Development) に基づき、失敗するテストを先に作成し、実装を肉付けしながらテストを緑化します。

**重要な設計決定**:
- ADR-001: 録音責務の一元化（Rust側AudioDeviceAdapterのみ）
- ADR-002: ハイブリッドモデル配布戦略（HuggingFace Hub + bundled base）
- ADR-003: IPCバージョニング（後方互換性保証）
- ADR-013: Sidecar Full-Duplex Final Design（stdin/stdout分離、lock-free ring buffer）

---

## 進捗サマリー

### MVP1 Core Implementation ✅ 完了（2025-10-19）

| フェーズ | 完了率 | ステータス | 備考 |
|---------|--------|-----------|------|
| Phase 1-8 | 100% | ✅ 完了 | 基盤整備〜WebSocket拡張 |
| Phase 9 | 40% | ⏸️ 部分完了 | UI拡張: 9.1-9.2完了、9.3-9.5延期 |
| Phase 10 | 14% | ⏸️ 部分完了 | E2E: 10.1完了、10.2-10.7→Phase 13 |
| Phase 11 | 20% | ⏸️ 部分完了 | 品質保証: 11.5完了、他延期 |
| Phase 12 | 100% | ✅ 完了 | ドキュメント・リリース準備 |

**テスト合格率**: 214テスト合格（Rust 71 + Python 143）
**完了タスク**: 42/66（64%）

---

### Phase 13: 検証負債解消 ⏸️ 部分完了（11/12タスク完了、2025-10-20更新）

**目的**: MVP1で延期した検証タスクを完了させ、本番リリース可能な状態にする

**サブタスク**:
- **13.1**: Rust E2Eテスト実装 ✅ 4/7完了（10.1/10.2/10.6/10.7）
- **13.2**: 長時間稼働テスト ✅ 完了（Task 11.3実施完了、メモリリークなし）
- **13.3**: セキュリティ修正 ✅ 4/5完了（SEC-001/002/004/005完了、SEC-003延期）

**完了タスク**: Task 10.1/10.2/10.6/10.7, Task 11.3（13.2.1-13.2.3）, SEC-001/002/004/005（11/12）
**延期タスク**: Task 10.3/10.4/10.5, SEC-003（4/12）

**推定残作業**: CI整備後にTask 10.3/10.4/10.5 + SEC-003（1-2日）

---

## タスク一覧（Phase 1-12の詳細は元ファイル参照）

### 完了済みPhase（MVP1 Core Implementation）

- [x] **Phase 1**: 基盤整備とプロジェクト準備
- [x] **Phase 2**: 実音声デバイス管理機能（Rust側）
- [x] **Phase 3**: faster-whisper統合（Python側）
- [x] **Phase 4**: VAD統合（Python側）
- [x] **Phase 5**: リソース監視・動的モデル管理
- [x] **Phase 6**: ローカルストレージ
- [x] **Phase 7**: IPC拡張・後方互換性（ADR-013完全実装）
- [x] **Phase 8**: WebSocket拡張
- [x] **Phase 9**: UI拡張（9.1-9.2完了、9.3-9.5延期）
- [x] **Phase 10**: E2Eテスト（10.1完了✅ 23.49秒緑化、10.2-10.7→Phase 13）
- [x] **Phase 11**: 品質保証・診断（11.5完了、他延期）
- [x] **Phase 12**: ドキュメント・リリース準備

**Phase 1-12の詳細タスク**: `tasks-old.md`参照（982行）

---

### Phase 13詳細タスク（Re-scoping、2025-10-20更新）

#### 13.1 Rust E2Eテスト実装 ✅ 4/7完了（CI依存2タスクを別SPEC移行）

- [x] 13.1.1: Task 10.1 - VAD→STT完全フローE2E（✅ 完了）
- [x] 13.1.2: Task 10.2 - オフラインモデルフォールバックE2E（✅ 完了）
- [ ] 13.1.3: Task 10.3 - 動的モデルダウングレードE2E（P13-PREP-001完了後に実施）
- [ ] 13.1.4: Task 10.4 - デバイス切断/再接続E2E（Phase 1完了、Phase 2実施中）
- [x] 13.1.5: Task 10.5 - クロスプラットフォーム互換性E2E（→ **meeting-minutes-ci spec移行**）
- [x] 13.1.6: Task 10.6 - 非機能要件E2E（✅ 完了）
- [x] 13.1.7: Task 10.7 - IPC/WebSocket後方互換性E2E（✅ 完了）

#### 13.2 長時間稼働テスト ✅ 完了（2025-10-20）

- [x] 13.2.1: Task 11.3 - 長時間稼働テストスクリプト作成・修正（✅ 完了）
  - `long_running_monitor.py` 作成（psutil監視、30秒間隔サンプリング）
  - ✅ Windows互換性修正（`arg.replace('\\', '/').endswith(...)` でクロスプラットフォーム対応）
  - ✅ 2時間連続テスト実施完了（2025-10-20 00:44）
- [x] 13.2.2: メモリリーク検証（✅ 合格）
  - **テスト結果**: 2.0時間（7209秒）、231サンプル
  - **Pythonメモリ**: 1104.3MB → 29.6MB（成長: -1074.6MB）
  - **判定**: ✅ メモリリークなし（100MB閾値を大幅にクリア）
  - **詳細**: `test_results/task_11_3_production.json`
- [x] 13.2.3: 長時間稼働ログ分析（✅ 完了）
  - **burn-inイベント統計**: 239イベント（no_speech: 224, speech_start/end: 3, partial/final_text: 9）
  - **IPC正常動作**: 音声イベントが正常に処理されている
  - **詳細**: `logs/platform/1760879038-burnin.log`
  - **制約**: Rust側プロセス監視は未実装（MVP2で対応予定）
    - burn-inバイナリ名が`stt_burn_in`のため、monitoring scriptで検出できず
    - Pythonメモリのみで安定性を確認（Rustは別途手動確認可能）

#### 13.3 セキュリティ修正 ✅ 4/5完了

- [x] 13.3.1: SEC-001 - pip-audit導入（30分、GHSA-4xh5-x5gv-qwph除外設定完了）
- [x] 13.3.2: SEC-002 - CSP設定強化（1時間、manifest.json更新完了）
- [ ] 13.3.3: SEC-003 - Windows ACL設定（1時間、CI整備後に実装）
- [x] 13.3.4: SEC-004 - cargo-audit導入（✅ 完了、Rust beta 1.91.0使用、16件warning（脆弱性0件））
- [x] 13.3.5: SEC-005 - TLS証明書検証（MVP1では未使用、将来実装）

---

### MVP2 Phase 0: 残タスク完了（Phase 13延期分）

**目的**: Phase 13延期タスクを完了し、MVP2本体開始準備を整える

**Week 1（1.5日）**:
- [ ] SEC-003: Windows ACL設定（1h）
- [x] SEC-004: cargo-audit実行（✅ 完了、Rust beta 1.91.0使用、16件warning（脆弱性0件））
- [x] Task 11.3: 2時間連続録音テスト本番実施（✅ 完了、メモリリークなし）

**Phase 13 Re-scoping (2025-10-20)**: CI依存タスクを別SPEC分離

**Week 2-3（2-3日、CI不要）**:
- [ ] P13-PREP-001: Python API追加（Task 10.3準備、2-3h）
- [x] P13-PREP-002: STT-REQ-004.11仕様確定（✅ 完了、元々定義済み）
- [ ] Task 10.3: 動的モデルダウングレードE2E（3h、P13-PREP-001完了後）
- [ ] Task 10.4完遂: 本番再接続ロジック実装（3-4h）
  - **Phase 1完了済み（下地）**:
    - ✅ FakeAudioDevice拡張（`src/audio.rs:51-88`）
    - ✅ 単体テスト6シナリオ（`tests/device_disconnect_e2e.rs`）
  - **Phase 2（本番実装、BLOCKING）**:
    - ❌ `commands.rs:193`リトライループ実装（最大3回、5秒間隔、`start_recording`再実行）
    - ❌ AppState統合テスト（DeviceGone → 自動再接続検証）
  - **STT-REQ-004.11ステータス**: 🔴 Phase 2未実装

**別SPEC移行（meeting-minutes-ci）**:
- [ ] CI/CD整備（GitHub Actions、クロスプラットフォームマトリックス、2-3日）
- [ ] Task 10.5: クロスプラットフォームE2E（6h、CI整備後に実施）
- [ ] SEC-003: Windows ACL設定（1h、Windows CI環境構築後）

**合計推定**: 6-7.5日



## Post-MVP1 Cleanup Tasks

MVP1実装完了後の技術的負債とコードクリーンアップタスク。これらは機能動作には影響しないが、コード品質とメンテナンス性を向上させます。

- [ ] 14. レガシーIPCプロトコルの削除
- [ ] 14.1 LegacyIpcMessage完全削除の検討
  - **状況**: `python_sidecar.rs` の `LegacyIpcMessage` enum が deprecated 警告を大量出力（9件）
  - **現状**: MVP0互換レイヤとして保持中。新プロトコル（`ipc_protocol::IpcMessage`）への完全移行済み
  - **選択肢**:
    1. **完全削除（推奨）**: MVP0互換性が不要なら、`LegacyIpcMessage` 定義と変換ロジックを削除
       - `src/python_sidecar.rs` L76-138: `impl LegacyIpcMessage` ブロック全削除
       - `ProtocolMessage::from_legacy()` ヘルパー削除
       - すべて `ipc_protocol::IpcMessage` に統一
    2. **局所抑制**: 互換性維持が必要なら `#[allow(deprecated)]` を付けて警告抑制
       ```rust
       #[allow(deprecated)]
       impl LegacyIpcMessage {
           pub fn to_protocol_message(self) -> ProtocolMessage { ... }
       }
       ```
  - **判断基準**: Python側（`python-stt/main.py`）とテストがすべて新プロトコル使用済み → 完全削除可能
  - **作業ステップ**（完全削除の場合）:
    1. `grep -r "LegacyIpcMessage" src/` で全参照箇所を確認
    2. `src/python_sidecar.rs` から `LegacyIpcMessage` enum定義を削除
    3. 変換ロジック（`to_protocol_message()`, `from_legacy()`）を削除
    4. `cargo check` でコンパイルエラーがないことを確認
    5. `cargo test --all` で全テスト通過確認（MVP0互換テストが失敗する場合は削除）
  - _Requirements: STT-REQ-007 (IPCバージョニング), コード品質向上_
  - _Priority: P2（機能影響なし、警告ノイズ削減）_

- [ ] 14.2 未使用コード削除
  - **src/commands.rs** の dead code 警告（2件）対応:
    1. `use crate::audio_device_adapter::AudioDeviceAdapter;` - 未使用import削除
       - 静的列挙実装（Task 2.2）でtrait使用を廃止したため
       - `AudioDeviceEvent` は使用中のため残す
    2. `async fn start_ipc_reader_task(...)` - 未使用関数の削除または保留判断
       - フェーズ10で使用予定なら `#[allow(dead_code)]` を付ける
       - 使用予定がないなら削除
  - **作業ステップ**:
    1. `src/commands.rs:7` の import を修正:
       ```rust
       // Before
       use crate::audio_device_adapter::{AudioDeviceAdapter, AudioDeviceEvent};
       // After
       use crate::audio_device_adapter::AudioDeviceEvent;
       ```
    2. `start_ipc_reader_task()` の扱いを判断:
       - 削除: フェーズ10で不要と確定した場合
       - 保留: `#[allow(dead_code)] async fn start_ipc_reader_task(...) { ... }`
    3. `cargo check` で警告が消えたことを確認
  - _Requirements: コード品質向上_
  - _Priority: P2（機能影響なし、警告ノイズ削減）_

- [ ] 14.3 クリーンビルド検証
  - **目的**: 上記2タスク完了後、警告ゼロでビルド通過することを確認
  - **作業ステップ**:
    1. `cargo clean` でクリーンビルド
    2. `cargo check --all-targets` で警告が11件 → 0件に減少することを確認
    3. `cargo test --all` で全テスト通過確認（44テスト以上）
    4. `cargo clippy -- -D warnings` で Clippy警告もゼロに
  - _Requirements: コード品質向上_
  - _Priority: P2（MVP1機能完成後の品質改善）_

**Note**: これらのタスクはMVP1機能に影響を与えません。優先度P2として、MVP1完了後またはリファクタリングフェーズで実施することを推奨します。



---

## 実装優先順位

### Phase 13実装順序（推奨）

1. **🔴 13.3 セキュリティ修正**（最優先、5時間）
   - 本番リリース前必須
   - SEC-001/002/003/005を即座に修正

2. **🟡 13.2 長時間稼働テスト**（1日）
   - リリース前必須
   - 2時間連続録音、メモリリーク検証

3. **🔵 13.1 Rust E2Eテスト**（3-4日）
   - 品質保証、並行作業可能
   - 13.1.4（クロスプラットフォーム）は最後（実機環境必要）

---

## 完了基準

### Phase 13完了基準（Re-scoping後、2025-10-20）
- [ ] 13.1: Rust E2Eテスト5件完了（Task 10.3, 10.4のみ、**10.5はCI spec移行**）
- [x] 13.2: 2時間連続録音成功、メモリリークなし（✅ 完了）
- [ ] 13.3: SEC-001/002/005修正完了（**SEC-003/004はCI spec移行**）
- [ ] 全テスト合格（Rust + Python、CI依存テスト除く）

**CI spec移行タスク** (`meeting-minutes-ci`):
- Task 10.5: クロスプラットフォームE2E
- SEC-003: Windows ACL設定
- SEC-004: cargo-audit（Rust 1.85リリース待ち）

### リリース判定基準
- [ ] Phase 13完了
- [ ] セキュリティ脆弱性0件（SEC-004除く、Rust 1.85待ち）
- [ ] クロスプラットフォーム動作確認（macOS/Windows/Linux）
- [ ] 2時間以上の連続録音成功

---

## 次のステップ

### Phase 13開始前
1. ✅ Phase 13タスク定義完了（本ドキュメント Phase 13セクション参照）
2. ⏸️ spec.json更新（phase: "verification"）
3. ⏸️ MVP2-HANDOFF.md用語統一（"MVP2 Phase 0" → "Phase 13"）

### Phase 13完了後
1. spec.json更新（phase: "completed"）
2. **meeting-minutes-docs-sync**（MVP2本体）spec初期化
3. Google Docs同期機能実装開始

---

## 参考資料

- **詳細タスク**: 本ドキュメント「Phase 13: 検証負債解消 (2025-10-19開始)」セクション参照
- **元のタスク一覧**: [tasks-old.md](./tasks-old.md)（Phase 1-12詳細、982行）
- **セキュリティレポート**: [security-test-report.md](./security-test-report.md)
- **MVP2申し送り**: [MVP2-HANDOFF.md](./MVP2-HANDOFF.md)
- **ADR実装レビュー**: [adr-implementation-review.md](./adr-implementation-review.md)

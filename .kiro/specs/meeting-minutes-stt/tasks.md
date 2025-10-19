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

### Phase 13: 検証負債解消 ⏸️ 未開始

**目的**: MVP1で延期した検証タスクを完了させ、本番リリース可能な状態にする

**サブタスク**:
- **13.1**: Rust E2Eテスト実装（Task 10.2-10.7、7テスト）
- **13.2**: 長時間稼働テスト（Task 11.3、2時間録音）
- **13.3**: セキュリティ修正（SEC-001〜005、5件）

**推定作業量**: 5-7日

**詳細**: 📄 **[tasks/phase-13-verification.md](./tasks/phase-13-verification.md)**

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

### 新規Phase（Phase 13: 検証負債解消）

**Phase 13の詳細タスク**: 📄 **[tasks/phase-13-verification.md](./tasks/phase-13-verification.md)**

#### 13.1 Rust E2Eテスト実装（27時間 = 3-4日）

- [ ] 13.1.1: Task 10.2 - オフラインモデルフォールバックE2E（4h）
- [ ] 13.1.2: Task 10.3 - 動的モデルダウングレードE2E（6h）
- [ ] 13.1.3: Task 10.4 - デバイス切断/再接続E2E（5h）
- [ ] 13.1.4: Task 10.5 - クロスプラットフォーム互換性E2E（6h）
- [ ] 13.1.5: Task 10.6 - 非機能要件E2E（3h）
- [ ] 13.1.6: Task 10.7 - IPC/WebSocket後方互換性E2E（3h）

#### 13.2 長時間稼働テスト（1日）

- [ ] 13.2.1: 2時間連続録音テスト
- [ ] 13.2.2: メモリリーク検証
- [ ] 13.2.3: 長時間稼働ログ分析

#### 13.3 セキュリティ修正（5時間）

- [ ] 13.3.1: SEC-001 - pip 25.0脆弱性修正（30分）
- [ ] 13.3.2: SEC-002 - CSP設定（1時間）
- [ ] 13.3.3: SEC-003 - ファイル権限強制（1時間）
- [ ] 13.3.4: SEC-005 - TLS 1.0/1.1接続失敗テスト（2時間）
- [ ] 13.3.5: SEC-004 - cargo-audit実施（Rust 1.85待ち、30分）

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

### Phase 13完了基準
- [ ] 13.1: Rust E2Eテスト7件全合格
- [ ] 13.2: 2時間連続録音成功、メモリリークなし
- [ ] 13.3: SEC-001/002/003/005修正完了、SEC-004待機中
- [ ] Windows/Linux実機検証完了（platform-verification.md更新）
- [ ] 全テスト合格（Rust 78 + Python 143 = 221テスト）

### リリース判定基準
- [ ] Phase 13完了
- [ ] セキュリティ脆弱性0件（SEC-004除く、Rust 1.85待ち）
- [ ] クロスプラットフォーム動作確認（macOS/Windows/Linux）
- [ ] 2時間以上の連続録音成功

---

## 次のステップ

### Phase 13開始前
1. ✅ Phase 13タスク定義完了（tasks/phase-13-verification.md）
2. ⏸️ spec.json更新（phase: "verification"）
3. ⏸️ MVP2-HANDOFF.md用語統一（"MVP2 Phase 0" → "Phase 13"）

### Phase 13完了後
1. spec.json更新（phase: "completed"）
2. **meeting-minutes-docs-sync**（MVP2本体）spec初期化
3. Google Docs同期機能実装開始

---

## 参考資料

- **詳細タスク**: [tasks/phase-13-verification.md](./tasks/phase-13-verification.md)
- **元のタスク一覧**: [tasks-old.md](./tasks-old.md)（Phase 1-12詳細、982行）
- **セキュリティレポート**: [security-test-report.md](./security-test-report.md)
- **MVP2申し送り**: [MVP2-HANDOFF.md](./MVP2-HANDOFF.md)
- **ADR実装レビュー**: [adr-implementation-review.md](./adr-implementation-review.md)

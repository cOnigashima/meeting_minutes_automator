# MVP1 → MVP2 申し送りドキュメント

**作成日**: 2025-10-19
**最終更新**: 2025-10-21
**作成/更新**: Claude（初稿） / Codex（ドキュメント再編）
**ステータス**: ✅ Phase 13+14完了、MVP2移行準備完了

**Purpose**: Phase 13+14完了後の成果物と、meeting-minutes-ci/meeting-minutes-docs-syncへの引き継ぎ事項を整理する。

---

## 1. Phase 13+14 完了サマリー（2025-10-21）

### Phase 13: 検証負債解消（✅ 完了）
- ✅ Task 10.1-10.4: E2Eテスト完全実装（VAD→STT、オフラインフォールバック、動的ダウングレード、デバイス再接続）
- ✅ Task 11.3: 2時間連続録音テスト（メモリリークなし）
- ✅ SEC-001/002/005: macOSセキュリティ修正完了
- ✅ P13-PREP-001: Python API追加（Task 10.3準備）

### Phase 14: Post-MVP1 Cleanup（✅ 完了）
- ✅ LegacyIpcMessage完全削除（tests/supportモジュールパターン採用）
- ✅ P0バグ修正（5秒タイムアウト削除、unit test更新）
- ✅ Rust 0 warnings達成
- ✅ Known Limitations文書化（ADR-018）

### テスト結果
- Unit: 76/76 ✅
- Integration: 31/31 ✅
- Build warnings: 0 ✅
- Python: 161/178（17件既存失敗、MVP1と無関係）

---

## 2. 次工程への引き継ぎ

### meeting-minutes-ci spec（CI/CD Pipeline）

**引き継ぎ済みタスク**（requirements.md L15-19, CI-INTAKE-001/002/003）:
- Task 10.5: クロスプラットフォームE2E（Windows/Linux検証、6h）
- SEC-003: Windows ACL設定（1h）
- SEC-004: cargo-audit継続監視（Rust 1.85待ち）

**次アクション**:
1. `/kiro:spec-tasks meeting-minutes-ci` 実行（タスク分解）
2. GitHub Actions matrix整備（macOS/Windows/Linux）
3. CI-INTAKE-001/002/003の実装

**参照**: `../meeting-minutes-ci/requirements.md`

### meeting-minutes-docs-sync spec（MVP2本体: Google Docs連携）

**状態**: design-validated（requirements/design承認済み、tasks未生成）

**前提条件**:
- ✅ meeting-minutes-stt Phase 13+14完了
- ⏸️ meeting-minutes-ci整備（並行実施、またはP2として扱う）

**次アクション**:
1. `/kiro:spec-tasks meeting-minutes-docs-sync` 実行（タスク分解）
2. MVP2 Phase 0実装開始（OAuth 2.0認証、Google Docs API統合）

**参照**: `../meeting-minutes-docs-sync/spec.json`

---

## 3. Known Limitations（ADR-018）

以下の制約は文書化済み、MVP2ブロッカーではない（優先度P2）:
- LIMIT-001: IPC Reader Mutex scope制約（send操作への影響は最小限）
- LIMIT-002: tests/support重複インポート（現状2ファイルのみ、管理可能）
- LIMIT-003: Python 17件既存失敗（model detection/upgrade tests、MVP1と無関係）

**詳細**: `.kiro/specs/meeting-minutes-stt/adrs/ADR-018-phase14-known-limitations.md`

---

## 4. Reference Documents

- `./tasks.md` — Phase 13+14完了記録
- `./phase-13-re-scoping-rationale.md` — Phase 13再スコープ決定記録
- `./adrs/ADR-018-phase14-known-limitations.md` — Known Limitations詳細
- `../meeting-minutes-ci/requirements.md` — CI引き継ぎタスク定義
- `../meeting-minutes-docs-sync/spec.json` — MVP2本体ステータス

---

**Next milestone**:
1. `/kiro:spec-tasks meeting-minutes-ci` → CI実装計画確定
2. `/kiro:spec-tasks meeting-minutes-docs-sync` → MVP2実装開始

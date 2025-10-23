# Design Artifacts - meeting-minutes-docs-sync

**目的**: Phase 0（設計検証）専用の成果物置き場

## Contents

### responsibility-matrix.md
全19クラスの責務定義（1行要約）とメトリクス。

**使用タスク**:
- Task 0.9（設計検証チェックリスト実行）でSOLID原則準拠を確認
- Task 0.10（Phase 0成果物レビュー）で承認判定

**メトリクス**:
- 公開メソッド数（5個以下を推奨）
- プライベートメソッド数（2個以下を推奨）
- テスト容易性（⭐4以上が80%以上を推奨）

### interface-contracts.md
全19インターフェースの契約定義（事前条件・事後条件・エラー型）。

**使用タスク**:
- Task 0.2（インターフェース契約定義の完成）で17インターフェース追加
- Task 0.10（Phase 0成果物レビュー）で承認判定

**フォーマット**:
```typescript
/**
 * @preconditions なし
 * @postconditions 認証コードが返される
 * @throws UserCancelledError ユーザーがキャンセル
 * @returns Result<認証コード, AuthFlowError>
 */
```

---

## Note

### ダイアグラムの保存場所

**クラス図・シーケンス図・コンポーネント図はすべて `docs/uml/<spec>/<category>/` に統一されています。**

- ❌ ~~`design-artifacts/class-diagrams/`~~ → 削除済み（2025-10-24に `docs/uml/meeting-minutes-docs-sync/cls/` へ移動）
- ❌ ~~`design-artifacts/sequence-diagrams/`~~ → 削除済み（空ディレクトリのため削除）
- ✅ **新規ダイアグラム作成先**: `docs/uml/meeting-minutes-docs-sync/<category>/`
  - `cls/`: クラス図（Class Diagrams）
  - `seq/`: シーケンス図（Sequence Diagrams）
  - `cmp/`: コンポーネント図（Component Diagrams）
  - `act/`: アクティビティ図（Activity Diagrams）
  - `state/`: 状態図（State Machine Diagrams）

詳細は [CLAUDE.md#ダイアグラム管理規則](../../../../CLAUDE.md) を参照。

---

## References

- [phase-0-design-validation.md](../task-details/phase-0-design-validation.md): Phase 0タスク詳細
- [docs/uml/meeting-minutes-docs-sync/cls/](../../../../docs/uml/meeting-minutes-docs-sync/cls/): クラス図（既存3ファイル）
- [CLAUDE.md#ダイアグラム管理規則](../../../../CLAUDE.md): プロジェクト全体のダイアグラム管理ルール

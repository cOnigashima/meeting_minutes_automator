# Spec Authoring Guide — SOLID & Clean Design Edition

このドキュメントは、Kiro ベースの仕様作成フローにおいて **読み手別に一貫性があり、変更に強く、拡張しやすい** 要件・設計ドキュメントを書くための実務ガイドです。
コード規約は `docs/dev/coding-standards.md`、全体フローは `CLAUDE.md` を参照してください。

---

## 1. 目標と読者

- **目的**: 仕様を **変更容易性（M）、検証容易性（V）、追跡可能性（T）** の観点で最適化する。
- **読者**: PM/PO、アーキテクト、実装者、テスター、SRE。
- **成果**: 誤読のない要件、矛盾しない設計、テスト可能な受け入れ条件、双方向トレーサビリティ。

---

## 2. 仕様フェーズの基本フロー（強化版）

1. `/kiro:spec-init <feature>`
   - テンプレート生成、著者・レビューア・期日を front-matter に記載。
2. `/kiro:spec-requirements <feature>` 直後
   - **ID採番**: `REQ-###`, `NFR-###`, `ARC-###`, `PRO-###`, `DEL-###`, `CON-###`, `FUT-###`。
   - **EARS 構文**で受入条件を記述（テンプレートは §4 参照）。
   - NFRはNon Functional Requirement（非機能要件）
   　- 品質の標準化:システムが満たすべき品質属性を明確にし、開発者と利用者の認識のズレを防ぎます。
   　- システムの実用性向上:パフォーマンス不足やセキュリティ欠陥、使いにくさといった、完成したシステムが「使えない」状況を防ぎます。﻿
   　- テストの具体化:非機能要件を測定可能な形で記述することで、具体的なテストケースの作成が可能になります。﻿
   - 末尾に **Requirement Traceability Matrix** を作成（親/子/テスト/図へのリンク）。
3. `/kiro:validate-requirements <feature>`
   - ID 一覧、トレーサビリティ表、`spec.json.approvals.requirements` を確認。NO-GO は修正。
4. `/kiro:spec-design <feature>`
   - `docs/uml/<spec>/` に **PlantUML 図**（コンポーネント/シーケンス/配置/状態）を追加。
   - 設計各節に **関連要件 ID** を明記。齟齬は要件へ差し戻し。
   - `/kiro:validate-design <feature>`もあります
5. `/kiro:spec-tasks <feature>`
   - タスクに **要件 ID を必ず紐付け**、作業中も ID を維持（変更は Deprecated 運用）。
   - `/kiro:validate-tasks <feature>`もあります

**ゲート（Definition of Ready / Done）**
- **DoR (Requirements)**: EARS完了 / NFRシナリオ化 / 競合なし / テストラベル付与 / トレーサビリティ更新済み
- **DoD (Design)**: 図と契約が一致 / 例外・失敗時挙動を記述 / AdapterとPortを分離 / テスト観点が列挙済み

---

## 3. チェックリスト

### 3.1 要件ドキュメント
- [ ] `requirements.md` に目次・章構成
- [ ] 各章の **ID プレフィックス**（Umbrella は `REQ-`、サブ spec は `CORE-REQ-` 等）
- [ ] **EARS 構文**で Acceptance Criteria を ID 付きで列挙
- [ ] `NFR-`/`CON-`/`DEL-`/`PRO-` など非機能・制約・成果物・プロセス ID を付与
- [ ] **トレーサビリティ表**：親⇔子、テスト、図、ADR をリンク
- [ ] `spec.json.approvals.requirements.generated = true`

### 3.2 設計ドキュメント
- [ ] 各節に **関連要件 ID**
- [ ] **Port/Adapter** と **Contract**（事前・事後・不変）を明記
- [ ] 図は `docs/uml/<spec>/ID_title.puml` に配置、1図=要素 5±2
- [ ] エラー処理・フォールバック・リトライ・レート制御の方針
- [ ] **観測可能なメトリクス** とテレメトリイベント

- 設計セクションごとに参照元の要件 ID を記載（例: 「関連要件: REQ-001.2」）
- 図の更新・新規追加ごとに `Requirement Traceability Matrix` の Notes に記録（例: 「図 `CMP-001` で REQ-005 を補足」）
- 設計の段階で要件に不足が見つかった場合は requirements.md を更新し、Traceability表も合わせて調整

---

## 4. テンプレート集（貼って使える雛形）

### 4.1 EARS 受け入れ条件（最小形）

```markdown
- **REQ-001.1**: WHEN <前提/イベント> THEN システム SHALL <期待動作> [WITH <計測基準>].
  - Rationale: <なぜ必要か>
  - Notes: <補足/制約>
```

### 4.2 NFR（Quality Attribute Scenario）

```markdown
- **NFR-Perf-001** (Performance):
  - Source: <誰が/何が>（例: ユーザー/バックグラウンド処理）
  - Stimulus: <何が起きる>（例: 100同時リクエスト）
  - Artifact: <対象>（例: Docs 同期 API）
  - Environment: <条件>（例: 通常負荷/回線遅延 200ms）
  - Response: <応答>（例: バッファリングして batchUpdate 実行）
  - Response Measure: <測定>（例: p95 ≤ 2s, エラー率 ≤ 0.1%）
```

### 4.3 ADR（Architecture Decision Record）

```markdown
# ADR-XXX: <決定の題名>
- Date: YYYY-MM-DD
- Status: Proposed | Accepted | Superseded by ADR-YYY
- Context: <背景/制約/選択肢>
- Decision: <採用方針>
- Consequences: <トレードオフ/負債/影響範囲>
- Links: REQ-### / NFR-### / 図 / PR
```

### 4.4 Port/Adapter（依存性逆転の型）

```markdown
- **Port (抽象契約)**: 仕様はこの契約に依存する（ステートレス/API署名/制約）
- **Adapter (実装詳細)**: ベンダー・SDK・REST 呼び出しをここに隔離
- **Invariants**: 入出力前提、レート制限、再試行ポリシー
```

---

## 5. トレーサビリティ運用（剛性のある履歴）

- Umbrella にマトリクスを置き、**サブ spec 採番時は相互更新**。
- 互換性を壊す変更は **`Deprecated` 列** を追加し **旧→新 ID** を明記。
- 図の更新は **Notes** に記録（例: 「`CMP-001` で `REQ-005` を補足」）。
- 自動検証: `/kiro:validate-*` で ID 整合とリンク切れチェック。
- 仕様変更でIDが無効になった場合は `Deprecated` 列を追加し、旧ID→新IDの移行を明記
- レビュー時 (`/kiro:validate-requirements`, `/kiro:validate-design`) はマトリクスの整合確認を必須項目にする

---

## 6. レビュー・ゲートとチェックポイント

- `/kiro:validate-requirements`：ID 付与 / EARS / トレーサビリティ / NFR シナリオ
- `/kiro:validate-design`：各節が要件 ID を参照 / Port/Adapter 分離 / 契約と図の一致
- 手動レビューでは **CLAUDE.md** のワークフロー節と本ガイドを併読

**レビュー観点（抜粋）**
- 要件⇔設計⇔テストの **三角整合**
- **失敗時の挙動** が仕様化されているか（例外/リトライ/バックオフ）
- **可観測性**（ログ/メトリクス/イベント）で NFR が検証可能か
- **抽象に依存** しているか（具体実装に引きずられていないか）

---

## 7. 反パターン（Spec Smells）

- 混在: 要件章で実装手段を詳述 / 設計章でビジネス要求を再定義
- 不可観測: NFR が「速い」「安定」など **測れない表現**
- 巨大 ID: 1 ID に複数意図（切り出して粒度を揃える）
- ベンダー縛り: 仕様が特定 SDK の例外仕様に依存
- 図の漂流: 図がテキストと **発散**（更新責任者不在）
- 暗黙の前提: 依存・制約・既知課題の未記載

---

## 8. 自動化と運用（Spec as Code）

- **プリコミット**: ID 形式 / リンク有効性 / 章テンプレ検証（EARS/NFR/ADR の雛形）
- **CI**: `/kiro:validate-*` を必須ゲート化、図の差分チェック、用語集の整合
- **バージョニング**: `spec.json` に semver と approvals を保持、派生 doc に埋め込み
- **観測**: リリースごとに「要件→テスト」の達成率ダッシュボード

---

## 9. ドキュメント間の参照

- CLAUDE.md: フェーズ別ワークフローとコマンドの使い方を参照。仕様作業を開始する前に確認する
- docs/dev/coding-standards.md: 実装に入る際のコーディング規約・テスト方針。仕様作成時は軽く把握しつつ、実装フェーズで詳細を参照
- *.kiro/specs/<feature>/requirements.md: 実際の要件本文（IDとTraceability表を維持）
- *.kiro/specs/<feature>/design.md: 設計文書。要件 ID を参照しながら作業


---

## 10. よくある更新例

- **ID を追加**: `requirements.md` へ追記 → マトリクス反映 → `spec.json` 確認
- **サブ spec で採番**: `CORE-REQ-###` 等 → 親のマトリクス更新
- **実装後に履行結果を追記**: `design.md` / `README` に結果を記録、必要なら要件へ `Status` 列追加


---

## 11. 今後の拡張（TODO）

- トレーサビリティ表の自動生成スクリプト
- `/kiro:validate-requirements` の ID 整合チェック強化
- PR テンプレートに **要件 ID チェックボックス** を追加
- Spec Lint（EARS/NFR/ADR の静的検証と観測性ルール）

---

### 付録 A: ミニガイド（1 分で確認）

- **まず要件**（EARS）→ **次に設計**（Port/Adapter, 契約）→ **最後に図**（5±2 ルール）
- **抽象に依存**、**具体は Adapter**、**変更は ID 追加**、**互換性破壊は ADR**
- **NFR はシナリオで測る**、**ログ/メトリクスで観測**
- **DoR/DoD を満たすまでゲート通過しない**






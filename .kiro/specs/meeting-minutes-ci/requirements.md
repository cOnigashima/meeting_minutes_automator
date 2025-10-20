# Requirements Document

## 0. Intake Sources（2025-10-20）

- **移管元 Decision Record**: `.kiro/specs/meeting-minutes-stt/phase-13-re-scoping-rationale.md`  
- **ハンドオフ概要**: `.kiro/specs/meeting-minutes-stt/MVP2-HANDOFF.md`  
- **移行前タスク管理原本**: `.kiro/specs/meeting-minutes-stt/tasks.md`（Phase 13 セクション）

> 運用ルール: meeting-minutes-ci spec内でタスクの状態（着手/完了/延期）を更新したら、上記STT側ドキュメントには概要のみ反映する。詳細の一次情報は本requirementsおよび今後生成する`tasks.md`で管理する。

---

## 1. Scope Intake Summary

| Transfer ID | Source Task (meeting-minutes-stt) | New Owner (this spec) | 背景/目的 | 引き継ぎ済み成果物 | 初期状態 |
|-------------|----------------------------------|-----------------------|-----------|--------------------|----------|
| CI-INTAKE-001 | Task 10.5: クロスプラットフォーム互換性E2E | CIチーム | Windows/Linux検証にはGitHub Actions整備が必須。Phase 13でブロックしたため本specへ移行。 | STT側E2Eテストケース草案（`tests/dynamic_model_downgrade_e2e.rs`）、macOS手動検証ログ | 未着手 |
| CI-INTAKE-002 | SEC-003: Windows ACL設定 | CIチーム（SecOps連携） | Windowsランナー上でのファイルパーミッション検証が必要。CI環境で自動化する。 | セキュリティ要件: `security-test-report.md` 該当節 | 未着手 |
| CI-INTAKE-003 | SEC-004: cargo-audit継続監視 | CIチーム | cargo-auditの定期実行をパイプラインへ組み込む。Rust 1.85安定版待ち。 | 暫定スクリプト（`scripts/security/cargo_audit.sh`、手動実行ログ） | ブロック中（Rust 1.85待ち） |

---

## 2. Cross-Spec Coordination Rules

1. **ステータス更新フロー**  
   - meeting-minutes-ci側で進捗を更新 → 本requirementsまたは将来の`tasks.md`へ反映。  
   - `meeting-minutes-stt/tasks.md` には「CI specへ移管済み」「進捗はCI spec参照」とする一行サマリーのみ追記。  

2. **レビュー/合意**  
   - Phase 13 Decision Recordに影響する仕様変更が発生した場合、STT担当者と合意した上でDecision Recordへ追記する。  
   - CI specの変更履歴には「出典: Phase 13 Re-scoping（2025-10-20）」を明記。

3. **成果物リンク**  
   - GitHub Actionsワークフロー、セキュリティスクリプトは`meeting-minutes-ci`配下に配置し、STT側からはリンクのみ提供。  
   - 進捗報告は隔週レビュー議事録にCIトラック用セクションを設ける。

---

## 3. Project Description (Input)

GitHub Actions CI/CD Pipeline for Meeting Minutes Automator. Automated testing pipeline with cross-platform matrix (macOS, Windows, Linux). Cost-optimized strategy: Linux-primary continuous testing, selective full-matrix runs on release tags. Includes unit tests, integration tests, E2E tests, security tests, and performance benchmarks. Free tier compliance: 2000 min/month private repo limit, with macOS 10x and Windows 2x multipliers. Supports automated releases, changelog generation, and artifact publishing.

---

## 4. Requirements Seed

この段階では formal requirements 未生成。以下を初期Seedとして /kiro:spec-requirements フェーズで展開する。

- **REQ-CI-001**: クロスプラットフォームE2E検証をGitHub Actions matrixで自動実行すること（macOS/Windows/Linux）。  
- **REQ-CI-002**: Windowsランナー上でSEC-003のACL検証を自動化し、成果レポートをアーティファクト化すること。  
- **REQ-CI-003**: cargo-auditを週次で自動実行し、Rust 1.85リリース後に安定版で警告ゼロを保証すること（SEC-004）。  
- **REQ-CI-004**: STT specから移行したタスクの状態をダッシュボード（`tasks.md`予定）で可視化し、STT側へ自動通知すること（Slack/メール T.B.D.）。  

> 次アクション: requirements生成時に上記Seedを基に詳細化し、`spec.json` の `approvals.requirements.generated` を true に更新する。

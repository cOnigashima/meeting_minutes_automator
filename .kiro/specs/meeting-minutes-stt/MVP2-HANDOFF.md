# MVP1 → MVP2 申し送りドキュメント

**作成日**: 2025-10-19  
**最終更新**: 2025-10-20  
**作成/更新**: Claude（初稿） / Codex（ドキュメント再編）  
**ステータス**: 運用中（Phase 13 wrap-up in progress）

**Purpose**: Phase 13で残る検証ワークと、MVP2着手前に引き継ぐToDo・所要時間を整理する。スコープ定義は`./phase-13-re-scoping-rationale.md`（Decision Record）、最新ステータスは`./tasks.md`（運用原本）を参照。

---

## 1. Phase 13 Snapshot（2025-10-20）

- **スコープ基準**: [`phase-13-re-scoping-rationale.md`](./phase-13-re-scoping-rationale.md) を公式Decision Recordとして採用。  
- **完了済み**: Task 10.3 動的モデルダウングレードE2E、13.2 長時間稼働テスト（Task 11.3）、SEC-001/002/005（macOSセキュリティ）。  
- **継続中**: Task 10.4 デバイス切断/再接続E2E Phase 2（自動再接続の本番検証）。  
- **CI specへ移行済み**: Task 10.5、SEC-003、SEC-004 → `meeting-minutes-ci`で管理。  
- **最新タスク原本**: [`tasks.md`](./tasks.md) の「Phase 13: 検証負債解消」節。更新は同ファイルのみで実施。  
- **MVP2開始判定**: Phase 13完了（Task 10.4達成）とCI側の着手計画を確認後、MVP2 Phase 0（Google Docs連携）へ移行。

---

## 2. MVP2 Handoff Work Queue

| 区分 | ToDo | 所要時間目安 | 担当/参考 |
|------|------|--------------|-----------|
| Phase 13 wrap-up | Task 10.4 Phase 2実装（`commands.rs`再試行ループ、AppState統合テスト）とリグレッション確認 | 3-4h | STTチーム / `./tasks.md` |
| Phase 13補完 | P13-PREP-001 Python API拡張（必要時のみ再開） | 2-3h | STTチーム / `./tasks.md` |
| CI連携トラック | GitHub Actionsマトリックス整備 → Task 10.5 / SEC-003 / SEC-004の順で実行 | 6-7.5日 | CIチーム / `../meeting-minutes-ci/spec.json` |
| MVP2 Kick-off Gate | Phase 13完了レビュー、`./archive/phase-13-completion-report.md`更新、MVP2スプリント計画確定 | 0.5日 | PM / STT + CI合同 |

> メモ: CI連携トラックの詳細タスク分解とスケジュールは`meeting-minutes-ci`側のtasks/requirements生成後に同期する。

---

## 3. Carry-over Risks for MVP2

- **macOS限定リリース**: Phase 13完了後もCI整備完了まではmacOSのみサポート。`phase-13-re-scoping-rationale.md` でリリースノート方針と段階的展開を定義。  
- **クロススペック追跡性**: Task 10.5 / SEC-003 / SEC-004は`meeting-minutes-ci`タスクとして管理。隔週レビューで双方の進捗をクロスチェック。  
- **CI整備遅延**: Windows/Linux対応が後ろ倒しになるリスク。MVP2スプリント計画にCIスプリント（MVP1.5）を組み込み、リソース配分をSTT 50% / CI 30% / Docs 20%で運用。  

---

## 4. Reference Documents

- `./phase-13-re-scoping-rationale.md` — Phase 13再スコープ決定記録  
- `./tasks.md` — Phase 13残作業・所要時間の運用原本  
- `./archive/phase-13-completion-report.md` — 完了レポート雛形  
- `.kiro/specs/meeting-minutes-ci/spec.json` — CIトラックのスコープ定義（tasks生成予定）

---

**Next milestone**: Phase 13 Task 10.4クローズ → CIスプリント計画承認 → MVP2 Phase 0開始

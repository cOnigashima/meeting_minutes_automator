# Phase 13 Re-scoping Rationale

**Document Version**: 1.0
**Created**: 2025-10-20
**Status**: Active
**Decision Authority**: Project Lead

**Purpose**: Phase 13の公式スコープ定義書。本決定はMVP2ハンドオフ資料（`./MVP2-HANDOFF.md`）の前提であり、最新の残作業・ステータスは`./tasks.md`を唯一の運用原本とする。

---

## Executive Summary

Phase 13（検証負債解消）のスコープを再定義し、CI依存タスク（Task 10.5, SEC-003, SEC-004）を`meeting-minutes-ci` specへ分離した。本ドキュメントは技術的根拠、リスク分析、完了基準の変更を記録する。

---

## 1. Re-scoping Decision

### 1.1 Decision Date
**2025-10-20**

### 1.2 Scope Changes

#### **分離タスク** → `meeting-minutes-ci` spec
1. **Task 10.5**: クロスプラットフォーム互換性E2E（6h）
   - 対象OS: Windows, Linux（macOSは既存環境で検証済み）
   - 前提条件: GitHub Actions CI/CD環境整備

2. **SEC-003**: Windows ACL設定（1h）
   - 対象: Windows環境でのファイルシステムセキュリティ
   - 前提条件: Windows CI環境構築

3. **SEC-004**: cargo-audit実行（継続監視）
   - 現状: Rust beta 1.91.0使用、16件warning（脆弱性0件）
   - ブロッカー: Rust 1.85安定版リリース待ち（2025-11予定）

#### Phase 13で維持する作業（meeting-minutes-stt）
- **Task 10.3**: 動的モデルダウングレードE2E  
  - ✅ 2025-10-20時点で完了。再開が必要な場合は`./tasks.md`のPhase 13セクションを更新する。  
- **Task 10.4**: デバイス切断/再接続E2E Phase 2  
  - ⏳ Phase 2実装・自動再接続検証を継続。最新の作業メモと所要時間は`./tasks.md`に統合。  
- **SEC-001/002/005**: macOS環境セキュリティ修正  
  - ✅ 対象ファイル権限・証明書設定は完了済み。再発時は`./tasks.md`でトラッキング。  

## 2. Technical Rationale

### 2.1 Why Separate CI-Dependent Tasks?

#### **Problem**: Blocking Dependency Chain
```
Task 10.5 → CI環境整備（2-3日） → Windows/Linux CI構築 → マトリックステスト実行
   ↓
SEC-003  → Windows CI環境 → ACL設定検証
   ↓
Phase 13完了判定 → ブロック
```

**Impact**:
- STT機能開発（Task 10.3, 10.4）が**CI整備待ちで停滞**
- CI整備は**インフラタスク**であり、STT機能とは関心が異なる

#### **Solution**: Parallel Tracks
```
Track 1 (meeting-minutes-stt):
  Task 10.3 → Task 10.4 → Phase 13完了（2-3日）
  ↓
  MVP1機能開発完了（macOS単一プラットフォーム）

Track 2 (meeting-minutes-ci):
  CI/CD整備 → Task 10.5 → SEC-003 → クロスプラットフォーム対応完了（2-3日）
  ↓
  完全プラットフォーム対応リリース
```

**Benefits**:
- ✅ STT機能開発の**ブロッカー除去**
- ✅ CI整備とSTT開発の**並行作業**可能
- ✅ **関心の分離**（機能開発 vs インフラ）

### 2.2 Why meeting-minutes-ci Spec?

**Existing Spec**: `.kiro/specs/meeting-minutes-ci/`
- **Status**: Initialized (requirements未生成)
- **Scope**: GitHub Actions CI/CD Pipeline for Meeting Minutes Automator
- **Design Goal**: Cross-platform testing matrix, cost optimization, automated releases

**Alignment**:
- Task 10.5（クロスプラットフォームE2E）は**CI環境が必須**
- SEC-003（Windows ACL）は**Windows CI環境が必須**
- 両タスクとも`meeting-minutes-ci`のスコープに**完全一致**

**Alternative Considered**: meeting-minutes-stt内でCI整備
- **Rejected**: STT機能specにインフラ要素を混入させる設計ミス
- **Rationale**: Separation of Concerns原則に違反

---

## 3. Risk Analysis

### 3.1 Key Risks（詳細な緩和策・最新状況は`./tasks.md`で追跡）

- **macOS単一リリース判断**  
  クロスプラットフォーム検証に先行してMVP1をmacOS限定で出荷する意思決定。リスクはユーザーベースの限定とRust依存アップデート待ち。  
  - 対応: リリースノートで対象OSを明記し、CI spec完了後にv1.1でWindows/Linux対応をリリース。  

- **タスク分離による追跡性低下**  
  Task 10.5/SEC-003/SEC-004が`meeting-minutes-ci`に移ることで進捗を見失う可能性。  
  - 対応: 本Decision Recordで移行理由・日付を明示し、`meeting-minutes-ci`側のtasks.mdには「移管元: Phase 13」と記録する。隔週レビュー時に両specをクロスチェック。  

- **CI整備遅延**  
  CI整備そのものが遅延するとWindows/Linux対応が後ろ倒しになる。  
  - 対応: MVP2着手前にCIスプリント（MVP1.5）を設定し、リソース配分をSTT 50% / CI 30% / Docs 20%で固定する。  

---

## 4. Updated Completion Criteria

### 4.1 Phase 13 Completion Criteria (Re-scoped)

#### **Before** (Original)
- [ ] 13.1: Rust E2Eテスト7件全合格
- [ ] 13.2: 2時間連続録音成功、メモリリークなし
- [ ] 13.3: SEC-001/002/003/005修正完了、SEC-004待機中
- [ ] Windows/Linux実機検証完了
- [ ] 全テスト合格（Rust 78 + Python 143 = 221テスト）

#### **After** (Re-scoped, 2025-10-20)
- [ ] 13.1: Rust E2Eテスト5件完了（Task 10.3, 10.4のみ）
  - **除外**: Task 10.5（→ CI spec移行）
- [x] 13.2: 2時間連続録音成功、メモリリークなし（✅ 完了）
- [ ] 13.3: SEC-001/002/005修正完了（macOS環境のみ）
  - **除外**: SEC-003（→ CI spec移行）
  - **除外**: SEC-004（→ CI spec移行）
- [ ] macOS環境で全テスト合格（CI依存テスト除く）

**現在の進捗サマリー（2025-10-20）**: Task 10.3完了・Task 10.4継続中・SEC-001/002/005完了。詳細ステータスは`./tasks.md`を参照し、そちらのみを更新対象とする。

### 4.2 meeting-minutes-ci Spec Completion Criteria (New)
- [ ] CI/CD環境整備（GitHub Actions、クロスプラットフォームマトリックス）
- [ ] Task 10.5: クロスプラットフォームE2E（Windows, Linux）
- [ ] SEC-003: Windows ACL設定
- [ ] SEC-004: cargo-audit（Rust 1.85リリース後）

### 4.3 Full Release Criteria (v1.1以降)
- [x] Phase 13完了（meeting-minutes-stt）
- [ ] meeting-minutes-ci spec完了
- [ ] クロスプラットフォーム動作確認（macOS/Windows/Linux）
- [ ] セキュリティ脆弱性0件（全プラットフォーム）

---

## 5. Communication Plan

### 5.1 Stakeholder Notification
**Date**: 2025-10-20
**Method**: ドキュメント更新 + tasks.md明記

**Updated Documents**:
1. `.kiro/specs/meeting-minutes-stt/tasks.md`
   - Phase 13 Re-scoping (2025-10-20)セクション追加
   - 別SPEC移行タスク明記

2. `.kiro/specs/meeting-minutes-ci/requirements.md`
   - Project Descriptionに移行タスク追加（後日）

3. `README.md` (Root)
   - MVP1リリース範囲の明記（macOS only）（後日）

### 5.2 Transition Plan
**Immediate Actions** (2025-10-20):
- ✅ phase-13-re-scoping-rationale.md作成
- ✅ tasks.md更新（Re-scoping説明追加）

**Follow-up Actions** (Within 1 week):
- [ ] meeting-minutes-ci spec requirements生成
- [ ] Task 10.5移行（設計・テストケース含む）
- [ ] CI spec tasks.md生成

---

## 6. Lessons Learned

### 6.1 What Went Well
- ✅ **早期発見**: CI依存を実装前に特定（手戻り回避）
- ✅ **柔軟な再計画**: スコープ変更を迅速に決断

### 6.2 What Could Be Improved
- ⚠️ **初期スコープ設計**: Phase 13計画時にCI依存を見落とし
- ⚠️ **依存関係分析**: タスク定義時の前提条件チェック不足

### 6.3 Action Items
- [ ] **Improvement 1**: 今後のPhase計画時に**依存関係マトリックス**作成
- [ ] **Improvement 2**: タスク定義テンプレートに**前提条件欄**追加
- [ ] **Improvement 3**: Spec初期化時に**インフラ要素の分離**を明示的にチェック

---

## 7. Approval

**Approved By**: Project Lead
**Date**: 2025-10-20
**Rationale**: Technical debt resolution prioritization, risk mitigation via parallel tracks

---

## 8. References

- **Related Specs**:
  - `.kiro/specs/meeting-minutes-stt/tasks.md`
  - `.kiro/specs/meeting-minutes-stt/MVP2-HANDOFF.md`
  - `.kiro/specs/meeting-minutes-ci/spec.json`

- **Related Requirements**:
  - STT-REQ-006.6-006.12（動的モデルダウングレード）
  - STT-REQ-004.11（デバイス自動再接続）

- **Related ADRs**:
  - ADR-013（Sidecar Full-Duplex Design）

# Phase 13 Re-scoping Rationale

**Document Version**: 1.0
**Created**: 2025-10-20
**Status**: Active
**Decision Authority**: Project Lead

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

#### **Phase 13残存タスク**（meeting-minutes-stt）
1. **Task 10.3**: 動的モデルダウングレードE2E（3h）
2. **Task 10.4**: デバイス切断/再接続E2E Phase 2（3-4h）
3. **SEC-001/002/005**: macOS環境セキュリティ修正（完了済み）

---

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

### 3.1 Identified Risks

#### **Risk 1**: CI未整備でのリリース判断
**Description**: macOS単一プラットフォームでのMVP1リリース時、Windows/Linux動作保証なし

**Likelihood**: High（CI整備が遅延する可能性）
**Impact**: Medium（macOS限定リリースは技術的に可能だが、ユーザーベース制限）

**Mitigation Strategy**:
1. **MVP1リリース範囲の明確化**
   - 対象OS: **macOS only**（明示的に記載）
   - リリースノート: "Windows/Linux support coming in v1.1"

2. **段階的リリース計画**
   - v1.0: macOS MVP1（Phase 13完了後）
   - v1.1: クロスプラットフォーム対応（CI spec完了後）

3. **リリース判定基準**（macOS MVP1）
   - ✅ Phase 13完了（Task 10.3, 10.4）
   - ✅ macOS環境で全テスト合格
   - ✅ 2時間連続録音成功
   - ✅ セキュリティ脆弱性0件（SEC-004除く、Rust 1.85待ち）

#### **Risk 2**: タスク分離による追跡性低下
**Description**: Task 10.5がCI specへ移動後、STT spec側で進捗確認困難

**Likelihood**: Low
**Impact**: Low（ドキュメント管理の問題）

**Mitigation Strategy**:
1. **Cross-Reference Maintenance**
   - `meeting-minutes-stt/tasks.md`: 別SPEC移行タスクを明記
   - `meeting-minutes-ci/tasks.md`: 移行元（STT spec）を記載

2. **Bi-weekly Sync**
   - 両specの進捗を隔週レビューで確認
   - ブロッカー相互依存の早期検出

#### **Risk 3**: CI整備遅延によるクロスプラットフォーム対応遅延
**Description**: CI spec完了が遅延し、Windows/Linux対応が長期間未完

**Likelihood**: Medium
**Impact**: Medium（ユーザーベース拡大遅延）

**Mitigation Strategy**:
1. **CI Spec優先度設定**
   - MVP2（Google Docs同期）と**同等優先度**で並行推進
   - リソース配分: STT 50% / CI 30% / Docs 20%

2. **MVP1.5マイルストーン**
   - MVP1完了後、MVP2着手前に**CI整備スプリント**設定（1週間）
   - 早期クロスプラットフォーム対応でリスク低減

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
  - `.kiro/specs/meeting-minutes-ci/spec.json`

- **Related Requirements**:
  - STT-REQ-006.6-006.12（動的モデルダウングレード）
  - STT-REQ-004.11（デバイス自動再接続）

- **Related ADRs**:
  - ADR-013（Sidecar Full-Duplex Design）

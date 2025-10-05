# Design Principles

## Overview

本ドキュメントでは、Meeting Minutes Automatorプロジェクト全体を貫く設計原則を定義します。これらの原則は、技術的意思決定の指針として機能し、アーキテクチャの一貫性と長期的な保守性を保証します。

## Core Design Principles

### 1. プロセス境界の明確化原則

**Process Boundary Clarity Principle**

**定義**: Rust（Tauri）/Python（音声処理）/Chrome拡張の3プロセス間で、各プロセスの起動・停止・異常終了時の責務と回復シーケンスを明確に定義する。

**適用ガイドライン**:
- 各プロセスは独立した責務を持ち、他プロセスへの依存を最小化
- プロセス間通信（IPC）のプロトコルを明文化し、バージョニングを行う
- 異常終了時の検知と自動回復メカニズムを設計段階で定義
- ゾンビプロセス防止とリソースリークの防止策を実装
- **録音責務の一元化**: 音声録音はRust側`AudioDeviceAdapter`のみが担当し、Pythonサイドカーは録音を行わない（レース条件防止）

**意思決定への影響**:
- 新機能追加時: どのプロセスに実装すべきかの判断基準
- バグ修正時: 影響範囲の特定と分離テスト戦略
- パフォーマンス最適化: プロセス境界を超える通信の最小化

**参照**:
- `tech.md` - Pythonサイドカーライフサイクル管理セクション
- **実装例**: [ADR-001: Recording Responsibility](../specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md)
  - 音声録音責務をRust側に一元化（Pythonサイドカーでの録音禁止）
  - 静的解析による強制（`scripts/check_forbidden_imports.py`）
  - レース条件とリソース競合の防止

---

### 2. オフラインファースト原則

**Offline-First Principle**

**定義**: ネットワーク依存機能（Google Docs同期、LLM要約）は全てオプショナルとし、ローカル完結モードでコア機能（録音→文字起こし）が動作することを保証する。

**適用ガイドライン**:
- コア機能（音声録音、VAD、STT）はインターネット接続不要で完全動作
- ネットワーク依存機能は段階的縮退（graceful degradation）を実装
- オフライン時のデータキューイングと自動同期機構を提供
- 初回セットアップ時のネットワーク依存を最小化（モデルバンドル等）

**機能分類**:
- **Tier 1 (オフライン必須)**: 録音、VAD、faster-whisper STT、ローカルストレージ
- **Tier 2 (オンライン推奨)**: LLM要約、モデル更新、アプリ更新チェック
- **Tier 3 (オンライン必須)**: Google Docs同期、OAuth認証

**意思決定への影響**:
- 新機能追加時: Tier分類の決定と縮退戦略の設計
- アーキテクチャ設計: ネットワーク層の抽象化と依存性注入
- テスト戦略: オフラインモードでのE2Eテスト必須化

**参照**:
- `requirements.md` - オフライン対応、モデルバンドル仕様
- **実装例**: [ADR-002: Model Distribution Strategy](../specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md)
  - ハイブリッド配布戦略（オンデマンドダウンロード + システム共有パス）
  - オフライン動作を前提としつつ、初回起動時の柔軟性を確保

---

### 3. セキュリティ責任境界の原則

**Security Responsibility Boundary Principle**

**定義**: 暗号化が必要な情報（OAuth token、音声データ）はChrome拡張ではなくTauriアプリが管理し、拡張は表示専用とする。

**適用ガイドライン**:
- **Tauri責務**: OAuth token保管（OS Keychain）、音声データ暗号化、SQLiteデータベース暗号化
- **Chrome拡張責務**: 表示専用UI、ユーザーインタラクション、Google Docs表示操作
- 機密情報はWebSocket経由でも平文送信禁止（必要時はTLS + payload暗号化）
- Chrome拡張のローカルストレージには設定情報のみ（機密情報禁止）

**セキュリティ境界**:
```
[Chrome拡張] ─(WebSocket/TLS)─> [Tauri App]
    │                                │
    └─ 表示情報のみ                  ├─ OS Keychain (OAuth token)
                                      ├─ SQLCipher (録音データ)
                                      └─ 暗号化通信管理
```

**意思決定への影響**:
- 新機能追加時: 機密情報の保存場所の決定
- アーキテクチャ設計: 認証フローとトークン管理の一元化
- セキュリティ監査: 攻撃面の最小化と責任範囲の明確化

**参照**: `design.md` - OAuth 2.0トークン管理セクション

---

### 4. 段階的リソース管理原則

**Gradual Resource Management Principle**

**定義**: ディスク/メモリ/CPU使用量に対して「警告閾値（黄色）」「制限閾値（赤色）」「強制停止閾値」の3段階を定義し、ユーザー体験を段階的に劣化させる。

**適用ガイドライン**:

**3段階閾値の定義**:

| リソース | 警告（黄） | 制限（赤） | 強制停止 |
|---------|----------|----------|----------|
| ディスク空き容量 | 1GB未満 | 500MB未満 | 100MB未満 |
| メモリ使用量 | 2GB | 3GB | 4GB |
| CPU使用率（持続） | 70% | 85% | 95% |

**各段階でのアクション**:
- **警告**: UI通知バナー表示、古いセッション削除提案
- **制限**: 録音品質低下（サンプルレート削減）、要約生成停止
- **強制停止**: 録音停止、データ保存、エラーログ記録

**意思決定への影響**:
- パフォーマンス最適化: リソース監視ポイントの設計
- ユーザー体験設計: エラーメッセージと回復手順の提示
- テスト戦略: リソース制約下での動作検証

**参照**: `requirements.md` - リソース管理の3段階閾値セクション

---

### 5. 依存関係のベンダーロックイン回避原則

**Vendor Lock-in Avoidance Principle**

**定義**: faster-whisperモデル、LLM API、Google Docs APIは全て交換可能な抽象化層（trait/interface）を設け、将来的な技術選択の自由度を保つ。

**適用ガイドライン**:

**抽象化層の設計**:
```rust
// STT Engine抽象化
pub trait SpeechToTextEngine {
    async fn transcribe(&self, audio: AudioSegment) -> Result<Transcription>;
    fn supported_languages(&self) -> Vec<Language>;
}

// 実装
struct FasterWhisperEngine { ... }
struct OpenAIWhisperEngine { ... }  // 将来的な選択肢

// LLM API抽象化
pub trait SummaryGenerator {
    async fn generate_summary(&self, text: &str) -> Result<Summary>;
}

// 実装
struct OpenAISummaryGenerator { ... }
struct LocalLlamaSummaryGenerator { ... }  // v2.0想定

// Document API抽象化
pub trait DocumentIntegration {
    async fn insert_text(&self, doc_id: &str, text: &str) -> Result<()>;
}

// 実装
struct GoogleDocsIntegration { ... }
struct NotionIntegration { ... }  // 将来拡張
```

**依存関係管理戦略**:
- 外部サービスは全てAdapterパターンで抽象化
- 設定ファイルでの実装切り替えをサポート
- モックを用いた単体テストの容易化

**意思決定への影響**:
- 新機能追加時: 抽象化層の設計コスト vs 将来柔軟性のトレードオフ
- ベンダー選定: SLA、コスト、技術制約の評価基準
- マイグレーション戦略: 段階的な移行パスの確保

**参照**:
- `structure.md` - 依存関係の抽象化層セクション
- **実装例**: [ADR-002: Model Distribution Strategy](../specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md)
  - faster-whisperモデルの交換可能性（HuggingFace Hub互換）
  - 将来的なOpenAI Whisper APIへの切り替え可能性を確保

---

### 6. スケルトン先行実装とTDD原則

**Skeleton-First with TDD Principle**

**定義**: 新規機能は最小限のスケルトン（I/O境界と主要インターフェース）の実装から着手し、ユニットテスト→統合テストの順でテスト駆動開発を適用する。

**適用ガイドライン**:
- スケルトン段階で公開インターフェース、例外契約、依存抽象を確定
- 失敗するユニットテストを先に用意し、スケルトンを肉付けしながら緑化
- スケルトンの段階的拡張ごとに統合テストを追加しリグレッション防止
- スケルトン完成後もテストカバレッジ閾値（ユニット80%/統合主要シナリオ100%）を維持

**意思決定への影響**:
- スプリント計画: スケルトン→テスト→機能深化の3段階タスク分割を必須化
- コードレビュー: テスト有無の確認をレビュー項目に追加
- 実装速度 vs 品質: テスト駆動の投資を初期段階で行い後工程の手戻りを防ぐ

**参照**:
- `design.md` - タスク分解セクション
- **実装基準**: [coding-standards.md](../../docs/dev/coding-standards.md) - Testing Baseline
  - 新規機能はスケルトン → ユニット → 統合 → E2E の順で実装
  - 失敗するテストを先に用意する（TDD原則）
  - カバレッジ閾値: ユニット80% / 統合主要シナリオ100%

---

### 7. 非機能ベースライン原則

**Non-Functional Baseline Principle**

**定義**: 各コンポーネントは最低限のログ方針、エラー分類、テスト方針、セキュリティ要件を必ず設計ドキュメントで単文明記し、実装時に遵守する。

**適用ガイドライン**:
- **ログ方針**: INFO/DEBUG/ERRORのレベル運用、構造化JSON出力、セッションID付与、PIIマスク、低頻度メトリクスとは別チャンネルでのVerboseログ許容を一文で明文化
- **エラー分類**: ユーザー提示可否（Recoverable/Non-Recoverable等）の分類基準を明記
- **テスト方針**: ユニット/統合/E2Eの必須シナリオを一文で定義
- **セキュリティ最低限**: 入出力バリデーションやトークン保護方法を一文で明記

**意思決定への影響**:
- ドキュメント更新: 新機能追加時に非機能セクションへの追記をレビューチェック項目化
- 実装レビュー: コード側ログ/エラー/テスト/セキュリティ実装が文書と整合するか確認
- 運用: 非機能欠落を原因とする障害の早期検知

**参照**:
- `requirements.md` - 非機能要件一覧
- **実装基準**: [coding-standards.md](../../docs/dev/coding-standards.md)
  - Rust: `cargo clippy -D warnings` による静的解析必須
  - Python: `mypy --strict` による型安全性保証
  - TypeScript: `eslint` による品質ゲート

---

### 8. 図版管理原則

**Diagram Governance Principle**

**定義**: アーキテクチャ図、シーケンス図、UIフロー図は全てPlantUMLソースを`docs/uml/`に配置し、更新責任者とレビュー手順を明記したうえで設計ドキュメントに埋め込む。

**適用ガイドライン**:
- PlantUMLファイルを`docs/uml/<spec-slug>/<カテゴリ>/ID_*.puml`に配置しGit管理
- 図更新時はPull Requestに責任者のチェックリストと差分スクリーンショットを添付
- `design.md`内からPlantUMLソースを参照し導出ドキュメントの整合性を保つ
- リリースごとに図版の棚卸しを行い過去バージョンの更新履歴を残す
- 図種別の標準化: UC/コンポーネント/シーケンス/クラス/ステート/アクティビティ/配置図にIDを付与し、PlantUMLファイルとして`docs/uml/<spec-slug>/<カテゴリ>/ID_*.puml`に配置
- スペック進行ガイド: spec-requirementsでユースケース、spec-designでコンポーネント/配置/クラス骨子、spec-tasksでシーケンスと必要なアクティビティ、spec-implでステートマシンとクラス詳細を確定
- 図は実装へ影響する変更時のみ更新し、1図=1画面(要素5±2)を目安に粒度を保つ

**意思決定への影響**:
- 情報共有: 開発者が最新の図版位置を迷わず参照可能
- レビュー: 図の更新漏れを防ぎ仕様と実装の乖離を最小化
- ナレッジ管理: 図版責任者（テックリード）を明示し保守を安定化

**参照**:
- `design.md` - アーキテクチャ図セクション
- **実装基準**: [coding-standards.md](../../docs/dev/coding-standards.md) - UML Assets
  - PlantUMLソースを`docs/uml/<spec-slug>/<カテゴリ>/ID_*.puml`に配置
  - 図種別の標準化: UC/CMP/SEQ/CLS/STM/ACT/DEP
  - PRで図版更新理由を明記

---

### 9. 次の一手具体化原則

**Next Action Concreteness Principle**

**定義**: 各仕様と設計セクションでは最初に着手する対象機能（例: 「リアルタイム転写メモリ管理」）を明示し、実装・テスト・デプロイの直近タスクを3件以内で列挙する。

**適用ガイドライン**:
- セクション末尾に「Next Actions」小節を追加し、担当者と期日を添記
- タスクは粒度を統一（1〜2日以内に完了可能な単位）しWIP上限を確認
- 次スプリント移行時に実行済みタスクを更新し未着手分は理由と再計画を書く
- タスク化できない方向性はADRで保留理由を記録

**意思決定への影響**:
- 計画: 各ドキュメントが即時の実装行動に直結し優先順位が明確化
- レビュー: 仕様レビュー時に着手タスクの妥当性を確認
- マネジメント: バックログの流動性を高めボトルネックを早期発見

**参照**: `design.md` - Implementation Plan セクション

---

## Principles Application Matrix

| 原則 | 影響範囲 | 優先度 | 検証方法 |
|-----|---------|-------|---------|
| プロセス境界の明確化 | アーキテクチャ全体 | 🔴 Critical | 統合テスト、異常系テスト |
| オフラインファースト | 機能設計、UX | 🔴 Critical | オフラインE2Eテスト |
| セキュリティ責任境界 | セキュリティ、データ管理 | 🔴 Critical | セキュリティ監査、脅威モデリング |
| 段階的リソース管理 | パフォーマンス、UX | 🟠 High | 負荷テスト、長時間稼働テスト |
| ベンダーロックイン回避 | 保守性、拡張性 | 🟡 Medium | アーキテクチャレビュー |

---

## Principle Violation Review Process

原則に反する設計判断を行う場合、以下のプロセスを経る必要があります:

1. **Justification**: 技術的・ビジネス的理由の明文化
2. **Alternative Analysis**: 原則に従う代替案の検討と却下理由
3. **Risk Assessment**: 将来的な技術的負債の評価
4. **Documentation**: ADR（Architecture Decision Record）への記録
5. **Review**: チーム/ステークホルダーのレビューと承認

---

## Implementation Traceability

本プロジェクトでは、設計原則 → ADR → コーディング規約 → 実装コードの一貫したトレーサビリティを確保しています。

### Principles → ADRs Mapping

| 原則 | 関連ADR | 状態 |
|-----|---------|-----|
| **1. プロセス境界の明確化** | [ADR-001: Recording Responsibility](../specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md) | ✅ 実装 |
| **2. オフラインファースト** | [ADR-002: Model Distribution Strategy](../specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md) | ✅ 実装 |
| **3. セキュリティ責任境界** | 未作成（MVP2で策定予定） | ⚪ 計画中 |
| **4. 段階的リソース管理** | 未作成（MVP0実装時に策定） | ⚪ 計画中 |
| **5. ベンダーロックイン回避** | [ADR-002: Model Distribution Strategy](../specs/meeting-minutes-stt/adrs/ADR-002-model-distribution-strategy.md) | ✅ 実装 |
| **6. TDD原則** | [coding-standards.md](../../docs/dev/coding-standards.md) - Testing Baseline | ✅ 基準策定 |
| **7. 非機能ベースライン** | [coding-standards.md](../../docs/dev/coding-standards.md) - Global Policies | ✅ 基準策定 |
| **8. 図版管理** | [coding-standards.md](../../docs/dev/coding-standards.md) - UML Assets | ✅ 基準策定 |
| **9. 次の一手具体化** | 各spec の design.md - Implementation Plan | 🔵 運用中 |

### ADRs → Coding Standards Enforcement

| ADR | 強制手段 | 実装状態 |
|-----|----------|---------|
| **ADR-001: Recording Responsibility** | `scripts/check_forbidden_imports.py` + pre-commit hooks | ✅ 完了 |
| **ADR-002: Model Distribution** | `python-stt/`のモデルローダー実装（未作成） | ⚪ 実装待ち |
| **ADR-003: IPC Versioning** | Rust/Python共有型定義 + バージョンチェック（未作成） | ⚪ 実装待ち |

### Quality Gates

各レベルでの品質保証メカニズム:

```
設計原則（Principles）
    ↓ [意思決定記録]
Architecture Decision Records（ADRs）
    ↓ [実装基準]
コーディング規約（coding-standards.md）
    ↓ [自動化]
静的解析・Linter・Pre-commit Hooks
    ↓ [実装]
Production Code
```

**品質ゲートの実装状態**:
- ✅ **Level 1**: 設計原則文書化（9原則確定）
- ✅ **Level 2**: ADR作成（3件完了）
- ✅ **Level 3**: コーディング規約策定（coding-standards.md完成）
- ✅ **Level 4**: 静的解析基盤（pre-commit hooks + check_forbidden_imports.py）
- ⚪ **Level 5**: 実装コード（未開始）

---

## Related Documents

- **技術スタック**: `.kiro/steering/tech.md`
- **プロジェクト構造**: `.kiro/steering/structure.md`
- **製品方針**: `.kiro/steering/product.md`
- **要件定義**: `.kiro/specs/meeting-minutes-automator/requirements.md`
- **技術設計**: `.kiro/specs/meeting-minutes-automator/design.md`

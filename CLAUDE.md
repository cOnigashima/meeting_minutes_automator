# Claude Code Spec-Driven Development

Kiro-style Spec Driven Development implementation using claude code slash commands, hooks and agents.

## Project Context

### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`
- Commands: `.claude/commands/`

### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalize development process for individual features

### Active Specifications
- Check `.kiro/specs/` for active specifications
- Use `/kiro:spec-status [feature-name]` to check progress

#### Umbrella Spec (Reference Only)
- `meeting-minutes-automator`: Google Meet音声取得・文字起こし・議事録作成ツール（全体アーキテクチャリファレンス、実装は以下4つのsub-specに分割）

#### Implementation Specs
- `meeting-minutes-core`: [MVP0] Walking Skeleton - Tauri+Python+Chrome拡張の最小疎通確認（Fake実装）（✅ tasks生成完了）
- `meeting-minutes-stt`: [MVP1] Real STT - faster-whisper統合、webrtcvad統合、リソースベースモデル選択、音声デバイス管理、ローカルストレージ（🔵 requirements生成完了）
- `meeting-minutes-docs-sync`: [MVP2] Google Docs同期 - OAuth 2.0認証、Google Docs API統合、Named Range管理、オフライン同期（🔵 spec初期化完了）
- `meeting-minutes-ci`: [Infrastructure] GitHub Actions CI/CD - クロスプラットフォームテストマトリックス、コスト最適化戦略、自動リリース、セキュリティ/パフォーマンステスト（🔵 spec初期化完了）
- `meeting-minutes-llm`: [MVP3] LLM要約 + UI - プロダクション準備（予定）

## Development Guidelines

## Tools Integration: Serena + cc-sdd

### 基本方針
本プロジェクトは **Serena（象徴的コードナビゲーション）** と **cc-sdd（Kiro仕様駆動開発）** を組み合わせて使用します。

- **仕様フェーズ**: `/kiro:*` コマンドで要件・設計・タスクを作成
- **コード探索**: Serenaツール（`mcp__serena__*`）で効率的にコードを理解
- **実装フェーズ**: 両方を組み合わせて、仕様とコードの整合性を維持

### プロジェクトステータス（2025-10-10現在）

- **MVP0 (meeting-minutes-core)**: ✅ 完了（Walking Skeleton実装済み、44テスト合格）
- **MVP1 (meeting-minutes-stt)**: 🔵 実装中（タスク2.4完了、requirements/design承認済み）
- **MVP2以降**: ⚪ 初期化済み

### フェーズ別ツール使用ガイド

#### 実装フェーズ（現在のフェーズ）
**原則**: Serenaで既存コード理解 → cc-sddで仕様確認 → TDD実装 → 検証

**タスク開始前**:
1. `mcp__serena__get_symbols_overview` → ファイル構造把握
2. `mcp__serena__find_symbol` → 関連コードの詳細取得（`include_body=True`）
3. `/kiro:spec-status <feature>` → 仕様ステータス確認

**実装中**:
1. `mcp__serena__find_symbol` → 編集対象の特定（**ファイル全体を読まない**）
2. `mcp__serena__replace_symbol_body` → 象徴的コード編集
3. `mcp__serena__find_referencing_symbols` → 影響範囲確認
4. RED → GREEN → REFACTOR サイクルでTDD実装

**実装後**:
1. `/kiro:validate-design <feature>` → 設計整合性確認
2. テスト実行（cargo test / pytest）
3. Requirement Traceability Matrix更新
4. コミットメッセージに要件ID含める（例: `feat(audio): REQ-001.4 音声ストリーム実装`）

#### 新規仕様作成時
- `/kiro:spec-requirements` → 要件定義
- `/kiro:spec-design` → 設計作成
- `/kiro:spec-tasks` → タスク生成
- **Serenaは最小限**（新機能のため既存コードが少ない）

#### レビュー・リファクタリング時
- `mcp__serena__search_for_pattern` → パターン検索（禁止ライブラリチェック等）
- `mcp__serena__find_referencing_symbols` → 依存関係の可視化
- `/kiro:validate-design` → 設計原則との整合性確認

### 重要な原則
1. **ファイル全体を読まない**: `get_symbols_overview` でまず概要把握
2. **象徴的検索を優先**: `find_symbol` で必要な部分のみ取得（`include_body=True`）
3. **要件IDとの紐付け**: 実装時は必ず関連要件ID（REQ-###等）をコミットメッセージに含める
4. **トレーサビリティ維持**: コード変更時はRequirement Traceability Matrixを更新

詳細は Serena メモリの `serena_and_cc-sdd_workflow.md` を参照。

### カスタムエージェント

本プロジェクトでは、仕様駆動開発を効率化するカスタムエージェントを提供しています。

#### **kiro-spec-implementer** （実装フェーズ推奨）

**目的**: Kiro仕様駆動開発 + Serena統合 + TDD実装の完全自動化

**使用場面**:
- 「タスクX.Xを実装して」と依頼する場合
- `/kiro:spec-impl`コマンド実行時
- 既存コードの修正で仕様整合性確認が必要な場合

**提供価値**:
- 🎯 **トークン効率**: Serenaで必要な部分のみ読み込み（ファイル全体読み込み禁止）
- 📋 **トレーサビリティ**: 要件 → 設計 → コードの自動リンク維持
- ✅ **品質保証**: 設計原則9項目の自動チェック
- 🔄 **TDD徹底**: テストファースト実装の強制

**使用例**:
```
User: タスク2.5を実装して
Agent: タスク2.5（デバイス切断検出と自動再接続）を実装します。
       1. 要件ID確認（requirements.md）
       2. 既存コード理解（Serena）
       3. TDD実装（RED → GREEN → REFACTOR）
       4. 検証（/kiro:validate-design）
```

**ワークフロー**:
1. **仕様確認**: `/kiro:spec-status` + `tasks.md` + `requirements.md`
2. **既存コード理解**: `mcp__serena__get_symbols_overview` → `find_symbol` → `find_referencing_symbols`
3. **TDD実装**: RED（失敗テスト） → GREEN（最小実装） → REFACTOR（設計原則確認）
4. **検証**: `/kiro:validate-design` + テスト実行 + トレーサビリティ更新

#### **kiro-spec-guardian** （仕様一貫性チェック）

既存のエージェント。仕様の一貫性チェック、要件・設計・タスクの整合性確認に使用。

---

## 実装前に参照するドキュメント
- `.kiro/steering/principles.md` — コア設計原則
- `.kiro/steering/tech.md` — 技術スタックと実装パターン
- `.kiro/specs/meeting-minutes-automator/requirements.md` — 最新要件
- `.kiro/specs/meeting-minutes-automator/design.md` — 実装タスクと設計詳細
- `docs/dev/coding-standards.md` — コーディング規約とテスト基準
- `docs/dev/spec-authoring.md` — 要件・設計ドキュメントの作成手順とチェックリスト

- Think in English, but generate responses in Japanese (思考は英語、回答の生成は日本語で行うように)


### Requirements Numbering & Traceability Workflow
1. `/kiro:spec-requirements <feature>` 実行直後に要件本文へ ID を採番する。Umbrella は `REQ-###`/`NFR-###`/`ARC-###`/`PRO-###`/`DEL-###`/`CON-###`/`FUT-###` を使用し、サブセクションは `REQ-001.1.a` のように階層化する。
2. 受け入れ条件は EARS 構文に ID を含めて記述し、テストケースやタスク作成時に同じ ID を引用する。
3. Umbrella spec では `Requirement Traceability Matrix` を `requirements.md` 末尾に置き、サブスペック側で採番した `CORE-REQ-###` などと相互リンクする。
4. サブスペックの requirements 生成時は、親 ID との対応を同様の表に記載し、更新した内容を Umbrella 側の表にも反映する。
5. `/kiro:validate-requirements <feature>` を使う際は、ID採番・Traceability表・`spec.json` の `approvals.requirements` フラグを必ず確認する。未整備の場合は NO-GO とし、修正後に再実行する。
6. 設計 (`/kiro:spec-design`)・タスク (`/kiro:spec-tasks`) フェーズでは、対象 ID を明示して参照する。仕様変更で ID をDeprecated扱いにする場合は表に履歴を残し、関連タスク/テストを更新する。


### EARS構文チートシート

Ubiquitous：The Recorder shall persist raw audio locally before any network transfer.

Event：When network connectivity is restored, the Syncer shall upload queued minutes within 60 s.

State：While free disk space < 500 MB, the system shall block new recordings and display a warning.

Optional：Where Google Docs integration is enabled, the system shall append minutes to the selected document.

Unwanted：If OAuth token validation fails, then the system shall abort upload and prompt re-authentication.

Complex：While recording, when the user presses Stop, the system shall finalize segments and start STT within 2s.

| 種別                           | 目的       | ひな形                                                                 |
| ---------------------------- | -------- | ------------------------------------------------------------------- |
| **Ubiquitous（常時）**           | 常に成立する要求 | **The <system> shall <response>.**                                  |
| **Event-driven（イベント）**       | 事象が起きたら  | **When <trigger>, the <system> shall <response>.**                  |
| **State-driven（状態）**         | 状態の間ずっと  | **While <state>, the <system> shall <response>.**                   |
| **Optional-feature（オプション）**  | 機能が有効なら  | **Where <feature> is enabled, the <system> shall <response>.**      |
| **Unwanted-behavior（異常/禁止）** | 好ましくない事象 | **If <undesired condition>, then the <system> shall <mitigation>.** |
| **Complex（複合）**              | 上記の組合せ   | **While …, when …, the <system> shall …**                           |


## Workflow

### Phase 0: Steering (Optional)
`/kiro:steering` - Create/update steering documents
`/kiro:steering-custom` - Create custom steering for specialized contexts

Note: Optional for new features or small additions. You can proceed directly to spec-init.

### Phase 1: Specification Creation
1. `/kiro:spec-init [detailed description]` - Initialize spec with detailed project description
2. `/kiro:spec-requirements [feature]` - Generate requirements document
3. `/kiro:spec-design [feature]` - Interactive: "Have you reviewed requirements.md? [y/N]"
4. `/kiro:spec-tasks [feature]` - Interactive: Confirms both requirements and design review

### Phase 2: Progress Tracking
`/kiro:spec-status [feature]` - Check current progress and phases

## Development Rules
1. **Consider steering**: Run `/kiro:steering` before major development (optional for new features)
2. **Follow 3-phase approval workflow**: Requirements → Design → Tasks → Implementation
3. **Approval required**: Each phase requires human review (interactive prompt or manual)
4. **No skipping phases**: Design requires approved requirements; Tasks require approved design
5. **Update task status**: Mark tasks as completed when working on them
6. **Keep steering current**: Run `/kiro:steering` after significant changes
7. **Check spec compliance**: Use `/kiro:spec-status` to verify alignment

## Steering Configuration

### Current Steering Files
Managed by `/kiro:steering` command. Updates here reflect command changes.

**Status**: ✅ All core steering files have been created and are active.
- `product.md`: Meeting Minutes Automator製品概要とバリュープロポジション
- `tech.md`: Tauri + Chrome拡張 + Python音声処理技術スタック
- `structure.md`: ドメイン駆動設計とマルチレイヤーアーキテクチャ
- `principles.md`: 5つのコア設計原則（プロセス境界、オフラインファースト、セキュリティ境界、リソース管理、ベンダーロックイン回避）

### Active Steering Files
- `product.md`: Always included - Product context and business objectives
- `tech.md`: Always included - Technology stack and architectural decisions
- `structure.md`: Always included - File organization and code patterns
- `principles.md`: Always included - Core design principles and decision criteria

### Custom Steering Files
<!-- Added by /kiro:steering-custom command -->
<!-- Format:
- `filename.md`: Mode - Pattern(s) - Description
  Mode: Always|Conditional|Manual
  Pattern: File patterns for Conditional mode
-->

### Inclusion Modes
- **Always**: Loaded in every interaction (default)
- **Conditional**: Loaded for specific file patterns (e.g., "*.test.js")
- **Manual**: Reference with `@filename.md` syntax


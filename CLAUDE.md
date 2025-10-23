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
- `meeting-minutes-dev-quality`: [Infrastructure] Development Quality Assurance - 開発フロー標準化、自動ガード機構、一貫性チェック、ADR/要件ID検証（⚪ spec初期化完了）
- `meeting-minutes-llm`: [MVP3] LLM要約 + UI - プロダクション準備（予定）
- `ui-hub`: [Tooling] UI Hub - Meeting Minutes Automator既存UI改善のためのトークン駆動開発環境（Penpot設計トークン→Style Dictionary→Storybook→MCPサーバ統合）（🔵 design生成完了、requirements承認済み）

## Development Guidelines

## Tools Integration: Serena + cc-sdd

### 基本方針
本プロジェクトは **Serena（象徴的コードナビゲーション）** と **cc-sdd（Kiro仕様駆動開発）** を組み合わせて使用します。

- **仕様フェーズ**: `/kiro:*` コマンドで要件・設計・タスクを作成
- **コード探索**: Serenaツール（`mcp__serena__*`）で効率的にコードを理解
- **実装フェーズ**: 両方を組み合わせて、仕様とコードの整合性を維持

### プロジェクトステータス（2025-10-21現在）

- **MVP0 (meeting-minutes-core)**: ✅ 完了（2025-10-10、Walking Skeleton実装済み）
- **MVP1 (meeting-minutes-stt)**: ✅ 完了（2025-10-21、Phase 13+14完了、267/285テスト合格）
  - 18件テスト失敗（優先度P2）はMVP2 Phase 0で対応検討
- **MVP2 (meeting-minutes-docs-sync)**: 📋 次工程（OAuth 2.0 + Google Docs API、tasks生成待ち）
- **meeting-minutes-ci**: 🔵 並行実施（CI依存タスク受入済み、tasks生成待ち）
- **MVP3 (meeting-minutes-llm)**: ⚪ MVP2完了後に要件定義開始

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

#### **kiro-spec-guardian** （仕様整合性 + 実装品質保証）

Spec consistency（要件・設計・タスクの整合性）と実装品質（TDD/テスト網羅/コーディング規約/ADR準拠）を検証。

**使用場面**:
- 設計フェーズ完了時（design.md承認後、tasks生成前）
- 実装フェーズ完了時（TDD準拠確認、テスト網羅検証）
- PRオープン前の品質ゲート

**詳細ワークフローは** `.serena/memories/serena_and_cc-sdd_workflow.md` **参照。**

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

---

## Editing Guidelines

### エージェント定義・重要ファイル編集時の注意

重要ファイル（`.claude/agents/*.md`, `.kiro/steering/*.md`, `.kiro/specs/*/requirements.md`, `.kiro/specs/*/design.md`）を編集する際は、以下のルールに従ってください。

#### ❌ 避けるべきパターン

1. **全面書き換え（Write）**
   - 既存機能の見落としリスクが非常に高い
   - 例: Skeleton Implementation検証が削除される
   - 例: Exampleが意図せず削除される

2. **Plan Mode省略**
   - 変更影響が不透明になる
   - 削除される要素を事前確認できない

3. **変更後の確認不足**
   - `git diff`で削除要素をチェックしない
   - 重要キーワード（ADR/Example/Principle）の削除を見逃す

#### ✅ 推奨パターン

1. **Edit優先**
   ```bash
   # Good: 部分更新で差分を明確に
   Edit(old_string="...", new_string="...")

   # Bad: 全面書き換え（見落としリスク大）
   Write(file_path="...", content="...")
   ```

2. **Plan Modeで事前宣言**
   ```markdown
   変更ファイル: kiro-spec-guardian.md
   変更方法: Edit（部分更新）
   変更箇所: L65（ADR-001 through ADR-004 → ADR-007）
   削除される要素: なし
   追加される要素: ADR-005〜007の説明
   ```

3. **変更後の差分確認**
   ```bash
   # 重要キーワードが削除されていないかチェック
   git diff HEAD -- .claude/agents/kiro-spec-guardian.md | grep -E "^-.*(ADR|Example|Skeleton|Principle)"
   ```

4. **ADR採番検証スクリプト実行**
   ```bash
   .kiro/scripts/validate_adrs.sh
   ```

#### チェックリスト

エージェント定義編集時:
- [ ] `Edit`ツールを使用（`Write`は新規ファイルのみ）
- [ ] Plan Modeで変更内容を事前宣言
- [ ] 削除される要素（Example/ADR/Principle）を確認
- [ ] `git diff`で差分確認
- [ ] ADR採番の場合は`validate_adrs.sh`実行

### ダイアグラム管理規則

**Single Source of Truth**: すべてのUML/Mermaidダイアグラムは `docs/uml/<spec-name>/<category>/` に集約する。

#### ディレクトリ構造

```
docs/uml/
└── <spec-name>/           # 例: meeting-minutes-stt, meeting-minutes-docs-sync
    ├── cls/               # クラス図 (Class Diagrams)
    ├── seq/               # シーケンス図 (Sequence Diagrams)
    ├── cmp/               # コンポーネント図 (Component Diagrams)
    ├── act/               # アクティビティ図 (Activity Diagrams)
    └── state/             # 状態図 (State Machine Diagrams)
```

#### フォーマット

- **PlantUML**: `*.puml` (レガシー、meeting-minutes-sttで使用)
- **Mermaid**: `*.md` (推奨、Markdown埋め込み形式)

#### 命名規則

- **PlantUML**: `<diagram-name>.puml` (例: `audio-pipeline.puml`)
- **Mermaid**: `<diagram-name>.md` (例: `sync-domain.md`)

#### ダイアグラム更新ワークフロー

1. **コード変更時**: Serena (`get_symbols_overview`) で実装変更を確認
2. **ダイアグラム更新**: `docs/uml/<spec>/cls/*.md` or `*.puml` を直接編集
3. **自動化なし**: スクリプト・ツール不要（LLMエージェントが直接編集）

#### エージェント分担

- **context-scout**: コード変更時に2-4ファイル範囲でダイアグラム整合性チェック（軽量）
- **docs-gardener**: 5+ファイル変更時にダイアグラム大規模同期（`scripts/docs_crawler.py`使用）

#### 禁止事項

- ❌ `.kiro/specs/<spec>/design-artifacts/class-diagrams/` への新規ダイアグラム作成
- ❌ `docs/diagrams/` への新規ダイアグラム作成（削除済み）
- ❌ ダイアグラムの自動生成スクリプト作成（YAGNI原則違反）

---

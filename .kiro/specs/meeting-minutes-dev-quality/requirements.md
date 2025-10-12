# Requirements Document

## Project Description (Input)

Development Quality Assurance Framework - 開発フロー標準化、自動ガード機構、一貫性チェックの確立。

### 目的

1. **開発フローの体系化**: Spec作成→実装準備→TDD実装→レビュー→統合の一貫したプロセス定義
2. **自動ガード機構**: ADR採番重複検出、要件ID整合性チェック、禁止パターン検出、エージェント定義検証
3. **一貫性チェック自動化**: Pre-commit hooks、kiro-spec-guardian拡張

### スコープ

- 開発フロードキュメント統合（CLAUDE.md + serena memory + 新規ガイド）
- 検証スクリプト（ADR採番、要件ID、禁止パターン、エージェント定義）
- Pre-commit hooks設定（Git hooks or Claude Code hooks）
- kiro-spec-guardian拡張（Documentation Change Review責務）

### 背景

- **既存問題**: ADR-004/005重複、Skeleton Implementation削除見落とし、全面書き換えリスク
- **根本原因**: 断片的なドキュメント、手動依存の検証、事後確認のみのガード
- **期待効果**: 初歩的ミスの防止、開発品質の向上、ベストプラクティスの体系化

---

## Requirements

### はじめに

本要件定義は、Meeting Minutes Automatorプロジェクトにおける **開発品質保証フレームワーク（Development Quality Assurance Framework）** を確立するための要件を定義します。

**背景と課題**:
- **既存問題**: ADR採番の重複（ADR-004/005の二重使用）、重要責務の削除見落とし（Skeleton Implementation検証の削除）、全面書き換えによる機能喪失リスク
- **根本原因**: 開発フロードキュメントの断片化、手動依存の検証プロセス、事後確認のみのガード機構
- **期待効果**: 初歩的ミスの自動検出、開発品質の向上、ベストプラクティスの体系化、チーム全体での一貫した開発フロー確立

**スコープ**:
1. **開発フロードキュメント統合**: CLAUDE.md、Serena memory、新規開発ガイドの体系化
2. **自動ガード機構**: ADR採番、要件ID、禁止パターン、エージェント定義の検証スクリプト
3. **Pre-commit hooks**: Git hooks または Claude Code hooks による事前チェック
4. **CI/CD連携**: meeting-minutes-ci specとの統合による継続的品質保証
5. **kiro-spec-guardian拡張**: Documentation Change Review責務の追加

**本要件の位置づけ**:
本開発品質保証フレームワークは、9つの設計原則（`.kiro/steering/principles.md`）とコーディング規約（`docs/dev/coding-standards.md`）を実装レベルで強制し、開発者が意図せず品質基準を逸脱することを防ぐための Infrastructure として機能します。

---

### 機能要件

#### REQ-DQ-001: 開発フロードキュメント統合

**目的**: 断片化した開発関連ドキュメントを体系化し、開発者が単一の情報源から必要な情報にアクセスできるようにする。

**REQ-DQ-001.1: 開発ワークフローガイド作成**

**受け入れ条件** (EARS):
- **WHEN** 開発者が新規機能開発を開始する場合、**THEN** システムは `/kiro:*` コマンド実行順序（spec-init → spec-requirements → spec-design → spec-tasks → spec-impl）を明示したガイドを提供すること。
- **WHERE** Serena象徴的ツールと `/kiro:*` コマンドの使い分けが必要な場合、**THEN** システムは仕様フェーズ（kiro優先）と実装フェーズ（Serena + kiro併用）の判断基準を文書化すること。

**REQ-DQ-001.2: エージェント定義使用ガイド**

**受け入れ条件** (EARS):
- **WHEN** 開発者がkiro-spec-implementerエージェント使用を検討する場合、**THEN** システムは通常実装との使い分け基準（タスク複雑度、要件ID追跡必要性、TDD徹底度）を提示すること。
- **WHEN** 開発者がkiro-spec-guardianエージェント使用を検討する場合、**THEN** システムは適用場面（spec作成後、design変更後、PR前の品質ゲート）を明示すること。

**REQ-DQ-001.3: ドキュメント相互参照マップ**

**受け入れ条件** (EARS):
- **THE** システムは、CLAUDE.md、principles.md、tech.md、coding-standards.md、spec-authoring.md、各ADRの相互参照マップを提供すること。
- **WHEN** 開発者が特定の設計判断（例: 状態管理方法）を調べる場合、**THEN** システムはPrinciples → ADR → Coding Standards → Implementation Codeのトレーサビリティパスを提示すること。

---

#### REQ-DQ-002: ADR採番検証スクリプト

**目的**: Umbrella spec全体でADR採番の重複と欠番を自動検出し、採番ミスを防止する。

**REQ-DQ-002.1: 重複検出機能**

**受け入れ条件** (EARS):
- **WHEN** `.kiro/scripts/validate_adrs.sh` が実行される場合、**THEN** システムは `.kiro/specs/*/adrs/ADR-*.md` ファイルを検索し、同一ADR番号の重複を検出すること。
- **IF** 重複が検出された場合、**THEN** システムはエラーメッセージに両方のファイルパスを表示し、終了コード 1 を返すこと。

**REQ-DQ-002.2: 欠番検出機能**

**受け入れ条件** (EARS):
- **WHEN** `.kiro/scripts/validate_adrs.sh` が実行される場合、**THEN** システムは ADR-001 から最大ADR番号までの範囲で欠番をリスト表示すること。
- **THE** 欠番検出は警告として表示し、終了コードには影響しないこと（欠番は必ずしもエラーではないため）。

**REQ-DQ-002.3: bash 3互換性**

**受け入れ条件** (EARS):
- **THE** `.kiro/scripts/validate_adrs.sh` は bash 3.x（macOS標準）および bash 4.x以降の両方で動作すること。
- **IF** bash 4+の連想配列機能が使用不可の場合、**THEN** システムは自動的にレガシーモード（indexed array + 線形探索）にフォールバックすること。

**REQ-DQ-002.4: 出力可読性**

**受け入れ条件** (EARS):
- **THE** スクリプトは検出されたADRを sub-spec名とともにリスト表示すること（例: `ADR-001: stt`, `ADR-004: core`）。
- **THE** 成功時（重複なし）、警告時（欠番あり）、エラー時（重複あり）で色分け表示（GREEN/YELLOW/RED）を行うこと。

---

#### REQ-DQ-003: 要件ID整合性チェック

**目的**: requirements.md、design.md、tasks.md、コードコメント、コミットメッセージ間での要件ID参照の一貫性を保証する。

**REQ-DQ-003.1: 要件ID形式検証**

**受け入れ条件** (EARS):
- **WHEN** 新規要件がrequirements.mdに追加される場合、**THEN** システムは要件ID形式（`REQ-###`, `NFR-###`, `ARC-###`, `PRO-###`, `DEL-###`, `CON-###`, `FUT-###`, `<SPEC>-REQ-###`）の正規表現マッチを検証すること。
- **IF** 形式不正な要件IDが検出された場合、**THEN** システムはエラーメッセージに該当行番号とファイル名を表示すること。

**REQ-DQ-003.2: 要件IDリンク整合性チェック**

**受け入れ条件** (EARS):
- **WHEN** design.mdまたはtasks.mdで要件IDが参照される場合、**THEN** システムは参照先要件がrequirements.mdに存在することを検証すること。
- **IF** 存在しない要件IDへの参照が検出された場合、**THEN** システムは警告メッセージに参照元ファイル名、行番号、未定義要件IDを表示すること。

**REQ-DQ-003.3: コミットメッセージ要件ID検証**

**受け入れ条件** (EARS):
- **WHEN** 実装関連のコミット（prefix: `feat`, `fix`, `refactor`）が作成される場合、**THEN** システムはコミットメッセージ本文に要件ID（`REQ-###`形式）が含まれることを検証すること。
- **IF** 要件IDが含まれない場合、**THEN** システムは警告を表示し、コミットを中断するかユーザーに確認を求めること。

**REQ-DQ-003.4: Traceability Matrix更新検証**

**受け入れ条件** (EARS):
- **WHEN** 新規テストケースまたはタスクが実装される場合、**THEN** システムはrequirements.md末尾のTraceability Matrix更新を促すこと。
- **WHERE** Umbrella specとsub-specの要件ID対応が必要な場合、**THEN** システムは両方のTraceability Matrix更新を確認すること。

---

#### REQ-DQ-004: 禁止パターン検出

**目的**: 設計原則やADRで禁止されているコードパターン（例: Python側での音声録音、全面書き換えリスクのあるWrite操作）を自動検出する。

**REQ-DQ-004.1: 禁止ライブラリインポート検出（ADR-001準拠）**

**受け入れ条件** (EARS):
- **WHEN** Python Sidecarコード（`python-stt/**/*.py`）が変更される場合、**THEN** システムは禁止音声録音ライブラリ（`sounddevice`, `pyaudio`, `wave.open('wb')`）のインポートを検出すること。
- **IF** 禁止インポートが検出された場合、**THEN** システはエラーメッセージにファイル名、行番号、検出パターンを表示し、ADR-001へのリンクを提示すること。

**REQ-DQ-004.2: 全面書き換えパターン検出**

**受け入れ条件** (EARS):
- **WHEN** エージェント定義ファイル（`.claude/agents/*.md`）、重要ドキュメント（`CLAUDE.md`, `.kiro/steering/*.md`）が変更される場合、**THEN** システムはWriteツール使用による全面書き換えを検出すること。
- **IF** 全面書き換えが検出された場合、**THEN** システムは警告を表示し、Editツール使用を推奨すること。

**REQ-DQ-004.3: 危険なAPI使用検出**

**受け入れ条件** (EARS):
- **WHEN** Rust/Tauriコードが変更される場合、**THEN** システムは `tauri::api::all` allowlistの使用を検出すること（最小権限原則違反）。
- **WHEN** Chrome拡張コードが変更される場合、**THEN** システムは `dangerouslySetInnerHTML` の使用を検出すること（XSSリスク）。

**REQ-DQ-004.4: 設計原則違反パターン検出**

**受け入れ条件** (EARS):
- **WHEN** 新規機能実装コードが追加される場合、**THEN** システムは設計原則違反の可能性があるパターン（例: ネットワーク必須のコア機能、暗号化なしのトークン保存）を検出すること。
- **IF** 違反パターンが検出された場合、**THEN** システムは該当する設計原則とADRへのリンクを表示すること。

---

#### REQ-DQ-005: エージェント定義検証

**目的**: カスタムエージェント定義ファイルの構造、必須セクション、設計原則との整合性を検証する。

**REQ-DQ-005.1: 必須セクション検証**

**受け入れ条件** (EARS):
- **WHEN** `.claude/agents/*.md` ファイルが変更される場合、**THEN** システムはFrontmatter（name, description, model, color）と必須セクション（Core Mission, Workflow, Success Criteria）の存在を検証すること。
- **IF** 必須セクションが欠落している場合、**THEN** システムはエラーメッセージに欠落セクション名を表示すること。

**REQ-DQ-005.2: ADR参照整合性チェック**

**受け入れ条件** (EARS):
- **WHEN** エージェント定義内でADR番号が参照される場合（例: "ADR-001 through ADR-007"）、**THEN** システムは `.kiro/specs/*/adrs/` ディレクトリ内の実際のADRファイルと照合すること。
- **IF** 存在しないADR番号が参照されている場合、**THEN** システムは警告を表示すること。

**REQ-DQ-005.3: 設計原則カバレッジ検証**

**受け入れ条件** (EARS):
- **WHEN** kiro-spec-guardianまたはkiro-spec-implementerエージェント定義が更新される場合、**THEN** システムは9つの設計原則すべてがドキュメント内で言及されていることを検証すること。
- **IF** 設計原則への言及が欠落している場合、**THEN** システムは警告を表示し、該当原則名をリスト表示すること。

**REQ-DQ-005.4: Example品質チェック**

**受け入れ条件** (EARS):
- **WHEN** エージェント定義内にExampleセクションが含まれる場合、**THEN** システムは各Exampleにuser/assistantの対話と`<commentary>`タグが含まれることを検証すること。
- **THE** Exampleは最低2つ以上含まれることを推奨し、不足時は警告を表示すること。

---

#### REQ-DQ-006: Pre-commit Hooks統合

**目的**: Git commitまたはClaude Code実行前に自動検証を実行し、品質基準違反を事前に防止する。

**REQ-DQ-006.1: Git Pre-commit Hooks設定**

**受け入れ条件** (EARS):
- **THE** システムは `.git/hooks/pre-commit` スクリプトを提供し、以下の検証を実行すること:
  - ADR採番検証（`.kiro/scripts/validate_adrs.sh`）
  - 禁止パターン検出（`.kiro/scripts/check_forbidden_patterns.sh`）
  - 要件ID整合性チェック（`.kiro/scripts/check_requirement_ids.sh`）
- **IF** いずれかの検証が失敗した場合、**THEN** システムはコミットを中断し、エラー詳細を表示すること。

**REQ-DQ-006.2: Claude Code Hooks統合**

**受け入れ条件** (EARS):
- **WHERE** Claude Code hookシステムが利用可能な場合、**THEN** システムは `.claude/hooks/pre-tool-use.js` を提供し、以下のツール使用前チェックを実行すること:
  - Writeツールによる重要ファイル全面書き換え警告
  - エージェント起動前の仕様ステータス確認（spec.json検証）
- **WHEN** 警告が発生した場合、**THEN** システムはユーザーに確認を求め、続行/中止を選択させること。

**REQ-DQ-006.3: Bypass機能**

**受け入れ条件** (EARS):
- **WHEN** 開発者が緊急時にPre-commit Hooksをバイパスする必要がある場合、**THEN** システムは `--no-verify` フラグの使用を許可すること。
- **IF** バイパスが使用された場合、**THEN** システムはコミットメッセージに `[skip-hooks: <理由>]` タグ追加を要求すること。

**REQ-DQ-006.4: CI/CD連携**

**受け入れ条件** (EARS):
- **THE** システムは、Pre-commit Hooksと同一の検証スクリプトをCI/CD（GitHub Actions）でも実行すること。
- **IF** ローカルでバイパスされた変更がpushされた場合、**THEN** CIはPRをブロックし、検証失敗の詳細を表示すること。

---

#### REQ-DQ-007: kiro-spec-guardian拡張

**目的**: kiro-spec-guardianエージェントにDocumentation Change Review責務を追加し、重要ドキュメント変更時の品質保証を強化する。

**REQ-DQ-007.1: Documentation Change Detection**

**受け入れ条件** (EARS):
- **WHEN** kiro-spec-guardianが起動される場合、**THEN** システムは以下のファイル変更を検出すること:
  - `.claude/agents/*.md`（エージェント定義）
  - `.kiro/steering/*.md`（steering documents）
  - `CLAUDE.md`（プロジェクト指示）
  - `.kiro/specs/*/adrs/*.md`（ADRs）
- **IF** 変更が検出された場合、**THEN** システムはDocumentation Change Reviewモードを起動すること。

**REQ-DQ-007.2: 削除要素チェック**

**受け入れ条件** (EARS):
- **WHEN** Documentation Change Reviewモードで重要ドキュメントの変更が解析される場合、**THEN** システムは `git diff` を実行し、削除行（`-`プレフィックス）を抽出すること。
- **IF** 重要なキーワード（"Skeleton Implementation", "ADR-", "Principle", "Example"）を含む削除が検出された場合、**THEN** システムは警告を表示し、削除理由の確認を要求すること。

**REQ-DQ-007.3: 整合性クロスチェック**

**受け入れ条件** (EARS):
- **WHEN** エージェント定義内のADR番号リストが変更される場合、**THEN** システムは `.kiro/specs/*/adrs/` ディレクトリ内の実際のADRファイルと照合すること。
- **WHEN** CLAUDE.md内のActive Specifications listが変更される場合、**THEN** システムは `.kiro/specs/*/spec.json` ファイルの存在を検証すること。

**REQ-DQ-007.4: 変更影響分析**

**受け入れ条件** (EARS):
- **WHEN** steering documentが変更される場合、**THEN** システムは影響を受ける可能性のあるspec（requirementsまたはdesignで該当steering documentを参照しているもの）をリスト表示すること。
- **THE** システムは、変更されたドキュメントに依存する他のドキュメントの更新要否を提案すること。

---

### 非機能要件

#### NFR-DQ-001: パフォーマンス

**NFR-DQ-001.1: 検証スクリプト実行時間**

**受け入れ条件** (EARS):
- **THE** ADR採番検証スクリプトは、ADRファイル数50件以下の場合、1秒以内に完了すること。
- **THE** 要件ID整合性チェックは、仕様ファイル合計20,000行以下の場合、5秒以内に完了すること。

**NFR-DQ-001.2: Pre-commit Hooks遅延**

**受け入れ条件** (EARS):
- **THE** Pre-commit Hooksの合計実行時間は、通常のコミット操作で10秒を超えないこと。
- **IF** 検証に10秒以上かかる場合、**THEN** システムは進捗インジケーターを表示すること。

#### NFR-DQ-002: 保守性

**NFR-DQ-002.1: スクリプト可読性**

**受け入れ条件** (EARS):
- **THE** すべての検証スクリプトは、冒頭にPurpose、Usage、Examplesを含むコメントを記載すること。
- **THE** エラーメッセージは、問題の内容、発生箇所、推奨修正方法を含むこと。

**NFR-DQ-002.2: 拡張性**

**受け入れ条件** (EARS):
- **THE** 新規検証ルール追加時、既存スクリプトを変更せずにプラグイン形式で追加できること。
- **THE** 検証ルールは `.kiro/scripts/rules/` ディレクトリに独立したファイルとして配置できること。

#### NFR-DQ-003: エラーハンドリング

**NFR-DQ-003.1: グレースフルデグラデーション**

**受け入れ条件** (EARS):
- **IF** 検証スクリプト実行中にファイル読み込みエラーが発生した場合、**THEN** システムはエラーをログに記録し、他の検証を続行すること。
- **IF** bash互換性の問題でスクリプトが実行不可の場合、**THEN** システムは警告を表示し、手動検証を促すこと。

**NFR-DQ-003.2: ログ出力**

**受け入れ条件** (EARS):
- **THE** すべての検証スクリプトは、標準エラー出力（stderr）にログを出力すること。
- **THE** ログレベル（INFO/WARNING/ERROR）を明示し、CIでのフィルタリングを容易にすること。

#### NFR-DQ-004: ユーザビリティ

**NFR-DQ-004.1: エラーメッセージ品質**

**受け入れ条件** (EARS):
- **WHEN** 検証エラーが発生した場合、**THEN** システムはエラーメッセージに以下を含むこと:
  - 問題の具体的な内容（例: "ADR-004 が重複しています"）
  - 発生箇所（ファイル名、行番号）
  - 推奨修正方法（例: "ADR-006 に変更してください"）
  - 関連ドキュメントへのリンク（例: "詳細は CLAUDE.md の Editing Guidelines を参照"）

**NFR-DQ-004.2: 色分け表示**

**受け入れ条件** (EARS):
- **THE** ターミナル出力は、成功（緑）、警告（黄）、エラー（赤）で色分けすること。
- **THE** CI環境ではANSIカラーコードを無効化し、プレーンテキスト出力とすること。

#### NFR-DQ-005: セキュリティ

**NFR-DQ-005.1: コードインジェクション防止**

**受け入れ条件** (EARS):
- **THE** 検証スクリプトは、ユーザー入力（ファイル名、コミットメッセージ）を直接 `eval` または `exec` に渡さないこと。
- **THE** 正規表現パターンマッチングには、ReDoS（Regular Expression Denial of Service）耐性のあるパターンを使用すること。

**NFR-DQ-005.2: 機密情報保護**

**受け入れ条件** (EARS):
- **THE** 検証スクリプトは、ログ出力にトークン、パスワード、APIキーを含めないこと。
- **IF** エラーメッセージに機密情報が含まれる可能性がある場合、**THEN** システムはマスキングを適用すること。

---

### 制約条件

#### CON-DQ-001: 技術制約

- **CON-DQ-001.1**: 検証スクリプトは bash 3.x 互換であること（macOS標準環境サポート）
- **CON-DQ-001.2**: 外部依存ツールは、プロジェクト標準ツール（ripgrep, jq, git）のみ使用すること
- **CON-DQ-001.3**: Claude Code hooks は、将来的な実装を前提とし、現時点では Git hooks を優先すること

#### CON-DQ-002: 運用制約

- **CON-DQ-002.1**: Pre-commit Hooksは、初回セットアップ時に `.git/hooks/` へのシンボリックリンク作成を要求すること
- **CON-DQ-002.2**: CI/CDでの検証失敗は、PRマージをブロックすること（ただし、`[skip-ci]` タグでバイパス可能）
- **CON-DQ-002.3**: 検証スクリプトの更新は、CLAUDE.md の Editing Guidelines に従うこと

#### CON-DQ-003: 互換性制約

- **CON-DQ-003.1**: 既存のmeeting-minutes-ci specとの統合を前提とし、GitHub Actions workflowの設定ファイル形式に準拠すること
- **CON-DQ-003.2**: kiro-spec-guardianエージェント拡張は、既存のCore Responsibilitiesを維持しつつ追加すること（削除や全面書き換え禁止）

---

### Requirement Traceability Matrix

| 要件ID | 要件名 | 優先度 | 検証方法 | 実装状態 | 関連ADR/Principle |
|-------|--------|-------|---------|---------|------------------|
| REQ-DQ-001 | 開発フロードキュメント統合 | High | ドキュメントレビュー | ⚪ 未着手 | Principle 9（次の一手具体化） |
| REQ-DQ-001.1 | 開発ワークフローガイド作成 | High | マニュアルレビュー | ⚪ 未着手 | - |
| REQ-DQ-001.2 | エージェント定義使用ガイド | High | マニュアルレビュー | ⚪ 未着手 | - |
| REQ-DQ-001.3 | ドキュメント相互参照マップ | Medium | マニュアルレビュー | ⚪ 未着手 | - |
| REQ-DQ-002 | ADR採番検証スクリプト | Critical | 自動テスト | ✅ 完了 | - |
| REQ-DQ-002.1 | 重複検出機能 | Critical | ユニットテスト | ✅ 完了 | - |
| REQ-DQ-002.2 | 欠番検出機能 | High | ユニットテスト | ✅ 完了 | - |
| REQ-DQ-002.3 | bash 3互換性 | High | macOS bash 3での実行確認 | ✅ 完了 | CON-DQ-001.1 |
| REQ-DQ-002.4 | 出力可読性 | Medium | 手動確認 | ✅ 完了 | - |
| REQ-DQ-003 | 要件ID整合性チェック | High | 自動テスト | ⚪ 未着手 | Principle 6（TDD） |
| REQ-DQ-003.1 | 要件ID形式検証 | High | ユニットテスト | ⚪ 未着手 | - |
| REQ-DQ-003.2 | 要件IDリンク整合性チェック | High | 統合テスト | ⚪ 未着手 | - |
| REQ-DQ-003.3 | コミットメッセージ要件ID検証 | Medium | E2Eテスト | ⚪ 未着手 | - |
| REQ-DQ-003.4 | Traceability Matrix更新検証 | Medium | マニュアルレビュー | ⚪ 未着手 | - |
| REQ-DQ-004 | 禁止パターン検出 | High | 自動テスト | 🔵 部分完了 | Principle 1（プロセス境界） |
| REQ-DQ-004.1 | 禁止ライブラリインポート検出 | Critical | ユニットテスト | ✅ 完了 | ADR-001 |
| REQ-DQ-004.2 | 全面書き換えパターン検出 | High | ユニットテスト | ⚪ 未着手 | - |
| REQ-DQ-004.3 | 危険なAPI使用検出 | Medium | ユニットテスト | ⚪ 未着手 | - |
| REQ-DQ-004.4 | 設計原則違反パターン検出 | Medium | 統合テスト | ⚪ 未着手 | All Principles |
| REQ-DQ-005 | エージェント定義検証 | High | 自動テスト | ⚪ 未着手 | - |
| REQ-DQ-005.1 | 必須セクション検証 | High | ユニットテスト | ⚪ 未着手 | - |
| REQ-DQ-005.2 | ADR参照整合性チェック | High | 統合テスト | ⚪ 未着手 | - |
| REQ-DQ-005.3 | 設計原則カバレッジ検証 | Medium | 統合テスト | ⚪ 未着手 | All Principles |
| REQ-DQ-005.4 | Example品質チェック | Low | マニュアルレビュー | ⚪ 未着手 | - |
| REQ-DQ-006 | Pre-commit Hooks統合 | High | E2Eテスト | ⚪ 未着手 | - |
| REQ-DQ-006.1 | Git Pre-commit Hooks設定 | High | E2Eテスト | ⚪ 未着手 | - |
| REQ-DQ-006.2 | Claude Code Hooks統合 | Low | E2Eテスト | ⚪ 未着手 | CON-DQ-001.3 |
| REQ-DQ-006.3 | Bypass機能 | Medium | E2Eテスト | ⚪ 未着手 | - |
| REQ-DQ-006.4 | CI/CD連携 | High | GitHub Actions実行確認 | ⚪ 未着手 | CON-DQ-003.1 |
| REQ-DQ-007 | kiro-spec-guardian拡張 | High | 統合テスト | ⚪ 未着手 | - |
| REQ-DQ-007.1 | Documentation Change Detection | High | ユニットテスト | ⚪ 未着手 | - |
| REQ-DQ-007.2 | 削除要素チェック | Critical | 統合テスト | ⚪ 未着手 | - |
| REQ-DQ-007.3 | 整合性クロスチェック | High | 統合テスト | ⚪ 未着手 | - |
| REQ-DQ-007.4 | 変更影響分析 | Medium | マニュアルレビュー | ⚪ 未着手 | - |
| NFR-DQ-001 | パフォーマンス | High | 負荷テスト | ⚪ 未着手 | - |
| NFR-DQ-002 | 保守性 | Medium | コードレビュー | ⚪ 未着手 | - |
| NFR-DQ-003 | エラーハンドリング | High | 異常系テスト | ⚪ 未着手 | - |
| NFR-DQ-004 | ユーザビリティ | Medium | ユーザビリティテスト | ⚪ 未着手 | - |
| NFR-DQ-005 | セキュリティ | High | セキュリティレビュー | ⚪ 未着手 | Principle 3（セキュリティ責任境界） |

---

### Next Actions

1. **Design Phase開始**: 要件承認後、`/kiro:spec-design meeting-minutes-dev-quality -y` を実行し、技術設計ドキュメントを生成
2. **既存スクリプト統合**: `.kiro/scripts/validate_adrs.sh` を新フレームワークに統合し、共通ライブラリ（`scripts/lib/common.sh`）を抽出
3. **要件IDチェックスクリプト作成**: REQ-DQ-003 実装のため、`.kiro/scripts/check_requirement_ids.sh` の初期実装を開始

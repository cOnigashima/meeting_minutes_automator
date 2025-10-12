# Design Modules

このディレクトリには、`design.md`を9つの独立したモジュールに分割したドキュメントが格納されています。

## 分割の目的

元の`design.md`（2273行、約32KBのトークン）は、Claude Codeの読み取り制限（25000トークン）を超えていたため、以下の利点を得るために分割しました：

1. **読み取り可能性の向上**: 各ドキュメントが独立して読み取り可能
2. **編集の容易性**: 必要なセクションのみを編集可能
3. **ナビゲーション改善**: 役割別に最適なドキュメントへ素早くアクセス
4. **保守性向上**: セクション単位での更新・レビューが容易

## ドキュメント一覧

| ファイル名 | 行数 | サイズ | 説明 |
|-----------|------|-------|------|
| `design-overview.md` | 66行 | 3.5KB | プロジェクト概要、Goals/Non-Goals、図版チェックリスト、非機能要件 |
| `design-architecture.md` | 168行 | 12KB | アーキテクチャ、技術スタック、ADR-001〜004 |
| `design-flows.md` | 318行 | 13KB | 4つの主要フロー（音声処理、オフラインフォールバック、動的ダウングレード、デバイス切断） |
| `design-components.md` | 809行 | 30KB | 各コンポーネントの詳細設計、契約定義、依存関係（最大セクション） |
| `design-data.md` | 241行 | 7.8KB | IPC通信プロトコル、WebSocketメッセージフォーマット、ローカルストレージスキーマ |
| `design-error.md` | 276行 | 10KB | エラー分類、エラーコード定義、エラーハンドリング戦略 |
| `design-testing.md` | 157行 | 6.3KB | ユニット/統合/E2Eテスト方針、カバレッジ目標 |
| `design-dependencies.md` | 85行 | 4.1KB | 外部依存関係、内部依存関係、要件トレーサビリティマトリックス |
| `design-implementation.md` | 153行 | 5.1KB | 実装タスク、TDD実装フロー、Next Actions、Revision History |

**合計**: 2273行（元のdesign.mdと同じ）

## 使い方

### 1. マスタードキュメントから開始

まず[../design.md](../design.md)を読んで、プロジェクト全体の概要と各ドキュメントへのリンクを確認してください。

### 2. 役割別推奨ルート

#### 新規参加者（オンボーディング）
1. [design-overview.md](design-overview.md) - プロジェクトの全体像
2. [design-architecture.md](design-architecture.md) - アーキテクチャと重要な設計決定
3. [design-flows.md](design-flows.md) - 主要フローの理解

#### 実装担当エンジニア
1. [design-implementation.md](design-implementation.md) - 次のタスク確認
2. [design-components.md](design-components.md) - 担当コンポーネントの詳細設計
3. [design-data.md](design-data.md) - メッセージフォーマット確認

#### テストエンジニア
1. [design-testing.md](design-testing.md) - テスト方針とカバレッジ目標
2. [design-flows.md](design-flows.md) - テストシナリオ理解
3. [design-error.md](design-error.md) - 異常系テストケース設計

#### トラブルシューティング
1. [design-flows.md](design-flows.md) - 問題発生箇所のフロー確認
2. [design-error.md](design-error.md) - エラーコードと対応方法
3. [design-components.md](design-components.md) - コンポーネントの契約定義

### 3. 検索とナビゲーション

各ドキュメントは独立しているため、`grep`や`rg`（ripgrep）を使って横断検索できます：

```bash
# 特定の要件IDを検索
grep -r "STT-REQ-003" design-modules/

# 特定のコンポーネント名を検索
rg "VoiceActivityDetector" design-modules/

# 特定のADRを参照している箇所を検索
rg "ADR-001" design-modules/
```

## 元のdesign.mdについて

- **バックアップ**: [../design.md.backup](../design.md.backup) - 元の完全版（2273行）
- **マスタードキュメント**: [../design.md](../design.md) - 各ドキュメントへのリンク集

## 更新ガイドライン

### 分割ドキュメントを更新する場合

1. **該当セクションのみを編集**: 必要な分割ドキュメントのみを編集
2. **バージョン管理**: git commitで変更を追跡
3. **マスタードキュメント更新不要**: 分割ドキュメントへのリンクは変更不要

### 新しいセクションを追加する場合

1. **適切な分割ドキュメントに追加**: 最も関連性の高いドキュメントに追記
2. **サイズ超過の場合**: 809行を超える場合は、さらに分割を検討
3. **マスタードキュメント更新**: [../design.md](../design.md)の目次を更新

## 技術詳細

### 分割方法

元のdesign.mdは、以下のコマンドで分割されました：

```bash
# 1-66行: Overview and Requirements
sed -n '1,66p' design.md > design-modules/design-overview.md

# 67-234行: Architecture
sed -n '67,234p' design.md > design-modules/design-architecture.md

# 以下同様...
```

### 分割基準

- **論理的なセクション境界**: Markdownの`## `セクションを基準に分割
- **サイズ制約**: 各ドキュメントが25000トークン以下になるよう調整
- **独立性**: 各ドキュメントが独立して理解可能なよう配慮

## 関連ドキュメント

- **要件定義**: [../requirements.md](../requirements.md)
- **実装タスク**: [../tasks.md](../tasks.md)
- **ADR一覧**: [../adrs/](../adrs/)
- **Steering Documents**: [../../../steering/](../../../steering/)

## フィードバック

分割構造の改善提案がある場合は、以下をご検討ください：

- さらなる分割が必要か（809行のdesign-components.mdは分割候補）
- セクション間のリンク改善
- ナビゲーション構造の最適化

**最終更新**: 2025年10月12日
**メンテナー**: Claude Code

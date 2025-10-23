# Design Modules

このディレクトリには、`design.md`を10の独立したモジュールに分割したドキュメントが格納されています。

## 分割の目的

元の`design.md`（2808行、約40KBのトークン）は、Claude Codeの読み取り制限（25000トークン）を超えていたため、以下の利点を得るために分割しました：

1. **読み取り可能性の向上**: 各ドキュメントが独立して読み取り可能
2. **編集の容易性**: 必要なセクションのみを編集可能
3. **ナビゲーション改善**: 役割別に最適なドキュメントへ素早くアクセス
4. **保守性向上**: セクション単位での更新・レビューが容易

## ドキュメント一覧

| ファイル名 | 行数 | 説明 |
|-----------|------|------|
| `design-overview.md` | 31行 | プロジェクト概要、Goals/Non-Goals |
| `design-architecture.md` | 265行 | アーキテクチャ、既存基盤の継承、コンポーネント間の関係 |
| `design-tech-stack.md` | 144行 | 技術スタックと設計決定（OAuth 2.0、Google Docs API、Chrome拡張構成） |
| `design-flows.md` | 162行 | 主要フロー（OAuth認証、リアルタイム同期、オフラインキュー、エラー処理） |
| `design-state-management.md` | 162行 | バックエンド状態管理（Tauri側のOAuthトークン管理とセッション管理） |
| `design-components.md` | 770行 | 各コンポーネントの詳細設計、契約定義、依存関係（最大セクション） |
| `design-data.md` | 467行 | データモデル（OAuth token、Named Range、キュースキーマ、WebSocketメッセージ拡張） |
| `design-error.md` | 243行 | エラー分類、エラーコード定義、ハンドリング戦略 |
| `design-testing-security.md` | 372行 | テスト方針、セキュリティ考慮事項、パフォーマンス目標 |
| `design-migration-appendix.md` | 192行 | 移行戦略、既存コードへの影響、Appendix、Revision History |

**合計**: 2808行（元のdesign.mdと同じ）

## 使い方

### 1. マスタードキュメントから開始

まず[../design.md](../design.md)を読んで、プロジェクト全体の概要と各ドキュメントへのリンクを確認してください。

### 2. 役割別推奨ルート

#### 新規参加者（オンボーディング）
1. [design-overview.md](design-overview.md) - プロジェクトの全体像
2. [design-architecture.md](design-architecture.md) - アーキテクチャと既存基盤の継承
3. [design-flows.md](design-flows.md) - 主要フローの理解

#### 実装担当エンジニア
1. [design-components.md](design-components.md) - 担当コンポーネントの詳細設計
2. [design-data.md](design-data.md) - データモデルとWebSocketメッセージ拡張
3. [design-tech-stack.md](design-tech-stack.md) - 技術スタック確認

#### テストエンジニア
1. [design-testing-security.md](design-testing-security.md) - テスト方針とセキュリティ考慮事項
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
grep -r "DOCS-REQ-001" design-modules/

# 特定のコンポーネント名を検索
rg "OAuthManager" design-modules/

# 特定のフローを参照している箇所を検索
rg "OAuth 2.0" design-modules/
```

## 元のdesign.mdについて

- **バックアップ**: [../design.md.backup](../design.md.backup) - 元の完全版（2808行）
- **マスタードキュメント**: [../design.md](../design.md) - 各ドキュメントへのリンク集

## 更新ガイドライン

### 分割ドキュメントを更新する場合

1. **該当セクションのみを編集**: 必要な分割ドキュメントのみを編集
2. **バージョン管理**: git commitで変更を追跡
3. **マスタードキュメント更新不要**: 分割ドキュメントへのリンクは変更不要

### 新しいセクションを追加する場合

1. **適切な分割ドキュメントに追加**: 最も関連性の高いドキュメントに追記
2. **サイズ超過の場合**: 770行（design-components.md）を超える場合は、さらに分割を検討
3. **マスタードキュメント更新**: [../design.md](../design.md)の目次を更新

## 技術詳細

### 分割方法

元のdesign.mdは、以下のコマンドで分割されました：

```bash
# 1-31行: Overview
sed -n '1,31p' design.md > design-modules/design-overview.md

# 32-296行: Architecture
sed -n '32,296p' design.md > design-modules/design-architecture.md

# 以下同様...
```

### 分割基準

- **論理的なセクション境界**: Markdownの`## `セクションを基準に分割
- **サイズ制約**: 各ドキュメントが25000トークン以下になるよう調整
- **独立性**: 各ドキュメントが独立して理解可能なよう配慮

## 関連ドキュメント

- **要件定義**: [../requirements.md](../requirements.md)
- **実装タスク**: [../tasks.md](../tasks.md)
- **Steering Documents**: [../../../steering/](../../../steering/)
- **参考実装**: [../../meeting-minutes-stt/design-modules/](../../meeting-minutes-stt/design-modules/)

## フィードバック

分割構造の改善提案がある場合は、以下をご検討ください：

- さらなる分割が必要か（770行のdesign-components.mdは分割候補）
- セクション間のリンク改善
- ナビゲーション構造の最適化

**最終更新**: 2025年10月21日
**メンテナー**: Claude Code
**分割パターン**: meeting-minutes-stt design-modules/を参考

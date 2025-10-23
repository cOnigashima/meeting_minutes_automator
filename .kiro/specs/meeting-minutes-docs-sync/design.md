# Technical Design Document - meeting-minutes-docs-sync

**プロジェクト概要**: Google MeetからGoogle Docsへの議事録自動同期（MVP2フェーズ）。OAuth 2.0認証、Google Docs API統合、Named Range管理、オフライン同期を実現し、手動転記作業を削減します。

**前提知識**: [MVP0](../meeting-minutes-core/)（Walking Skeleton）と[MVP1](../meeting-minutes-stt/)（STT実装）の完了

---

**本ドキュメントは各設計モジュールへのリンク集です。詳細は [design-modules/](design-modules/) を参照してください。**

**背景**: 元のdesign.md（2808行、約40KBトークン）はClaude Codeの読み取り制限を超えていたため、10モジュールに分割しました。分割により、読み取り可能性・編集容易性・ナビゲーション性が向上しています。

## Modules

### 📖 Overview
**[design-modules/design-overview.md](design-modules/design-overview.md)** (31行)
- プロジェクト概要
- Goals / Non-Goals
- Purpose と Impact

### 🏗️ Architecture
**[design-modules/design-architecture.md](design-modules/design-architecture.md)** (265行)
- システムアーキテクチャ
- 既存基盤の継承（MVP0/MVP1からの拡張）
- コンポーネント間の関係

### 🔧 Technology Stack
**[design-modules/design-tech-stack.md](design-modules/design-tech-stack.md)** (144行)
- OAuth 2.0 選定理由
- Google Docs API 統合方針
- Chrome拡張構成

### 🔄 System Flows
**[design-modules/design-flows.md](design-modules/design-flows.md)** (162行)
- OAuth 2.0 認証フロー
- リアルタイム同期フロー
- オフラインキューイングと再同期
- エラー処理フロー

### 💾 State Management
**[design-modules/design-state-management.md](design-modules/design-state-management.md)** (162行)
- Tauri側のOAuthトークン管理
- セッション管理とライフサイクル
- トークンリフレッシュ戦略

### 🧩 Components and Interfaces
**[design-modules/design-components.md](design-modules/design-components.md)** (770行) — 最大セクション
- OAuthManager詳細設計
- DocsSyncManager詳細設計
- OfflineQueueManager詳細設計
- NamedRangeManager詳細設計
- 各コンポーネントの契約定義と依存関係

### 📊 Data Models
**[design-modules/design-data.md](design-modules/design-data.md)** (467行)
- OAuth Token スキーマ
- Named Range スキーマ
- Offline Queue スキーマ
- WebSocketメッセージ拡張（docsSyncフィールド）

### ⚠️ Error Handling
**[design-modules/design-error.md](design-modules/design-error.md)** (243行)
- エラー分類（Auth/Sync/Queue/API）
- エラーコード定義
- エラーハンドリング戦略
- ユーザー通知方針

### 🧪 Testing & Security
**[design-modules/design-testing-security.md](design-modules/design-testing-security.md)** (372行)
- テスト方針（ユニット/統合/E2E）
- セキュリティ考慮事項
- パフォーマンス目標
- スケーラビリティ戦略

### 🔀 Migration & Appendix
**[design-modules/design-migration-appendix.md](design-modules/design-migration-appendix.md)** (192行)
- 既存コードへの影響
- 移行戦略（段階的展開）
- Appendix（用語集、外部リンク）
- Revision History

---

## Quick Navigation

### 👤 役割別推奨ルート

**新規参加者（オンボーディング）:**
1. [Overview](design-modules/design-overview.md) → [Architecture](design-modules/design-architecture.md) → [Flows](design-modules/design-flows.md)

**実装担当エンジニア:**
1. [Components](design-modules/design-components.md) → [Data](design-modules/design-data.md) → [Tech Stack](design-modules/design-tech-stack.md)

**テストエンジニア:**
1. [Testing & Security](design-modules/design-testing-security.md) → [Flows](design-modules/design-flows.md) → [Error](design-modules/design-error.md)

**トラブルシューティング:**
1. [Flows](design-modules/design-flows.md) → [Error](design-modules/design-error.md) → [Components](design-modules/design-components.md)

---

## 関連ドキュメント

- **要件定義**: [requirements.md](requirements.md)
- **Steering Documents**: [../../steering/](../../steering/)
- **参考実装（MVP1）**: [../meeting-minutes-stt/design-modules/](../meeting-minutes-stt/design-modules/)

---

## 元のドキュメント

- **バックアップ**: [design.md.backup](design.md.backup) - 元の完全版（2808行）
- **分割理由**: Claude Code読み取り制限（25000トークン）超過
- **分割日**: 2025年10月21日

詳細なナビゲーションガイドは [design-modules/README.md](design-modules/README.md) を参照してください。

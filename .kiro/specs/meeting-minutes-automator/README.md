# Meeting Minutes Automator - Umbrella Spec

## 概要

このディレクトリは、Meeting Minutes Automatorプロジェクト全体のアーキテクチャと要件を定義する**アンブレラ仕様**です。

実装は以下の4つの独立したspecに分割されています:

## 実装Spec一覧

### 1. meeting-minutes-core (MVP0: Walking Skeleton)
**ステータス**: 🔵 準備中
**スコープ**: 全コンポーネントの最小疎通確認
- Tauriアプリスケルトン（音声録音インターフェース）
- Pythonサイドカー起動/終了管理
- WebSocketサーバー（Tauri側）
- Chrome拡張スケルトン（WebSocket接続のみ）
- Fake実装での端から端までのE2Eテスト

**成果物**: 録音→Fake処理→WebSocket→Chrome拡張コンソール表示

---

### 2. meeting-minutes-stt (MVP1: Real STT)
**ステータス**: ⚪ 未開始
**依存**: meeting-minutes-core完了
**スコープ**: 実音声処理の実装
- faster-whisper統合（モデルダウンロード、推論）
- webrtcvad統合（リアルタイムVAD）
- リソースベースモデル選択
- 音声デバイス管理（マイク/ループバック）
- ローカルストレージ（録音ファイル保存）

**成果物**: 実音声→文字起こし→ローカル保存

---

### 3. meeting-minutes-docs-sync (MVP2: Google Docs)
**ステータス**: ⚪ 未開始
**依存**: meeting-minutes-stt完了
**スコープ**: Google Docs連携
- OAuth 2.0フロー（Tauri側でOS Keychain統合）
- Google Docs API統合（Rate Limit Management）
- WebSocket Sync Protocol（再接続時のデータ整合性）
- Named Range管理とクリーンアップ
- Chrome拡張Content Script（Docs検出・挿入）

**成果物**: 文字起こし→Google Docsリアルタイム同期

---

### 4. meeting-minutes-llm (MVP3: LLM要約)
**ステータス**: ⚪ 未開始
**依存**: meeting-minutes-docs-sync完了
**スコープ**: 要約生成とUI洗練
- LLM API統合（OpenAI/ローカルLLM抽象化）
- セグメント要約/ローリングサマリー
- Tauri UI（録音制御、設定画面）
- リソース管理3段階閾値の完全実装
- エラーハンドリング、ログ運用方針の実装

**成果物**: プロダクション準備完了

---

## ドキュメント構成

- **requirements.md**: 全機能の要件定義（全体像のリファレンス）
- **design.md**: 全体アーキテクチャと技術設計（全体像のリファレンス）
- **spec.json**: メタデータ（phase: "archived-as-umbrella"）

## 使い方

### 新規実装開始時
各sub-specのrequirements/design生成時に、このアンブレラspecを参照:

```bash
# 例: meeting-minutes-core の要件定義時
# 「このspecはumbrella spec (.kiro/specs/meeting-minutes-automator) の
# Walking Skeleton部分を実装する」と明記
```

### 全体設計の更新
新機能追加時は、まずこのアンブレラspecのrequirements.mdとdesign.mdを更新し、その後各sub-specに反映します。

---

## 進捗管理

| Spec | Phase | Status | 完了予定 |
|------|-------|--------|---------|
| meeting-minutes-core | spec-init | 🔵 準備中 | 2025-10-07 |
| meeting-minutes-stt | - | ⚪ 未開始 | 2025-10-14 |
| meeting-minutes-docs-sync | - | ⚪ 未開始 | 2025-10-21 |
| meeting-minutes-llm | - | ⚪ 未開始 | 2025-10-28 |

---

## 関連ドキュメント

- **Steering Documents**: `.kiro/steering/`
  - `product.md`: 製品戦略
  - `tech.md`: 技術スタック
  - `structure.md`: プロジェクト構造
  - `principles.md`: 設計原則

- **Sub-Specs**: `.kiro/specs/`
  - `meeting-minutes-core/`
  - `meeting-minutes-stt/`
  - `meeting-minutes-docs-sync/`
  - `meeting-minutes-llm/`

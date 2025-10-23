# Technical Design - meeting-minutes-docs-sync: Overview

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Overview

meeting-minutes-docs-syncは、MVP1（meeting-minutes-stt）で確立した文字起こし機能の出力先として、Google Docsへのリアルタイム同期機能を実装するMVP2フェーズです。本設計は、Chrome拡張経由でのOAuth 2.0認証、Google Docs API統合、Named Range管理、オフライン時の自動キューイングと再同期を実現します。

**Purpose**: 文字起こし結果を即座にGoogle Docsへ反映し、構造化された議事録を自動生成することで、手動転記作業を削減し、チーム共有を加速します。

**Users**: 会議参加者がTauriアプリで録音・文字起こしを実行し、Chrome拡張を通じてGoogle Docsに自動的に議事録を作成します。

**Impact**: meeting-minutes-sttの文字起こし結果配信フローを拡張し、Chrome拡張にOAuth 2.0認証レイヤーとGoogle Docs API統合レイヤーを追加します。オフライン時のキューイング機構により、ネットワーク断絶時も作業継続性を保証します。

### Goals

- **OAuth 2.0認証フロー**: Chrome拡張からGoogleアカウント認証とトークン管理を実現
- **リアルタイム同期**: 文字起こし結果を2秒以内にGoogle Docsへ反映
- **Named Range管理**: 構造化された議事録フォーマットの自動生成と挿入位置管理
- **オフライン対応**: ネットワーク切断時のローカルキューイングと自動再同期
- **エラーハンドリング**: トークンリフレッシュ、APIエラー、レート制限への適切な対処
- **ユーザー設定**: 同期動作のカスタマイズ（タイムスタンプ表示、バッファリング時間等）

### Non-Goals

- LLM要約生成（MVP3 meeting-minutes-llmで実装）
- 本格的なUI洗練（MVP3で実施）
- 複数ドキュメント同時編集
- リアルタイムコラボレーション機能
- トークンの暗号化保存（MVP2では`chrome.storage.local`に平文保存、MVP3で暗号化実装予定）

---


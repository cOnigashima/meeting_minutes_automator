# Release Notes - v0.2.0 (MVP2: Google Docs Sync)

## Overview

Meeting Minutes Automator v0.2.0では、Google Docsへのリアルタイム同期機能を追加しました。
会議中の文字起こし結果を自動的にGoogle Docsに保存できるようになります。

## New Features

### Google Docs Integration

- **OAuth 2.0認証**: Googleアカウントとの安全な連携
- **リアルタイム同期**: 文字起こし結果が2秒以内にGoogle Docsに反映
- **オフライン対応**: ネットワーク切断時もキューに保存、復帰後に自動同期
- **Named Range管理**: 構造化されたフォーマットでドキュメントに挿入

### Settings

- Google Docs同期の有効/無効切り替え
- タイムスタンプ表示のオン/オフ
- 話者名表示のオン/オフ
- バッファリング時間の調整（1-5秒）

### Error Handling

- Exponential Backoffによる自動リトライ
- Token Bucketレート制限
- 自動トークンリフレッシュ

## Improvements

- WebSocket接続の安定性向上（Offscreen Document使用）
- ポートスキャンの高速化（キャッシング機能）
- エラーメッセージの改善

## Bug Fixes

- なし（新機能リリース）

## Breaking Changes

- なし

## Dependencies

### Chrome Extension

- `@playwright/test: ^1.40.0` (dev)
- `ws: ^8.16.0` (dev)

### Tauri App

- 変更なし

## Requirements

- Chrome 120以上
- Googleアカウント
- Google Docs APIアクセス権限

## Known Issues

- Chrome拡張E2EテストはHeadedモードのみ対応
- UAT実施前のため、実ユーザーフィードバックは未収集

## Upgrade Guide

1. Chrome拡張機能を更新
2. 「Googleアカウントと連携」をクリック
3. 権限を許可
4. ドキュメントIDを設定

## Rollback

v0.1.0にロールバックする場合:
1. Chrome拡張機能をアンインストール
2. v0.1.0のzipをダウンロード
3. `chrome://extensions`で読み込み

詳細: `docs/release/rollback-procedure.md`

---

**Release Date**: TBD
**Version**: 0.2.0
**Codename**: MVP2-DocsSync

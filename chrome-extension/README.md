# Meeting Minutes Automator - Chrome Extension

Google Meetで自動議事録作成を行うChrome拡張（Walking Skeleton - MVP0）

## 概要

この拡張は、Google Meetでの会議中にTauriアプリケーションとWebSocket接続を行い、文字起こし結果を受け取ります。

## インストール手順

### 1. 拡張機能の読み込み

1. Chromeブラウザで `chrome://extensions/` を開く
2. 右上の「デベロッパーモード」をONにする
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. このディレクトリ（`chrome-extension/`）を選択

### 2. Tauriアプリケーションの起動

拡張機能がWebSocket接続を確立するには、Tauriアプリケーションが起動している必要があります。

```bash
cd src-tauri
cargo run
```

WebSocketサーバーがポート9001-9100の範囲で起動します。

### 3. Google Meetでの使用

1. https://meet.google.com にアクセス
2. 会議に参加またはテストミーティングを作成
3. ブラウザの開発者ツール（F12）を開き、Consoleタブを確認
4. 以下のようなログが表示されればWebSocket接続成功：

```
[Meeting Minutes] Content script loaded on Google Meet
[Meeting Minutes] Starting WebSocket connection...
[Meeting Minutes] ✅ Connected to WebSocket server on port 9001
[Meeting Minutes] ✅ Connection established - Session: [session-id]
```

## アーキテクチャ

### Content Script（content-script.js）

- Google Meetページに注入される
- WebSocketクライアントを実装
- ポートスキャン（9001-9100）とリトライロジックを実装
- 接続が切れた場合、指数バックオフでリトライ（1s→2s→4s→8s→16s）

### Service Worker（service-worker.js）

- Manifest V3対応のバックグラウンドスクリプト
- 現在は最小限の実装（メッセージリレーのみ）
- Content Scriptからのメッセージを処理

## WebSocketメッセージ仕様

### 受信メッセージ

#### Connected
```json
{
  "type": "connected",
  "message_id": "ws-1",
  "session_id": "uuid-v4",
  "timestamp": 1234567890
}
```

#### Transcription
```json
{
  "type": "transcription",
  "message_id": "ws-2",
  "session_id": "uuid-v4",
  "text": "文字起こし結果",
  "timestamp": 1234567890
}
```

#### Error
```json
{
  "type": "error",
  "message_id": "ws-3",
  "session_id": "uuid-v4",
  "message": "エラーメッセージ",
  "timestamp": 1234567890
}
```

## トラブルシューティング

### WebSocket接続できない

**症状**: `No WebSocket server found in port range 9001-9100`

**原因**:
- Tauriアプリケーションが起動していない
- ポート9001-9100がすべて使用中

**対処法**:
1. Tauriアプリケーションが起動しているか確認
2. ターミナルでポート状態を確認: `lsof -i :9001-9100`

### 拡張機能が読み込めない

**症状**: 拡張機能が表示されない、エラーが出る

**対処法**:
1. `chrome://extensions/` でエラー内容を確認
2. manifest.jsonの構文エラーがないか確認
3. 拡張機能を削除して再度読み込む

### コンソールにログが表示されない

**症状**: Content scriptのログが表示されない

**対処法**:
1. Google Meetページで開発者ツールを開く（F12）
2. Consoleタブを選択
3. フィルタで `[Meeting Minutes]` を検索
4. ページを再読み込み（Ctrl+R / Cmd+R）

## 開発者向け情報

### ファイル構成

```
chrome-extension/
├── manifest.json          # Manifest V3設定
├── content-script.js      # WebSocketクライアント（192行）
├── service-worker.js      # サービスワーカー（37行）
└── README.md             # このファイル
```

### 次のステップ（MVP1+）

- [ ] UI実装（文字起こし結果の表示）
- [ ] 録音開始/停止ボタン
- [ ] Popupページ（設定画面）
- [ ] エラー通知（chrome.notifications API使用）
- [ ] リアルタイム文字起こし表示

## ライセンス

Meeting Minutes Automator プロジェクトの一部として配布

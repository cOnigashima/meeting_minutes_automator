# Requirements Document

## Project Description (Input)
meeting-minutes-docs-sync: [MVP2] Google Docs同期。meeting-minutes-stt完了後に実装。OAuth 2.0認証フロー、Google Docs API統合（documents.batchUpdate）、Named Range管理、オフライン時のキューイングと再同期。成果物: 文字起こしテキスト→Google Docsリアルタイム挿入

## Introduction

meeting-minutes-docs-syncは、meeting-minutes-stt（MVP1）で確立した文字起こし機能の出力先として、Google Docsへのリアルタイム同期機能を実装するMVP2フェーズです。このspecでは、Chrome拡張経由でのOAuth 2.0認証、Google Docs API統合、オフライン時のキューイングと自動再同期を実現します。

**ビジネス価値**:
- 文字起こし結果の即座なドキュメント化
- チームメンバーとのリアルタイム共有
- 構造化された議事録の自動生成
- オフライン環境での作業継続性

**スコープ制限（MVP2の範囲）**:
- LLM要約生成は含まない（MVP3 meeting-minutes-llmで実装）
- UIの本格的な洗練は含まない（MVP3で実施）
- 複数ドキュメント同時編集は含まない（単一ドキュメントのみ）
- リアルタイムコラボレーション機能は含まない

**meeting-minutes-sttからの拡張**:
- Chrome拡張にOAuth 2.0フローを追加
- Google Docs API統合レイヤーの実装
- オフライン時のメッセージキューイング機能
- WebSocketメッセージに同期ステータスフィールドを追加

---

## Glossary

| 用語 | 英語表記 | 定義 |
|-----|---------|------|
| **OAuth 2.0** | OAuth 2.0 | Googleアカウント認証のための業界標準プロトコル。ユーザーの明示的な許可を得てAPIアクセス権を取得。 |
| **Google Docs API** | Google Docs API | Google Docsドキュメントをプログラマティックに操作するためのREST API。documents.batchUpdateメソッドを使用。 |
| **Named Range** | Named Range | Google Docs内の特定位置に付けられた名前付き範囲。プログラムから特定の位置を一意に識別するために使用。 |
| **batchUpdate** | batchUpdate | Google Docs APIのメソッド。複数の編集操作を1つのリクエストにまとめて実行し、API呼び出し回数を削減。 |
| **オフラインキュー** | Offline Queue | ネットワーク切断時に同期待ちのメッセージをローカルに保存するキュー。再接続時に自動再送信。 |
| **同期カーソル** | Sync Cursor | Google Docs内の現在の挿入位置を示すカーソル。文字起こし結果を順次追加するために使用。 |
| **トークンリフレッシュ** | Token Refresh | OAuth 2.0アクセストークンの有効期限切れ時に、リフレッシュトークンを使用して新しいアクセストークンを取得する処理。 |

---

## Requirements

### DOCS-REQ-001: OAuth 2.0 Authentication Flow

**Objective**: Chromeユーザーとして、Chrome拡張からGoogleアカウントにログインしたい。これにより、Google Docs APIへのアクセス権を取得できる。

#### Acceptance Criteria

1. **DOCS-REQ-001.1**: WHEN ユーザーがChrome拡張のポップアップで「Google連携」ボタンをクリック THEN Chrome拡張 SHALL OAuth 2.0認証フローを開始する
2. **DOCS-REQ-001.2**: WHEN OAuth 2.0認証フローが開始される THEN Chrome拡張 SHALL Googleの認証画面を新しいタブで開く
3. **DOCS-REQ-001.3**: WHEN ユーザーがGoogleアカウントを選択し、権限を許可 THEN Chrome拡張 SHALL 認証コードを受け取る
4. **DOCS-REQ-001.4**: WHEN 認証コードを受け取る THEN Chrome拡張 SHALL 認証コードをアクセストークンとリフレッシュトークンに交換する
5. **DOCS-REQ-001.5**: WHEN トークン交換が完了 THEN Chrome拡張 SHALL アクセストークンとリフレッシュトークンを`chrome.storage.local`に保存する
6. **DOCS-REQ-001.6**: WHEN トークンが保存される THEN Chrome拡張 SHALL WebSocket経由でTauriアプリに「認証成功」メッセージを送信する
7. **DOCS-REQ-001.7**: WHEN ユーザーが権限を拒否 THEN Chrome拡張 SHALL エラーメッセージ「Google連携がキャンセルされました」を表示する
8. **DOCS-REQ-001.8**: IF アクセストークンの有効期限が切れる THEN Chrome拡張 SHALL リフレッシュトークンを使用して自動的に新しいアクセストークンを取得する
9. **DOCS-REQ-001.9**: IF リフレッシュトークンも無効 THEN Chrome拡張 SHALL ユーザーに再認証を促すメッセージを表示する
10. **DOCS-REQ-001.10**: IF トークンリフレッシュが失敗し再認証が必要 THEN Chrome拡張 SHALL 再認証完了までの文字起こしメッセージをオフラインキューと同様にローカルキュー（`chrome.storage.local`の`auth_pending_queue`）に保存する
11. **DOCS-REQ-001.11**: WHEN 再認証が完了 THEN Chrome拡張 SHALL ローカルキューに保存されたメッセージを自動的にGoogle Docs APIへ時系列順に送信する

---

### DOCS-REQ-002: Google Docs API Integration

**Objective**: ソフトウェアエンジニアとして、Chrome拡張からGoogle Docs APIを呼び出したい。これにより、文字起こし結果をプログラマティックにドキュメントへ挿入できる。

#### Acceptance Criteria

1. **DOCS-REQ-002.1**: WHEN ユーザーがChrome拡張で「Google Docs同期開始」を実行 THEN Chrome拡張 SHALL アクティブなGoogle DocsタブのドキュメントIDとタイトルを取得する
2. **DOCS-REQ-002.2**: WHEN ドキュメントIDとタイトルを取得 THEN Chrome拡張 SHALL それらを`chrome.storage.local`の`active_document`に保存する
3. **DOCS-REQ-002.3**: WHEN ドキュメント情報が保存される THEN Chrome拡張 SHALL Google Docs API `documents.batchUpdate`メソッドを呼び出す
4. **DOCS-REQ-002.4**: WHEN `batchUpdate`リクエストを送信 THEN Chrome拡張 SHALL 以下のJSON形式でリクエストボディを構築する:
   ```json
   {
     "requests": [
       {
         "insertText": {
           "location": {
             "index": 1
           },
           "text": "文字起こし結果テキスト\n"
         }
       }
     ]
   }
   ```
5. **DOCS-REQ-002.5**: WHEN `batchUpdate`リクエストが成功 THEN Google Docs API SHALL HTTPステータス200とレスポンスボディを返す
6. **DOCS-REQ-002.6**: WHEN APIレスポンスを受信 THEN Chrome拡張 SHALL 挿入成功ログを記録する
7. **DOCS-REQ-002.7**: IF `batchUpdate`リクエストが失敗（401 Unauthorized） THEN Chrome拡張 SHALL トークンリフレッシュを試行し、リクエストを再送信する
8. **DOCS-REQ-002.8**: IF `batchUpdate`リクエストが失敗（403 Forbidden） THEN Chrome拡張 SHALL エラーメッセージ「ドキュメントへのアクセス権限がありません」を表示する
9. **DOCS-REQ-002.9**: IF `batchUpdate`リクエストが失敗（429 Too Many Requests） THEN Chrome拡張 SHALL 指数バックオフ（1秒、2秒、4秒）でリトライする
10. **DOCS-REQ-002.10**: WHEN 保存済みドキュメントIDが存在 THEN Chrome拡張 SHALL タブのアクティブ状態に関わらずバックグラウンドでGoogle Docs APIを呼び出す
11. **DOCS-REQ-002.11**: IF Google Docsタブが閉じられた THEN Chrome拡張 SHALL バックグラウンド同期を継続し、ポップアップUIに「タブが閉じられていますが同期中」と表示する
12. **DOCS-REQ-002.12**: IF 同期開始時にアクティブタブがGoogle Docsでない THEN Chrome拡張 SHALL エラーメッセージ「Google Docsのタブをアクティブにしてから同期開始してください」を表示する

---

### DOCS-REQ-003: Named Range Management

**Objective**: ソフトウェアエンジニアとして、Named Rangeを使用してドキュメント内の特定位置を管理したい。これにより、文字起こし結果を構造化して挿入できる。

#### Acceptance Criteria

1. **DOCS-REQ-003.1**: WHEN 初回のGoogle Docs同期を開始 THEN Chrome拡張 SHALL ドキュメント内に`transcript_cursor`という名前のNamed Rangeを作成する
2. **DOCS-REQ-003.2**: WHEN Named Rangeを作成 THEN Chrome拡張 SHALL `documents.batchUpdate`の`createNamedRange`リクエストを送信する:
   ```json
   {
     "requests": [
       {
         "createNamedRange": {
           "name": "transcript_cursor",
           "range": {
             "startIndex": 1,
             "endIndex": 1
           }
         }
       }
     ]
   }
   ```
3. **DOCS-REQ-003.3**: WHEN 文字起こし結果を挿入 THEN Chrome拡張 SHALL `transcript_cursor`の現在位置を取得する
4. **DOCS-REQ-003.4**: WHEN 現在位置を取得 THEN Chrome拡張 SHALL Google Docs API `documents.get`メソッドで`transcript_cursor`の`startIndex`を読み取る
5. **DOCS-REQ-003.5**: WHEN 文字起こしテキストを挿入後 THEN Chrome拡張 SHALL `transcript_cursor`の位置を挿入したテキストの末尾に更新する
6. **DOCS-REQ-003.6**: WHEN Named Rangeの位置を更新 THEN Chrome拡張 SHALL `documents.batchUpdate`の`updateNamedRange`リクエストを送信する
7. **DOCS-REQ-003.7**: IF `transcript_cursor`が存在しない THEN Chrome拡張 SHALL 以下の優先順位でNamed Rangeを再作成する:
   - a. ドキュメント内に「## 文字起こし」見出しが存在する場合、その直後の位置（見出しテキスト検索で特定）
   - b. 上記が見つからない場合、ドキュメントの末尾位置（`documents.get`でドキュメント全体の`endIndex`を取得）
   - c. ドキュメントが空の場合、index=1（先頭）
8. **DOCS-REQ-003.8**: WHEN Named Range再作成時 THEN Chrome拡張 SHALL ERRORレベルでログに記録し、ポップアップUIに「挿入位置が再設定されました」と通知する

---

### DOCS-REQ-004: Real-Time Text Insertion

**Objective**: エンドユーザーとして、文字起こし結果がリアルタイムでGoogle Docsに反映されることを確認したい。これにより、会議中に同時進行で議事録が作成される。

#### Acceptance Criteria

1. **DOCS-REQ-004.1**: WHEN Tauriアプリが文字起こし結果をWebSocket配信 THEN Chrome拡張 SHALL 1秒以内に文字起こしメッセージを受信する
2. **DOCS-REQ-004.2**: WHEN 確定テキスト（`isPartial: false`）を受信 THEN Chrome拡張 SHALL Google Docsへの挿入処理を開始する
3. **DOCS-REQ-004.3**: WHEN 部分テキスト（`isPartial: true`）を受信 THEN Chrome拡張 SHALL Google Docsへの挿入を行わず、ポップアップUIにのみ表示する
4. **DOCS-REQ-004.4**: WHEN Google Docs挿入処理を開始 THEN Chrome拡張 SHALL `transcript_cursor`の位置にテキストを挿入する
5. **DOCS-REQ-004.5**: WHEN テキスト挿入が完了 THEN Chrome拡張 SHALL 挿入完了通知をWebSocket経由でTauriアプリに送信する:
   ```json
   {
     "type": "docs_sync_success",
     "messageId": 123,
     "insertedAt": "2025-10-02T10:30:00Z"
   }
   ```
6. **DOCS-REQ-004.6**: WHEN 複数の文字起こし結果が短時間に連続して届く THEN Chrome拡張 SHALL 最大3秒間のバッファリングを行い、1回の`batchUpdate`リクエストにまとめて送信する
7. **DOCS-REQ-004.7**: WHEN バッファリング中のテキストが500文字を超える THEN Chrome拡張 SHALL 即座に`batchUpdate`リクエストを送信する（バッファリング時間を待たない）

---

### DOCS-REQ-005: Offline Queueing and Auto-Resync

**Objective**: エンドユーザーとして、ネットワーク切断時も文字起こしが継続し、再接続時に自動的にGoogle Docsへ同期されることを期待する。これにより、オフライン環境でも作業を継続できる。

#### Acceptance Criteria

1. **DOCS-REQ-005.1**: WHEN ネットワーク接続が切断される THEN Chrome拡張 SHALL ネットワーク切断を検知する
2. **DOCS-REQ-005.2**: WHEN ネットワーク切断を検知 THEN Chrome拡張 SHALL ポップアップUIに「オフライン（同期待機中）」ステータスを表示する
3. **DOCS-REQ-005.3**: WHEN オフライン状態で文字起こしメッセージを受信 THEN Chrome拡張 SHALL メッセージをローカルキューに保存する
4. **DOCS-REQ-005.4**: WHEN メッセージをローカルキューに保存 THEN Chrome拡張 SHALL `chrome.storage.local`の`offline_queue`配列に追加する
5. **DOCS-REQ-005.5**: WHEN ネットワーク接続が回復 THEN Chrome拡張 SHALL ネットワーク回復を検知する
6. **DOCS-REQ-005.6**: WHEN ネットワーク回復を検知 THEN Chrome拡張 SHALL ローカルキューに保存されたメッセージを取得する
7. **DOCS-REQ-005.7**: WHEN ローカルキューのメッセージを取得 THEN Chrome拡張 SHALL メッセージを時系列順にGoogle Docs APIへ再送信する
8. **DOCS-REQ-005.8**: WHEN 全メッセージの再送信が完了 THEN Chrome拡張 SHALL ローカルキューをクリアする
9. **DOCS-REQ-005.9**: WHEN 全メッセージの再送信が完了 THEN Chrome拡張 SHALL ポップアップUIに「同期完了」通知を表示する
10. **DOCS-REQ-005.10**: IF 再送信中にエラーが発生 THEN Chrome拡張 SHALL 失敗したメッセージのみをキューに残し、ユーザーに手動再試行を促す
11. **DOCS-REQ-005.11**: WHEN オフラインキューが最大サイズの80%に達する THEN Chrome拡張 SHALL ポップアップUIに警告「オフラインキューが残り[N]件です。ネットワーク接続を確認してください」を表示する
12. **DOCS-REQ-005.12**: WHEN オフラインキューが最大サイズに達する THEN Chrome拡張 SHALL chrome.notifications APIで全画面通知「オフラインキューが上限に達しました。これ以上の文字起こしは保存されません。録音を停止するか、ネットワーク接続を回復してください」を表示する

---

### DOCS-REQ-006: Document Structure Management

**Objective**: ソフトウェアエンジニアとして、Google Docsに構造化された議事録フォーマットを自動生成したい。これにより、読みやすい議事録が作成される。

#### Acceptance Criteria

1. **DOCS-REQ-006.1**: WHEN 初回の文字起こし結果をGoogle Docsに挿入 THEN Chrome拡張 SHALL ドキュメントの先頭に以下の構造を自動生成する:
   ```
   # 議事録 - [会議タイトル]
   日時: [録音開始時刻]

   ## 文字起こし
   [ここから文字起こし結果が挿入される]
   ```
2. **DOCS-REQ-006.2**: WHEN 構造を自動生成 THEN Chrome拡張 SHALL Markdown形式のテキストをGoogle Docs形式（見出し、段落）に変換する
3. **DOCS-REQ-006.3**: WHEN 文字起こし結果を挿入 THEN Chrome拡張 SHALL 「## 文字起こし」セクションの末尾に追加する
4. **DOCS-REQ-006.4**: WHEN 各発話セグメントを挿入 THEN Chrome拡張 SHALL タイムスタンプを含むフォーマットで挿入する:
   ```
   [10:30:15] 文字起こしテキスト
   ```
5. **DOCS-REQ-006.5**: WHEN 録音セッションが終了 THEN Chrome拡張 SHALL ドキュメントの末尾に以下を追加する:
   ```
   ---
   録音終了時刻: [終了時刻]
   総時間: [録音時間]
   ```
6. **DOCS-REQ-006.6**: WHEN ドキュメント構造を自動生成 THEN Chrome拡張 SHALL 「## 文字起こし」セクションの直前に以下の注意書きを挿入する:
   ```
   > ⚠️ 注意: 「## 文字起こし」セクション以下は自動更新されます。会議中の手動編集は避けてください。メモは上部に記入してください。
   ```

---

### DOCS-REQ-007: WebSocket Protocol Extension

**Objective**: ソフトウェアエンジニアとして、WebSocket通信にGoogle Docs同期ステータスを含めたい。これにより、Tauriアプリ側で同期状態を監視できる。

#### Acceptance Criteria

1. **DOCS-REQ-007.1**: WHEN Chrome拡張がGoogle Docs同期を開始 THEN Chrome拡張 SHALL WebSocket経由でTauriアプリに以下のメッセージを送信する:
   ```json
   {
     "type": "docs_sync_started",
     "documentId": "google-docs-id",
     "documentTitle": "議事録 - 2025-10-02",
     "timestamp": 1696234567890
   }
   ```
2. **DOCS-REQ-007.2**: WHEN Chrome拡張がオフライン状態になる THEN Chrome拡張 SHALL WebSocket経由でTauriアプリに以下のメッセージを送信する:
   ```json
   {
     "type": "docs_sync_offline",
     "queuedMessages": 5,
     "timestamp": 1696234567890
   }
   ```
3. **DOCS-REQ-007.3**: WHEN Chrome拡張がオンライン復帰 THEN Chrome拡張 SHALL WebSocket経由でTauriアプリに以下のメッセージを送信する:
   ```json
   {
     "type": "docs_sync_online",
     "resyncInProgress": true,
     "timestamp": 1696234567890
   }
   ```
4. **DOCS-REQ-007.4**: WHEN 文字起こしメッセージをWebSocket配信 THEN Tauriアプリ SHALL 既存のメッセージ形式に`docsSync`フィールドを追加する:
   ```json
   {
     "messageId": 123,
     "sessionId": "session-uuid",
     "timestamp": 1696234567890,
     "type": "transcription",
     "isPartial": false,
     "text": "文字起こし結果",
     "docsSync": {
       "enabled": true,
       "status": "synced",
       "documentId": "google-docs-id"
     }
   }
   ```

---

### DOCS-REQ-008: Settings and Preferences

**Objective**: エンドユーザーとして、Google Docs同期の動作をカスタマイズしたい。これにより、個人の作業スタイルに合わせた設定ができる。

#### Acceptance Criteria

1. **DOCS-REQ-008.1**: WHEN ユーザーがChrome拡張の設定画面を開く THEN Chrome拡張 SHALL 以下の設定項目を表示する:
   - Google Docs自動同期の有効/無効
   - タイムスタンプ表示の有効/無効
   - バッファリング時間（1〜5秒）
   - オフラインキューの最大サイズ（100〜1000メッセージ）
2. **DOCS-REQ-008.2**: WHEN ユーザーが「Google Docs自動同期」を無効化 THEN Chrome拡張 SHALL 文字起こしメッセージを受信してもGoogle Docs APIを呼び出さない
3. **DOCS-REQ-008.3**: WHEN ユーザーが「タイムスタンプ表示」を無効化 THEN Chrome拡張 SHALL 文字起こしテキストを`[HH:MM:SS]`プレフィックスなしで挿入する
4. **DOCS-REQ-008.4**: WHEN ユーザーがバッファリング時間を変更 THEN Chrome拡張 SHALL 次の文字起こしメッセージから新しいバッファリング時間を適用する
5. **DOCS-REQ-008.5**: WHEN オフラインキューが最大サイズに達する THEN Chrome拡張 SHALL 新しいメッセージの受信を停止し、警告メッセージ「オフラインキューが上限に達しました」を表示する

---

## Non-Functional Requirements

### DOCS-NFR-001: Performance

1. **DOCS-NFR-001.1**: WHEN 文字起こしメッセージを受信してからGoogle Docsに挿入完了まで THEN Chrome拡張 SHALL 2秒以内に処理を完了する（ネットワーク遅延を除く）
2. **DOCS-NFR-001.2**: WHEN `batchUpdate`リクエストを送信 THEN Google Docs API SHALL 95パーセンタイルで3秒以内に応答する
3. **DOCS-NFR-001.3**: WHEN オフラインキューから再送信 THEN Chrome拡張 SHALL Google Docs API Rate Limit（60件/分）を遵守し、100メッセージあたり最大120秒（2分）以内に処理する
4. **DOCS-NFR-001.4**: WHEN ローカルキューにメッセージを保存 THEN Chrome拡張 SHALL 10ms以内に`chrome.storage.local`への書き込みを完了する

### DOCS-NFR-002: Reliability

1. **DOCS-NFR-002.1**: WHEN OAuth 2.0トークンリフレッシュが失敗 THEN Chrome拡張 SHALL 最大3回まで自動再試行する
2. **DOCS-NFR-002.2**: WHEN Google Docs APIがエラーを返す THEN Chrome拡張 SHALL エラーの種類に応じた適切なフォールバック処理を実行する
3. **DOCS-NFR-002.3**: WHEN ネットワークが不安定（断続的切断） THEN Chrome拡張 SHALL 指数バックオフで再接続を試行し、ユーザーに現在の状態を通知する
4. **DOCS-NFR-002.4**: WHEN オフラインキューのサイズが100メッセージを超える THEN Chrome拡張 SHALL ユーザーに「大量の同期待ちメッセージがあります」警告を表示する

### DOCS-NFR-003: Security

1. **DOCS-NFR-003.1**: WHEN OAuth 2.0トークンを保存 THEN Chrome拡張 SHALL `chrome.storage.local`に保存する（注: MVP2段階では暗号化なし、MVP3で暗号化実装予定）
2. **DOCS-NFR-003.2**: WHEN Google Docs APIリクエストを送信 THEN Chrome拡張 SHALL HTTPSプロトコルを使用する
3. **DOCS-NFR-003.3**: WHEN アクセストークンをAPIリクエストに含める THEN Chrome拡張 SHALL `Authorization: Bearer [token]`ヘッダーに設定する
4. **DOCS-NFR-003.4**: WHEN ユーザーが「Google連携解除」を実行 THEN Chrome拡張 SHALL `chrome.storage.local`からトークンを削除し、Googleへトークン無効化リクエストを送信する

### DOCS-NFR-004: Compatibility

1. **DOCS-NFR-004.1**: Chrome拡張 SHALL Google Chrome 116以降で動作する
2. **DOCS-NFR-004.2**: Chrome拡張 SHALL Google Docs API v1を使用する
3. **DOCS-NFR-004.3**: Chrome拡張 SHALL OAuth 2.0プロトコルに準拠する
4. **DOCS-NFR-004.4**: Chrome拡張 SHALL Manifest V3の仕様に準拠する

### DOCS-NFR-005: Usability

1. **DOCS-NFR-005.1**: WHEN OAuth 2.0認証フローを実行 THEN Chrome拡張 SHALL ユーザーに明確な手順を示すガイドを表示する
2. **DOCS-NFR-005.2**: WHEN Google Docs同期エラーが発生 THEN Chrome拡張 SHALL ユーザーに理解しやすいエラーメッセージと解決策を表示する
3. **DOCS-NFR-005.3**: WHEN オフライン状態 THEN Chrome拡張 SHALL ポップアップUIにオフライン状態とキュー内メッセージ数を表示する
4. **DOCS-NFR-005.4**: WHEN Google Docs同期が成功 THEN Chrome拡張 SHALL 控えめな成功通知（トーストまたはバッジ）を表示する

### DOCS-NFR-006: Logging

1. **DOCS-NFR-006.1**: WHEN OAuth 2.0認証フローを実行 THEN Chrome拡張 SHALL 認証ステータス（開始、成功、失敗）をINFOレベルでログに記録する
2. **DOCS-NFR-006.2**: WHEN Google Docs APIリクエストを送信 THEN Chrome拡張 SHALL リクエストURL、メソッド、レスポンスステータスをDEBUGレベルでログに記録する
3. **DOCS-NFR-006.3**: WHEN エラーが発生 THEN Chrome拡張 SHALL エラーメッセージ、スタックトレース、コンテキスト（ドキュメントID等）をERRORレベルでログに記録する
4. **DOCS-NFR-006.4**: WHEN オフラインキューに保存/再送信 THEN Chrome拡張 SHALL キュー操作（追加、削除、クリア）をINFOレベルでログに記録する

---

## Out of Scope (明確な非スコープ)

以下の機能は、本MVP2実装には**含まれません**。後続のMVPまたは将来拡張で実装されます:

### MVP3 (meeting-minutes-llm) で実装予定
- LLM API統合（OpenAI/ローカルLLM）
- セグメント要約/ローリングサマリー生成
- 議事録の自動整形とアクションアイテム抽出
- Tauri UI洗練（設定画面、履歴表示）

### 将来検討事項（スコープ外）
- 複数Google Docsドキュメントへの同時同期
- リアルタイムコラボレーション機能（複数ユーザーの同時編集）
- Google Slides、Google Sheetsへの同期
- 他のドキュメントサービス（Notion、Confluence等）への対応
- 音声ファイルのGoogle Driveへの自動バックアップ
- 長時間会議（3時間以上）でのドキュメントサイズ最適化とパフォーマンス改善
- ユーザー手動編集領域と自動挿入領域の技術的分離（Google Docs API書き込み制限活用）

---

## Success Criteria

本MVP2実装は、以下の条件を全て満たした場合に成功とみなされます:

1. ✅ **OAuth 2.0認証**: Chrome拡張からGoogleアカウントにログインし、OAuth 2.0トークンを取得できる
2. ✅ **リアルタイム同期**: 文字起こし結果がリアルタイム（2秒以内）でGoogle Docsに反映される
3. ✅ **Named Range管理**: 文字起こし結果が構造化されたフォーマットでドキュメントに挿入される
4. ✅ **オフライン対応**: ネットワーク切断時もローカルキューに保存され、再接続時に自動同期される
5. ✅ **エラーハンドリング**: トークンリフレッシュ、APIエラー、ネットワークエラーに対して適切に対処する
6. ✅ **ユーザー設定**: Google Docs同期の有効/無効、タイムスタンプ表示等の設定が可能

---

## Dependencies

### Upstream Dependencies (Blocking)

本specの実装開始前に、以下の成果物が完了している必要があります:

- **meeting-minutes-core** (phase: design-validated以降):
  - **CORE-REQ-006**: WebSocketサーバー (ポート9001-9100)
  - **CORE-REQ-007**: Chrome拡張スケルトン (WebSocket接続機能)
- **meeting-minutes-stt** (phase: implementation-completed):
  - **STT-REQ-008**: WebSocketメッセージ拡張 (confidence, language, isPartial フィールド)

### External Dependencies
- **Google Docs API**: v1
- **Google OAuth 2.0**: Google Identity Services
- **Chrome Extensions API**: Manifest V3
- **Chrome Storage API**: chrome.storage.local

### Internal Dependencies
- **meeting-minutes-stt**: 文字起こし結果のWebSocket配信（MVP1で実装済み）
- **meeting-minutes-core**: WebSocketサーバーとChrome拡張スケルトン（MVP0で実装済み）
- **Umbrella Spec**: `.kiro/specs/meeting-minutes-automator` - REQ-003.2, REQ-006（Google Docs連携要件）
- **Steering Documents**:
  - `tech.md`: Chrome拡張技術スタック、WebSocket通信仕様
  - `structure.md`: Chrome拡張のディレクトリ構造
  - `principles.md`: オフラインファースト原則、セキュリティ責任境界の原則

---

## Requirement Traceability Matrix

本サブスペックとアンブレラ仕様（meeting-minutes-automator）の要件対応表。

| DOCS ID | 要件概要 | アンブレラID | 備考 |
|---------|---------|-------------|------|
| DOCS-REQ-001 | OAuth 2.0 Authentication Flow | REQ-003.2.a, REQ-006.d | Google認証トークン管理 |
| DOCS-REQ-002 | Google Docs API Integration | REQ-003.2.b, REQ-006.a | batchUpdate API |
| DOCS-REQ-003 | Named Range Management | REQ-003.2.d, REQ-006.b | 挿入位置管理 |
| DOCS-REQ-004 | Real-Time Text Insertion | REQ-003.1.b, REQ-003.2 | リアルタイム同期 |
| DOCS-REQ-005 | Offline Queueing and Auto-Resync | REQ-006.c | オフライン対応 |
| DOCS-REQ-006 | Document Structure Management | REQ-003.2.e | 構造化議事録 |
| DOCS-REQ-007 | WebSocket Protocol Extension | REQ-003.1.d | 双方向通信制御 |
| DOCS-REQ-008 | Settings and Preferences | - | MVP2固有設定 |
| DOCS-NFR-001 | Performance | NFR-001 | リアルタイム性能要件 |
| DOCS-NFR-002 | Reliability | NFR-004 | 可用性・自動復旧 |
| DOCS-NFR-003 | Security | NFR-003 | セキュリティ・OAuth管理 |
| DOCS-NFR-004 | Compatibility | - | Chrome拡張互換性 |
| DOCS-NFR-005 | Usability | NFR-005 | 使いやすさ |
| DOCS-NFR-006 | Logging | - | MVP2固有ログ要件 |

**上流依存**:
- **meeting-minutes-core**: CORE-REQ-006 (WebSocketサーバー), CORE-REQ-007 (Chrome拡張スケルトン)
- **meeting-minutes-stt**: STT-REQ-008 (WebSocketメッセージ拡張: confidence, language, isPartial)

**下流影響**:
- **meeting-minutes-llm**: DOCS-REQ-006の構造化フォーマットを要約入力として利用予定

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-02 | 1.0 | Claude Code | 初版作成（MVP2 Google Docs同期要件定義） |
| 2025-10-02 | 1.1 | Claude Code | 要件ID採番（DOCS-REQ-###, DOCS-NFR-###）、Upstream Dependencies (Blocking)追加、Requirement Traceability Matrix追加 |
| 2025-10-02 | 1.2 | Claude Code | Critical欠陥修正: OAuth認証エラー時キューイング（DOCS-REQ-001.10-11）、バックグラウンド同期対応（DOCS-REQ-002.10-12）、Named Range復旧ロジック明確化（DOCS-REQ-003.7-8）、オフラインキュー警告強化（DOCS-REQ-005.11-12）、パフォーマンス目標修正（DOCS-NFR-001.3）、自動挿入領域編集禁止ガイド（DOCS-REQ-006.6） |

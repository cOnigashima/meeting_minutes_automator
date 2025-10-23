# Technical Design - meeting-minutes-docs-sync: System Flows

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## System Flows

### Sequence: OAuth 2.0 Authentication Flow

```mermaid
sequenceDiagram
    participant U as User
    participant PU as Popup UI
    participant AM as AuthManager
    participant CI as Chrome Identity API
    participant OAUTH as Google OAuth 2.0
    participant TS as Token Store<br/>(chrome.storage.local)

    U->>PU: Click "Google連携"
    PU->>AM: initiateAuth()
    AM->>CI: launchWebAuthFlow({<br/>url: authUrl,<br/>interactive: true<br/>})
    CI->>OAUTH: Open Auth Dialog
    OAUTH->>U: Request permissions
    U->>OAUTH: Grant permissions
    OAUTH->>CI: Redirect with code
    CI->>AM: Return code
    AM->>OAUTH: POST /token<br/>(code, client_id, client_secret)
    OAUTH->>AM: {access_token, refresh_token, expires_in}
    AM->>TS: save({accessToken, refreshToken, expiresAt})
    AM->>PU: authSuccess
    PU->>U: Display "認証成功"

    Note over AM,TS: Token Refresh Flow (DOCS-REQ-001.8)
    AM->>AM: checkTokenExpiry() [60s buffer]
    AM->>OAUTH: POST /token<br/>(refresh_token, grant_type: refresh_token)
    OAUTH->>AM: {access_token, expires_in}
    AM->>TS: update({accessToken, expiresAt})
```

**フロー詳細**:
1. **認証開始** (DOCS-REQ-001.1-2): ユーザーが「Google連携」ボタンをクリックし、`chrome.identity.launchWebAuthFlow()`が認証ダイアログを開く
2. **権限許可** (DOCS-REQ-001.3): ユーザーがGoogleアカウントを選択し、`drive.file`スコープの権限を許可
3. **トークン交換** (DOCS-REQ-001.4-5): 認証コードをアクセストークンとリフレッシュトークンに交換し、`chrome.storage.local`に保存
4. **トークンリフレッシュ** (DOCS-REQ-001.8): アクセストークン期限切れの60秒前に自動リフレッシュ（クロックスキュー対策）

---

### Sequence: Real-Time Text Insertion Flow

```mermaid
sequenceDiagram
    participant T as Tauri App<br/>(STT Manager)
    participant WS as WebSocket Server
    participant BG as Background Worker
    participant SM as SyncManager
    participant QM as QueueManager
    participant GD as GoogleDocsClient
    participant NR as NamedRangeManager
    participant API as Google Docs API

    T->>WS: transcription message<br/>{isPartial: false, text: "..."}
    WS->>BG: WebSocket message
    BG->>SM: processTranscription(msg)

    alt Online Mode
        SM->>QM: enqueue(msg)
        QM->>QM: checkBuffering() [3s or 500 chars]

        alt Buffer threshold reached
            QM->>GD: insertText(messages[])
            GD->>NR: getCurrentCursorPosition()
            NR->>API: GET /documents/{id}
            API->>NR: namedRanges: {transcript_cursor: {startIndex: 123}}
            NR->>GD: position: 123

            GD->>API: POST /documents/{id}:batchUpdate<br/>{insertText: {location: {index: 123}, text: "..."}}
            API->>GD: 200 OK

            GD->>NR: updateCursorPosition(newIndex: 456)
            NR->>API: POST /documents/{id}:batchUpdate<br/>{updateNamedRange: {...}}
            API->>NR: 200 OK

            GD->>SM: syncSuccess
            SM->>WS: docs_sync_success
            WS->>T: notification
        end
    else Offline Mode (DOCS-REQ-005.3-4)
        SM->>QM: saveToOfflineQueue(msg)
        QM->>QM: checkStorageLimit()

        alt Storage < 80%
            QM-->>BG: saved
        else Storage >= 80% (DOCS-REQ-005.11)
            QM->>PU: showWarning("残り容量警告")
        else Storage >= 100% (DOCS-REQ-005.12)
            QM->>BG: chrome.notifications.create("上限到達")
            QM->>BG: stopReceiving()
        end
    end

    Note over SM,API: Network Recovery (DOCS-REQ-005.5-9)
    BG->>SM: onNetworkRestored()
    SM->>QM: getOfflineQueue()
    QM->>SM: messages[] (sorted by timestamp)
    SM->>GD: insertBatchMessages(messages[])
    GD->>API: Multiple batchUpdate calls [Rate Limit: 60/min]
    API->>GD: 200 OK
    GD->>SM: resyncComplete
    SM->>QM: clearQueue()
    SM->>PU: showNotification("同期完了")
```

**フロー詳細**:
1. **文字起こし受信** (DOCS-REQ-004.1-2): Tauriアプリから確定テキスト（`isPartial: false`）をWebSocket経由で受信
2. **バッファリング** (DOCS-REQ-004.6-7): 最大3秒間または500文字までバッファリングし、1回の`batchUpdate`リクエストにまとめる
3. **挿入位置取得** (DOCS-REQ-003.3-4): `transcript_cursor` Named Rangeの現在位置を取得
4. **テキスト挿入** (DOCS-REQ-002.3-6): `insertText`リクエストでテキストを挿入し、Named Rangeの位置を更新
5. **オフライン処理** (DOCS-REQ-005.3-4): ネットワーク切断時は`chrome.storage.local`の`offline_queue`に保存
6. **自動再同期** (DOCS-REQ-005.5-9): ネットワーク復帰時にキュー内のメッセージを時系列順に再送信

---

### Process Flow: Named Range Recovery Logic

```mermaid
flowchart TD
    Start([Named Range消失検知]) --> GetDoc[GET documents API<br/>namedRanges確認]
    GetDoc --> CheckExists{transcript_cursor<br/>存在?}

    CheckExists -->|Yes| GetPosition[startIndexを取得]
    GetPosition --> Success([正常挿入位置取得])

    CheckExists -->|No| Priority1[Priority 1:<br/>見出し検索]
    Priority1 --> SearchHeading[ドキュメント内を検索<br/>"## 文字起こし"]
    SearchHeading --> HeadingFound{見出し発見?}

    HeadingFound -->|Yes| InsertAfterHeading[見出し直後の位置を計算<br/>headingIndex + 1]
    InsertAfterHeading --> CreateNR1[Named Range再作成<br/>createNamedRange API]
    CreateNR1 --> LogWarn1[ERRORログ記録:<br/>"Named Range消失 - 見出し後に再作成"]
    LogWarn1 --> NotifyUser1[UI通知:<br/>"挿入位置が再設定されました"]
    NotifyUser1 --> Success

    HeadingFound -->|No| Priority2[Priority 2:<br/>ドキュメント末尾]
    Priority2 --> GetEndIndex[endIndex取得<br/>body.content最終要素]
    GetEndIndex --> CheckEmpty{ドキュメント<br/>空?}

    CheckEmpty -->|No| CreateNR2[Named Range再作成<br/>position: endIndex - 1]
    CreateNR2 --> LogWarn2[ERRORログ記録:<br/>"Named Range消失 - 末尾に再作成"]
    LogWarn2 --> NotifyUser2[UI通知:<br/>"挿入位置が再設定されました"]
    NotifyUser2 --> Success

    CheckEmpty -->|Yes| Priority3[Priority 3:<br/>先頭位置]
    Priority3 --> CreateNR3[Named Range再作成<br/>position: 1]
    CreateNR3 --> LogWarn3[ERRORログ記録:<br/>"Named Range消失 - 先頭に再作成"]
    LogWarn3 --> NotifyUser3[UI通知:<br/>"挿入位置が再設定されました"]
    NotifyUser3 --> Success
```

**フロー詳細** (DOCS-REQ-003.7-8):
1. **検出**: `documents.get` APIで`transcript_cursor` Named Rangeが存在しないことを検出
2. **Priority 1**: ドキュメント内のテキストを検索し、「## 文字起こし」見出しを検出。見つかった場合は見出し直後にNamed Rangeを再作成
3. **Priority 2**: 見出しが見つからない場合、ドキュメントの`endIndex`（末尾）にNamed Rangeを再作成
4. **Priority 3**: ドキュメントが空の場合、index=1（先頭）にNamed Rangeを再作成
5. **ログと通知**: ERRORレベルログ + ポップアップUI通知により、ユーザーに異常を認識させる

---


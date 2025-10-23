# Technical Design - meeting-minutes-docs-sync: State Management

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Backend State Management (Tauri Side)

### Overview

meeting-minutes-docs-syncでは、Chrome拡張からのGoogle Docs同期イベント（`docs_sync_started`, `docs_sync_success`等）をTauri側で管理し、STT Managerから配信される原型TranscriptionMessageに`docsSync`フィールドを合成する必要があります。

この責務を担うのが**SyncStateStore**です。

### Architecture

```
[STT Manager] → 原型TranscriptionMessage (docsSyncなし)
     ↓
[WebSocketService] → SyncStateStoreで状態読み取り
     ↓
[enrich_message()] → docsSyncフィールドを合成
     ↓
[Chrome拡張] ← 拡張版TranscriptionMessage (docsSyncあり)
```

**責務の明確化**:
- **STT Manager**: 文字起こし結果の生成のみ（Google Docs同期は関知しない）
- **WebSocketService**: セッション状態の管理とメッセージ拡張
- **SyncStateStore**: 同期状態の永続化と読み取り
- **Chrome拡張**: Google Docs API呼び出しと状態イベントの送信

### SyncStateStore Design

#### Rust実装（src-tauri/src/services/sync_state_store.rs）

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Google Docs同期状態ストア
pub struct SyncStateStore {
    /// sessionId → SyncState のマッピング
    states: Arc<RwLock<HashMap<String, SyncState>>>,
}

impl SyncStateStore {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Chrome拡張からのイベントを状態に反映
    pub async fn handle_sync_event(&self, event: SyncEvent) {
        let mut states = self.states.write().await;
        match event {
            SyncEvent::DocsSyncStarted { session_id, document_id, document_title } => {
                states.insert(session_id, SyncState {
                    enabled: true,
                    status: SyncStatus::Queued,
                    document_id: Some(document_id),
                    document_title: Some(document_title),
                    last_updated: current_timestamp(),
                });
            }
            SyncEvent::DocsSyncSuccess { session_id, .. } => {
                if let Some(state) = states.get_mut(&session_id) {
                    state.status = SyncStatus::Synced;
                    state.last_updated = current_timestamp();
                }
            }
            // ... 他のイベント処理
        }
    }

    /// TranscriptionMessageにdocsSyncフィールドを合成
    pub async fn enrich_message(
        &self,
        session_id: &str,
        mut msg: TranscriptionMessage,
    ) -> TranscriptionMessage {
        let states = self.states.read().await;
        if let Some(state) = states.get(session_id) {
            msg.docs_sync = Some(DocsSyncField {
                enabled: state.enabled,
                status: match state.status {
                    SyncStatus::Synced => "synced",
                    SyncStatus::Queued => "queued",
                    SyncStatus::Failed => "failed",
                }.to_string(),
                document_id: state.document_id.clone(),
            });
        }
        msg
    }
}

/// セッションごとの同期状態
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncState {
    pub enabled: bool,
    pub status: SyncStatus,
    pub document_id: Option<String>,
    pub document_title: Option<String>,
    pub last_updated: u64, // Unix timestamp
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    Queued,
    Failed,
}
```

**完全な実装詳細**: [design-backend-state-insert.md](./design-backend-state-insert.md)（376行のRust実装例、テスト戦略、メッセージフロー）

### WebSocketService統合

```rust
pub struct WebSocketService {
    clients: Arc<RwLock<HashMap<String, WebSocketClient>>>,
    sync_store: Arc<SyncStateStore>, // NEW
}

impl WebSocketService {
    /// STT Managerから受信したTranscriptionMessageを送信
    pub async fn send_transcription(&self, session_id: &str, msg: TranscriptionMessage) -> Result<()> {
        // 原型メッセージ（docsSyncなし）
        // ↓
        // SyncStateStoreで状態を読み取り、docsSyncフィールドを合成
        let enriched = self.sync_store.enrich_message(session_id, msg).await;
        // ↓
        // Chrome拡張へ送信
        self.send_to_client(session_id, enriched).await
    }

    /// Chrome拡張からのイベントを処理
    pub async fn handle_chrome_event(&self, event: SyncEvent) -> Result<()> {
        self.sync_store.handle_sync_event(event).await;
        Ok(())
    }
}
```

---

## Requirements Traceability

| Requirement | Components | Interfaces | Flows |
|-------------|-----------|------------|-------|
| **DOCS-REQ-001**: OAuth 2.0 Authentication | AuthManager, TokenStore | `initiateAuth()`, `refreshToken()`, `revokeToken()` | OAuth 2.0 Authentication Flow |
| **DOCS-REQ-002**: Google Docs API Integration | GoogleDocsClient, NamedRangeManager | `insertText()`, `batchUpdate()`, `getDocument()` | Real-Time Text Insertion Flow |
| **DOCS-REQ-003**: Named Range Management | NamedRangeManager | `createNamedRange()`, `getCurrentPosition()`, `updatePosition()` | Named Range Recovery Logic |
| **DOCS-REQ-004**: Real-Time Text Insertion | SyncManager, QueueManager | `processTranscription()`, `checkBuffering()` | Real-Time Text Insertion Flow |
| **DOCS-REQ-005**: Offline Queueing | QueueManager, OfflineQueue | `saveToOfflineQueue()`, `getOfflineQueue()`, `clearQueue()` | Real-Time Text Insertion Flow (Offline Mode) |
| **DOCS-REQ-006**: Document Structure | NamedRangeManager, GoogleDocsClient | `generateStructure()`, `insertWithTimestamp()` | Real-Time Text Insertion Flow |
| **DOCS-REQ-007**: WebSocket Protocol Extension | **SyncStateStore (Tauri)**, SyncManager, Background Worker | `handle_sync_event()`, `enrich_message()`, `docs_sync_*` events | Backend State Management, Real-Time Text Insertion Flow |
| **DOCS-REQ-008**: Settings and Preferences | SettingsManager | `getSetting()`, `updateSetting()` | Real-Time Text Insertion Flow (Buffering) |

**DOCS-REQ-007詳細**: Tauri側の`SyncStateStore`がChrome拡張からの同期イベント（`docs_sync_started`, `docs_sync_success`等）を受信し、セッション状態を管理します。STT Managerからの原型TranscriptionMessageに`docsSync`フィールドを`enrich_message()`で合成してChrome拡張へ配信します。

---


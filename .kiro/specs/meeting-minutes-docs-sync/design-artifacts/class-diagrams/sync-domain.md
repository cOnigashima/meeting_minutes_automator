# Sync Domain - Class Diagram

> **親ドキュメント**: [phase-0-design-validation.md](../../task-details/phase-0-design-validation.md)
> **関連設計**: [design-components.md#Sync Domain](../../design-modules/design-components.md)
> **Requirements**: DOCS-REQ-004.1-9, DOCS-REQ-005.1-12, DOCS-NFR-001.1-4

## Class Diagram

```mermaid
classDiagram
    class ISyncManager {
        <<interface>>
        +startSync(documentId string) Promise~Result~void, SyncStartError~~
        +stopSync() Promise~void~
        +processTranscription(message TranscriptionMessage) Promise~Result~void, ProcessError~~
        +getStatus() SyncStatus
    }

    class SyncManager {
        <<責務: 同期フロー統合>>
        -stateMachine ISyncStateMachine
        -queueManager IQueueManager
        -bufferingManager IBufferingManager
        -rateLimiter ITokenBucketRateLimiter
        -networkMonitor INetworkMonitor
        -resyncOrchestrator IResyncOrchestrator
        +startSync(documentId string) Promise~Result~void, SyncStartError~~
        +stopSync() Promise~void~
        +processTranscription(message TranscriptionMessage) Promise~Result~void, ProcessError~~
        +getStatus() SyncStatus
    }

    class ISyncStateMachine {
        <<interface>>
        +transition(event SyncEvent) SyncState
        +getCurrentState() SyncState
        +canTransition(event SyncEvent) boolean
    }

    class SyncStateMachine {
        <<責務: 状態遷移管理>>
        -currentState SyncState
        -transitions Map~SyncState, Map~SyncEvent, SyncState~~
        +transition(event SyncEvent) SyncState
        +getCurrentState() SyncState
        +canTransition(event SyncEvent) boolean
        -validateTransition(from SyncState, to SyncState) boolean
    }

    class IQueueManager {
        <<interface>>
        +enqueue(message TranscriptionMessage) Promise~Result~void, QueueFullError~~
        +getAll() Promise~TranscriptionMessage[]~
        +clear() Promise~void~
        +getStorageUsage() Promise~number~
    }

    class QueueManager {
        <<責務: オフラインキュー操作>>
        -STORAGE_KEY string
        -storageMonitor IStorageMonitor
        +enqueue(message TranscriptionMessage) Promise~Result~void, QueueFullError~~
        +getAll() Promise~TranscriptionMessage[]~
        +clear() Promise~void~
        +getStorageUsage() Promise~number~
        -sortByTimestamp(messages TranscriptionMessage[]) TranscriptionMessage[]
    }

    class IStorageMonitor {
        <<interface>>
        +startMonitoring() void
        +stopMonitoring() void
        +getCurrentUsage() Promise~number~
    }

    class StorageMonitor {
        <<責務: setInterval + ストレージ監視 (Offscreen Document)>>
        -INTERVAL_ID number | null
        -CHECK_INTERVAL_MS number
        -STORAGE_LIMIT number
        -warningThreshold number
        +startMonitoring() void
        +stopMonitoring() void
        +getCurrentUsage() Promise~number~
        -checkUsage() Promise~void~
        -showWarning(usageRatio number) void
    }

    class IBufferingManager {
        <<interface>>
        +addToBuffer(message TranscriptionMessage) Promise~void~
        +flush() Promise~void~
        +clear() void
    }

    class BufferingManager {
        <<責務: バッファリング + setInterval (Offscreen Document)>>
        -buffer TranscriptionMessage[]
        -MAX_BUFFER_TIME_MS number
        -MAX_BUFFER_SIZE number
        -INTERVAL_ID number | null
        +addToBuffer(message TranscriptionMessage) Promise~void~
        +flush() Promise~void~
        +clear() void
        -shouldFlush() boolean
    }

    class ITokenBucketRateLimiter {
        <<interface>>
        +acquire() Promise~void~
        +getAvailableTokens() number
    }

    class TokenBucketRateLimiter {
        <<責務: レート制限制御>>
        -tokens number
        -capacity number
        -refillRate number
        -lastRefillTime number
        +acquire() Promise~void~
        +getAvailableTokens() number
        -refill() void
    }

    class INetworkMonitor {
        <<interface>>
        +startMonitoring(onOnline function, onOffline function) void
        +stopMonitoring() void
        +isOnline() boolean
    }

    class NetworkMonitor {
        <<責務: オンライン/オフライン検知>>
        -onlineCallback function | null
        -offlineCallback function | null
        +startMonitoring(onOnline function, onOffline function) void
        +stopMonitoring() void
        +isOnline() boolean
        -onOnlineEvent() void
        -onOfflineEvent() void
    }

    class IResyncOrchestrator {
        <<interface>>
        +resync(messages TranscriptionMessage[]) Promise~Result~void, ResyncError~~
    }

    class ResyncOrchestrator {
        <<責務: 再同期制御>>
        -rateLimiter ITokenBucketRateLimiter
        -googleDocsClient IGoogleDocsClient
        +resync(messages TranscriptionMessage[]) Promise~Result~void, ResyncError~~
        -batchMessages(messages TranscriptionMessage[]) TranscriptionMessage[][]
    }

    ISyncManager <|.. SyncManager
    ISyncStateMachine <|.. SyncStateMachine
    IQueueManager <|.. QueueManager
    IStorageMonitor <|.. StorageMonitor
    IBufferingManager <|.. BufferingManager
    ITokenBucketRateLimiter <|.. TokenBucketRateLimiter
    INetworkMonitor <|.. NetworkMonitor
    IResyncOrchestrator <|.. ResyncOrchestrator

    SyncManager --> ISyncStateMachine
    SyncManager --> IQueueManager
    SyncManager --> IBufferingManager
    SyncManager --> ITokenBucketRateLimiter
    SyncManager --> INetworkMonitor
    SyncManager --> IResyncOrchestrator
    QueueManager --> IStorageMonitor
    ResyncOrchestrator --> ITokenBucketRateLimiter
```

## Metrics

| クラス | 公開メソッド数 | プライベートメソッド数 | 依存先数 | テスト容易性 |
|--------|---------------|-------------------|---------|-------------|
| SyncManager | 4 | 0 | 6 | ⭐⭐⭐⭐ |
| SyncStateMachine | 3 | 1 | 0 | ⭐⭐⭐⭐⭐ |
| QueueManager | 4 | 1 | 1 | ⭐⭐⭐⭐ |
| StorageMonitor | 3 | 2 | 0 | ⭐⭐⭐⭐ |
| BufferingManager | 3 | 1 | 0 | ⭐⭐⭐⭐⭐ |
| TokenBucketRateLimiter | 2 | 1 | 0 | ⭐⭐⭐⭐⭐ |
| NetworkMonitor | 3 | 2 | 0 | ⭐⭐⭐⭐ |
| ResyncOrchestrator | 1 | 1 | 2 | ⭐⭐⭐⭐ |

**Total Classes**: 8
**Average Public Methods**: 2.9
**Test Ease ⭐4+**: 100% (8/8 classes)

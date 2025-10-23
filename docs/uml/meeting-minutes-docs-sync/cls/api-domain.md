# API Domain - Class Diagram

> **親ドキュメント**: [phase-0-design-validation.md](/.kiro/specs/meeting-minutes-docs-sync/task-details/phase-0-design-validation.md)
> **関連設計**: [design-components.md#API Domain](/.kiro/specs/meeting-minutes-docs-sync/design-modules/design-components.md)
> **Requirements**: DOCS-REQ-002.1-13, DOCS-REQ-003.1-8, DOCS-REQ-006.1-6

## Class Diagram

```mermaid
classDiagram
    class IGoogleDocsClient {
        <<interface>>
        +getDocument(documentId string) Promise~Result~Document, ApiError~~
        +insertTextWithLock(documentId string, text string, position number) Promise~Result~void, ApiError | ConflictError~~
        +batchUpdate(documentId string, requests Request[], writeControl WriteControl) Promise~Result~BatchUpdateResponse, ApiError~~
    }

    class GoogleDocsClient {
        <<責務: API呼び出し統合>>
        -backoffHandler IExponentialBackoffHandler
        -lockHandler IOptimisticLockHandler
        -requestBuilder IApiRequestBuilder
        +getDocument(documentId string) Promise~Result~Document, ApiError~~
        +insertTextWithLock(documentId string, text string, position number) Promise~Result~void, ApiError | ConflictError~~
        +batchUpdate(documentId string, requests Request[], writeControl WriteControl) Promise~Result~BatchUpdateResponse, ApiError~~
    }

    class IExponentialBackoffHandler {
        <<interface>>
        +execute(fn function, maxRetries number) Promise~T~
    }

    class ExponentialBackoffHandler {
        <<責務: リトライ戦略>>
        -MAX_RETRIES number
        -INITIAL_DELAY_MS number
        +execute(fn function, maxRetries number) Promise~T~
        -isRetryableError(error Error) boolean
        -calculateDelay(attempt number) number
    }

    class IOptimisticLockHandler {
        <<interface>>
        +executeWithLock(documentId string, operation function) Promise~Result~T, ConflictError~~
    }

    class OptimisticLockHandler {
        <<責務: 楽観ロック制御>>
        -MAX_CONFLICT_RETRIES number
        -googleDocsClient IGoogleDocsClient
        +executeWithLock(documentId string, operation function) Promise~Result~T, ConflictError~~
        -recalculateCursorPosition(documentId string) Promise~number~
    }

    class INamedRangeManager {
        <<interface>>
        +createRange(documentId string, rangeName string, position number) Promise~Result~void, ApiError~~
        +getRange(documentId string, rangeName string) Promise~Result~NamedRange, NotFoundError~~
        +updateRange(documentId string, rangeName string, position number) Promise~Result~void, ApiError~~
    }

    class NamedRangeManager {
        <<責務: Named Range統合>>
        -googleDocsClient IGoogleDocsClient
        -recoveryStrategy INamedRangeRecoveryStrategy
        -styleFormatter IParagraphStyleFormatter
        +createRange(documentId string, rangeName string, position number) Promise~Result~void, ApiError~~
        +getRange(documentId string, rangeName string) Promise~Result~NamedRange, NotFoundError~~
        +updateRange(documentId string, rangeName string, position number) Promise~Result~void, ApiError~~
    }

    class INamedRangeRecoveryStrategy {
        <<interface>>
        +recover(documentId string, rangeName string) Promise~Result~number, RecoveryError~~
    }

    class NamedRangeRecoveryStrategy {
        <<責務: Named Range自動復旧>>
        -googleDocsClient IGoogleDocsClient
        -strategies RecoveryStrategyImpl[]
        +recover(documentId string, rangeName string) Promise~Result~number, RecoveryError~~
        -tryHeadingSearch(documentId string) Promise~number | null~
        -tryEndOfDocument(documentId string) Promise~number | null~
        -tryStartOfDocument() Promise~number~
    }

    class IParagraphStyleFormatter {
        <<interface>>
        +formatHeading(text string) Request[]
        +formatTimestamp(timestamp number) string
        +formatSpeaker(speakerName string) string
    }

    class ParagraphStyleFormatter {
        <<責務: 段落スタイル設定>>
        -HEADING_STYLE ParagraphStyle
        -NORMAL_STYLE ParagraphStyle
        +formatHeading(text string) Request[]
        +formatTimestamp(timestamp number) string
        +formatSpeaker(speakerName string) string
    }

    IGoogleDocsClient <|.. GoogleDocsClient
    IExponentialBackoffHandler <|.. ExponentialBackoffHandler
    IOptimisticLockHandler <|.. OptimisticLockHandler
    INamedRangeManager <|.. NamedRangeManager
    INamedRangeRecoveryStrategy <|.. NamedRangeRecoveryStrategy
    IParagraphStyleFormatter <|.. ParagraphStyleFormatter

    GoogleDocsClient --> IExponentialBackoffHandler
    GoogleDocsClient --> IOptimisticLockHandler
    OptimisticLockHandler --> IGoogleDocsClient
    NamedRangeManager --> IGoogleDocsClient
    NamedRangeManager --> INamedRangeRecoveryStrategy
    NamedRangeManager --> IParagraphStyleFormatter
    NamedRangeRecoveryStrategy --> IGoogleDocsClient
```

## Metrics

| クラス | 公開メソッド数 | プライベートメソッド数 | 依存先数 | テスト容易性 |
|--------|---------------|-------------------|---------|-------------|
| GoogleDocsClient | 3 | 0 | 3 | ⭐⭐⭐⭐ |
| ExponentialBackoffHandler | 1 | 2 | 0 | ⭐⭐⭐⭐⭐ |
| OptimisticLockHandler | 1 | 1 | 1 | ⭐⭐⭐⭐ |
| NamedRangeManager | 3 | 0 | 3 | ⭐⭐⭐⭐ |
| NamedRangeRecoveryStrategy | 1 | 3 | 1 | ⭐⭐⭐⭐ |
| ParagraphStyleFormatter | 3 | 0 | 0 | ⭐⭐⭐⭐⭐ |

**Total Classes**: 6
**Average Public Methods**: 2.0
**Test Ease ⭐4+**: 100% (6/6 classes)

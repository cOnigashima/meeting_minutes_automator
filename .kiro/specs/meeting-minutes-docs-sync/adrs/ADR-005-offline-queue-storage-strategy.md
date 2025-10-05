# ADR-005: Offline Queue Storage Strategy

## Status
Accepted

## Context

meeting-minutes-docs-sync (MVP2) では、ネットワーク切断時に文字起こしメッセージをローカルキューに保存し、オンライン復帰時に自動同期する必要がある。このオフラインキューには議事録テキスト（PII: Personally Identifiable Information）が含まれるため、保管場所とストレージ戦略の決定が重要である。

### 設計原則との関係

プロジェクトの設計原則（`.kiro/steering/principles.md`）では、以下のように定義されている:

> **Principle 2: オフラインファースト原則**
> - コア機能（音声録音、VAD、STT）はインターネット接続不要で完全動作
> - ネットワーク依存機能は段階的縮退（graceful degradation）を実装
> - オフライン時のデータキューイングと自動同期機構を提供

> **Principle 3: セキュリティ責任境界の原則**
> - **Tauri責務**: OAuth token保管（OS Keychain）、音声データ暗号化、SQLiteデータベース暗号化
> - **Chrome拡張責務**: 表示専用UI、ユーザーインタラクション、Google Docs表示操作

これらの原則に従えば、オフラインキューはTauri側のSQLCipher（暗号化SQLite）に保管すべきである。しかし、MVP2の技術制約により、Chrome拡張の`chrome.storage.local`に保存する設計を採用する。

### 技術的制約

1. **chrome.storage.local制約**:
   - デフォルト上限: 10MB（QUOTA_BYTES）
   - `unlimitedStorage`パーミッション: ユーザー信頼低下の懸念
   - 暗号化機能: 標準APIでは提供されない

2. **Service Worker制約**:
   - MV3 Service Workerは5分でサスペンド
   - インメモリキャッシュは揮発性（再起動時に消失）
   - 永続化には`chrome.storage.local`が唯一の選択肢

3. **オフライン動作の優先度**:
   - Principle 2（オフラインファースト）に基づき、ネットワーク切断時の作業継続性を最優先
   - 長時間会議（2-3時間）でのオフライン期間（30分程度）をカバーする必要がある

### 代替案の検討

#### Option A: Tauri側SQLCipherでキュー管理（原則準拠）

**構成**:
- Tauri: オフラインキュー保管（SQLCipher暗号化）、Google Docs API呼び出し
- Chrome拡張: Doc ID検出、UI表示、WebSocket経由でTauriへメッセージ送信

**利点**:
- ✅ セキュリティ責任境界の原則に完全準拠
- ✅ SQLCipherによる暗号化（AES-256）
- ✅ ストレージ上限なし（ディスク容量依存）

**欠点**:
- ❌ Service WorkerとTauri間のオフライン状態同期が複雑化
- ❌ キュー状態のUI表示遅延（拡張ポップアップ → Tauri IPC → レスポンス）
- ❌ WebSocket切断時のメッセージロスリスク（再接続前のメッセージが失われる可能性）
- ❌ 実装工数の増加（MVP2スコープ超過）

#### Option B: Chrome拡張chrome.storage.localでキュー管理（現行設計）

**構成**:
- Chrome拡張: オフラインキュー保管（`chrome.storage.local`）、ストレージ監視、2段階警告システム
- Tauri: 文字起こしテキストの配信のみ（WebSocket経由）

**利点**:
- ✅ Service Workerから直接アクセス可能（レイテンシ最小）
- ✅ オフライン状態の検出と保存が同一プロセス内で完結
- ✅ UI通知とキュー状態表示の即時性
- ✅ 実装工数がMVP2スコープ内

**欠点**:
- ❌ 平文保存（暗号化なし）
- ❌ 10MB上限（約2-3時間の会議で到達可能）
- ❌ PII（議事録テキスト）の漏洩リスク

## Decision

**MVP2では、Chrome拡張の`chrome.storage.local`にオフラインキューを保存する（Option B）。**

ただし、以下のリスク緩和策を実装する:

### リスク緩和策

#### 1. 2段階警告システム（design.md:289-327）

**80%警告（黄色）**:
```typescript
if (queueSize >= MAX_QUEUE_SIZE * 0.8) {
  showPopupWarning(`オフラインキューが残り${MAX_QUEUE_SIZE - queueSize}件です。ネットワーク接続を確認してください`);
}
```
- ユーザーに事前警告を提示
- ネットワーク復旧の時間的猶予を与える

**100%到達時（赤色）**:
```typescript
if (queueSize >= MAX_QUEUE_SIZE) {
  chrome.notifications.create({
    type: 'basic',
    iconUrl: 'icon.png',
    title: 'オフラインキュー上限到達',
    message: 'これ以上の文字起こしは保存されません。録音を停止するか、ネットワーク接続を回復してください'
  });
  stopReceivingMessages();
}
```
- 全画面通知で明確な警告
- 新規メッセージの受信を停止（データロスではなく、ユーザー判断を促す）

#### 2. ストレージ使用量の定期監視（chrome.alarms）

```typescript
chrome.alarms.create('monitor-storage', { periodInMinutes: 0.1 }); // 6秒間隔

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'monitor-storage') {
    monitorStorageUsage();
  }
});
```
- Service Workerサスペンドに影響されない定期監視
- 動的な閾値チェックと警告発火

#### 3. FIFOキュー管理（古いメッセージ優先送信）

```typescript
const queue = messages.sort((a, b) => a.timestamp - b.timestamp);
```
- オンライン復帰時、タイムスタンプ順に送信
- メッセージの時系列整合性を保証

#### 4. Non-Goalsで暗号化をMVP3に明示

- design.md:28 に記載済み
- MVP3での移行パスを明確化

## Consequences

### Positive
- ✅ オフラインファースト原則の実現
- ✅ Service Worker制約下での最適実装
- ✅ UI通知の即時性とユーザー体験の向上
- ✅ 実装工数がMVP2スコープ内

### Negative
- ⚠️ PII（議事録テキスト）の平文保存
- ⚠️ 10MB上限による長時間会議対応の制約
- ⚠️ セキュリティ責任境界の原則に一時的に違反（MVP3で解消予定）

### Neutral
- 🔄 MVP3での再設計が必須（Tauri SQLCipher移行）
- 🔄 2段階警告システムによるユーザー教育効果

## Rationale

### なぜ10MB上限を許容するか

**想定シナリオ**:
- 平均的な会議: 1時間あたり約1-2MBの文字起こしテキスト
- 10MB上限 = 約5-10時間分のオフライン蓄積

**2段階警告の効果**:
- 80%到達（8MB）時点で警告 → ユーザーは残り20%の猶予を得る
- 100%到達時は新規受信停止 → データロスではなく、意図的な停止

**MVP2のスコープ判断**:
- 10MB以上の長時間オフラインは、MVP2のユースケース（会議室でのWi-Fi一時切断等）を超える極端なエッジケース
- MVP3でTauri SQLCipher移行により無制限化

### なぜ平文保存を一時的に許容するか

**リスク評価**:
- 議事録テキストは機密性中程度（音声録音データより低い）
- 攻撃者が拡張ストレージにアクセスする場合、既にOS権限を持つ状態（他の脅威も成立）
- トークンと異なり、キュー内のメッセージは一時的（オンライン復帰で削除）

**緩和策の効果**:
- 2段階警告により、オフライン期間を最小化
- ストレージ定期監視により、異常蓄積を早期検出

## Migration Path to MVP3

MVP3では、以下の段階的移行を実施する:

1. **Phase 1: Tauri側キューマネージャー実装**
   - SQLCipher統合（AES-256暗号化）
   - WebSocket経由のキューメッセージ転送プロトコル

2. **Phase 2: Chrome拡張のキューファサード化**
   - Chrome拡張はキューの存在を検知し、UI表示のみ
   - 実際の保存はTauriへ委譲

3. **Phase 3: Chrome拡張からのキューコード削除**
   - Tauri側の安定稼働を確認後、Chrome拡張のキュー管理コードを削除

## References

- `.kiro/steering/principles.md` - Principle 2: オフラインファースト原則
- `.kiro/steering/principles.md` - Principle 3: セキュリティ責任境界の原則
- `.kiro/specs/meeting-minutes-docs-sync/design.md` - Offline Queueing セクション
- `.kiro/specs/meeting-minutes-docs-sync/requirements.md` - DOCS-REQ-005: Offline Queueing
- Chrome Storage API: https://developer.chrome.com/docs/extensions/reference/api/storage
- SQLCipher: https://www.zetetic.net/sqlcipher/

## Notes

本ADRは、オフラインファースト原則とセキュリティ原則のトレードオフを明示的に文書化したものである。MVP2では、ユーザー体験（作業継続性）を優先し、2段階警告とストレージ監視により、セキュリティリスクを許容可能なレベルに緩和している。

MVP3での暗号化とTauri移行は必須であり、プロジェクトロードマップに明記されている。

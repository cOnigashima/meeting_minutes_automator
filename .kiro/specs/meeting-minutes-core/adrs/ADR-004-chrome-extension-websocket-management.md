# ADR-004: Chrome拡張WebSocket管理方式の選定

## Status
Accepted (2025-10-10)

## Context
MVP0実装時にChrome拡張のWebSocket管理をContent Scriptで実装したが、設計書ではService Worker管理を想定していた。MVP1（meeting-minutes-stt）実装前に正式な方式を決定する必要がある。

Manifest V3の制約により、Service Workerは30秒のアイドルタイムアウト後に自動的に終了する。これはWebSocketのような永続的な接続を必要とするアプリケーションには大きな制約となる。

## Decision
**Content Script方式を正式採用する。**

理由：
1. MVP0で既に実装・検証済みで安定動作している
2. WebSocket接続の永続性が保証される
3. Manifest V3の30秒制限の影響を受けない
4. 実装がシンプルで保守性が高い

## Detailed Comparison

### 技術的比較

| 評価項目 | Content Script方式 | Service Worker方式 | 重要度 |
|---------|-------------------|-------------------|--------|
| **接続永続性** | ✅ ページ存続中は安定 | ❌ 30秒で終了（MV3制約） | 高 |
| **WebSocket再接続頻度** | ✅ ほぼ不要 | ❌ 頻繁に必要 | 高 |
| **実装複雑度** | ✅ シンプル（192行） | ❌ keepalive機構が複雑 | 中 |
| **メモリ使用量** | ⚠️ タブごとにインスタンス | ✅ 単一インスタンス | 低 |
| **CPU使用率** | ✅ アイドル時は低い | ❌ keepaliveで定期実行 | 中 |
| **検証状況** | ✅ E2Eテスト済み（44テスト合格） | ❌ 未検証 | 高 |

### Google Meet特有の考慮事項

| 評価項目 | Content Script方式 | Service Worker方式 | 対応策 |
|---------|-------------------|-------------------|--------|
| **SPA対応** | ⚠️ ページ遷移時に再注入必要 | ✅ バックグラウンド継続 | URL監視 + 重複防止 |
| **複数タブ** | ⚠️ 各タブ独立 | ✅ 統一管理 | chrome.storage.local同期 |
| **録音状態管理** | ⚠️ タブ間同期必要 | ✅ 中央管理 | storage APIで共有 |
| **会議終了検知** | ✅ DOM監視で即座に検知 | ⚠️ メッセージ経由 | MutationObserver使用 |

### Manifest V3制約の影響

| 制約事項 | Content Script | Service Worker | 影響度 |
|---------|---------------|-----------------|--------|
| **30秒アイドル制限** | 影響なし | 致命的（頻繁な再起動） | 極高 |
| **永続的バックグラウンド禁止** | 影響なし | keepalive実装必須 | 高 |
| **DOM操作** | 可能 | 不可能 | 中 |
| **chrome.alarms最短間隔** | 不要 | 1分制限あり | 中 |

### 実装コスト比較

| 項目 | Content Script | Service Worker | 備考 |
|------|---------------|-----------------|------|
| **初期実装** | 0日（実装済み） | 3-5日 | keepalive実装含む |
| **テスト作成** | 0日（検証済み） | 2-3日 | E2E再実装必要 |
| **デバッグ容易性** | 高（DevToolsで直接確認） | 低（バックグラウンド） | - |
| **保守性** | 高（シンプル） | 低（複雑） | - |

## Implementation Details

### Current Implementation (Content Script)
```javascript
// chrome-extension/content-script.js
class WebSocketClient {
  constructor() {
    this.ws = null;
    this.portRange = { start: 9001, end: 9100 };
    // ポートスキャン + 自動再接続 + エクスポネンシャルバックオフ
  }

  async connect() {
    // 9001-9100の範囲でポートスキャン
    // chrome.storage.localに最後の成功ポート記録
  }
}
```

### Alternative (Service Worker) - Not Adopted
```javascript
// Would require:
// 1. chrome.alarms API for keepalive (1分間隔)
// 2. WebSocket ping every 20-30 seconds
// 3. Complex reconnection state machine
// 4. Message relay to content scripts
```

## Consequences

### Positive
- ✅ 即座にMVP1へ移行可能（追加実装不要）
- ✅ WebSocket接続が安定（30秒制限の影響なし）
- ✅ 実装・保守がシンプル
- ✅ デバッグが容易（DevToolsで直接確認可能）
- ✅ DOM操作による会議状態の直接監視が可能

### Negative
- ❌ タブごとにメモリ使用（約5-10MB/タブ）
- ❌ Google Meet SPA対応に追加実装必要
- ❌ 複数タブ間の状態管理が必要

### Mitigation Strategies

#### SPA対応（ページ遷移検知）
```javascript
// URL変更監視
const observer = new MutationObserver(() => {
  if (location.pathname.includes('/meet/')) {
    reinitializeIfNeeded();
  }
});
observer.observe(document.body, { childList: true, subtree: true });
```

#### 複数タブ管理
```javascript
// chrome.storage.localで録音状態を共有
chrome.storage.local.set({ recordingState: 'active', tabId: chrome.runtime.id });
chrome.storage.onChanged.addListener((changes) => {
  if (changes.recordingState) {
    updateLocalState(changes.recordingState.newValue);
  }
});
```

#### メモリ最適化
```javascript
// 非アクティブタブで接続解放
document.addEventListener('visibilitychange', () => {
  if (document.hidden && !isRecording) {
    wsClient.disconnect();
  } else {
    wsClient.reconnect();
  }
});
```

## Popup UI / Options Page Integration

### State Sharing Architecture

Content Script方式では、Popup UIやOptions Pageが直接Content Scriptにアクセスできない制約がある。以下の状態共有メカニズムで解決：

#### 状態管理フロー
```javascript
// Content Script: 状態を storage に書き込み
class StateManager {
  static async updateState(state) {
    await chrome.storage.local.set({
      recordingState: state.recording,
      connectionState: state.connected,
      wsPort: state.port,
      timestamp: Date.now(),
      tabId: chrome.runtime.id
    });
  }
}

// Popup UI: storage から状態を読み取り
chrome.storage.local.get(['recordingState', 'connectionState'], (result) => {
  updateUI(result);
});

// リアルタイム更新: storage 変更を監視
chrome.storage.onChanged.addListener((changes, namespace) => {
  if (namespace === 'local' && changes.recordingState) {
    updateRecordingButton(changes.recordingState.newValue);
  }
});
```

#### Service Worker の役割
- **状態ブリッジ**: Content Script ↔ Popup UI 間の仲介
- **コマンドリレー**: Popup UIからの操作をContent Scriptへ転送
- **タブ管理**: 複数タブの状態集約

### Performance Considerations

| 操作 | レイテンシ | 備考 |
|------|-----------|------|
| 状態書き込み | < 5ms | storage.local は同期的 |
| 状態読み取り | < 5ms | キャッシュ済みの場合 |
| 変更通知 | < 10ms | onChanged リスナー |
| **合計** | **< 20ms** | AC-NFR-PERF.4 (50ms) を満たす |

## Migration Path

1. **Phase 1 (Current)**: Content Script実装を維持
2. **Phase 2 (MVP1)**:
   - SPA対応機能追加
   - 状態管理レイヤー実装
3. **Phase 3 (MVP2)**:
   - Popup UI実装時に状態共有メカニズム活用
   - 複数タブ管理強化
4. **Phase 4 (Optional)**: Service Worker再評価（MV4待ち）

## References

- **実装コード**: `chrome-extension/content-script.js` (192行)
- **軽量Service Worker**: `chrome-extension/service-worker.js` (38行、メッセージリレーのみ)
- **設計書（旧）**: `.kiro/specs/meeting-minutes-core/design.md:1387-1446`
- **ブロッカー**: `.kiro/specs/meeting-minutes-stt/spec.json` (BLOCK-001)
- **Known Issues**: `docs/mvp0-known-issues.md#ask-10`
- **E2Eテスト**: `src-tauri/tests/integration/e2e_test.rs`
- **Manifest V3仕様**: https://developer.chrome.com/docs/extensions/mv3/

## Decision Date
2025-10-10

## Participants
- Tauri-Python-Chrome Extension Walking Skeleton Team
- Based on MVP0 implementation experience and E2E test results
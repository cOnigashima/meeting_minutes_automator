# Chrome Storage API Best Practices

## 概要

本ドキュメントは、Chrome拡張開発における`chrome.storage.local`の正しい使い方と、よくある誤解を解説します。

## ⚠️ 重要な制約

### ドット記法による部分更新は不可能

`chrome.storage.local`は、**トップレベルのキー単位**でしか操作できません。ドット記法を使った部分更新は動作しません。

#### ❌ 誤った使い方

```javascript
// これは動作しない！
await chrome.storage.local.set({
  'user.settings.theme': 'dark',
  'user.settings.language': 'ja'
});

// 結果: 'user.settings.theme'という文字列がキーになってしまう
// {
//   'user.settings.theme': 'dark',
//   'user.settings.language': 'ja'
// }

// userオブジェクトは作成されない！
const { user } = await chrome.storage.local.get(['user']);
console.log(user); // undefined
```

#### ✅ 正しい使い方

```javascript
// 既存のオブジェクトを取得
const { user = {} } = await chrome.storage.local.get(['user']);

// イミュータブルに更新
const updatedUser = {
  ...user,
  settings: {
    ...user.settings,
    theme: 'dark',
    language: 'ja'
  }
};

// オブジェクト全体を保存
await chrome.storage.local.set({ user: updatedUser });

// 正しく取得できる
const { user: savedUser } = await chrome.storage.local.get(['user']);
console.log(savedUser.settings.theme); // 'dark'
```

## 推奨パターン

### 1. State Manager クラスパターン

```javascript
class ExtensionStateManager {
  // 特定の状態を取得
  static async getState(key) {
    const result = await chrome.storage.local.get([key]);
    return result[key];
  }

  // 状態を更新（マージ）
  static async updateState(key, updates) {
    const current = await this.getState(key) || {};
    const updated = { ...current, ...updates };
    await chrome.storage.local.set({ [key]: updated });
    return updated;
  }

  // ネストしたプロパティを更新
  static async updateNested(key, path, value) {
    const current = await this.getState(key) || {};
    const updated = structuredClone(current);

    // パスに従ってネストしたプロパティを設定
    const keys = path.split('.');
    let target = updated;

    for (let i = 0; i < keys.length - 1; i++) {
      if (!target[keys[i]]) {
        target[keys[i]] = {};
      }
      target = target[keys[i]];
    }

    target[keys[keys.length - 1]] = value;
    await chrome.storage.local.set({ [key]: updated });
    return updated;
  }
}

// 使用例
await ExtensionStateManager.updateState('docsSync', {
  syncStatus: 'syncing',
  lastSyncAt: Date.now()
});

await ExtensionStateManager.updateNested(
  'docsSync',
  'offlineQueue.sizeBytes',
  1024
);
```

### 2. イベント監視パターン

```javascript
class StateObserver {
  constructor(keys) {
    this.keys = keys;
    this.listeners = new Map();
    this.initializeListener();
  }

  initializeListener() {
    chrome.storage.onChanged.addListener((changes, namespace) => {
      if (namespace !== 'local') return;

      // 監視対象のキーの変更のみ処理
      for (const key of this.keys) {
        if (changes[key]) {
          const callbacks = this.listeners.get(key) || [];
          callbacks.forEach(callback => {
            callback(changes[key].newValue, changes[key].oldValue);
          });
        }
      }
    });
  }

  // 特定のキーの変更を監視
  on(key, callback) {
    if (!this.keys.includes(key)) {
      throw new Error(`Key "${key}" is not being observed`);
    }

    if (!this.listeners.has(key)) {
      this.listeners.set(key, []);
    }

    this.listeners.get(key).push(callback);
  }

  // 監視を解除
  off(key, callback) {
    const callbacks = this.listeners.get(key) || [];
    const index = callbacks.indexOf(callback);
    if (index > -1) {
      callbacks.splice(index, 1);
    }
  }
}

// 使用例
const observer = new StateObserver(['docsSync', 'auth', 'recording']);

observer.on('docsSync', (newValue, oldValue) => {
  console.log('Docs sync state changed:', newValue);

  if (newValue?.syncStatus === 'error') {
    showErrorNotification(newValue.syncErrorMessage);
  }
});

observer.on('recording', (newValue) => {
  updateRecordingButton(newValue?.isActive);
});
```

### 3. トランザクション風の更新

```javascript
class StorageTransaction {
  constructor() {
    this.updates = {};
  }

  // 更新をバッファリング
  set(key, value) {
    this.updates[key] = value;
    return this;
  }

  // 既存の値とマージ
  async merge(key, partial) {
    const current = await chrome.storage.local.get([key]);
    this.updates[key] = { ...(current[key] || {}), ...partial };
    return this;
  }

  // 全ての更新を一括適用
  async commit() {
    if (Object.keys(this.updates).length > 0) {
      await chrome.storage.local.set(this.updates);
      this.updates = {};
    }
  }

  // 更新をキャンセル
  rollback() {
    this.updates = {};
  }
}

// 使用例
const transaction = new StorageTransaction();

await transaction
  .merge('docsSync', { syncStatus: 'syncing' })
  .merge('auth', { isRefreshing: true })
  .set('lastActivity', Date.now())
  .commit();
```

## パフォーマンス最適化

### 1. バッチ読み取り

```javascript
// ❌ 非効率: 複数回の読み取り
const docsSync = await chrome.storage.local.get(['docsSync']);
const auth = await chrome.storage.local.get(['auth']);
const recording = await chrome.storage.local.get(['recording']);

// ✅ 効率的: 一括読み取り
const state = await chrome.storage.local.get(['docsSync', 'auth', 'recording']);
const { docsSync, auth, recording } = state;
```

### 2. デバウンス処理

```javascript
class DebouncedStorage {
  constructor(delay = 100) {
    this.delay = delay;
    this.pending = new Map();
    this.timers = new Map();
  }

  async set(key, value) {
    // 既存のタイマーをクリア
    if (this.timers.has(key)) {
      clearTimeout(this.timers.get(key));
    }

    // 更新をペンディング
    this.pending.set(key, value);

    // デバウンス処理
    return new Promise((resolve) => {
      const timer = setTimeout(async () => {
        const updates = {};
        updates[key] = this.pending.get(key);

        await chrome.storage.local.set(updates);

        this.pending.delete(key);
        this.timers.delete(key);
        resolve();
      }, this.delay);

      this.timers.set(key, timer);
    });
  }
}

const debouncedStorage = new DebouncedStorage(100);

// 高頻度の更新もデバウンスされる
for (let i = 0; i < 100; i++) {
  await debouncedStorage.set('counter', i);
}
// 実際のストレージ書き込みは1回のみ
```

## 容量管理

### ストレージ使用量の監視

```javascript
class StorageQuotaManager {
  static async getUsage() {
    const bytesInUse = await chrome.storage.local.getBytesInUse();
    const quota = chrome.storage.local.QUOTA_BYTES; // 10MB for local

    return {
      used: bytesInUse,
      quota: quota,
      percentage: (bytesInUse / quota) * 100,
      remaining: quota - bytesInUse
    };
  }

  static async checkQuota(threshold = 80) {
    const usage = await this.getUsage();

    if (usage.percentage > threshold) {
      console.warn(`Storage usage at ${usage.percentage.toFixed(1)}%`);

      // 古いデータの削除などの処理
      await this.cleanup();
    }

    return usage;
  }

  static async cleanup() {
    // 一時的なデータやキャッシュを削除
    const allData = await chrome.storage.local.get(null);
    const keysToRemove = [];

    for (const [key, value] of Object.entries(allData)) {
      // 例: 1週間以上前のイベントを削除
      if (key.startsWith('_event_')) {
        const timestamp = value.timestamp || 0;
        if (Date.now() - timestamp > 7 * 24 * 60 * 60 * 1000) {
          keysToRemove.push(key);
        }
      }
    }

    if (keysToRemove.length > 0) {
      await chrome.storage.local.remove(keysToRemove);
      console.log(`Cleaned up ${keysToRemove.length} old entries`);
    }
  }
}

// 定期的な容量チェック
setInterval(() => {
  StorageQuotaManager.checkQuota(80);
}, 60 * 60 * 1000); // 1時間ごと
```

## テスト方法

### モックの作成

```javascript
// test/mocks/chrome-storage-mock.js
class ChromeStorageMock {
  constructor() {
    this.data = {};
    this.listeners = [];
  }

  async get(keys) {
    if (keys === null) return { ...this.data };
    if (typeof keys === 'string') keys = [keys];

    const result = {};
    for (const key of keys) {
      if (key in this.data) {
        result[key] = this.data[key];
      }
    }
    return result;
  }

  async set(items) {
    const changes = {};

    for (const [key, value] of Object.entries(items)) {
      const oldValue = this.data[key];
      this.data[key] = value;

      changes[key] = { oldValue, newValue: value };
    }

    // リスナーに通知
    this.listeners.forEach(listener => {
      listener(changes, 'local');
    });
  }

  onChanged = {
    addListener: (callback) => {
      this.listeners.push(callback);
    },
    removeListener: (callback) => {
      const index = this.listeners.indexOf(callback);
      if (index > -1) {
        this.listeners.splice(index, 1);
      }
    }
  };

  async clear() {
    this.data = {};
  }
}

// テストでの使用
global.chrome = {
  storage: {
    local: new ChromeStorageMock()
  }
};
```

## よくある間違いと対処法

### 1. 型安全性の欠如

```typescript
// ❌ 型チェックなし
const state = await chrome.storage.local.get(['docsSync']);
console.log(state.docsSync.syncStatus); // 実行時エラーの可能性

// ✅ 型安全な実装
interface StorageSchema {
  docsSync?: {
    syncStatus: 'idle' | 'syncing' | 'offline' | 'error';
    lastSyncAt: number;
  };
}

async function getTypedStorage<K extends keyof StorageSchema>(
  key: K
): Promise<StorageSchema[K] | undefined> {
  const result = await chrome.storage.local.get([key]);
  return result[key] as StorageSchema[K];
}

const docsSync = await getTypedStorage('docsSync');
if (docsSync) {
  console.log(docsSync.syncStatus); // 型安全
}
```

### 2. 初期値の設定忘れ

```javascript
// ❌ undefinedの可能性を考慮していない
const { settings } = await chrome.storage.local.get(['settings']);
const theme = settings.theme; // settings が undefined の場合エラー

// ✅ デフォルト値を設定
const { settings = { theme: 'light', language: 'en' } } =
  await chrome.storage.local.get(['settings']);
const theme = settings.theme; // 必ず値が存在
```

### 3. 非同期処理の誤り

```javascript
// ❌ awaitを忘れる
chrome.storage.local.set({ key: 'value' });
const { key } = chrome.storage.local.get(['key']); // Promise が返る

// ✅ 正しい非同期処理
await chrome.storage.local.set({ key: 'value' });
const { key } = await chrome.storage.local.get(['key']);
```

## まとめ

1. **ドット記法は使えない** - トップレベルキー単位でオブジェクト全体を更新する
2. **イミュータブルな更新** - 既存データを取得し、新しいオブジェクトを作成して保存
3. **バッチ処理** - 可能な限り読み書きをまとめる
4. **型安全性** - TypeScriptを使用して実行時エラーを防ぐ
5. **容量管理** - 定期的に使用量をチェックし、不要なデータを削除
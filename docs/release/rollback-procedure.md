# Rollback Procedure

Meeting Minutes Automator v0.2.0 からのロールバック手順。

## Rollback Triggers

以下の条件を満たす場合、ロールバックを検討します：

| トリガー | 閾値 | 測定方法 |
|----------|------|----------|
| 認証失敗率 | 50%以上 | Error logs |
| API成功率 | 80%未満 | API metrics |
| キュー保存失敗率 | 10%以上 | Storage errors |
| セキュリティ脆弱性 | Critical発見 | Security audit |

## Pre-Rollback Checklist

- [ ] 影響範囲の確認（ユーザー数、データ量）
- [ ] 根本原因の特定
- [ ] ロールバック後の影響評価
- [ ] 関係者への通知

## Rollback Steps

### Step 1: Chrome Extension Rollback

```bash
# 1. 現行バージョンをバックアップ
cp -r chrome-extension/dist chrome-extension/dist-v0.2.0-backup

# 2. 前バージョンのビルドをチェックアウト
git checkout v0.1.0 -- chrome-extension/

# 3. リビルド
cd chrome-extension
npm install
npm run build

# 4. Chrome拡張を再読み込み
# chrome://extensions → 更新ボタン
```

### Step 2: User Data Migration

```javascript
// chrome.storage.local のデータ移行（必要な場合）

// 新形式のデータをバックアップ
chrome.storage.local.get(null, (data) => {
  console.log('Backup:', JSON.stringify(data));
});

// 不要なキーを削除
chrome.storage.local.remove([
  'docs_sync_settings',
  'offline_queue',
  'ws_cached_port',
]);
```

### Step 3: Verify Rollback

1. Chrome拡張のバージョン確認
2. 基本機能（録音、文字起こし）の動作確認
3. エラーログの確認
4. ユーザー報告の監視

## Post-Rollback Actions

- [ ] インシデントレポートの作成
- [ ] 根本原因の修正
- [ ] 修正版のテスト
- [ ] 再リリース計画の策定

## Rollback Timeline

| 時間 | アクション |
|------|------------|
| T+0 | 問題検知 |
| T+15min | 影響評価、ロールバック決定 |
| T+30min | ロールバック実行 |
| T+45min | 検証完了 |
| T+1h | 関係者通知 |

## Emergency Contacts

| 役割 | 連絡先 |
|------|--------|
| 技術リード | TBD |
| インフラ担当 | TBD |
| プロダクトオーナー | TBD |

## Version History

| バージョン | リリース日 | 主な変更 |
|------------|------------|----------|
| v0.1.0 | 2025-10-10 | MVP0 Walking Skeleton |
| v0.2.0 | TBD | MVP2 Google Docs Sync |

---

*Last Updated: 2025-12-30*

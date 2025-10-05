# ADR-004: Token Storage Strategy

## Status
Accepted

## Context

meeting-minutes-docs-sync (MVP2) では、Google Docs APIへのアクセスに必要なOAuth 2.0トークン（アクセストークン、リフレッシュトークン）の保管場所を決定する必要がある。

### 設計原則との関係

プロジェクトの設計原則（`.kiro/steering/principles.md`）では、以下のように定義されている:

> **Principle 3: セキュリティ責任境界の原則**
> - **Tauri責務**: OAuth token保管（OS Keychain）、音声データ暗号化、SQLiteデータベース暗号化
> - **Chrome拡張責務**: 表示専用UI、ユーザーインタラクション、Google Docs表示操作
> - Chrome拡張のローカルストレージには設定情報のみ（機密情報禁止）

この原則に従えば、トークンはTauri側のOS Keychainに保管すべきである。しかし、MVP2の機能要件と技術制約により、Chrome拡張側でトークンを管理する設計を採用する。

### 技術的制約

1. **Doc ID検出の責務**: Content ScriptがGoogle Docsページのアクティブタブから`documentId`を検出する必要がある
2. **OAuth API制約**: `chrome.identity.launchWebAuthFlow()`はChrome拡張専用API。Tauri側で代替実装する場合、Native Appフロー（loopback localhost）が必要で、Google審査が厳格化する
3. **リアルタイムUX**: Google Docsページ上の同期状態インジケーターやエラー通知をContent Scriptで即座に表示する必要がある

### 代替案の検討

#### Option A: Tauri側でトークン管理（原則準拠）

**構成**:
- Tauri: OAuth 2.0フロー実行、トークン保管（OS Keychain/SQLCipher）、Google Docs API呼び出し
- Chrome拡張: Doc ID検出、UI表示のみ

**利点**:
- ✅ セキュリティ責任境界の原則に完全準拠
- ✅ トークン暗号化（OS Keychain、SQLCipher）
- ✅ 攻撃面の最小化（拡張からトークン完全除外）

**欠点**:
- ❌ Tauriは現在のタブURLを知る手段がない → Chrome拡張との双方向IPC実装が必須
- ❌ Native Appフロー（loopback localhost:8080等）でGoogle OAuth審査が厳格化
- ❌ Content ScriptからのリアルタイムUI更新が複雑化（Tauri → WebSocket → Offscreen Document → Content Script）
- ❌ 実装工数の大幅増加（MVP2スコープ超過）

#### Option B: Chrome拡張でトークン管理（現行設計）

**構成**:
- Chrome拡張: OAuth 2.0フロー実行、トークン保管（`chrome.storage.local`）、Google Docs API呼び出し
- Tauri: 文字起こしテキストの配信のみ（WebSocket経由）

**利点**:
- ✅ `chrome.identity.launchWebAuthFlow()`のネイティブサポート
- ✅ Content ScriptからのDoc ID検出が簡潔
- ✅ リアルタイムUI更新が容易
- ✅ 実装工数がMVP2スコープ内

**欠点**:
- ❌ セキュリティ責任境界の原則に違反
- ❌ `chrome.storage.local`への平文保存（暗号化なし）
- ❌ 拡張配布物に`client_secret`を含める必要がある（難読化で緩和）

## Decision

**MVP2では、Chrome拡張の`chrome.storage.local`にトークンを保存する（Option B）。**

ただし、以下のリスク緩和策を実装する:

### リスク緩和策

1. **短命トークン（30分有効期限）**
   ```typescript
   const expiresAt = Math.floor(Date.now() / 1000) + 1800; // 30分（1800秒）
   ```
   - デフォルトの1時間から30分に短縮
   - 漏洩時の悪用ウィンドウを50%削減

2. **Service Workerサスペンド時の自動削除**
   ```typescript
   chrome.runtime.onSuspend.addListener(async () => {
     await chrome.storage.local.remove(['accessToken', 'refreshToken']);
     logger.info('Tokens cleared on Service Worker suspend');
   });
   ```
   - MV3 Service Workerの5分タイムアウト時にトークンを自動削除
   - メモリ上の残留リスクを最小化

3. **client_secretの難読化**
   - ビルド時に環境変数からインジェクション
   - 拡張ソースコードへの平文記載を回避

4. **Non-Goalsで暗号化をMVP3に明示**
   - design.md:28 に記載済み
   - MVP3での移行パスを明確化

## Consequences

### Positive
- ✅ MVP2のスコープ内で実装可能
- ✅ `chrome.identity` APIのネイティブサポート活用
- ✅ リアルタイムUXの実現
- ✅ Doc ID検出ロジックの簡潔性維持

### Negative
- ⚠️ セキュリティ責任境界の原則に一時的に違反（MVP3で解消予定）
- ⚠️ トークン漏洩リスクの増加（緩和策で最小化）
- ⚠️ `client_secret`の配布物への含有（難読化で緩和）

### Neutral
- 🔄 MVP3での再設計が必須（Tauri OS Keychain移行）
- 🔄 移行時の互換性対応が必要

## Migration Path to MVP3

MVP3では、以下の段階的移行を実施する:

1. **Phase 1: Tauri側OAuth実装**
   - Native Appフロー（loopback localhost）の実装
   - OS Keychain統合（macOS: Keychain, Windows: Credential Manager, Linux: Secret Service）

2. **Phase 2: 並行稼働期間**
   - Chrome拡張とTauriの両方でトークン管理を並行稼働
   - Feature Flagで切り替え可能にする

3. **Phase 3: Chrome拡張からの削除**
   - Tauri側の安定稼働を確認後、Chrome拡張のトークン管理コードを削除
   - 拡張はDoc ID検出とUI表示のみに特化

## References

- `.kiro/steering/principles.md` - Principle 3: セキュリティ責任境界の原則
- `.kiro/specs/meeting-minutes-docs-sync/design.md` - Token Management セクション
- Chrome Identity API: https://developer.chrome.com/docs/extensions/reference/api/identity
- OAuth 2.0 Native App Best Practices: https://datatracker.ietf.org/doc/html/rfc8252

## Notes

本ADRは、セキュリティ原則と実装現実のトレードオフを明示的に文書化したものである。MVP2の時間的制約とスコープ制約の下での合理的な判断として、短命トークンとサスペンド時削除により、リスクを許容可能なレベルに緩和している。

MVP3での原則準拠への回帰は必須であり、プロジェクトロードマップに明記されている。
